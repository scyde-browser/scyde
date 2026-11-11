import Foundation
import ScydeCore
import SwiftUI
import WebKit

/// One tab: a WKWebView and what the island shows for it.
@MainActor
final class Tab: NSObject, ObservableObject, Identifiable {
    let id = UUID()
    let webView: WKWebView
    @Published var url: URL?
    @Published var title = ""
    @Published var loading = false
    @Published var progress = 0.0
    @Published var canGoBack = false
    @Published var canGoForward = false
    var typed = false
    private var observers: [NSKeyValueObservation] = []

    init(configuration: WKWebViewConfiguration) {
        webView = WKWebView(frame: .zero, configuration: configuration)
        webView.allowsBackForwardNavigationGestures = true
        webView.scrollView.contentInsetAdjustmentBehavior = .always
        #if DEBUG
        webView.isInspectable = true
        #endif
        super.init()
        observers = [
            webView.observe(\.url) { [weak self] wv, _ in Task { @MainActor in self?.url = wv.url } },
            webView.observe(\.title) { [weak self] wv, _ in Task { @MainActor in self?.title = wv.title ?? "" } },
            webView.observe(\.isLoading) { [weak self] wv, _ in Task { @MainActor in self?.loading = wv.isLoading } },
            webView.observe(\.estimatedProgress) { [weak self] wv, _ in Task { @MainActor in self?.progress = wv.estimatedProgress } },
            webView.observe(\.canGoBack) { [weak self] wv, _ in Task { @MainActor in self?.canGoBack = wv.canGoBack } },
            webView.observe(\.canGoForward) { [weak self] wv, _ in Task { @MainActor in self?.canGoForward = wv.canGoForward } },
        ]
    }

    var display: DisplayUrl? {
        guard let url, !InternalPages.isInternal(url) else { return nil }
        return omniboxDisplayUrl(url: url.absoluteString)
    }

    var editableURL: String {
        guard let url else { return "" }
        return url.absoluteString == InternalPages.newTab.absoluteString ? "" : url.absoluteString
    }
}

/// The browser: tabs, core stores (ScydeCore), Shield rule lists, internal pages.
@MainActor
final class Browser: NSObject, ObservableObject, WKNavigationDelegate, WKUIDelegate {
    @Published var tabs: [Tab] = []
    @Published var activeID: UUID?
    @Published var shieldReady = false
    @Published var filtersVersion = ""
    @Published var appearance: [String: String] = [:]

    let places: PlacesStore?
    let settings: SettingsStore
    private let settingsURL: URL
    private let internalPages = InternalPages()
    private let bridge = Bridge()
    private var ruleLists: [WKContentRuleList] = []
    private let processPool = WKProcessPool()

    var active: Tab? { tabs.first { $0.id == activeID } }

    override init() {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("SCYDE", isDirectory: true)
        try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
        settingsURL = support.appendingPathComponent("settings.json")
        places = try? PlacesStore.open(path: support.appendingPathComponent("places.sqlite").path)
        settings = SettingsStore.load(json: try? String(contentsOf: settingsURL, encoding: .utf8))
        super.init()
        bridge.browser = self
        refreshAppearance()
        loadShield()
    }

    // MARK: tabs

    func makeConfiguration() -> WKWebViewConfiguration {
        let c = WKWebViewConfiguration()
        c.processPool = processPool
        c.websiteDataStore = .default()
        c.setURLSchemeHandler(internalPages, forURLScheme: InternalPages.scheme)
        // Google sign-in needs the Safari suffix on iOS (PLAN §2.2, G1).
        let major = UIDevice.current.systemVersion.split(separator: ".").first.map(String.init) ?? "18"
        c.applicationNameForUserAgent = "Version/\(major).0 Mobile/15E148 Safari/604.1"
        c.preferences.javaScriptCanOpenWindowsAutomatically = false // popups need a gesture (PLAN §5.5)
        c.userContentController.addScriptMessageHandler(bridge, contentWorld: .page, name: "scyde")
        for list in ruleLists { c.userContentController.add(list) }
        return c
    }

    @discardableResult
    func newTab(_ url: URL? = nil, configuration: WKWebViewConfiguration? = nil, activate: Bool = true) -> Tab {
        let tab = Tab(configuration: configuration ?? makeConfiguration())
        tab.webView.navigationDelegate = self
        tab.webView.uiDelegate = self
        tabs.append(tab)
        if configuration == nil { tab.webView.load(URLRequest(url: url ?? InternalPages.newTab)) }
        if activate { activeID = tab.id }
        return tab
    }

    func close(_ tab: Tab) {
        guard let i = tabs.firstIndex(where: { $0.id == tab.id }) else { return }
        tab.webView.stopLoading()
        tabs.remove(at: i)
        if activeID == tab.id { activeID = tabs.isEmpty ? nil : tabs[min(i, tabs.count - 1)].id }
        if tabs.isEmpty { newTab() }
    }

    func closeAll() {
        tabs.removeAll()
        newTab()
    }

    // MARK: omnibox

    func go(_ input: String, inNewTab: Bool = false) {
        let target: URL?
        switch omniboxClassify(text: input) {
        case let .url(url): target = URL(string: url)
        case let .search(query): target = URL(string: omniboxSearchUrl(template: settings.searchTemplate(), query: query))
        case .empty: return
        }
        guard let target else { return }
        if inNewTab || active == nil {
            newTab(target)
        } else if let tab = active {
            tab.typed = true
            tab.webView.load(URLRequest(url: target))
        }
    }

    func suggestion(for text: String) -> String? {
        guard let first = try? places?.suggest(query: text, limit: 1).first, first.hostPrefix else { return nil }
        var host = first.url.replacingOccurrences(of: "https://", with: "").replacingOccurrences(of: "http://", with: "")
        if host.hasPrefix("www.") { host.removeFirst(4) }
        host = String(host.prefix { $0 != "/" && $0 != "?" && $0 != "#" })
        let typed = text.lowercased()
        return host.hasPrefix(typed) && host.count > typed.count ? String(host.dropFirst(typed.count)) : nil
    }

    func open(page: String) { newTab(URL(string: "scyde://\(page)/")) }

    // MARK: Shield (ADR-0006: compile once, cache, never on the main thread's critical path)

    private func loadShield() {
        guard let dir = Bundle.main.resourceURL?.appendingPathComponent("filters"),
              let data = try? Data(contentsOf: dir.appendingPathComponent("manifest.json")),
              let manifest = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let lists = manifest["ios"] as? [[String: Any]]
        else { return }
        let version = manifest["version"] as? String ?? "dev"
        filtersVersion = version
        let store = WKContentRuleListStore.default()!
        Task {
            var compiled: [WKContentRuleList] = []
            for entry in lists {
                guard let path = entry["path"] as? String else { continue }
                let id = "scyde-\(version)-\((path as NSString).lastPathComponent)"
                if let cached = try? await store.contentRuleList(forIdentifier: id) {
                    compiled.append(cached)
                    continue
                }
                guard let json = try? String(contentsOf: dir.appendingPathComponent(path), encoding: .utf8),
                      let list = try? await store.compileContentRuleList(forIdentifier: id, encodedContentRuleList: json)
                else { continue }
                compiled.append(list)
            }
            ruleLists = compiled
            for tab in tabs { for list in compiled { tab.webView.configuration.userContentController.add(list) } }
            shieldReady = !compiled.isEmpty
            // Drop lists from older bundles.
            if let ids = try? await store.availableIdentifiers() {
                for old in ids where old.hasPrefix("scyde-") && !old.hasPrefix("scyde-\(version)-") {
                    try? await store.removeContentRuleList(forIdentifier: old)
                }
            }
        }
    }

    // MARK: settings

    private func refreshAppearance() {
        var a: [String: String] = [:]
        for key in ["theme", "accent", "density", "reduce_motion"] {
            if let raw = settings.get(path: "appearance.\(key)"), let v = try? JSONDecoder().decode(String.self, from: Data(raw.utf8)) {
                a[key] = v
            }
        }
        appearance = a
    }

    private func saveSettings() {
        try? settings.toJson().write(to: settingsURL, atomically: true, encoding: .utf8)
        refreshAppearance()
    }

    // MARK: navigation delegate

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard let tab = tabs.first(where: { $0.webView === webView }), let url = webView.url,
              url.scheme == "https" || url.scheme == "http" else { return }
        let kind: VisitKind = tab.typed ? .typed : .link
        tab.typed = false
        _ = try? places?.recordVisit(url: url.absoluteString, title: webView.title, kind: kind, at: Int64(Date().timeIntervalSince1970))
    }

    /// A popup that passed WebKit's gesture check opens as a tab with the given configuration,
    /// so `window.opener` keeps working (OAuth).
    func webView(
        _ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
        for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures
    ) -> WKWebView? {
        newTab(configuration: configuration).webView
    }

    func webViewDidClose(_ webView: WKWebView) {
        if let tab = tabs.first(where: { $0.webView === webView }) { close(tab) }
    }

    // MARK: bridge methods (same contract as web/packages/bridge/src/api.ts)

    func handle(_ method: String, _ p: [String: Any]) throws -> Any {
        func visits(_ entries: [HistoryEntry]) -> [[String: Any]] {
            entries.map { ["url": $0.url, "title": $0.title, "lastVisit": $0.lastVisit, "visitCount": $0.visitCount] }
        }
        switch method {
        case "newtab.state":
            let recent = (try? places?.history(query: nil, limit: 6, offset: 0)) ?? []
            let top = (try? places?.topSites(limit: 8)) ?? []
            return [
                "shield": ["blocked": 0, "bytesSaved": 0, "msSaved": 0],
                "recent": visits(recent),
                "topSites": top.map { ["url": $0.url, "title": $0.title, "pinned": $0.pinned] },
                "storiesEnabled": settings.get(path: "newtab.top_stories") == "true",
            ]
        case "navigate":
            go(p["input"] as? String ?? "", inNewTab: p["newTab"] as? Bool ?? false)
            return ["ok": true]
        case "settings.schema":
            return ["schema": (try? JSONSerialization.jsonObject(with: Data(settingsSchema().utf8))) ?? NSNull()]
        case "settings.get":
            let json = (try? JSONSerialization.jsonObject(with: Data(settings.toJson().utf8))) ?? [:]
            return ["settings": json, "issues": settings.loadIssues().map { ["path": $0.path, "message": $0.message] }]
        case "settings.set":
            let value = p["value"] ?? NSNull()
            let data = try JSONSerialization.data(withJSONObject: [value], options: [.fragmentsAllowed])
            let text = String(decoding: data, as: UTF8.self).dropFirst().dropLast() // [v] → v
            do {
                try settings.set(path: p["path"] as? String ?? "", jsonValue: String(text))
                saveSettings()
                return ["ok": true]
            } catch {
                return ["ok": false, "error": "\(error)"]
            }
        case "settings.reset":
            try settings.reset(path: p["path"] as? String ?? "")
            saveSettings()
            return ["ok": true]
        case "history.query":
            let entries = try places?.history(
                query: p["query"] as? String, limit: UInt32(p["limit"] as? Int ?? 50), offset: UInt32(p["offset"] as? Int ?? 0))
            return ["entries": visits(entries ?? [])]
        case "history.delete":
            try places?.deleteUrl(url: p["url"] as? String ?? "")
            return ["ok": true]
        case "history.clear":
            try places?.clearHistory()
            return ["ok": true]
        case "bookmarks.children":
            let items = try places?.children(parent: Int64(p["parent"] as? Int ?? Int(bookmarksBarId()))) ?? []
            return ["items": items.map { ["id": $0.id, "parent": $0.parent, "isFolder": $0.isFolder, "url": $0.url ?? NSNull(), "title": $0.title] }]
        case "bookmarks.remove":
            try places?.removeBookmark(id: Int64(p["id"] as? Int ?? 0))
            return ["ok": true]
        case "shield.summary":
            let zero = ["blocked": 0, "bytesSaved": 0, "msSaved": 0]
            return ["today": zero, "total": zero, "filtersVersion": filtersVersion, "sites": []]
        case "shield.setSite":
            return ["ok": false, "error": "Per-site Shield switches arrive in a later update on iOS."]
        default:
            return ["ok": false, "error": "method not allowed"]
        }
    }
}
