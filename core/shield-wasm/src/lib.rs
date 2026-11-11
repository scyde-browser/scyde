//! Shield engine for the `GeckoView` built-in extension (ADR-0004).
//!
//! A thin `wasm-bindgen` layer over [`shield::Shield`]: the background script loads `engine.dat`
//! from the filter bundle (fast start), answers `webRequest` decisions and serves cosmetics to
//! content scripts. Strings cross the boundary as JSON to keep the generated glue small.

use shield::{ResourceType, Shield};
use wasm_bindgen::prelude::*;

/// One Shield engine instance in the extension's background page.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ShieldEngine {
    inner: Shield,
}

#[wasm_bindgen]
impl ShieldEngine {
    /// Loads `engine.dat` from a filter bundle (the normal path).
    ///
    /// # Errors
    /// The `ShieldError` message; the extension then falls back to [`ShieldEngine::from_lists`].
    #[wasm_bindgen(js_name = fromDat)]
    pub fn from_dat(bytes: &[u8]) -> Result<ShieldEngine, JsError> {
        Shield::from_dat(bytes)
            .map(|inner| ShieldEngine { inner })
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Builds an engine from filter list text (fallback and tests).
    #[wasm_bindgen(js_name = fromLists)]
    #[must_use]
    pub fn from_lists(text: &str) -> ShieldEngine {
        ShieldEngine {
            inner: Shield::from_lists(&[text]),
        }
    }

    /// Loads scriptlet/redirect resources (`resources.json` from the bundle); returns their count.
    ///
    /// # Errors
    /// If the JSON does not describe resources.
    #[wasm_bindgen(js_name = useResources)]
    pub fn use_resources(&mut self, json: &str) -> Result<usize, JsError> {
        self.inner
            .use_resources_json(json)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// `true` if the request must be cancelled. `request_type` is a `webRequest` type
    /// (`main_frame`, `sub_frame`, `script`, `xmlhttprequest`, …).
    #[wasm_bindgen(js_name = shouldBlock)]
    #[must_use]
    pub fn should_block(&self, url: &str, source_url: &str, request_type: &str) -> bool {
        self.inner
            .should_block(url, source_url, ResourceType::from_host_name(request_type))
    }

    /// Full decision as JSON: `{"block":bool,"excepted":bool,"redirect":str|null,"rewrittenUrl":str|null}`.
    #[must_use]
    pub fn check(&self, url: &str, source_url: &str, request_type: &str) -> String {
        let d = self
            .inner
            .check(url, source_url, ResourceType::from_host_name(request_type));
        serde_json::json!({
            "block": d.block,
            "excepted": d.excepted,
            "redirect": d.redirect,
            "rewrittenUrl": d.rewritten_url,
        })
        .to_string()
    }

    /// Page cosmetics as JSON:
    /// `{"hide":[…],"procedural":[…],"exceptions":[…],"script":"…","generichide":bool}`.
    #[must_use]
    pub fn cosmetics(&self, page_url: &str) -> String {
        let c = self.inner.cosmetics(page_url);
        let mut exceptions: Vec<&String> = c.exceptions.iter().collect();
        exceptions.sort_unstable();
        serde_json::json!({
            "hide": c.hide_selectors,
            "procedural": c.procedural_actions,
            "exceptions": exceptions,
            "script": c.injected_script,
            "generichide": c.generichide,
        })
        .to_string()
    }

    /// Generic selectors for classes and ids seen in the DOM (JSON arrays in, JSON array out).
    /// `exceptions_json` is the `exceptions` array from [`ShieldEngine::cosmetics`].
    #[wasm_bindgen(js_name = hiddenSelectors)]
    #[must_use]
    pub fn hidden_selectors(
        &self,
        classes_json: &str,
        ids_json: &str,
        exceptions_json: &str,
    ) -> String {
        let parse = |s: &str| serde_json::from_str::<Vec<String>>(s).unwrap_or_default();
        let (classes, ids) = (parse(classes_json), parse(ids_json));
        let exceptions = parse(exceptions_json).into_iter().collect();
        let c: Vec<&str> = classes.iter().map(String::as_str).collect();
        let i: Vec<&str> = ids.iter().map(String::as_str).collect();
        serde_json::to_string(&self.inner.hidden_selectors(&c, &i, &exceptions))
            .unwrap_or_else(|_| "[]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../qa/lists/fixture.txt");
    const PAGE: &str = "http://127.0.0.1:8787/adblock/index.html";

    #[test]
    fn decisions_and_cosmetics_cross_the_boundary_as_json() {
        let e = ShieldEngine::from_lists(FIXTURE);
        assert!(e.should_block(
            "http://127.0.0.1:8787/adblock/ad-network/frame.html",
            PAGE,
            "sub_frame"
        ));
        assert!(!e.should_block(
            "http://127.0.0.1:8787/adblock/cdn/prebid-allowed.js",
            PAGE,
            "script"
        ));
        let d: serde_json::Value = serde_json::from_str(&e.check(
            "http://127.0.0.1:8787/adblock/cdn/prebid-allowed.js",
            PAGE,
            "script",
        ))
        .unwrap();
        assert_eq!(d["excepted"], true);
        let c: serde_json::Value = serde_json::from_str(&e.cosmetics(PAGE)).unwrap();
        assert!(
            c["hide"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s == ".scyde-specific-ad")
        );
        let g = e.hidden_selectors(r#"["ad-banner"]"#, "[]", &c["exceptions"].to_string());
        assert_eq!(g, r#"[".ad-banner"]"#);
        assert_eq!(e.hidden_selectors("not json", "[]", "[]"), "[]");
    }

    #[test]
    fn loads_engine_dat() {
        let dat = Shield::from_lists(&[FIXTURE]).to_dat();
        let e = ShieldEngine::from_dat(&dat).unwrap();
        assert!(e.should_block(
            "http://127.0.0.1:8787/adblock/tracker/pixel.gif",
            PAGE,
            "image"
        ));
    }
}
