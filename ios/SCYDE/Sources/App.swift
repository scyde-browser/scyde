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

    init(scheme: ColorScheme, accent name: String?) {
        let dark = scheme == .dark
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
                WebContainer(webView: tab.webView)
                    .id(tab.id)
                    .ignoresSafeArea(.container, edges: .bottom)
            }
            IslandView(showTabs: $showTabs)
                .padding(.horizontal, ScydeSize.islandMobileInset)
                .padding(.bottom, ScydeSpace.s4)
        }
        .environment(\.palette, palette)
        .preferredColorScheme(browser.appearance["theme"] == "system" || browser.appearance["theme"] == nil ? nil : scheme)
        .sheet(isPresented: $showTabs) { TabGrid(isPresented: $showTabs).environment(\.palette, palette) }
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
