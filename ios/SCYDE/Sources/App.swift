import SwiftUI
import UIKit
import WebKit

@main
struct ScydeApp: App {
    @StateObject private var browser = Browser()
    @Environment(\.scenePhase) private var phase

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
                        } else if !browser.restoreSession() {
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
        .onChange(of: phase) { p in if p != .active { browser.saveSession() } }
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
        accent = Palette.accent(name, dark: dark)
    }
}

extension Palette {
    static func accent(_ name: String?, dark: Bool) -> ScydeAccent {
        switch name {
        case "sky": return dark ? ScydeAccents.skyDark : ScydeAccents.skyLight
        case "periwinkle": return dark ? ScydeAccents.periwinkleDark : ScydeAccents.periwinkleLight
        case "lavender": return dark ? ScydeAccents.lavenderDark : ScydeAccents.lavenderLight
        case "mint": return dark ? ScydeAccents.mintDark : ScydeAccents.mintLight
        case "onyx": return dark ? ScydeAccents.onyxDark : ScydeAccents.onyxLight
        default: return dark ? ScydeAccents.emberDark : ScydeAccents.emberLight
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
                // Keyed by the web view, which a sleeping tab replaces.
                WebContainer(webView: tab.webView)
                    .id(ObjectIdentifier(tab.webView))
                    .ignoresSafeArea()
            }
            if let tab = browser.active, !tab.immersive {
                // Behind the status bar: the page's own colour, so text never runs under the clock.
                // The band covers only the top safe area (zero height plus the ignored safe area).
                // The status-bar text follows the app's scheme, so the page colour is used only when the
                // clock stays readable on it; otherwise the band is the app's canvas.
                bandColor(tab, palette)
                    .ignoresSafeArea(edges: .top)
                    .frame(height: 0)
                    .frame(maxHeight: .infinity, alignment: .top)
                    .allowsHitTesting(false)
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
        .onAppear {
            // `-panel menu|shield|omnibox|tabs` opens that surface at launch (CI screenshots).
            let args = CommandLine.arguments
            guard let i = args.firstIndex(of: "-panel"), i + 1 < args.count else { return }
            switch args[i + 1] {
            case "menu": panel = .menu
            case "shield": panel = .shield
            case "omnibox": panel = .omnibox
            case "tabs": showTabs = true
            default: break
            }
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
        // `?embedded`: the sheet has its own title, the page hides its heading (welcome keeps its header).
        let query = parts.first == "welcome" ? "" : "?embedded"
        v.load(URLRequest(url: URL(string: "scyde://\(parts.first ?? page)/\(query)\(fragment)")!))
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

@MainActor
private func bandColor(_ tab: Tab, _ palette: Palette) -> Color {
    let page: UIColor? = tab.themeColor ?? tab.webView.underPageBackgroundColor
    if let page, page.isLight != palette.dark { return Color(uiColor: page) }
    return palette.colors.surfaceCanvas
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
