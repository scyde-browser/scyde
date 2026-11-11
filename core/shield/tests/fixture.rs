//! `qa/lists/fixture.txt` must produce exactly the decisions `qa/adblock/index.html` expects.

use shield::{ResourceType as T, Shield};

const FIXTURE: &str = include_str!("../../../qa/lists/fixture.txt");
const PAGE: &str = "http://127.0.0.1:8787/adblock/index.html";

fn shield() -> Shield {
    Shield::from_lists(&[FIXTURE])
}

fn url(path: &str) -> String {
    format!("http://127.0.0.1:8787/adblock/{path}")
}

#[test]
fn network_decisions() {
    let s = shield();
    let blocked = [
        ("ad-network/ad.js", T::Script),
        ("ad-network/banner.gif", T::Image),
        ("tracker/pixel.gif", T::Image),
        ("analytics/collect.js", T::Script),
        ("cdn/prebid-v8.js", T::Script),
        ("ad-network/frame.html", T::Subdocument),
    ];
    for (path, ty) in blocked {
        assert!(
            s.should_block(&url(path), PAGE, ty),
            "{path} must be blocked"
        );
    }
    let allowed = s.check(&url("cdn/prebid-allowed.js"), PAGE, T::Script);
    assert!(!allowed.block, "exception must win");
    assert!(allowed.excepted);
    assert!(
        !s.should_block(PAGE, PAGE, T::Document),
        "page itself stays"
    );
    // Type options are honoured: ad-network rule is $script,image only.
    assert!(!s.should_block(&url("ad-network/data.json"), PAGE, T::Xhr));
}

#[test]
fn cosmetic_decisions() {
    let s = shield();
    for host in ["127.0.0.1", "localhost", "10.0.2.2"] {
        let c = s.cosmetics(&format!("http://{host}:8787/adblock/index.html"));
        assert!(
            c.hide_selectors.contains(&".scyde-specific-ad".to_owned()),
            "{host}"
        );
        assert!(!c.generichide);
    }
    let other = s.cosmetics("https://example.com/");
    assert!(
        !other
            .hide_selectors
            .contains(&".scyde-specific-ad".to_owned())
    );

    let c = s.cosmetics(PAGE);
    let generic = s.hidden_selectors(
        &["ad-banner", "content"],
        &["sponsor-top", "keep"],
        &c.exceptions,
    );
    assert!(generic.contains(&".ad-banner".to_owned()));
    // Attribute selectors are not class/id keyed: they arrive with the page's cosmetics.
    assert!(
        c.hide_selectors.iter().any(|g| g.contains("sponsor-")),
        "{:?}",
        c.hide_selectors
    );
    assert!(
        !generic
            .iter()
            .any(|g| g.contains("content") || g.contains("keep"))
    );
}

#[test]
fn engine_dat_round_trip_keeps_decisions() {
    let dat = shield().to_dat();
    let s = Shield::from_dat(&dat).expect("load engine.dat");
    assert!(s.should_block(&url("tracker/pixel.gif"), PAGE, T::Image));
    assert!(!s.should_block(&url("cdn/prebid-allowed.js"), PAGE, T::Script));
    assert!(
        s.cosmetics(PAGE)
            .hide_selectors
            .contains(&".scyde-specific-ad".to_owned())
    );
}

#[test]
fn scriptlets_from_resources() {
    use std::fmt::Write as _;
    // base64("(function(){window.__scyde_scriptlet='{{1}}'})();")
    let js = "(function(){window.__scyde_scriptlet='{{1}}'})();";
    let b64 = base64(js.as_bytes());
    let mut json = String::new();
    write!(
        json,
        r#"[{{"name":"scyde-mark.js","aliases":[],"kind":{{"mime":"application/javascript"}},"content":"{b64}"}}]"#
    )
    .unwrap();
    let mut s = Shield::from_lists(&["example.com##+js(scyde-mark, on)"]);
    assert_eq!(s.use_resources_json(&json).unwrap(), 1);
    let c = s.cosmetics("https://example.com/");
    assert!(
        c.injected_script.contains("__scyde_scriptlet"),
        "{}",
        c.injected_script
    );
    assert!(s.use_resources_json("{").is_err());
}

fn base64(input: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}
