// swift-tools-version:5.10
// Spike S4 (G4): compile >= 150k WKContentRuleList rules split into lists, measure compile + cached load,
// and check that the qa/adblock fixture is blocked. Run on an iOS simulator via xcodebuild (see .github/workflows/spikes.yml).
import PackageDescription

let package = Package(
    name: "RuleListSpike",
    platforms: [.iOS(.v16)],
    products: [.library(name: "RuleListSpike", targets: ["RuleListSpike"])],
    targets: [
        .target(name: "RuleListSpike"),
        .testTarget(
            name: "RuleListSpikeTests",
            dependencies: ["RuleListSpike"],
            resources: [.copy("Resources/rulelists")]
        ),
    ]
)
