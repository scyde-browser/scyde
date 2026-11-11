#!/usr/bin/env bash
# libscyde_core for Android (PLAN §9.2): one .so per ABI + Kotlin bindings.
#   core/ffi/build-android.sh [out=core/target/ffi-android]
# Needs: cargo-ndk, ANDROID_NDK_HOME, rustup targets aarch64-linux-android armv7-linux-androideabi x86_64-linux-android.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-target/ffi-android}"
rm -rf "$OUT" && mkdir -p "$OUT"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$OUT/jniLibs" build -p ffi --release
# Bindings come from an unstripped host build (the release profile strips the UniFFI metadata).
cargo build -p ffi
LIB=$(ls target/debug/libscyde_core.so target/debug/libscyde_core.dylib 2>/dev/null | head -1)
cargo run -q -p ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "$LIB" --language kotlin --no-format --out-dir "$OUT/kotlin"
find "$OUT" -type f -exec ls -la {} \;
