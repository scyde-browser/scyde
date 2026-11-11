# Filter bundle signing `[OWNER]`

The daily `filters` workflow signs each bundle with ed25519 (ADR-0011). Until both steps below are done, it builds and tests the bundle and uploads it as a workflow artifact, but **does not publish** a release.

## One-time setup (owner)
1. On your own machine (not in CI and not in a chat), generate the key pair:
   ```bash
   cd core && cargo run --release -p shield-compiler -- keygen
   ```
   It prints two base64 lines: the **secret seed** and the **public key**.
2. In GitHub, go to **Settings → Secrets and variables → Actions → New repository secret**. Create `FILTERS_SIGNING_KEY` and paste the secret seed.
3. Commit the public key as `brand/keys/filters.pub`, one line, base64, through a normal PR.
4. Run the `filters` workflow manually (`publish = true`), or wait for the daily run at 04:17 UTC.

Keep the seed in a password manager. Anyone with the seed can sign filter bundles that every SCYDE install will accept.

## What gets published
- **`filters-YYYYMMDD`:** a dated release. Releases older than 14 days are pruned.
- **`filters-latest`:** updated in place; this is the asset set clients poll (NETWORK.md N1).

Each release has these assets:
- `manifest.json` and `manifest.json.sig`
- every bundle file as a flat asset (`dnr/rules-00.json` → `dnr__rules-00.json`), so a platform downloads only what it needs
- `scyde-filters-YYYYMMDD.tar.gz` with the whole bundle

## Client rule (Phases 3–5)
1. Download `manifest.json` (with `If-None-Match`) and `manifest.json.sig`.
2. Verify the signature with the public key built into the app. Only then trust the hashes.
3. Download the files the platform needs, and check each one's size and SHA-256 against the manifest.
4. Build the new engine off the main thread, then swap atomically. On any failure, keep the old bundle.

## Key rotation
1. Generate a new pair, replace the secret, and commit the new `filters.pub`.
2. Ship an app update that trusts both keys during the overlap.
3. Remove the old key in the following release.
