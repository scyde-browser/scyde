import Foundation
import WebKit

/// `scyde://<page>/<file>` from the bundled `pages/<page>/` (web/apps/*). Same pages as Windows and
/// Android; the bridge decides what they may do (CLAUDE.md rule 6).
final class InternalPages: NSObject, WKURLSchemeHandler {
    static let scheme = "scyde"
    static let pages = ["newtab", "settings", "shields", "library"]
    static let newTab = URL(string: "scyde://newtab/")!

    private static let csp =
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; "
        + "connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"

    static func isInternal(_ url: URL?) -> Bool { url?.scheme == scheme }

    func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        guard let url = task.request.url, let page = url.host, Self.pages.contains(page) else {
            return fail(task, 404)
        }
        var file = url.path
        if file.isEmpty || file == "/" { file = "/index.html" }
        guard !file.contains(".."),
              let base = Bundle.main.resourceURL?.appendingPathComponent("pages").appendingPathComponent(page),
              let data = try? Data(contentsOf: base.appendingPathComponent(String(file.dropFirst())))
        else { return fail(task, 404) }
        let headers = [
            "Content-Type": Self.mime(file),
            "Content-Security-Policy": Self.csp,
            "X-Content-Type-Options": "nosniff",
            "Cache-Control": "no-cache",
        ]
        let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: headers)!
        task.didReceive(response)
        task.didReceive(data)
        task.didFinish()
    }

    func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}

    private func fail(_ task: WKURLSchemeTask, _ code: Int) {
        task.didFailWithError(NSError(domain: NSURLErrorDomain, code: NSURLErrorFileDoesNotExist))
    }

    private static func mime(_ path: String) -> String {
        switch (path as NSString).pathExtension {
        case "html": return "text/html; charset=utf-8"
        case "js": return "text/javascript; charset=utf-8"
        case "css": return "text/css; charset=utf-8"
        case "json": return "application/json"
        case "woff2": return "font/woff2"
        case "svg": return "image/svg+xml"
        default: return "application/octet-stream"
        }
    }
}

/// Bridge for internal pages: `window.webkit.messageHandlers.scyde.postMessage(msg)` → Promise.
/// Only the main frame of a `scyde://` page is answered, only allowlisted methods.
@MainActor
final class Bridge: NSObject, WKScriptMessageHandlerWithReply {
    static let methods: Set<String> = [
        "newtab.state", "navigate", "settings.schema", "settings.get", "settings.set", "settings.reset",
        "history.query", "history.delete", "history.clear", "bookmarks.children", "bookmarks.remove",
        "shield.summary", "shield.setSite",
    ]
    weak var browser: Browser?

    func userContentController(
        _ controller: WKUserContentController, didReceive message: WKScriptMessage,
        replyHandler: @escaping (Any?, String?) -> Void
    ) {
        let origin = message.frameInfo.securityOrigin
        guard message.frameInfo.isMainFrame, origin.protocol == InternalPages.scheme,
              InternalPages.pages.contains(origin.host) else {
            return replyHandler(["ok": false, "error": "untrusted origin"], nil)
        }
        guard let body = message.body as? [String: Any], let method = body["method"] as? String,
              Self.methods.contains(method), let browser else {
            return replyHandler(["ok": false, "error": "method not allowed"], nil)
        }
        let params = body["params"] as? [String: Any] ?? [:]
        do {
            replyHandler(["id": body["id"] ?? 0, "ok": true, "result": try browser.handle(method, params)], nil)
        } catch {
            replyHandler(["id": body["id"] ?? 0, "ok": false, "error": "\(error)"], nil)
        }
    }
}
