import Foundation
import WebKit

/// Compiles and looks up content rule lists (PLAN §5.4). The app does this in the background (ADR-0006).
public enum RuleLists {
    public struct Timing: Codable, Sendable {
        public let identifier: String
        public let rules: Int
        public let compileMs: Double
        public let lookupMs: Double

        public init(identifier: String, rules: Int, compileMs: Double, lookupMs: Double) {
            self.identifier = identifier
            self.rules = rules
            self.compileMs = compileMs
            self.lookupMs = lookupMs
        }
    }

    @MainActor
    public static func compile(identifier: String, json: String, store: WKContentRuleListStore) async throws -> (WKContentRuleList, Double) {
        let start = DispatchTime.now()
        let list = try await withCheckedThrowingContinuation { (cont: CheckedContinuation<WKContentRuleList, Error>) in
            store.compileContentRuleList(forIdentifier: identifier, encodedContentRuleList: json) { list, error in
                if let list { cont.resume(returning: list) } else { cont.resume(throwing: error ?? NSError(domain: "scyde", code: 1)) }
            }
        }
        return (list, ms(since: start))
    }

    @MainActor
    public static func lookUp(identifier: String, store: WKContentRuleListStore) async throws -> (WKContentRuleList, Double) {
        let start = DispatchTime.now()
        let list = try await withCheckedThrowingContinuation { (cont: CheckedContinuation<WKContentRuleList, Error>) in
            store.lookUpContentRuleList(forIdentifier: identifier) { list, error in
                if let list { cont.resume(returning: list) } else { cont.resume(throwing: error ?? NSError(domain: "scyde", code: 2)) }
            }
        }
        return (list, ms(since: start))
    }

    static func ms(since start: DispatchTime) -> Double {
        Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000
    }
}
