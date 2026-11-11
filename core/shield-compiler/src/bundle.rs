//! The filter bundle on disk (PLAN §5.2, §5.3): every output file, `manifest.json` with SHA-256
//! hashes, and `manifest.json.sig`, an ed25519 signature over the exact manifest bytes.
//!
//! ```text
//! engine.dat            serialized Shield engine (shield::Shield::to_dat)
//! dnr/rules-NN.json     DNR static rulesets (≤ 30k rules each)
//! ios/scyde-NN.json     WKContentRuleList JSON (≤ 150k rules each)
//! cosmetic/generic.json generic selectors keyed by class / id
//! cosmetic/hosts.json   per-host hide / unhide / scriptlets / procedural
//! resources.json        scriptlet and redirect resources
//! report.json           what every converter dropped and why
//! manifest.json         version, matcher, file hashes, ruleset index
//! manifest.json.sig     base64 ed25519 signature of manifest.json
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signature, Signer as _, SigningKey, VerifyingKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{cosmetic, dnr, ios};

/// Bundle layout version.
pub const BUNDLE_FORMAT: u32 = 1;

/// A named filter list source.
#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub text: String,
}

/// Compiler options.
#[derive(Debug, Clone)]
pub struct Options {
    /// Bundle version, e.g. `20261111`. Kept out of timestamps so builds are reproducible.
    pub version: String,
    pub dnr: dnr::DnrLimits,
    pub ios_max_per_list: usize,
    /// `resources.json` contents (uBO-compatible array); `None` writes `[]`.
    pub resources: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            version: "dev".into(),
            dnr: dnr::DnrLimits::default(),
            ios_max_per_list: ios::MAX_PER_LIST,
            resources: None,
        }
    }
}

#[derive(Debug)]
pub enum BundleError {
    Io(PathBuf, std::io::Error),
    Resources(shield::ShieldError),
    BadKey(String),
    Unsigned,
    BadSignature,
    Manifest(String),
    HashMismatch(String),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(p, e) => write!(f, "{}: {e}", p.display()),
            Self::Resources(e) => write!(f, "{e}"),
            Self::BadKey(e) => write!(f, "signing key: {e}"),
            Self::Unsigned => f.write_str("bundle has no manifest.json.sig"),
            Self::BadSignature => f.write_str("manifest signature does not verify"),
            Self::Manifest(e) => write!(f, "manifest.json: {e}"),
            Self::HashMismatch(p) => write!(f, "{p}: hash or size does not match the manifest"),
        }
    }
}

impl std::error::Error for BundleError {}

fn io<T>(p: &Path, r: std::io::Result<T>) -> Result<T, BundleError> {
    r.map_err(|e| BundleError::Io(p.to_owned(), e))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Short key id: first 8 bytes of SHA-256 of the public key, hex.
#[must_use]
pub fn key_id(key: &VerifyingKey) -> String {
    sha256_hex(key.as_bytes())[..16].to_owned()
}

/// Parses a base64 32-byte ed25519 seed (the `FILTERS_SIGNING_KEY` secret).
///
/// # Errors
/// [`BundleError::BadKey`] if it is not base64 of 32 bytes.
pub fn signing_key_from_b64(s: &str) -> Result<SigningKey, BundleError> {
    let bytes = B64
        .decode(s.trim())
        .map_err(|e| BundleError::BadKey(e.to_string()))?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| BundleError::BadKey("expected 32 bytes".into()))?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Parses a base64 32-byte ed25519 public key (`brand/keys/filters.pub`).
///
/// # Errors
/// [`BundleError::BadKey`] if it is not a valid public key.
pub fn verifying_key_from_b64(s: &str) -> Result<VerifyingKey, BundleError> {
    let bytes = B64
        .decode(s.trim())
        .map_err(|e| BundleError::BadKey(e.to_string()))?;
    let pk: [u8; 32] = bytes
        .try_into()
        .map_err(|_| BundleError::BadKey("expected 32 bytes".into()))?;
    VerifyingKey::from_bytes(&pk).map_err(|e| BundleError::BadKey(e.to_string()))
}

/// Base64 of a public key, for `brand/keys/`.
#[must_use]
pub fn verifying_key_to_b64(k: &VerifyingKey) -> String {
    B64.encode(k.as_bytes())
}

/// Base64 of a signing key seed, for the `FILTERS_SIGNING_KEY` secret.
#[must_use]
pub fn signing_key_to_b64(k: &SigningKey) -> String {
    B64.encode(k.to_bytes())
}

/// Result of a compile run.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub manifest: Value,
    pub report: Value,
}

struct Writer {
    root: PathBuf,
    files: BTreeMap<String, Value>,
}

impl Writer {
    fn put(&mut self, rel: &str, bytes: &[u8]) -> Result<(), BundleError> {
        let p = self.root.join(rel);
        if let Some(dir) = p.parent() {
            io(dir, fs::create_dir_all(dir))?;
        }
        io(&p, fs::write(&p, bytes))?;
        self.files.insert(
            rel.to_owned(),
            json!({ "bytes": bytes.len(), "sha256": sha256_hex(bytes) }),
        );
        Ok(())
    }
}

/// Compiles `sources` into a bundle in `out`, signing it when `key` is given.
///
/// # Errors
/// I/O errors, or invalid `resources.json`.
///
/// # Panics
/// Never: all outputs are plain JSON values.
pub fn compile(
    sources: &[Source],
    opts: &Options,
    out: &Path,
    key: Option<&SigningKey>,
) -> Result<Compiled, BundleError> {
    let texts: Vec<&str> = sources.iter().map(|s| s.text.as_str()).collect();
    let mut w = Writer {
        root: out.to_owned(),
        files: BTreeMap::new(),
    };
    io(out, fs::create_dir_all(out))?;

    // engine.dat (+ resources validated against the engine)
    let mut engine = shield::Shield::from_lists(&texts);
    let resources = opts.resources.clone().unwrap_or_else(|| "[]".into());
    let resource_count = engine
        .use_resources_json(&resources)
        .map_err(BundleError::Resources)?;
    w.put("engine.dat", &engine.to_dat())?;
    w.put("resources.json", resources.as_bytes())?;

    // DNR
    let d = dnr::convert(&texts, opts.dnr);
    let mut dnr_index = Vec::new();
    for (i, rs) in d.rulesets.iter().enumerate() {
        let path = format!("dnr/rules-{i:02}.json");
        w.put(&path, serde_json::to_string(rs).expect("json").as_bytes())?;
        dnr_index.push(json!({ "id": format!("scyde-{i:02}"), "path": path, "rules": rs.len() }));
    }

    // iOS
    let i = ios::convert(&texts, opts.ios_max_per_list);
    let mut ios_index = Vec::new();
    for (n, list) in i.lists.iter().enumerate() {
        let path = format!("ios/scyde-{n:02}.json");
        w.put(&path, serde_json::to_string(list).expect("json").as_bytes())?;
        ios_index.push(json!({ "id": format!("scyde-{n:02}"), "path": path, "rules": list.len() }));
    }

    // Cosmetics
    let (idx, crep) = cosmetic::build(&texts);
    w.put(
        "cosmetic/generic.json",
        serde_json::to_string(&idx.generic)
            .expect("json")
            .as_bytes(),
    )?;
    w.put(
        "cosmetic/hosts.json",
        serde_json::to_string(&idx.hosts).expect("json").as_bytes(),
    )?;

    let report = json!({
        "dnr": d.report,
        "ios": i.report,
        "cosmetic": crep,
        "resources": resource_count,
    });
    w.put(
        "report.json",
        serde_json::to_string_pretty(&report)
            .expect("json")
            .as_bytes(),
    )?;

    let mut manifest = json!({
        "format": BUNDLE_FORMAT,
        "version": opts.version,
        "matcher": shield::MATCHER_VERSION,
        "sources": sources.iter().map(|s| json!({
            "name": s.name, "bytes": s.text.len(), "sha256": sha256_hex(s.text.as_bytes())
        })).collect::<Vec<_>>(),
        "dnr": dnr_index,
        "ios": ios_index,
        "files": w.files,
    });
    if let Some(k) = key {
        manifest["key_id"] = json!(key_id(&k.verifying_key()));
    }
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).expect("json");
    let mp = out.join("manifest.json");
    io(&mp, fs::write(&mp, &manifest_bytes))?;
    let sp = out.join("manifest.json.sig");
    if let Some(k) = key {
        let sig = k.sign(&manifest_bytes);
        io(&sp, fs::write(&sp, B64.encode(sig.to_bytes())))?;
    } else if sp.exists() {
        io(&sp, fs::remove_file(&sp))?; // never leave a stale signature next to a new manifest
    }
    Ok(Compiled { manifest, report })
}

/// Verifies a bundle: the signature over `manifest.json`, then the size and hash of every file.
/// This is what the updater does before swapping engines (PLAN §5.3).
///
/// # Errors
/// [`BundleError::Unsigned`], [`BundleError::BadSignature`], [`BundleError::HashMismatch`], …
pub fn verify(dir: &Path, key: &VerifyingKey) -> Result<Value, BundleError> {
    let mp = dir.join("manifest.json");
    let manifest_bytes = io(&mp, fs::read(&mp))?;
    let sp = dir.join("manifest.json.sig");
    if !sp.exists() {
        return Err(BundleError::Unsigned);
    }
    let sig_b64 = io(&sp, fs::read_to_string(&sp))?;
    let sig: [u8; 64] = B64
        .decode(sig_b64.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or(BundleError::BadSignature)?;
    key.verify_strict(&manifest_bytes, &Signature::from_bytes(&sig))
        .map_err(|_| BundleError::BadSignature)?;

    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| BundleError::Manifest(e.to_string()))?;
    let files = manifest["files"]
        .as_object()
        .ok_or_else(|| BundleError::Manifest("no files".into()))?;
    for (rel, meta) in files {
        if rel.contains("..") || rel.starts_with('/') {
            return Err(BundleError::Manifest(format!("unsafe path {rel}")));
        }
        let p = dir.join(rel);
        let bytes = io(&p, fs::read(&p))?;
        if meta["bytes"].as_u64() != Some(bytes.len() as u64)
            || meta["sha256"].as_str() != Some(sha256_hex(&bytes).as_str())
        {
            return Err(BundleError::HashMismatch(rel.clone()));
        }
    }
    Ok(manifest)
}
