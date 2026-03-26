# Performance harness (runjucks vs Nunjucks)

This folder benchmarks **Node.js** rendering: the `runjucks` native addon vs the **`nunjucks` npm package** (pinned in root `devDependencies`).

## Run

From the package root (`runjucks/`):

```bash
npm run build
npm run perf
# optional: machine-readable output for local trend logs (gitignored)
npm run perf:json
```

A **release** build of the `.node` binary is required (`npm run build`, not `build:debug`); otherwise Rust hot loops are massively skewed and comparisons to Nunjucks are meaningless.

## Rust-only microbenches (Criterion)

The Node harness includes JSON marshalling and N-API. For **pure renderer** throughput, use Criterion from the workspace root:

```bash
cd native && cargo bench -p runjucks_core --bench render_hotspots
```

Scenarios mirror [`synthetic.mjs`](synthetic.mjs): 200-iteration `{% for %}`, 80 interpolations, nested `for`.

**Profiling (Linux, `perf`):** after `cargo install flamegraph`, from `native/`:

```bash
cargo flamegraph --bench render_hotspots -p runjucks_core
```

On macOS you can use Instruments or sample the same bench binary. Expected hotspots before tuning were: per-iteration `loop` object construction, `serde_json::Value` cloning on variable reads, and string buffer growth — the core now reuses the `loop` map in place, borrows context/globals for bare `{{ var }}` output, and **reserves** accumulation buffers where cheap heuristics exist.

Optional npm alias from package root: `npm run bench:rust`.

## What it measures

- **Synthetic** templates in [`synthetic.mjs`](synthetic.mjs) (size / loops / filters).
- **Conformance subset** via IDs in [`conformance-allowlist.json`](conformance-allowlist.json), loaded through [`__test__/conformance/load-fixtures.mjs`](../__test__/conformance/load-fixtures.mjs) (same vectors as Rust + Node): `render_cases.json`, `filter_cases.json`, and `tag_parity_cases.json`.

Each case:

1. Renders once with both engines (with `structuredClone` context) and checks **identical output** between runjucks and Nunjucks.
2. For allowlisted fixtures, also checks output matches the JSON `expected` field.
3. Runs **[tinybench](https://github.com/tinylibs/tinybench)** with warmup + timed iterations; prints mean latency (ms) per engine and **nj/rj** (Nunjucks mean / Runjucks mean).

Interpretation: **nj/rj > 1** means Nunjucks is slower on average for that case (Runjucks faster). Values **&lt; 1** mean Runjucks was slower.

**Warm environment:** The harness builds **one** `runjucks.Environment` per case and reuses it for the timed loop (same as Nunjucks’ reuse of compiled templates). That exercises the **cached parse** path for repeated `renderString` — the intended steady-state for hot paths.

**Cold parse (optional):** Pass **`--cold`** to measure Runjucks with a **fresh** `Environment` each iteration (full lex+parse every time). Nunjucks is unchanged. Use this to see parse overhead in isolation; headline numbers without `--cold` are “warm cache” semantics.

**`npm run perf:json`** writes [`last-run.json`](last-run.json) (gitignored) with per-case latencies and skip reasons; useful for comparing runs on one machine, not for CI gates.

## Fairness notes

- **Environment options match conformance fixtures:** [`run.mjs`](run.mjs) builds each engine with [`harness-env.mjs`](harness-env.mjs) — the same logic as [`__test__/parity.test.mjs`](../__test__/parity.test.mjs): `trimBlocks` / `lstripBlocks`, custom `tags`, `templateMap` loaders, `globals`, `randomSeed`, and (for Jinja-style slice cases) `nunjucks.installJinjaCompat()` while measuring. Older versions of the harness only toggled `autoescape`, which **skipped** most tag-parity cases and skewed numbers.
- Nunjucks uses `new nunjucks.Environment(loader?, opts)` with the same flags and optional template-map loader as Runjucks’ `setTemplateMap`.
- Context is **cloned** every iteration so neither engine can rely on in-place mutation across calls.

## Mutex vs RwLock (N-API)

The addon keeps a **`Mutex<Environment>`** around the Rust `Environment`. Node runs rendering on a single thread; an `RwLock` was not adopted — uncontended mutex cost is negligible, and migration would touch every mutating N-API method without proven gain on realistic workloads.

## Not in CI

These numbers are **machine- and load-dependent**. This script is for **local** comparison only; it is **not** wired into GitHub Actions as a gate.

## Maintaining the allowlist

When parity improves, add fixture `id`s to [`conformance-allowlist.json`](conformance-allowlist.json). If a case starts failing the parity check, the runner **skips** it and prints a reason—remove or fix the fixture before re-adding.
