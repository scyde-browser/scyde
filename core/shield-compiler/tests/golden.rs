//! Golden tests: `qa/lists/fixture.txt` → expected DNR / iOS / cosmetic output (PLAN §11).
//! Regenerate after an intended change with `UPDATE_GOLDEN=1 cargo test -p shield-compiler`.
#![allow(clippy::format_collect)] // test data builders

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use shield_compiler::bundle::{self, BundleError, Options, Source};
use shield_compiler::{dnr, ios};

const FIXTURE: &str = include_str!("../../../qa/lists/fixture.txt");
const GOLDEN: &[&str] = &[
    "dnr/rules-00.json",
    "ios/scyde-00.json",
    "cosmetic/generic.json",
    "cosmetic/hosts.json",
    "report.json",
];

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "scyde-shield-compiler-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&p);
    p
}

fn sources() -> Vec<Source> {
    vec![Source {
        name: "fixture.txt".into(),
        text: FIXTURE.into(),
    }]
}

fn test_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[7u8; 32])
}

fn pretty(p: &Path) -> String {
    let v: Value = serde_json::from_slice(&fs::read(p).unwrap()).unwrap();
    serde_json::to_string_pretty(&v).unwrap() + "\n"
}

#[test]
fn fixture_matches_golden() {
    let out = tmp("golden");
    bundle::compile(
        &sources(),
        &Options {
            version: "golden".into(),
            ..Options::default()
        },
        &out,
        None,
    )
    .unwrap();
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    for rel in GOLDEN {
        let got = pretty(&out.join(rel));
        let want_path = golden_dir.join(rel.replace('/', "__"));
        if update {
            fs::write(&want_path, &got).unwrap();
            continue;
        }
        let want = fs::read_to_string(&want_path).unwrap_or_else(|_| {
            panic!(
                "missing golden {}; run with UPDATE_GOLDEN=1",
                want_path.display()
            )
        });
        assert_eq!(got, want, "{rel} differs from golden");
    }
}

#[test]
fn fixture_semantics() {
    let out = tmp("semantics");
    bundle::compile(&sources(), &Options::default(), &out, None).unwrap();
    let dnr: Vec<Value> =
        serde_json::from_slice(&fs::read(out.join("dnr/rules-00.json")).unwrap()).unwrap();
    let blocks = dnr
        .iter()
        .filter(|r| r["action"]["type"] == "block")
        .count();
    let allows = dnr
        .iter()
        .filter(|r| r["action"]["type"] == "allow")
        .count();
    assert_eq!(
        (blocks, allows),
        (5, 1),
        "5 network block rules + 1 exception"
    );
    let ios_text = fs::read_to_string(out.join("ios/scyde-00.json")).unwrap();
    assert!(
        !ios_text.contains("\\\\^"),
        "no literal \\^ may reach WebKit (ADR-0006)"
    );
    let ios: Vec<Value> = serde_json::from_str(&ios_text).unwrap();
    assert!(ios.iter().any(|r| {
        r["action"]["type"] == "ignore-previous-rules"
            && r["trigger"]["url-filter"]
                .as_str()
                .is_some_and(|f| f.contains("prebid-allowed"))
    }));
    assert!(ios.iter().any(|r| r["action"]["selector"] == ".ad-banner"));
    // engine.dat in the bundle decides like the source lists
    let s = shield::Shield::from_dat(&fs::read(out.join("engine.dat")).unwrap()).unwrap();
    assert!(s.should_block(
        "http://127.0.0.1/tracker/pixel.gif",
        "http://127.0.0.1/",
        shield::ResourceType::Image
    ));
}

#[test]
fn outputs_stay_within_platform_limits() {
    // 70k host rules → 3 DNR rulesets of ≤ 30k; iOS split at a small limit to keep the test fast.
    let list: String = (0..70_000).map(|i| format!("||h{i}.example^\n")).collect();
    let d = dnr::convert(&[&list], dnr::DnrLimits::default());
    assert_eq!(d.report.converted, 70_000);
    assert_eq!(d.rulesets.len(), 3);
    assert!(d.rulesets.iter().all(|r| r.len() <= dnr::GUARANTEED_RULES));
    let ids: std::collections::HashSet<u64> = d
        .rulesets
        .iter()
        .flatten()
        .map(|r| r["id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids.len(), 70_000, "rule ids are unique across rulesets");

    let small: String = (0..2_500).map(|i| format!("||h{i}.example^\n")).collect();
    let i = ios::convert(&[&small], 1_000);
    assert!(i.lists.iter().all(|l| l.len() <= 1_000));
    assert_eq!(i.lists.iter().map(Vec::len).sum::<usize>(), i.report.rules);
    assert_eq!(ios::MAX_PER_LIST, 150_000);
}

#[test]
fn signature_is_verified() {
    let key = test_key();
    let pk = key.verifying_key();
    let out = tmp("signed");
    let c = bundle::compile(&sources(), &Options::default(), &out, Some(&key)).unwrap();
    assert_eq!(c.manifest["key_id"], bundle::key_id(&pk));
    let m = bundle::verify(&out, &pk).expect("signed bundle verifies");
    assert!(m["files"]["engine.dat"]["sha256"].as_str().is_some());

    // Base64 round trips for the CI secret and brand/keys/filters.pub
    let k2 = bundle::signing_key_from_b64(&bundle::signing_key_to_b64(&key)).unwrap();
    assert_eq!(k2.verifying_key(), pk);
    assert_eq!(
        bundle::verifying_key_from_b64(&bundle::verifying_key_to_b64(&pk)).unwrap(),
        pk
    );

    // Wrong key
    let other = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]).verifying_key();
    assert!(matches!(
        bundle::verify(&out, &other),
        Err(BundleError::BadSignature)
    ));

    // Tampered payload file
    let p = out.join("cosmetic/generic.json");
    let orig = fs::read(&p).unwrap();
    fs::write(&p, b"{}").unwrap();
    assert!(
        matches!(bundle::verify(&out, &pk), Err(BundleError::HashMismatch(f)) if f == "cosmetic/generic.json")
    );
    fs::write(&p, orig).unwrap();

    // Tampered manifest (e.g. a hash swapped to match a malicious file)
    let mp = out.join("manifest.json");
    let m = fs::read_to_string(&mp).unwrap();
    fs::write(
        &mp,
        m.replace("\"version\": \"dev\"", "\"version\": \"evil\""),
    )
    .unwrap();
    assert!(matches!(
        bundle::verify(&out, &pk),
        Err(BundleError::BadSignature)
    ));

    // Unsigned rebuild removes the stale signature
    bundle::compile(&sources(), &Options::default(), &out, None).unwrap();
    assert!(matches!(
        bundle::verify(&out, &pk),
        Err(BundleError::Unsigned)
    ));
}

#[test]
fn bundles_are_reproducible() {
    let (a, b) = (tmp("repro-a"), tmp("repro-b"));
    let key = test_key();
    let ma = bundle::compile(&sources(), &Options::default(), &a, Some(&key))
        .unwrap()
        .manifest;
    let mb = bundle::compile(&sources(), &Options::default(), &b, Some(&key))
        .unwrap()
        .manifest;
    assert_eq!(ma, mb, "same input → same hashes (engine.dat included)");
    assert_eq!(
        fs::read(a.join("manifest.json.sig")).unwrap(),
        fs::read(b.join("manifest.json.sig")).unwrap()
    );
}
