#!/usr/bin/env bash
# ScydeCore.xcframework + Swift package for iOS (PLAN §9.3). macOS only.
#   core/ffi/build-apple.sh [out=core/target/ffi-apple]
# Produces <out>/ScydeCore/ (Package.swift, ScydeCore.xcframework, Sources/ScydeCore/scyde_core.swift).
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-target/ffi-apple}"
rm -rf "$OUT" && mkdir -p "$OUT"
for t in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  rustup target add "$t" >/dev/null
  cargo build -p ffi --release --lib --target "$t"
done
mkdir -p "$OUT/sim"
lipo -create target/aarch64-apple-ios-sim/release/libscyde_core.a target/x86_64-apple-ios/release/libscyde_core.a \
  -output "$OUT/sim/libscyde_core.a"

# Bindings from an unstripped host build (the release profile strips the UniFFI metadata).
cargo build -p ffi
cargo run -q -p ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library target/debug/libscyde_core.dylib --language swift --no-format --out-dir "$OUT/swift"
mkdir -p "$OUT/headers"
cp "$OUT/swift/scyde_coreFFI.h" "$OUT/headers/"
cp "$OUT/swift/scyde_coreFFI.modulemap" "$OUT/headers/module.modulemap"

PKG="$OUT/ScydeCore"
mkdir -p "$PKG/Sources/ScydeCore"
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libscyde_core.a -headers "$OUT/headers" \
  -library "$OUT/sim/libscyde_core.a" -headers "$OUT/headers" \
  -output "$PKG/ScydeCore.xcframework"
cp "$OUT/swift/scyde_core.swift" "$PKG/Sources/ScydeCore/"
cat > "$PKG/Package.swift" <<'SWIFT'
// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "ScydeCore",
    platforms: [.iOS(.v16)],
    products: [.library(name: "ScydeCore", targets: ["ScydeCore"])],
    targets: [
        .binaryTarget(name: "scyde_coreFFI", path: "ScydeCore.xcframework"),
        .target(name: "ScydeCore", dependencies: ["scyde_coreFFI"]),
        .testTarget(name: "ScydeCoreTests", dependencies: ["ScydeCore"]),
    ]
)
SWIFT
mkdir -p "$PKG/Tests/ScydeCoreTests"
cp ffi/tests-swift/ScydeCoreTests.swift "$PKG/Tests/ScydeCoreTests/"
du -sh "$PKG/ScydeCore.xcframework"
