import ScydeCore
import SwiftUI
import UIKit
import WebKit

/// What a page asks of the browser beyond showing itself: JavaScript dialogs, downloads, links for
/// other apps (`tel:`, `mailto:`…) and an error page when a page does not open.
extension Browser: WKDownloadDelegate {
    // MARK: JavaScript dialogs (alert, confirm, prompt)

    private func present(_ alert: UIAlertController) -> Bool {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        guard var top = scenes.flatMap(\.windows).first(where: \.isKeyWindow)?.rootViewController else { return false }
        while let next = top.presentedViewController { top = next }
        top.present(alert, animated: true)
        return true
    }

    private func dialogTitle(_ frame: WKFrameInfo) -> String? {
        // The site that asks, so a page cannot pretend to be the browser.
        omniboxDisplayUrl(url: frame.request.url?.absoluteString ?? "")?.domain ?? frame.request.url?.host
    }

    func webView(_ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo) async {
        await withCheckedContinuation { (done: CheckedContinuation<Void, Never>) in
            let alert = UIAlertController(title: dialogTitle(frame), message: message, preferredStyle: .alert)
            alert.addAction(UIAlertAction(title: String(localized: "prompt.ok"), style: .default) { _ in done.resume() })
            if !present(alert) { done.resume() }
        }
    }

    func webView(_ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo) async -> Bool {
        await withCheckedContinuation { (done: CheckedContinuation<Bool, Never>) in
            let alert = UIAlertController(title: dialogTitle(frame), message: message, preferredStyle: .alert)
            alert.addAction(UIAlertAction(title: String(localized: "omnibox.cancel"), style: .cancel) { _ in done.resume(returning: false) })
            alert.addAction(UIAlertAction(title: String(localized: "prompt.ok"), style: .default) { _ in done.resume(returning: true) })
            if !present(alert) { done.resume(returning: false) }
        }
    }

    func webView(
        _ webView: WKWebView, runJavaScriptTextInputPanelWithPrompt prompt: String, defaultText: String?, initiatedByFrame frame: WKFrameInfo
    ) async -> String? {
        await withCheckedContinuation { (done: CheckedContinuation<String?, Never>) in
            let alert = UIAlertController(title: dialogTitle(frame), message: prompt, preferredStyle: .alert)
            alert.addTextField { $0.text = defaultText }
            alert.addAction(UIAlertAction(title: String(localized: "omnibox.cancel"), style: .cancel) { _ in done.resume(returning: nil) })
            alert.addAction(UIAlertAction(title: String(localized: "prompt.ok"), style: .default) { [weak alert] _ in
                done.resume(returning: alert?.textFields?.first?.text ?? "")
            })
            if !present(alert) { done.resume(returning: nil) }
        }
    }

    // MARK: links for other apps

    static let engineSchemes: Set<String> = ["http", "https", "about", "data", "blob", "file", "javascript", InternalPages.scheme]

    /// `tel:`, `mailto:`, `sms:`, `maps:`… open in the app that handles them, and only after a tap.
    func openExternal(_ action: WKNavigationAction) -> Bool {
        guard let url = action.request.url, let scheme = url.scheme?.lowercased(), !Self.engineSchemes.contains(scheme) else { return false }
        if action.navigationType == .linkActivated || action.targetFrame == nil {
            UIApplication.shared.open(url) { [weak self] opened in
                if !opened { self?.show(String(localized: "external.none")) }
            }
        }
        return true
    }

    // MARK: downloads (saved to the app's Documents/Downloads, visible in the Files app)

    func webView(_ webView: WKWebView, decidePolicyFor navigationResponse: WKNavigationResponse) async -> WKNavigationResponsePolicy {
        let attachment = (navigationResponse.response as? HTTPURLResponse)?
            .value(forHTTPHeaderField: "Content-Disposition")?.lowercased().hasPrefix("attachment") == true
        return navigationResponse.canShowMIMEType && !attachment ? .allow : .download
    }

    func webView(_ webView: WKWebView, navigationAction: WKNavigationAction, didBecome download: WKDownload) {
        download.delegate = self
    }

    func webView(_ webView: WKWebView, navigationResponse: WKNavigationResponse, didBecome download: WKDownload) {
        download.delegate = self
    }

    func download(_ download: WKDownload, decideDestinationUsing response: URLResponse, suggestedFilename: String) async -> URL? {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Downloads", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        // No path parts in a name the page chose; an existing file is never overwritten.
        let clean = suggestedFilename.components(separatedBy: CharacterSet(charactersIn: "/\\:")).joined(separator: "_")
        let name = clean.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "download" : String(clean.prefix(120))
        let base = (name as NSString).deletingPathExtension
        let ext = (name as NSString).pathExtension
        var target = dir.appendingPathComponent(name)
        var n = 1
        while FileManager.default.fileExists(atPath: target.path) {
            n += 1
            target = dir.appendingPathComponent(ext.isEmpty ? "\(base) \(n)" : "\(base) \(n).\(ext)")
        }
        downloads[ObjectIdentifier(download)] = target.lastPathComponent
        show(String(format: String(localized: "download.started"), target.lastPathComponent))
        return target
    }

    func downloadDidFinish(_ download: WKDownload) {
        let name = downloads.removeValue(forKey: ObjectIdentifier(download)) ?? ""
        show(String(format: String(localized: "download.done"), name))
    }

    func download(_ download: WKDownload, didFailWithError error: Error, resumeData: Data?) {
        downloads.removeValue(forKey: ObjectIdentifier(download))
        show(String(localized: "download.failed"))
    }

    // MARK: error page

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        let e = error as NSError
        // Cancelled loads, downloads and policy decisions are not failures the user needs to see.
        if e.domain == NSURLErrorDomain && e.code == NSURLErrorCancelled { return }
        if e.domain == "WebKitErrorDomain" && (e.code == 102 || e.code == 101) { return }
        guard let url = (e.userInfo[NSURLErrorFailingURLErrorKey] as? URL) ?? webView.url,
              url.scheme == "https" || url.scheme == "http" else { return }
        tab(for: webView)?.skipVisit = true
        webView.loadHTMLString(errorPage(for: url), baseURL: url)
    }

    private func errorPage(for url: URL) -> String {
        func esc(_ v: String) -> String {
            v.replacingOccurrences(of: "&", with: "&amp;").replacingOccurrences(of: "<", with: "&lt;")
                .replacingOccurrences(of: ">", with: "&gt;").replacingOccurrences(of: "\"", with: "&quot;")
                .replacingOccurrences(of: "'", with: "&#39;")
        }
        func hex(_ c: Color) -> String {
            var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
            UIColor(c).getRed(&r, green: &g, blue: &b, alpha: &a)
            return String(format: "#%02X%02X%02X", Int(r * 255), Int(g * 255), Int(b * 255))
        }
        let l = ScydeColors.light, d = ScydeColors.dark
        let host = url.host ?? url.absoluteString
        let target = (try? JSONSerialization.data(withJSONObject: [url.absoluteString]))
            .map { String(decoding: $0, as: UTF8.self).dropFirst().dropLast() }.map(String.init) ?? "\"\""
        let px = { (v: CGFloat) in "\(Int(v))px" }
        return """
        <!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
        <meta name="color-scheme" content="light dark"><title>\(esc(String(localized: "loaderror.title")))</title><style>
        :root{--bg:\(hex(l.surfaceCanvas));--fg:\(hex(l.textPrimary));--muted:\(hex(l.textSecondary))}
        @media (prefers-color-scheme:dark){:root{--bg:\(hex(d.surfaceCanvas));--fg:\(hex(d.textPrimary));--muted:\(hex(d.textSecondary))}}
        html,body{margin:0;background:var(--bg);color:var(--fg);font-family:-apple-system,system-ui,sans-serif}
        main{box-sizing:border-box;min-height:100vh;display:flex;flex-direction:column;justify-content:center;padding:\(px(ScydeSpace.s24));max-width:\(px(ScydeSpace.s80 * 6));margin:0 auto}
        h1{font-size:\(px(ScydeType.titleSize));margin:0 0 \(px(ScydeSpace.s8))}
        p{color:var(--muted);font-size:\(px(ScydeType.bodySize));line-height:1.45;margin:0 0 \(px(ScydeSpace.s24))}
        button{align-self:flex-start;min-height:\(px(ScydeSize.hitIos));padding:0 \(px(ScydeSpace.s24));border:0;border-radius:\(px(ScydeRadius.full));background:var(--fg);color:var(--bg);font:inherit;font-weight:500}
        </style></head><body><main><h1>\(esc(String(localized: "loaderror.title")))</h1>
        <p>\(esc(String(format: String(localized: "loaderror.body"), host)))</p>
        <button onclick='location.replace(\(esc(target)))'>\(esc(String(localized: "loaderror.retry")))</button></main></body></html>
        """
    }
}
