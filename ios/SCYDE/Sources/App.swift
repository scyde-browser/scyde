import SwiftUI
import UIKit
import WebKit

@main
struct ScydeApp: App {
    @StateObject private var browser = Browser()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(browser)
                .onAppear {
                    if browser.tabs.isEmpty {
                        // `-open <url>` (CI screenshots / qa fixtures), otherwise New Tab.
                        let args = CommandLine.arguments
                        if let i = args.firstIndex(of: "-open"), i + 1 < args.count, let url = URL(string: args[i + 1]) {
                            browser.newTab(url)
                        } else {
                            browser.newTab()
                        }
                        // `-page settings` opens an internal page as a sheet (CI screenshots).
                        if let i = args.firstIndex(of: "-page"), i + 1 < args.count {
                            browser.open(page: args[i + 1])
                        } else if browser.needsOnboarding && !args.contains("-open") && !args.contains("-skipWelcome") {
                            browser.open(page: "welcome")
                        }
                    }
                }
                .onOpenURL { url in browser.newTab(url) }
        }
    }
}

/// Island palette from the generated tokens (rule 4), resolved for the current scheme and accent.
struct Palette {
    let colors: ScydeColors
    let accent: ScydeAccent
    let dark: Bool

    init(scheme: ColorScheme, accent name: String?) {
        let dark = scheme == .dark
        self.dark = dark
        colors = dark ? .dark : .light
        switch name {
        case "sky": accent = dark ? ScydeAccents.skyDark : ScydeAccents.skyLight
        case "periwinkle": accent = dark ? ScydeAccents.periwinkleDark : ScydeAccents.periwinkleLight
        case "lavender": accent = dark ? ScydeAccents.lavenderDark : ScydeAccents.lavenderLight
        case "mint": accent = dark ? ScydeAccents.mintDark : ScydeAccents.mintLight
        case "onyx": accent = dark ? ScydeAccents.onyxDark : ScydeAccents.onyxLight
        default: accent = dark ? ScydeAccents.emberDark : ScydeAccents.emberLight
        }
    }
}

struct RootView: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.colorScheme) private var system
    @State private var showTabs = false
    @State private var panel: Panel = .none
    @State private var text = ""

    private var scheme: ColorScheme {
        switch browser.appearance["theme"] {
        case "dark": return .dark
        case "light": return .light
        default: return system
        }
    }

    var body: some View {
        let palette = Palette(scheme: scheme, accent: browser.appearance["accent"])
        ZStack(alignment: .bottom) {
            palette.colors.surfaceCanvas.ignoresSafeArea()
            if let tab = browser.active {
                // Edge to edge: the page runs under the status bar and the home indicator (WebKit insets
                // its content for the safe area), so there is no band around it.
                WebContainer(webView: tab.webView)
                    .id(tab.id)
                    .ignoresSafeArea()
            }
            if panel != .none {
                palette.colors.scrim.ignoresSafeArea()
                    .onTapGesture { withAnimation(ScydeMotion.springSmooth.animation) { panel = .none } }
                    .transition(.opacity)
            }
            VStack(spacing: ScydeSpace.s8) {
                if let message = browser.toast, panel == .none {
                    ToastView(message: message)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }
                if panel != .none {
                    PanelView(panel: $panel, text: $text)
                        .transition(.scale(scale: ScydeMotion.bloomScale, anchor: .bottom).combined(with: .opacity))
                }
                IslandView(panel: $panel, showTabs: $showTabs, text: $text)
            }
            .padding(.horizontal, ScydeSize.islandMobileInset)
            .padding(.bottom, ScydeSpace.s4)
            .animation(ScydeMotion.springSmooth.animation, value: browser.toast)
            .animation(ScydeMotion.springSmooth.animation, value: browser.active?.immersive)
        }
        .statusBarHidden(browser.active?.immersive == true && panel == .none)
        .persistentSystemOverlays(browser.active?.immersive == true ? .hidden : .automatic)
        .environment(\.palette, palette)
        .preferredColorScheme(browser.appearance["theme"] == "system" || browser.appearance["theme"] == nil ? nil : scheme)
        .sheet(isPresented: $showTabs) { TabGrid(isPresented: $showTabs).environment(\.palette, palette) }
        .background {
            // A second sheet modifier on its own view, so it never competes with the tab grid.
            Color.clear.sheet(isPresented: Binding(get: { browser.sheetPage != nil }, set: { if !$0 { browser.sheetPage = nil } })) {
                PageSheet(page: browser.sheetPage ?? "settings").environmentObject(browser).environment(\.palette, palette)
            }
        }
    }
}

/// Settings, Library and Shield as a sheet over the tabs: Done or a swipe down closes it, and a link
/// to a website opens in the current tab instead of inside the sheet.
struct PageSheet: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    let page: String
    @StateObject private var web = SheetWeb()

    private var title: String {
        switch page.split(separator: "#").first.map(String.init) ?? page {
        case "library": return String(localized: "library.title")
        case "shields": return String(localized: "shield.label")
        case "welcome": return String(localized: "welcome.title")
        default: return String(localized: "rail.settings")
        }
    }

    var body: some View {
        NavigationStack {
            WebContainer(webView: web.webView(for: page, browser: browser))
                .ignoresSafeArea(.container, edges: .bottom)
                .background(palette.colors.surfaceCanvas)
                .navigationTitle(title)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button(String(localized: "mobile.tabs.done")) { browser.sheetPage = nil }.bold()
                    }
                }
        }
        .presentationDetents([.large])
        .presentationDragIndicator(.visible)
    }
}

@MainActor
final class SheetWeb: NSObject, ObservableObject, WKNavigationDelegate {
    private var view: WKWebView?
    private weak var browser: Browser?

    func webView(for page: String, browser: Browser) -> WKWebView {
        if let view { return view }
        self.browser = browser
        let v = WKWebView(frame: .zero, configuration: browser.makeConfiguration())
        v.navigationDelegate = self
        v.isOpaque = false
        // "library#bookmarks" → scyde://library/#bookmarks
        let parts = page.split(separator: "#", maxSplits: 1).map(String.init)
        let fragment = parts.count > 1 ? "#\(parts[1])" : ""
        v.load(URLRequest(url: URL(string: "scyde://\(parts.first ?? page)/\(fragment)")!))
        view = v
        return v
    }

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async -> WKNavigationActionPolicy {
        guard let url = navigationAction.request.url, url.scheme == "http" || url.scheme == "https" else { return .allow }
        browser?.sheetPage = nil
        browser?.go(url.absoluteString)
        return .cancel
    }
}

struct WebContainer: UIViewRepresentable {
    let webView: WKWebView
    func makeUIView(context: Context) -> WKWebView { webView }
    func updateUIView(_ uiView: WKWebView, context: Context) {}
}

private struct PaletteKey: EnvironmentKey {
    static let defaultValue = Palette(scheme: .light, accent: nil)
}

extension EnvironmentValues {
    var palette: Palette {
        get { self[PaletteKey.self] }
        set { self[PaletteKey.self] = newValue }
    }
}
