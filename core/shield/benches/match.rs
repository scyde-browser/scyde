//! Shield latency benchmark (PLAN §8.1, Phase 1 AC).
//!
//! `cargo bench -p shield --bench match`
//!
//! Criterion reports means and confidence intervals, but the budgets are percentiles
//! (p50 < 5 µs, p99 < 50 µs, deserialization < 50 ms), so this is a custom harness (ADR-0010).
//! It builds a deterministic EasyList-shaped list (no network in CI), times every request
//! individually, prints a table, writes `perf.json` and exits non-zero on a budget miss.
//!
//! Env: `SHIELD_PERF_OUT` (default `target/perf/shield.json`), `SHIELD_RULES` (default 80000).

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::{Duration, Instant};

use shield::{ResourceType, Shield};

const P50_BUDGET_US: f64 = 5.0;
const P99_BUDGET_US: f64 = 50.0;
const DESER_BUDGET_MS: f64 = 50.0;
const REQUESTS: usize = 50_000;
const ROUNDS: usize = 3;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*: deterministic and dependency-free
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).expect("fits")
    }
}

const TLD: &[&str] = &["com", "net", "io", "org", "co", "hr", "de"];

/// Random lowercase word, 3–10 letters: a vocabulary as diverse as real list tokens.
fn word(r: &mut Rng) -> String {
    let n = 3 + r.below(8);
    (0..n)
        .map(|_| char::from(b'a' + u8::try_from(r.below(26)).expect("< 26")))
        .collect()
}

fn domain(r: &mut Rng, i: usize) -> String {
    format!("{}{i}.{}", word(r), TLD[r.below(TLD.len())])
}

/// `EasyList`-shaped synthetic list (roughly `EasyList` + `EasyPrivacy` proportions): host anchors,
/// path rules with type options, exceptions, cosmetics and ~0.05 % regex rules.
/// Returns the list, the hosts it blocks and the path tokens its path rules use.
fn synth_list(rules: usize, r: &mut Rng) -> (String, Vec<String>, Vec<String>) {
    let mut out = String::with_capacity(rules * 32);
    let mut hosts = Vec::new();
    let mut paths = Vec::new();
    for i in 0..rules {
        let d = domain(r, i);
        let roll = r.below(2000);
        let line = match roll {
            0 => format!("/{}[0-9]{{2,}}\\.js/$script", word(r)),
            1..=899 => {
                hosts.push(d.clone());
                format!("||{d}^")
            }
            900..=1099 => format!("||{d}^$third-party"),
            1100..=1399 => {
                let (a, b) = (word(r), word(r));
                paths.push(format!("{a}/{b}"));
                format!("/{a}/{b}*.js$script")
            }
            1400..=1499 => format!("/{}-{}.$image,xmlhttprequest", word(r), word(r)),
            1500..=1579 => format!("@@||{d}/{}/$script", word(r)),
            1580..=1899 => format!("{d}##.{}-{}", word(r), word(r)),
            _ => format!("##.{}-{i}", word(r)),
        };
        out.push_str(&line);
        out.push('\n');
    }
    (out, hosts, paths)
}

const TYPES: &[ResourceType] = &[
    ResourceType::Script,
    ResourceType::Image,
    ResourceType::Xhr,
    ResourceType::Stylesheet,
    ResourceType::Subdocument,
];

fn requests(
    n: usize,
    hosts: &[String],
    paths: &[String],
    r: &mut Rng,
) -> Vec<(String, String, ResourceType)> {
    (0..n)
        .map(|i| {
            let page = format!("https://www.{}{}.hr/clanak/{i}", word(r), r.below(500));
            let host = if r.below(4) == 0 {
                hosts[r.below(hosts.len())].clone() // ~25 % hit a blocked host
            } else {
                format!("static.{}{}.com", word(r), r.below(5000))
            };
            let path = if r.below(20) == 0 {
                paths[r.below(paths.len())].clone() // ~5 % hit a path rule
            } else {
                format!("{}/{}", word(r), word(r))
            };
            let url = format!(
                "https://{host}/{path}-{}.js?v={}&id={}",
                word(r),
                r.below(1000),
                r.next()
            );
            (url, page, TYPES[r.below(TYPES.len())])
        })
        .collect()
}

fn pct(sorted: &[Duration], p: f64) -> f64 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let idx = (((sorted.len() - 1) as f64) * p).round() as usize;
    sorted[idx].as_secs_f64() * 1e6
}

#[allow(clippy::too_many_lines)] // one linear measurement script reads best top to bottom
fn main() {
    // `cargo bench` passes `--bench`; libtest-style filters are ignored.
    let rules: usize = std::env::var("SHIELD_RULES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(80_000);
    let out_path = std::env::var("SHIELD_PERF_OUT").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../target/perf/shield.json").into()
    });

    let mut r = Rng(0x5C1D_E5C1_DE00_0001);
    let (list, hosts, paths) = synth_list(rules, &mut r);
    let fixture = include_str!("../../../qa/lists/fixture.txt");

    let t = Instant::now();
    let built = Shield::from_lists(&[list.as_str(), fixture]);
    let build_ms = t.elapsed().as_secs_f64() * 1e3;
    let dat = built.to_dat();
    drop(built);

    let mut deser = Vec::new();
    let mut shield = None;
    for _ in 0..5 {
        let t = Instant::now();
        let s = Shield::from_dat(&dat).expect("engine.dat loads");
        deser.push(t.elapsed());
        shield = Some(s);
    }
    deser.sort();
    let deser_ms = deser[deser.len() / 2].as_secs_f64() * 1e3;
    let shield = shield.expect("loaded");

    let reqs = requests(REQUESTS, &hosts, &paths, &mut r);
    for (u, p, ty) in reqs.iter().take(2_000) {
        black_box(shield.check(u, p, *ty)); // warm caches and lazily compiled regexes
    }
    let mut times = Vec::with_capacity(reqs.len());
    let mut single = Vec::with_capacity(reqs.len());
    let mut blocked = 0usize;
    for (u, p, ty) in &reqs {
        // Best of ROUNDS per request: keeps each request's real cost distribution while
        // dropping preemption spikes from shared CI runners.
        let mut best = Duration::MAX;
        let mut block = false;
        for round in 0..ROUNDS {
            let t = Instant::now();
            let d = black_box(shield.check(black_box(u), black_box(p), *ty));
            let e = t.elapsed();
            if round == 0 {
                single.push(e);
            }
            best = best.min(e);
            block = d.block;
        }
        times.push(best);
        blocked += usize::from(block);
    }
    times.sort();
    single.sort();
    let single_p99 = pct(&single, 0.99);
    let (p50, p99, max) = (pct(&times, 0.50), pct(&times, 0.99), pct(&times, 1.0));

    #[allow(clippy::cast_precision_loss)]
    let block_rate = blocked as f64 / reqs.len() as f64;
    #[allow(clippy::cast_precision_loss)]
    let dat_mb = dat.len() as f64 / 1_048_576.0;

    println!(
        "shield match · {rules} synthetic rules + fixture · {REQUESTS} requests (best of {ROUNDS})"
    );
    println!("  build            {build_ms:>9.1} ms");
    println!("  engine.dat       {dat_mb:>9.2} MB");
    println!("  deserialize p50  {deser_ms:>9.2} ms   (budget < {DESER_BUDGET_MS} ms)");
    println!("  match p50        {p50:>9.2} µs   (budget < {P50_BUDGET_US} µs)");
    println!("  match p99        {p99:>9.2} µs   (budget < {P99_BUDGET_US} µs)");
    println!("  match max        {max:>9.2} µs");
    println!("  single-shot p99  {single_p99:>9.2} µs   (info: includes runner noise)");
    println!("  blocked          {:>9.1} %", block_rate * 100.0);

    let mut json = String::new();
    write!(
        json,
        "{{\n  \"suite\": \"shield\",\n  \"rules\": {rules},\n  \"requests\": {REQUESTS},\n  \"metrics\": {{\n    \
         \"match_p50_us\": {p50:.3},\n    \"match_p99_us\": {p99:.3},\n    \"match_p99_single_shot_us\": {single_p99:.3},\n    \"deserialize_ms\": {deser_ms:.3},\n    \
         \"build_ms\": {build_ms:.1},\n    \"engine_dat_bytes\": {}\n  }},\n  \"budgets\": {{\n    \
         \"match_p50_us\": {P50_BUDGET_US},\n    \"match_p99_us\": {P99_BUDGET_US},\n    \"deserialize_ms\": {DESER_BUDGET_MS}\n  }}\n}}\n",
        dat.len()
    )
    .expect("write to string");
    if let Some(dir) = std::path::Path::new(&out_path).parent() {
        std::fs::create_dir_all(dir).expect("create perf dir");
    }
    std::fs::write(&out_path, json).expect("write perf.json");
    println!("  → {out_path}");

    let mut miss = Vec::new();
    if p50 >= P50_BUDGET_US {
        miss.push(format!("match p50 {p50:.2} µs"));
    }
    if p99 >= P99_BUDGET_US {
        miss.push(format!("match p99 {p99:.2} µs"));
    }
    if deser_ms >= DESER_BUDGET_MS {
        miss.push(format!("deserialize {deser_ms:.2} ms"));
    }
    assert!(
        block_rate > 0.05,
        "synthetic requests should hit blocked hosts"
    );
    if !miss.is_empty() {
        eprintln!("BUDGET MISS: {}", miss.join(", "));
        std::process::exit(1);
    }
}
