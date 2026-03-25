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

A release build of the `.node` binary is required; otherwise results are meaningless.

## What it measures

- **Synthetic** templates in [`synthetic.mjs`](synthetic.mjs) (size / loops / filters).
- **Conformance subset** via IDs in [`conformance-allowlist.json`](conformance-allowlist.json), loaded through [`__test__/conformance/load-fixtures.mjs`](../__test__/conformance/load-fixtures.mjs) (same vectors as Rust + Node): `render_cases.json`, `filter_cases.json`, and `tag_parity_cases.json`.

Each case:

1. Renders once with both engines (with `structuredClone` context) and checks **identical output** between runjucks and Nunjucks.
2. For allowlisted fixtures, also checks output matches the JSON `expected` field.
3. Runs **[tinybench](https://github.com/tinylibs/tinybench)** with warmup + timed iterations; prints mean latency (ms) per engine and **nj/rj** (Nunjucks mean / Runjucks mean).

Interpretation: **nj/rj > 1** means Nunjucks is slower on average for that case (Runjucks faster). Values **&lt; 1** mean Runjucks was slower.

**`npm run perf:json`** writes [`last-run.json`](last-run.json) (gitignored) with per-case latencies and skip reasons; useful for comparing runs on one machine, not for CI gates.

## Fairness notes

- Nunjucks uses `new nunjucks.Environment(null, { autoescape })` aligned to each case’s `env.autoescape` when present (default **on**, matching Runjucks).
- Context is **cloned** every iteration so neither engine can rely on in-place mutation across calls.

## Not in CI

These numbers are **machine- and load-dependent**. This script is for **local** comparison only; it is **not** wired into GitHub Actions as a gate.

## Maintaining the allowlist

When parity improves, add fixture `id`s to [`conformance-allowlist.json`](conformance-allowlist.json). If a case starts failing the parity check, the runner **skips** it and prints a reason—remove or fix the fixture before re-adding.
