#!/usr/bin/env bash
# Builds the Shield WASM engine (ADR-0004).
#   core/shield-wasm/build.sh [out-dir=core/target/shield-wasm]
# Produces <out>/no-modules (classic script for the GeckoView MV2 background), <out>/web (ES module)
# and <out>/node (for bench.mjs).
# Needs: rustup target wasm32-unknown-unknown, wasm-bindgen-cli at the version pinned in Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-target/shield-wasm}"
WANT=$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' shield-wasm/Cargo.toml)
HAVE=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}')
if [ "$WANT" != "$HAVE" ]; then
  echo "wasm-bindgen-cli $WANT required (found ${HAVE:-none}): cargo install wasm-bindgen-cli --version $WANT --locked" >&2
  exit 1
fi
# Keep the target_features section for wasm-bindgen; it strips names/producers itself.
CARGO_PROFILE_RELEASE_STRIP=false cargo build --release -p shield-wasm --target wasm32-unknown-unknown
WASM=target/wasm32-unknown-unknown/release/shield_wasm.wasm
for t in web nodejs no-modules; do
  dir="$OUT/${t/nodejs/node}"
  rm -rf "$dir"
  wasm-bindgen --target "$t" --remove-name-section --remove-producers-section --out-dir "$dir" "$WASM"
done
echo '{"type":"commonjs"}' > "$OUT/node/package.json"
ls -la "$OUT/web"
