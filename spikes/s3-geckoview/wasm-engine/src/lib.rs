//! Spike S3: Shield decision engine for the GeckoView extension, compiled to WASM.
//! Thin wasm-bindgen wrapper over adblock-rust; the real crate lives in core/shield (Phase 1).

use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use adblock::Engine;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ShieldEngine {
    engine: Engine,
}

#[wasm_bindgen]
impl ShieldEngine {
    /// Builds an engine from newline-separated filter list text.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(rules: &str) -> ShieldEngine {
        let mut set = FilterSet::new(false);
        set.add_filter_list(rules.to_owned(), ParseOptions::default());
        ShieldEngine { engine: Engine::new_with_filter_set(set) }
    }

    /// Returns true if the request must be blocked.
    #[must_use]
    pub fn should_block(&self, url: &str, source_url: &str, request_type: &str) -> bool {
        match Request::new(url, source_url, request_type, "GET") {
            Ok(req) => self.engine.check_network_request(&req).should_block(),
            Err(_) => false,
        }
    }

    /// Cosmetic resources for a page URL as JSON (hide selectors, injected scriptlets).
    #[must_use]
    pub fn cosmetic(&self, url: &str) -> String {
        let r = self.engine.url_cosmetic_resources(url);
        serde_json::json!({
            "hide": r.hide_selectors,
            "generichide": r.generichide,
            "exceptions": r.exceptions,
            "injected_script_len": r.injected_script.len(),
        })
        .to_string()
    }

    /// Generic class/id selectors to hide, given classes and ids seen in the DOM.
    #[must_use]
    pub fn hidden_selectors(&self, classes: Vec<String>, ids: Vec<String>, exceptions_json: &str) -> Vec<String> {
        let exceptions: std::collections::HashSet<String> = serde_json::from_str(exceptions_json).unwrap_or_default();
        self.engine.hidden_class_id_selectors(&classes, &ids, &exceptions)
    }

    /// Serialized engine size, to estimate the `engine.dat` cost.
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        self.engine.serialize().len()
    }
}
