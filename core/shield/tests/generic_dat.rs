//! Generic class/id cosmetics must survive the engine.dat round trip (Android loads the engine
//! only from engine.dat).
use std::collections::HashSet;

use shield::Shield;

const FIXTURE: &str = include_str!("../../../qa/lists/fixture.txt");

#[test]
fn generic_selectors_survive_engine_dat() {
    let built = Shield::from_lists(&[FIXTURE]);
    let none = HashSet::new();
    assert_eq!(
        built.hidden_selectors(&["ad-banner"], &[], &none),
        vec![".ad-banner".to_owned()]
    );
    let loaded = Shield::from_dat(&built.to_dat()).expect("engine.dat loads");
    assert_eq!(
        loaded.hidden_selectors(&["ad-banner"], &[], &none),
        vec![".ad-banner".to_owned()]
    );
}
