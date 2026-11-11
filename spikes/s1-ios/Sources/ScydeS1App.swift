import SwiftUI
import UIKit
import WebKit

/// Spike S1 (G1) on iOS: WKWebView with a Safari-like user agent suffix (PLAN §2.2) for a manual Google sign-in test.
@main
struct ScydeS1App: App {
    var body: some Scene {
        WindowGroup { ContentView() }
    }
}

struct Target: Identifiable, Hashable {
    let id: String
    let title: String
}

let targets = [
    Target(id: "https://accounts.google.com/", title: "Google prijava"),
    Target(id: "https://mail.google.com/", title: "Gmail"),
    Target(id: "https://www.youtube.com/", title: "YouTube"),
    Target(id: "https://drive.google.com/", title: "Drive"),
    Target(id: "https://stackoverflow.com/users/login", title: "Sign in with Google"),
]

@MainActor
final class Browser: ObservableObject {
    let webView: WKWebView
    @Published var url = ""

    init() {
        let config = WKWebViewConfiguration()
        // Safari suffix keeps Google's embedded-webview check happy; the version follows the OS.
        let major = UIDevice.current.systemVersion.split(separator: ".").first.map(String.init) ?? "18"
        config.applicationNameForUserAgent = "Version/\(major).0 Mobile/15E148 Safari/604.1"
        config.websiteDataStore = .default()
        webView = WKWebView(frame: .zero, configuration: config)
        webView.allowsBackForwardNavigationGestures = true
        #if DEBUG
        if #available(iOS 16.4, *) { webView.isInspectable = true }
        #endif
    }

    func open(_ s: String) {
        guard let u = URL(string: s) else { return }
        webView.load(URLRequest(url: u))
        url = s
    }
}

struct WebView: UIViewRepresentable {
    let webView: WKWebView
    func makeUIView(context: Context) -> WKWebView { webView }
    func updateUIView(_ uiView: WKWebView, context: Context) {}
}

struct ContentView: View {
    @StateObject private var browser = Browser()
    @State private var selection = targets[0].id

    var body: some View {
        VStack(spacing: 0) {
            WebView(webView: browser.webView)
            HStack(spacing: 12) {
                Button { browser.webView.goBack() } label: { Image(systemName: "chevron.backward") }
                    .accessibilityLabel("Natrag")
                Picker("Cilj", selection: $selection) {
                    ForEach(targets) { Text($0.title).tag($0.id) }
                }
                .pickerStyle(.menu)
                .frame(maxWidth: .infinity)
                Button { browser.webView.reload() } label: { Image(systemName: "arrow.clockwise") }
                    .accessibilityLabel("Ponovno učitaj")
            }
            .padding(.horizontal, 16)
            .frame(height: 56)
            .background(.ultraThinMaterial)
        }
        .onAppear { browser.open(selection) }
        .onChange(of: selection) { browser.open($0) }
    }
}
