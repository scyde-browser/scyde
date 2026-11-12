import Combine
import Foundation
import ScydeCore
import SwiftUI
import WebKit

/// A Space: a named group of tabs with its own Aura colour (PLAN Phase 6 #8).
struct Space: Codable, Identifiable, Equatable {
    var id: String
    var name: String
    /// An accent preset name (ember, sky, periwinkle, lavender, mint, onyx).
    var color: String

    static let colors = ["ember", "sky", "mint", "lavender", "periwinkle", "onyx"]
}

/// One tab: a WKWebView and what the island shows for it.
@MainActor
final class Tab: NSObject, ObservableObject, Identifiable {
    let id: UUID
    /// Replaced by a fresh, empty web view while the tab sleeps (that frees its memory).
    private(set) var webView: WKWebView
    /// Private tabs use a non-persistent data store and leave no history (ADR-0013).
    let isPrivate: Bool
    var spaceID: String
    @Published var url: URL?
    @Published var title = ""
    @Published var loading = false
    @Published var progress = 0.0
    @Published var canGoBack = false
    @Published var canGoForward = false
    /// The page's `theme-color` (Aura tint, DESIGN.md §9).
    @Published var themeColor: UIColor?
    /// Full screen while reading (owner, 2026-11-12): scrolling down hides the island and the status
    /// bar; scrolling up, reaching the end of the page or a tap at the bottom edge brings them back.
    @Published var immersive = false
    @Published var bookmarked = false
    /// Pinned tabs come first, survive "Close all" and are restored with the session.
    @Published var pinned = false
    /// Asleep: no page in memory; it comes back (with its history) when the tab is opened.
    @Published var sleeping = false
    var lastActive = Date()
    var desktop = false
    var typed = false
    private var lastOffset: CGFloat = 0
    private var observers: [NSKeyValueObservation] = []
    /// What a sleeping or restored tab loads when it wakes.
    private var savedState: Any?
    private var pendingURL: URL?

    init(id: UUID = UUID(), configuration: WKWebViewConfiguration, isPrivate: Bool, spaceID: String) {
        self.id = id
        self.isPrivate = isPrivate
        self.spaceID = spaceID
        webView = Tab.makeWebView(configuration)
        super.init()
        observe()
    }

    private static func makeWebView(_ configuration: WKWebViewConfiguration) -> WKWebView {
        let wv = WKWebView(frame: .zero, configuration: configuration)
        wv.allowsBackForwardNavigationGestures = true
        wv.allowsLinkPreview = true
        wv.isFindInteractionEnabled = true
        wv.scrollView.contentInsetAdjustmentBehavior = .always
        #if DEBUG
        wv.isInspectable = true
        #endif
        return wv
    }

    private func observe() {
        observers = [
            webView.observe(\.url) { [weak self] wv, _ in Task { @MainActor in if let u = wv.url { self?.url = u } } },
            webView.observe(\.title) { [weak self] wv, _ in Task { @MainActor in if let t = wv.title, !t.isEmpty { self?.title = t } } },
            webView.observe(\.isLoading) { [weak self] wv, _ in Task { @MainActor in self?.loading = wv.isLoading } },
            webView.observe(\.estimatedProgress) { [weak self] wv, _ in Task { @MainActor in self?.progress = wv.estimatedProgress } },
            webView.observe(\.canGoBack) { [weak self] wv, _ in Task { @MainActor in self?.canGoBack = wv.canGoBack } },
            webView.observe(\.canGoForward) { [weak self] wv, _ in Task { @MainActor in self?.canGoForward = wv.canGoForward } },
            webView.observe(\.themeColor) { [weak self] wv, _ in Task { @MainActor in self?.themeColor = wv.themeColor } },
            webView.scrollView.observe(\.contentOffset) { [weak self] sv, _ in
                let y = sv.contentOffset.y + sv.adjustedContentInset.top
                let end = sv.contentSize.height + sv.adjustedContentInset.top + sv.adjustedContentInset.bottom - sv.bounds.height
                let atEnd = sv.contentSize.height > sv.bounds.height && y >= end - ScydeSpace.s8
                Task { @MainActor in self?.scrolled(to: y, atEnd: atEnd) }
            },
        ]
    }

    /// A restored tab: shows its URL and title, loads only when it is opened.
    func restore(url: URL, title: String) {
        self.url = url
        self.title = title
        pendingURL = url
        sleeping = true
    }

    /// Frees the page: keeps its back/forward history and swaps in an empty web view.
    func sleep(fresh configuration: WKWebViewConfiguration, delegate: (WKNavigationDelegate & WKUIDelegate)?) {
        guard !sleeping else { return }
        savedState = webView.interactionState
        pendingURL = url
        webView.stopLoading()
        webView = Tab.makeWebView(configuration)
        webView.navigationDelegate = delegate
        webView.uiDelegate = delegate
        observe()
        loading = false
        sleeping = true
    }

    func wake() {
        guard sleeping else { return }
        sleeping = false
        if let state = savedState {
            webView.interactionState = state
        } else if let url = pendingURL {
            webView.load(URLRequest(url: url))
        }
        savedState = nil
        pendingURL = nil
    }

    private func scrolled(to y: CGFloat, atEnd: Bool) {
        let delta = y - lastOffset
        if y <= ScydeSpace.s8 || atEnd {
            if immersive { immersive = false }
        } else if delta > ScydeSpace.s12 && y > ScydeSpace.s64 {
            if !immersive { immersive = true }
        } else if delta < -ScydeSpace.s12 {
            if immersive { immersive = false }
        } else {
            return // small moves do not move the reference point, so slow scrolls still count
        }
        lastOffset = y
    }

    var display: DisplayUrl? {
        guard let url, !InternalPages.isInternal(url) else { return nil }
        return omniboxDisplayUrl(url: url.absoluteString)
    }

    /// The registrable domain (eTLD+1) Shield switches are keyed by.
    var site: String? { display?.domain }

    var isWeb: Bool { url?.scheme == "https" || url?.scheme == "http" }

    var editableURL: String {
        guard let url else { return "" }
        return url.absoluteString == InternalPages.newTab.absoluteString ? "" : url.absoluteString
    }
}

/// The browser: tabs, core stores (ScydeCore), Shield rule lists, internal pages.
@MainActor
final class Browser: NSObject, ObservableObject, WKNavigationDelegate, WKUIDelegate {
    @Published var tabs: [Tab] = []
    @Published var activeID: UUID? {
        didSet {
            guard let tab = active else { return }
            tab.lastActive = Date()
            tab.wake()
            if !tab.isPrivate { spaceID = tab.spaceID }
            saveSession()
        }
    }
    /// Spaces (tab groups with their own colour); private tabs belong to none.
    @Published var spaces: [Space] = [Space(id: "personal", name: String(localized: "space.default"), color: "ember")]
    @Published var spaceID = "personal"
    @Published var shieldReady = false
    @Published var filtersVersion = ""
    @Published var appearance: [String: String] = [:]
    /// Settings, Library and Shield open as a sheet over the tabs, not as tabs (closed with Done or a swipe down).
    @Published var sheetPage: String?
    /// Sites (eTLD+1) where the owner turned Shield off (`shield.disabled_sites`).
    @Published var disabledSites: Set<String> = []
    /// A short message that rises out of the island (DESIGN.md §8 Toast).
    @Published var toast: String?

    let places: PlacesStore?
    let settings: SettingsStore
    private let settingsURL: URL
    private let internalPages = InternalPages()
    private let bridge = Bridge()
    private var ruleLists: [WKContentRuleList] = []
    private let processPool = WKProcessPool()
    /// One in-memory store for all private tabs; replaced when the last private tab closes.
    private var privateStore = WKWebsiteDataStore.nonPersistent()
    private var toastTask: Task<Void, Never>?
    private let sessionURL: URL
    private var sleepTimer: Timer?
    private var restoring = false
    /// Tab changes (URL, title, progress, back/forward) re-render the island.
    private var tabChanges: [UUID: AnyCancellable] = [:]
    private lazy var scrollGuard: WKUserScript? = {
        guard let url = Bundle.main.url(forResource: "scroll-guard", withExtension: "js", subdirectory: "filters"),
              let source = try? String(contentsOf: url, encoding: .utf8) else { return nil }
        return WKUserScript(source: source, injectionTime: .atDocumentEnd, forMainFrameOnly: true)
    }()

    var active: Tab? { tabs.first { $0.id == activeID } }

    override init() {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("SCYDE", isDirectory: true)
        try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
        settingsURL = support.appendingPathComponent("settings.json")
        sessionURL = support.appendingPathComponent("session.json")
        places = try? PlacesStore.open(path: support.appendingPathComponent("places.sqlite").path)
        settings = SettingsStore.load(json: try? String(contentsOf: settingsURL, encoding: .utf8))
        super.init()
        bridge.browser = self
        refreshAppearance()
        loadShield()
        // Sleeping tabs (PLAN Phase 6 #3): once a minute, tabs unused for `tabs.sleep_after_minutes`.
        sleepTimer = Timer.scheduledTimer(withTimeInterval: 60, repeats: true) { [weak self] _ in
            Task { @MainActor in self?.sleepIdleTabs() }
        }
    }

    // MARK: tabs

    func makeConfiguration(isPrivate: Bool = false) -> WKWebViewConfiguration {
        let c = WKWebViewConfiguration()
        c.processPool = processPool
        c.websiteDataStore = isPrivate ? privateStore : .default()
        c.setURLSchemeHandler(internalPages, forURLScheme: InternalPages.scheme)
        // Google sign-in needs the Safari suffix on iOS (PLAN §2.2, G1).
        let major = UIDevice.current.systemVersion.split(separator: ".").first.map(String.init) ?? "18"
        c.applicationNameForUserAgent = "Version/\(major).0 Mobile/15E148 Safari/604.1"
        c.preferences.javaScriptCanOpenWindowsAutomatically = false // popups need a gesture (PLAN §5.5)
        c.userContentController.addScriptMessageHandler(bridge, contentWorld: .page, name: "scyde")
        for list in ruleLists { c.userContentController.add(list) }
        if let scrollGuard { c.userContentController.addUserScript(scrollGuard) }
        return c
    }

    @discardableResult
    func newTab(_ url: URL? = nil, configuration: WKWebViewConfiguration? = nil, activate: Bool = true, isPrivate: Bool = false) -> Tab {
        let tab = Tab(configuration: configuration ?? makeConfiguration(isPrivate: isPrivate), isPrivate: isPrivate, spaceID: spaceID)
        tab.webView.navigationDelegate = self
        tab.webView.uiDelegate = self
        tabs.append(tab)
        tabChanges[tab.id] = tab.objectWillChange.sink { [weak self, weak tab] _ in
            // Only the visible tab drives the island; background tabs do not re-render it.
            if let self, let tab, tab.id == self.activeID { self.objectWillChange.send() }
        }
        if configuration == nil { tab.webView.load(URLRequest(url: url ?? InternalPages.newTab)) }
        if activate { activeID = tab.id } else { saveSession() }
        return tab
    }

    func close(_ tab: Tab) {
        guard let i = tabs.firstIndex(where: { $0.id == tab.id }) else { return }
        tab.webView.stopLoading()
        tabChanges[tab.id] = nil
        tabs.remove(at: i)
        if activeID == tab.id {
            // Stay in the same mode and Space when there is a tab left in it.
            let same = tabs.filter { $0.isPrivate == tab.isPrivate && ($0.isPrivate || $0.spaceID == tab.spaceID) }
            let fallback = visibleTabs(isPrivate: false)
            if let next = (same.isEmpty ? fallback : same).max(by: { $0.lastActive < $1.lastActive }) { activeID = next.id } else { activeID = nil }
        }
        if tab.isPrivate && !tabs.contains(where: \.isPrivate) { privateStore = .nonPersistent() }
        if active == nil { newTab(isPrivate: false) }
        saveSession()
    }

    /// Closes the private tabs, or the unpinned tabs of the current Space.
    func closeAll(isPrivate: Bool) {
        let doomed = tabs.filter { isPrivate ? $0.isPrivate : (!$0.isPrivate && $0.spaceID == spaceID && !$0.pinned) }
        for t in doomed { tabChanges[t.id] = nil }
        tabs.removeAll { t in doomed.contains { $0 === t } }
        if isPrivate { privateStore = .nonPersistent() }
        if active == nil {
            if let pinned = tabs.first(where: { !$0.isPrivate && $0.spaceID == spaceID }) { activeID = pinned.id } else { newTab() }
        }
        saveSession()
    }

    /// The tabs the island counts and swipes through: the active mode and, for normal tabs, the Space.
    func visibleTabs(isPrivate: Bool) -> [Tab] {
        let list = tabs.filter { $0.isPrivate == isPrivate && (isPrivate || $0.spaceID == spaceID) }
        return list.filter(\.pinned) + list.filter { !$0.pinned }
    }

    func togglePin(_ tab: Tab) {
        tab.pinned.toggle()
        objectWillChange.send()
        saveSession()
    }

    // MARK: Spaces

    var currentSpace: Space { spaces.first { $0.id == spaceID } ?? spaces[0] }

    func switchSpace(_ id: String) {
        guard spaces.contains(where: { $0.id == id }) else { return }
        spaceID = id
        let inSpace = tabs.filter { !$0.isPrivate && $0.spaceID == id }
        if let last = inSpace.max(by: { $0.lastActive < $1.lastActive }) { activeID = last.id } else { newTab() }
        UIImpactFeedbackGenerator(style: .light).impactOccurred()
    }

    func addSpace(name: String) {
        let used = Set(spaces.map(\.color))
        let color = Space.colors.first { !used.contains($0) } ?? Space.colors[spaces.count % Space.colors.count]
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        let space = Space(id: UUID().uuidString, name: trimmed.isEmpty ? String(localized: "space.new") : trimmed, color: color)
        spaces.append(space)
        switchSpace(space.id)
    }

    func renameSpace(_ id: String, to name: String) {
        guard let i = spaces.firstIndex(where: { $0.id == id }), !name.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        spaces[i].name = name.trimmingCharacters(in: .whitespaces)
        saveSession()
    }

    func recolorSpace(_ id: String, to color: String) {
        guard let i = spaces.firstIndex(where: { $0.id == id }) else { return }
        spaces[i].color = color
        saveSession()
    }

    /// Deletes a Space; its tabs move to the first remaining Space.
    func deleteSpace(_ id: String) {
        guard spaces.count > 1 else { return }
        spaces.removeAll { $0.id == id }
        let target = spaces[0].id
        for t in tabs where t.spaceID == id { t.spaceID = target }
        if spaceID == id { switchSpace(target) } else { saveSession() }
    }

    func move(_ tab: Tab, to space: String) {
        tab.spaceID = space
        if tab.id == activeID { spaceID = space }
        objectWillChange.send()
        saveSession()
    }

    // MARK: session (restore on launch, `startup.open`)

    private struct SessionFile: Codable {
        struct SavedTab: Codable {
            var id: UUID
            var url: String
            var title: String
            var pinned: Bool
            var space: String
        }

        var version = 1
        var active: UUID?
        var space: String
        var spaces: [Space]
        var tabs: [SavedTab]
    }

    /// Saves the open (non-private) tabs and Spaces; private tabs are never written anywhere.
    func saveSession() {
        guard !restoring else { return }
        let saved = tabs.filter { !$0.isPrivate }.compactMap { t -> SessionFile.SavedTab? in
            guard let url = t.url else { return nil }
            return SessionFile.SavedTab(id: t.id, url: url.absoluteString, title: t.title, pinned: t.pinned, space: t.spaceID)
        }
        let file = SessionFile(active: active?.isPrivate == false ? activeID : nil, space: spaceID, spaces: spaces, tabs: saved)
        if let data = try? JSONEncoder().encode(file) { try? data.write(to: sessionURL, options: .atomic) }
    }

    /// Restores the last session as sleeping tabs (only the active one loads). Returns false when
    /// there is nothing to restore or the owner chose to start with a New Tab.
    func restoreSession() -> Bool {
        let mode = settings.get(path: "startup.open").flatMap { try? JSONDecoder().decode(String.self, from: Data($0.utf8)) } ?? "restore"
        guard let data = try? Data(contentsOf: sessionURL), let file = try? JSONDecoder().decode(SessionFile.self, from: data) else { return false }
        if !file.spaces.isEmpty { spaces = file.spaces }
        spaceID = spaces.contains { $0.id == file.space } ? file.space : spaces[0].id
        let keep = mode == "restore" ? file.tabs : file.tabs.filter(\.pinned)
        guard !keep.isEmpty else { return false }
        restoring = true
        for saved in keep {
            guard let url = URL(string: saved.url) else { continue }
            let tab = Tab(id: saved.id, configuration: makeConfiguration(), isPrivate: false,
                          spaceID: spaces.contains { $0.id == saved.space } ? saved.space : spaces[0].id)
            tab.webView.navigationDelegate = self
            tab.webView.uiDelegate = self
            tab.pinned = saved.pinned
            tab.restore(url: url, title: saved.title)
            tabs.append(tab)
            tabChanges[tab.id] = tab.objectWillChange.sink { [weak self, weak tab] _ in
                if let self, let tab, tab.id == self.activeID { self.objectWillChange.send() }
            }
        }
        restoring = false
        let first = tabs.first { $0.id == file.active } ?? tabs.first { $0.spaceID == spaceID } ?? tabs.first
        if mode != "restore" || first == nil { newTab() } else { activeID = first?.id }
        return !tabs.isEmpty
    }

    // MARK: sleeping tabs

    private func sleepIdleTabs() {
        let minutes = settings.get(path: "tabs.sleep_after_minutes").flatMap(Int.init) ?? 15
        guard minutes > 0 else { return }
        let limit = Date().addingTimeInterval(-Double(minutes) * 60)
        for t in tabs where t.id != activeID && !t.sleeping && t.lastActive < limit {
            t.sleep(fresh: makeConfiguration(isPrivate: t.isPrivate), delegate: self)
        }
    }

    /// Moves to the neighbouring tab of the same mode; past the last one opens a new tab (like Safari).
    func switchTab(by delta: Int) {
        guard let current = active else { return }
        let list = visibleTabs(isPrivate: current.isPrivate)
        guard let i = list.firstIndex(where: { $0.id == current.id }) else { return }
        let j = i + delta
        if j >= list.count { newTab(isPrivate: current.isPrivate) } else if j >= 0 { activeID = list[j].id }
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
            newTab(target, isPrivate: active?.isPrivate ?? false)
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

    func open(page: String) { sheetPage = page }

    /// First run: the welcome sheet (theme, search engine, Shield) until it is finished or skipped.
    var needsOnboarding: Bool { settings.get(path: "onboarding.completed") != "true" }

    // MARK: page tools (control center)

    func reloadOrStop() {
        guard let wv = active?.webView else { return }
        if wv.isLoading { wv.stopLoading() } else { wv.reload() }
    }

    func toggleBookmark() {
        guard let tab = active, tab.isWeb, let url = tab.url?.absoluteString, let places else { return }
        if tab.bookmarked {
            _ = try? places.unbookmarkUrl(url: url)
            show(String(localized: "toast.unbookmarked"))
        } else {
            _ = try? places.addBookmark(parent: bookmarksBarId(), url: url, title: tab.title, now: Int64(Date().timeIntervalSince1970))
            show(String(localized: "toast.bookmarked"))
        }
        tab.bookmarked = (try? places.isBookmarked(url: url)) ?? false
    }

    func toggleDesktop() {
        guard let tab = active else { return }
        tab.desktop.toggle()
        tab.webView.reload()
    }

    func find() {
        active?.webView.findInteraction?.presentFindNavigator(showingReplace: false)
    }

    func copyLink() {
        guard let url = active?.url, active?.isWeb == true else { return }
        UIPasteboard.general.url = url
        show(String(localized: "toast.link_copied"))
    }

    func show(_ message: String) {
        toast = message
        toastTask?.cancel()
        toastTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: UInt64(ScydeMotion.toastDismiss * 1_000_000_000))
            if !Task.isCancelled { self?.toast = nil }
        }
    }

    // MARK: Shield (ADR-0006: compile once, cache, never on the main thread's critical path)

    func shieldOn(for tab: Tab?) -> Bool {
        guard let site = tab?.site else { return true }
        return !disabledSites.contains(site)
    }

    /// Turns Shield off or on for a site, saves it and reloads tabs showing that site.
    func setShield(_ on: Bool, site: String) {
        guard !site.isEmpty else { return }
        if on { disabledSites.remove(site) } else { disabledSites.insert(site) }
        let list = (try? JSONSerialization.data(withJSONObject: disabledSites.sorted())).map { String(decoding: $0, as: UTF8.self) } ?? "[]"
        try? settings.set(path: "shield.disabled_sites", jsonValue: list)
        try? settings.toJson().write(to: settingsURL, atomically: true, encoding: .utf8)
        for tab in tabs where tab.site == site {
            applyShield(to: tab.webView, on: on)
            tab.webView.reload()
        }
        show(String(format: String(localized: on ? "toast.shield_on" : "toast.shield_off"), site))
    }

    /// Rule lists are per web view (each tab has its own content controller), so a site with Shield
    /// off simply loads without them.
    private func applyShield(to webView: WKWebView, on: Bool) {
        let ucc = webView.configuration.userContentController
        ucc.removeAllContentRuleLists()
        if on { for list in ruleLists { ucc.add(list) } }
    }

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
            for tab in tabs { applyShield(to: tab.webView, on: shieldOn(for: tab)) }
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
        if let raw = settings.get(path: "shield.disabled_sites"), let v = try? JSONDecoder().decode([String].self, from: Data(raw.utf8)) {
            disabledSites = Set(v)
        }
    }

    private func saveSettings() {
        try? settings.toJson().write(to: settingsURL, atomically: true, encoding: .utf8)
        refreshAppearance()
    }

    // MARK: navigation delegate

    func tab(for webView: WKWebView) -> Tab? { tabs.first { $0.webView === webView } }

    /// De-AMP, per-site Shield and the desktop-site switch, decided before each load.
    func webView(
        _ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, preferences: WKWebpagePreferences
    ) async -> (WKNavigationActionPolicy, WKWebpagePreferences) {
        let tab = tab(for: webView)
        preferences.preferredContentMode = tab?.desktop == true ? .desktop : .mobile
        guard navigationAction.targetFrame?.isMainFrame != false, let url = navigationAction.request.url else {
            return (.allow, preferences)
        }
        if let target = omniboxDeamp(url: url.absoluteString).flatMap(URL.init(string:)) {
            webView.load(URLRequest(url: target))
            return (.cancel, preferences)
        }
        if url.scheme == "https" || url.scheme == "http", let site = omniboxDisplayUrl(url: url.absoluteString)?.domain {
            applyShield(to: webView, on: !disabledSites.contains(site))
        }
        return (.allow, preferences)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard let tab = tab(for: webView), let url = webView.url else { return }
        if !tab.isPrivate { saveSession() }
        tab.bookmarked = tab.isWeb && ((try? places?.isBookmarked(url: url.absoluteString)) ?? false)
        guard tab.isWeb, !tab.isPrivate else { return } // private tabs leave no history
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
        newTab(configuration: configuration, isPrivate: tab(for: webView)?.isPrivate ?? false).webView
    }

    func webViewDidClose(_ webView: WKWebView) {
        if let tab = tab(for: webView) { close(tab) }
    }

    // MARK: Peek (long-press a link: a live preview and actions, PLAN Phase 6 #6)

    func webView(_ webView: WKWebView, contextMenuConfigurationFor elementInfo: WKContextMenuElementInfo) async -> UIContextMenuConfiguration? {
        guard let url = elementInfo.linkURL, url.scheme == "https" || url.scheme == "http" else { return nil }
        let isPrivate = tab(for: webView)?.isPrivate ?? false
        let previewConfig = makeConfiguration(isPrivate: isPrivate)
        return UIContextMenuConfiguration(
            identifier: url as NSURL,
            previewProvider: { PeekController(url: url, configuration: previewConfig) },
            actionProvider: { [weak self] _ in
                guard let self else { return nil }
                return UIMenu(children: [
                    UIAction(title: String(localized: "peek.new_tab"), image: UIImage(systemName: "plus.square.on.square")) { _ in
                        self.newTab(url, isPrivate: isPrivate)
                    },
                    UIAction(title: String(localized: "peek.background"), image: UIImage(systemName: "square.on.square")) { _ in
                        self.newTab(url, activate: false, isPrivate: isPrivate)
                        self.show(String(localized: "toast.background"))
                    },
                    UIAction(title: String(localized: "peek.private"), image: UIImage(systemName: "eye.slash")) { _ in
                        self.newTab(url, isPrivate: true)
                    },
                    UIAction(title: String(localized: "peek.copy"), image: UIImage(systemName: "doc.on.doc")) { _ in
                        UIPasteboard.general.url = url
                        self.show(String(localized: "toast.link_copied"))
                    },
                ])
            }
        )
    }

    /// Tapping the preview opens the link in the current tab.
    func webView(
        _ webView: WKWebView, contextMenuForElement elementInfo: WKContextMenuElementInfo,
        willCommitWithAnimator animator: UIContextMenuInteractionCommitAnimating
    ) {
        guard let url = elementInfo.linkURL else { return }
        animator.addCompletion { webView.load(URLRequest(url: url)) }
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
            sheetPage = nil
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
                let path = p["path"] as? String ?? ""
                try settings.set(path: path, jsonValue: String(text))
                saveSettings()
                // The welcome page is finished: close its sheet.
                if path == "onboarding.completed", value as? Bool == true { sheetPage = nil }
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
            let sites = disabledSites.sorted().map { ["host": $0, "enabled": false, "blocked": 0] as [String: Any] }
            return ["today": zero, "total": zero, "filtersVersion": filtersVersion, "sites": sites]
        case "shield.setSite":
            setShield(p["enabled"] as? Bool ?? true, site: p["host"] as? String ?? "")
            return ["ok": true]
        default:
            return ["ok": false, "error": "method not allowed"]
        }
    }
}
