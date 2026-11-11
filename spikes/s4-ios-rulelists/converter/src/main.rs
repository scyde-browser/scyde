//! Spike S4: converts filter lists into WebKit content-blocker JSON, split into
//! chunks of at most `--max` rules (WebKit limit is 150k per list).
//! Usage: scyde-cb-convert-spike --out <dir> [--max 150000] [--repeat N] <list.txt>...
//! `--repeat` duplicates the rule set with unique url-filter suffixes to reach a target size for stress tests.

use adblock::lists::{FilterSet, ParseOptions};
use std::{env, fs, path::PathBuf};

fn main() {
    let mut args = env::args().skip(1);
    let mut out = PathBuf::from("rulelists");
    let mut max = 150_000usize;
    let mut repeat = 1usize;
    let mut lists = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = PathBuf::from(args.next().expect("--out <dir>")),
            "--max" => max = args.next().expect("--max <n>").parse().expect("number"),
            "--repeat" => repeat = args.next().expect("--repeat <n>").parse().expect("number"),
            _ => lists.push(a),
        }
    }
    assert!(!lists.is_empty(), "pass at least one list");

    let mut set = FilterSet::new(true);
    for l in &lists {
        set.add_filter_list(fs::read_to_string(l).expect("read list"), ParseOptions::default());
    }
    let (rules, used_filters) = set.into_content_blocking().expect("debug filter set");
    let mut json: Vec<serde_json::Value> = rules.iter().map(|r| serde_json::to_value(r).unwrap()).collect();
    let base = json.len();
    if repeat > 1 {
        // Stress mode: clone network rules with a unique query marker so WebKit cannot dedupe them.
        let originals = json.clone();
        for n in 1..repeat {
            for v in &originals {
                let mut v = v.clone();
                if let Some(f) = v.pointer_mut("/trigger/url-filter") {
                    if let Some(s) = f.as_str() {
                        *f = serde_json::Value::String(format!("{s}.*scyde-stress-{n}"));
                    }
                }
                json.push(v);
            }
        }
    }

    fs::create_dir_all(&out).unwrap();
    let mut manifest = Vec::new();
    for (i, chunk) in json.chunks(max).enumerate() {
        let name = format!("scyde-{i:02}.json");
        let body = serde_json::to_string(chunk).unwrap();
        fs::write(out.join(&name), &body).unwrap();
        manifest.push(serde_json::json!({ "file": name, "rules": chunk.len(), "bytes": body.len() }));
    }
    let summary = serde_json::json!({
        "lists": lists,
        "filters_used": used_filters.len(),
        "cb_rules_base": base,
        "cb_rules_total": json.len(),
        "max_per_list": max,
        "chunks": manifest,
    });
    fs::write(out.join("manifest.json"), serde_json::to_string_pretty(&summary).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}
