// swift-tools-version:5.10
// Shield rule-list check (PLAN §11 Phase 1.2, ADR-0006): every ios/scyde-NN.json that
// shield-compiler produces must compile in WebKit, stay within 150k rules, and block the
// qa/adblock fixture. Runs on the iOS simulator in .github/workflows/ios.yml (job shield-rulelists).
import PackageDescription

let package = Package(
    name: "ShieldRuleLists",
    platforms: [.iOS(.v16)],
    products: [.library(name: "ShieldRuleLists", targets: ["ShieldRuleLists"])],
    targets: [
        .target(name: "ShieldRuleLists"),
        .testTarget(
            name: "ShieldRuleListsTests",
            dependencies: ["ShieldRuleLists"],
            resources: [.copy("Resources/rulelists")]
        ),
    ]
)
