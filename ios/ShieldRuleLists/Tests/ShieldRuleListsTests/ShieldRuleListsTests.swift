import ShieldRuleLists
import WebKit
import XCTest

/// Resources (filled by CI before `xcodebuild test`):
/// - `rulelists/bundle/scyde-NN.json`: iOS output of a shield-compiler bundle
/// - `rulelists/fixture/scyde-00.json`: iOS output for `qa/lists/fixture.txt`
/// - `rulelists/qa-adblock/`: the `qa/adblock` page
/// Results are printed as `SCYDE_RESULT <name> <json>` lines.
@MainActor
final class ShieldRuleListsTests: XCTestCase {
    private func dir(_ name: String) throws -> URL {
        let root = try XCTUnwrap(Bundle.module.url(forResource: "rulelists", withExtension: nil))
        return root.appendingPathComponent(name)
    }

    private func report(_ name: String, _ value: some Encodable) {
        let data = try! JSONEncoder().encode(value)
        print("SCYDE_RESULT \(name) \(String(decoding: data, as: UTF8.self))")
    }

    private func store() throws -> WKContentRuleListStore {
        let d = FileManager.default.temporaryDirectory.appendingPathComponent("scyde-rules-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return try XCTUnwrap(WKContentRuleListStore(url: d))
    }

    func testBundleListsCompile() async throws {
        let bundle = try dir("bundle")
        let files = try FileManager.default.contentsOfDirectory(atPath: bundle.path)
            .filter { $0.hasPrefix("scyde-") && $0.hasSuffix(".json") }.sorted()
        XCTAssertFalse(files.isEmpty, "no iOS lists in the bundle")
        let store = try store()
        var timings: [RuleLists.Timing] = []
        for file in files {
            let json = try String(contentsOf: bundle.appendingPathComponent(file), encoding: .utf8)
            let rules = (try JSONSerialization.jsonObject(with: Data(json.utf8)) as? [Any])?.count ?? 0
            XCTAssertLessThanOrEqual(rules, 150_000, "WebKit limit per list: \(file)")
            let id = (file as NSString).deletingPathExtension
            let (_, compileMs) = try await RuleLists.compile(identifier: id, json: json, store: store)
            let (_, lookupMs) = try await RuleLists.lookUp(identifier: id, store: store)
            timings.append(.init(identifier: id, rules: rules, compileMs: compileMs, lookupMs: lookupMs))
        }
        report("bundle", timings)
    }

    func testFixtureIsBlocked() async throws {
        let json = try String(contentsOf: try dir("fixture").appendingPathComponent("scyde-00.json"), encoding: .utf8)
        let (list, _) = try await RuleLists.compile(identifier: "fixture", json: json, store: try store())

        let config = WKWebViewConfiguration()
        config.userContentController.add(list)
        let web = WKWebView(frame: CGRect(x: 0, y: 0, width: 390, height: 844), configuration: config)
        let window = UIWindow(frame: web.frame)
        window.addSubview(web)
        window.makeKeyAndVisible()

        let page = try dir("qa-adblock").appendingPathComponent("index.html")
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
        let r = try XCTUnwrap(result, "fixture page did not report")
        print("SCYDE_RESULT fixture \(String(decoding: try JSONSerialization.data(withJSONObject: r), as: UTF8.self))")
        let results = r["results"] as? [String: Bool] ?? [:]
        for key in results.keys where key.hasPrefix("net:") {
            XCTAssertEqual(results[key], true, "network rule not applied: \(key)")
        }
        XCTAssertEqual(results["allow:exception"], true)
        XCTAssertEqual(results["cos:generic-class"], true)
        XCTAssertEqual(results["keep:content"], true)
    }
}
