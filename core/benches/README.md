# Benchmarks

| Bench | Command | Budgets | Output |
|---|---|---|---|
| Shield match + engine.dat load | `cargo bench -p shield --bench match` | p50 < 5 µs, p99 < 50 µs, deserialize < 50 ms | `target/perf/shield.json` |
| Shield WASM (V8) | `shield-wasm/build.sh && node shield-wasm/bench.mjs target/shield-wasm/node` | p99 < 200 µs, ≤ 1000 KB gz | `target/perf/shield-wasm.json` |

The Shield bench is a percentile harness, not criterion (ADR-0010). It exits non-zero on a budget miss.
Env: `SHIELD_RULES` (default 80000), `SHIELD_PERF_OUT`.
