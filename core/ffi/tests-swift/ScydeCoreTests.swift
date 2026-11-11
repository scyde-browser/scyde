import ScydeCore
import XCTest

/// Smoke test of the Swift bindings on the iOS simulator (PLAN §11 Phase 1.9).
final class ScydeCoreTests: XCTestCase {
    func testCoreFromSwift() throws {
        XCTAssertTrue(coreVersion().contains("adblock"))

        switch omniboxClassify(text: "localhost:3000") {
        case let .url(url): XCTAssertEqual(url, "http://localhost:3000")
        default: XCTFail("expected a URL")
        }
        XCTAssertEqual(omniboxDisplayUrl(url: "https://xn--pple-43d.com/")?.punycode, true)

        let engine = ShieldEngine.fromLists(text: "||ads.example^\n##.ad-banner\n")
        XCTAssertTrue(engine.check(url: "https://ads.example/x.js", sourceUrl: "https://news.example/", requestType: "script").block)
        XCTAssertThrowsError(try ShieldEngine.fromDat(bytes: Data([1, 2, 3])))

        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let places = try PlacesStore.open(path: dir.appendingPathComponent("places.sqlite").path)
        _ = try places.recordVisit(url: "https://index.hr/", title: "Index", kind: .typed, at: 1_800_000_000)
        XCTAssertEqual(try places.suggest(query: "ind", limit: 5).first?.hostPrefix, true)

        let settings = SettingsStore.load(json: nil)
        XCTAssertEqual(settings.get(path: "search.engine"), "\"duckduckgo\"")
        XCTAssertThrowsError(try settings.set(path: "appearance.theme", jsonValue: "\"neon\""))

        let accent = try themeDeriveAccent(seedHex: "#FF9F7E")
        XCTAssertEqual(accent.light.fill, "#FF9F7E")
    }
}
