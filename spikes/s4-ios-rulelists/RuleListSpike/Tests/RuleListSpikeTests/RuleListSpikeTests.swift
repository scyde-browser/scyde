import RuleListSpike
import WebKit
import XCTest

/// Results are printed as `SCYDE_RESULT <json>` lines; CI collects them into s4-results.json.
@MainActor
final class RuleListSpikeTests: XCTestCase {
    private func resource(_ name: String) throws -> URL {
        let dir = try XCTUnwrap(Bundle.module.url(forResource: "rulelists", withExtension: nil))
        return dir.appendingPathComponent(name)
    }

    private func report(_ name: String, _ value: some Encodable) {
        let data = try! JSONEncoder().encode(value)
        print("SCYDE_RESULT \(name) \(String(decoding: data, as: UTF8.self))")
    }

    func testCompileAndCachedLoad() async throws {
        let dir = try XCTUnwrap(Bundle.module.url(forResource: "rulelists", withExtension: nil))
        let files = try FileManager.default.contentsOfDirectory(atPath: dir.path)
            .filter { $0.hasPrefix("scyde-") && $0.hasSuffix(".json") }.sorted()
        XCTAssertGreaterThanOrEqual(files.count, 2, "need the >=150k rule set split into several lists")

        let storeDir = FileManager.default.temporaryDirectory.appendingPathComponent("scyde-rules-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: storeDir, withIntermediateDirectories: true)
        let store = try XCTUnwrap(WKContentRuleListStore(url: storeDir))

        var timings: [RuleLists.Timing] = []
        var totalRules = 0
        for file in files {
            let json = try String(contentsOf: dir.appendingPathComponent(file), encoding: .utf8)
            let rules = (try JSONSerialization.jsonObject(with: Data(json.utf8)) as? [Any])?.count ?? 0
            XCTAssertLessThanOrEqual(rules, 150_000, "WebKit limit per list")
            totalRules += rules
            let id = (file as NSString).deletingPathExtension
            let (_, compileMs) = try await RuleLists.compile(identifier: id, json: json, store: store)
            let (_, lookupMs) = try await RuleLists.lookUp(identifier: id, store: store)
            timings.append(.init(identifier: id, rules: rules, compileMs: compileMs, lookupMs: lookupMs))
        }
        XCTAssertGreaterThanOrEqual(totalRules, 150_000)

        // Cached load of all lists together, as at app start.
        let start = DispatchTime.now()
        for t in timings { _ = try await RuleLists.lookUp(identifier: t.identifier, store: store) }
        let cachedAllMs = Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000

        struct Summary: Encodable { let totalRules: Int; let lists: [RuleLists.Timing]; let cachedLoadAllMs: Double; let g4CachedUnder100ms: Bool }
        report("compile", Summary(totalRules: totalRules, lists: timings, cachedLoadAllMs: cachedAllMs, g4CachedUnder100ms: cachedAllMs < 100))
        XCTAssertLessThan(cachedAllMs, 100, "G4: cached load < 100 ms")
    }

    func testFixtureIsBlocked() async throws {
        let json = try String(contentsOf: resource("fixture.json"), encoding: .utf8)
        let store = try XCTUnwrap(WKContentRuleListStore(url: FileManager.default.temporaryDirectory.appendingPathComponent("scyde-fixture-\(UUID().uuidString)")))
        let (list, _) = try await RuleLists.compile(identifier: "fixture", json: json, store: store)

        let config = WKWebViewConfiguration()
        config.userContentController.add(list)
        let web = WKWebView(frame: CGRect(x: 0, y: 0, width: 390, height: 844), configuration: config)
        let window = UIWindow(frame: web.frame)
        window.addSubview(web)
        window.makeKeyAndVisible()

        // qa/adblock is bundled as a test resource (copied by CI): no network and no ATS exception needed.
        let page = try resource("qa-adblock/index.html")
        let nav = NavRecorder()
        web.navigationDelegate = nav
        web.loadFileURL(page, allowingReadAccessTo: page.deletingLastPathComponent())

        var result: [String: Any]?
        for _ in 0..<60 {
            try await Task.sleep(nanoseconds: 250_000_000)
            if let r = try? await web.evaluateJavaScript("window.__scydeQa ? JSON.stringify(window.__scydeQa) : null") as? String,
               let obj = try? JSONSerialization.jsonObject(with: Data(r.utf8)) as? [String: Any] {
                result = obj
                break
            }
        }
        let r = try XCTUnwrap(result, "fixture page did not report; navigation log: \(nav.log)")
        print("SCYDE_RESULT fixture \(String(decoding: try JSONSerialization.data(withJSONObject: r), as: UTF8.self))")
        let results = r["results"] as? [String: Bool] ?? [:]
        for key in results.keys where key.hasPrefix("net:") {
            XCTAssertEqual(results[key], true, "network rule not applied: \(key)")
        }
        XCTAssertEqual(results["allow:exception"], true)
        XCTAssertEqual(results["keep:content"], true)
    }
}

@MainActor
final class NavRecorder: NSObject, WKNavigationDelegate {
    var log: [String] = []
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) { log.append("finish \(webView.url?.absoluteString ?? "")") }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) { log.append("fail \(error)") }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) { log.append("provisional fail \(error)") }
}
