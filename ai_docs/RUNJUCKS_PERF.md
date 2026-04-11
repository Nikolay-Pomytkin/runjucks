# Runjucks performance backlog

**Audience:** maintainers optimizing throughput and memory. For **users**, see [Performance](docs/src/content/docs/guides/performance.mdx) (practical tips, published snapshot), [JavaScript API](docs/src/content/docs/guides/javascript-api.md) (caching behavior), and [Limitations](docs/src/content/docs/guides/limitations.md). This file is the **engineering plan** for profiling, hot-path work, and what *not* to chase; it is not the public product doc.

**Related:** [NUNJUCKS_PARITY.md](NUNJUCKS_PARITY.md) (behavior vs Nunjucks), [FFI_OVERHEAD_PROFILE_2026-04-11.md](FFI_OVERHEAD_PROFILE_2026-04-11.md) (Node/Rust boundary: profiling notes, **napi-rs** research, prioritized ingress ideas), [perf/README.md](perf/README.md) (Node harness), [native/crates/runjucks-core/benches/render_hotspots.rs](native/crates/runjucks-core/benches/render_hotspots.rs) (Rust-only **render** Criterion benches), [native/crates/runjucks-core/benches/parse_hotspots.rs](native/crates/runjucks-core/benches/parse_hotspots.rs) (**lex + parse** only).

---

## Priorities

| Tier | Meaning |
|------|---------|
| **P0** | Already shipped or baseline measurement — must not regress |
| **P1** | High impact / clear profiling signal — do next |
| **P2** | Meaningful but larger refactors or niche wins |
| **P3** | Experimental, product-shaped, or low ROI on typical Node workloads |
| **Defer** | Explicitly out of scope for the core engine (see **Non-goals**) |

---

## Executive summary

**Correctness vs Nunjucks** is enforced by [`__test__/parity.test.mjs`](../__test__/parity.test.mjs) (and JSON goldens), not by this perf harness. `perf/run.mjs` measures throughput only; it skips fixtures marked `compareWithNunjucks: false` (Runjucks-only goldens).

Runjucks time is spent in roughly four buckets:

1. **Lex + parse** — mitigated by **parsed-template caching** on [`Environment`](native/crates/runjucks-core/src/environment.rs) (inline + named, `ParseSignature` invalidation) and **per-`Template` AST** in [runjucks-napi](native/crates/runjucks-napi/src/lib.rs).
2. **Render / eval** — [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs): `CtxStack`, loops, expressions, filters, string building.
3. **Node boundary** — N-API: JSON context marshalling, `Mutex<Environment>` lock per call, optional JS callbacks (filters/globals). Maintainer-oriented breakdown and napi-rs-oriented ideas: [FFI_OVERHEAD_PROFILE_2026-04-11.md](FFI_OVERHEAD_PROFILE_2026-04-11.md).
4. **Comparison to Nunjucks** — upstream runs hot templates in **V8**; Runjucks pays for **Rust + serde_json::Value + FFI**. **5–10× vs Nunjucks on every microbench is not a realistic global bar**; target **worst rows** on [perf/run.mjs](perf/run.mjs) and **Criterion** deltas on Rust-only benches.

---

## P0 — Shipped baseline (do not regress)

| Area | What | Where |
|------|------|--------|
| **Inline parse cache** | `ahash::AHasher` hash of source bytes + `ParseSignature` → `Arc<Node>`; hit path still checks stored source `==` lookup `&str` | [`environment.rs`](native/crates/runjucks-core/src/environment.rs): `parse_or_cached_inline`, `hash_source`, `inline_parse_cache` (`Arc<Mutex<HashMap<…>>>` for `Send`/`Sync`) |
| **Named parse cache** | `TemplateLoader::cache_key` + loaded source equality; map loader returns `Some(name)` | [`loader.rs`](native/crates/runjucks-core/src/loader.rs), [`load_and_parse_named`](native/crates/runjucks-core/src/environment.rs) |
| **Nested composition** | Include / extends / import / from / scan_literal_import_graph use cache | [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) |
| **NAPI `Template` AST** | `cached_ast` + `render_parsed` / `parse_or_cached_inline` | [`runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs) |
| **Clear named cache on map replace** | `set_template_map` → `clear_named_parse_cache` | NAPI `set_template_map` |
| **Perf harness fairness** | Fixture `env` (trim/lstrip/tags/templateMap/globals/jinja compat) | [`perf/harness-env.mjs`](perf/harness-env.mjs), [`perf/run.mjs`](perf/run.mjs) |
| **Warm vs cold** | `--cold` for fresh `Environment` per iteration | [`perf/run.mjs`](perf/run.mjs) |
| **Loop object reuse** | In-place update of `loop` `Map` after first iteration | [`inject_loop` / `fill_loop_object`](native/crates/runjucks-core/src/renderer.rs) |
| **Variable output borrow path** | `resolve_variable_ref` + `get_ref`; `eval_for_output` for `Expr::Variable` | [`environment.rs`](native/crates/runjucks-core/src/environment.rs), [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) |
| **String `reserve` heuristics** | `render_for`, `render_children`, `render_output`, root `Node::Root` | [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) |
| **Rust microbenches (render)** | Criterion: `for_200_int_concat`, `for_200_loop_index_and_item`, `many_vars_80`, `nested_for_small`, `literal_string_upper_filter`, `attr_chain_three_depth`, `literal_string_length_filter`, `variable_chained_upper_lower_filters`, `variable_trim_upper_filters`, `variable_trim_capitalize_filters` | [`benches/render_hotspots.rs`](native/crates/runjucks-core/benches/render_hotspots.rs) |
| **Rust microbenches (parse)** | `tokenize_*`, `parse_cold_*`, `parse_vs_render_*` — cold lex+parse vs `render_parsed` | [`benches/parse_hotspots.rs`](native/crates/runjucks-core/benches/parse_hotspots.rs), `npm run bench:rust:parse` |
| **Lexer / parser pre-capacity** | `tokenize` output `Vec` heuristic; root `nodes` in `parse_template_tokens` | [`lexer.rs`](native/crates/runjucks-core/src/lexer.rs), [`parser/template.rs`](native/crates/runjucks-core/src/parser/template.rs) |
| **Variable `upper` / `lower` / `length` fast path** | Plain `Expr::Variable` input + no args + no custom filter → `resolve_variable_ref` + dispatch (mirrors literal fast path semantics) | [`eval_to_value` / `eval_for_output` → `Expr::Filter`](native/crates/runjucks-core/src/renderer.rs) |
| **`is` test borrow (empty args)** | Plain variable LHS + no test args → `resolve_variable_ref` + `apply_is_test` without an extra `eval_to_value` clone | [`BinOp::Is`](native/crates/runjucks-core/src/renderer.rs) |
| **JSON / Buffer ingress** | `renderStringFromJson` / `renderStringFromJsonBuffer`; default `runjucks-napi` build uses **`simd-json`** (`fast-json`). `--no-default-features` → `serde_json` parse only | [`runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs) |
| **`Node::Text` backing** | `Arc<str>` per segment; render copies into output string once | [`ast.rs`](native/crates/runjucks-core/src/ast.rs), [`parser/template.rs`](native/crates/runjucks-core/src/parser/template.rs) |
| **`CtxStack` slots** | `ahash::AHashMap<String, Arc<Value>>` per frame (fast string-key lookup); `inject_loop` uses `Arc::make_mut` | [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) (`CtxStack`, `inject_loop`) |
| **Unary + `resolve_variable_ref`** | Plain `Expr::Variable` under unary `+` / `-` / `not` avoids extra `Value` clone where applicable | [`eval_to_value`](native/crates/runjucks-core/src/renderer.rs) |
| **Literal `upper` / `lower` fast path** | String literal + no args + no custom filter → direct case mapping | [`eval_to_value` → `Expr::Filter`](native/crates/runjucks-core/src/renderer.rs) |
| **Literal `length` fast path** | String or array literal + no args + no custom `length` → `chars().count()` / `len()` without full filter dispatch | [`eval_to_value` / `eval_for_output` → `Expr::Filter`](native/crates/runjucks-core/src/renderer.rs) |
| **`GetAttr` / `GetItem` borrow paths** | `x.y` / `x[i]` / `x[lo:hi]` when `x` is a plain `Expr::Variable` (not import namespace) use `resolve_variable_ref` + field/index/slice without cloning the whole container | [`eval_to_value`](native/crates/runjucks-core/src/renderer.rs) |
| **Dotted chains on variables** | `foo.bar.baz` (pure `.` chain on one identifier) resolves in one walk: one `resolve_variable_ref` + successive object gets — fewer recursive `eval_to_value` calls than nested `GetAttr` | [`collect_attr_chain_from_getattr`](native/crates/runjucks-core/src/renderer.rs) |
| **Output filter fast path** | `eval_for_output` mirrors literal `upper` / `lower` / `length` (same guards as `eval_to_value`) so `{{ … }}` skips extra work when applicable | [`eval_for_output`](native/crates/runjucks-core/src/renderer.rs) |

**Tests:** [`tests/cache_correctness.rs`](native/crates/runjucks-core/tests/cache_correctness.rs), [`tests/perf_regressions.rs`](native/crates/runjucks-core/tests/perf_regressions.rs) (P1 hot-path edge cases), Node [`__test__/render.test.mjs`](__test__/render.test.mjs).

---

## P1 — Next optimizations (remaining / follow-on)

First P1 tranche is **shipped** (see **P0** and **Changelog**). [`tests/perf_regressions.rs`](native/crates/runjucks-core/tests/perf_regressions.rs) covers edge cases for those changes.

### 1. Extend borrow / `&Value` paths further (remaining)

**Problem:** Unary-on-variable and output paths are done; some hot shapes still clone more than necessary (nested filters, chained `GetAttr`, etc.).

**Shipped in this track:** `Expr::GetAttr` when `base` is a plain variable (non–import namespace); **full dotted chains** `a.b.c…` on a non–import root collapse to one walk; `Expr::GetItem` when `base` is a variable and the index is a literal number or string, or when the index is a slice (bounds evaluated first, then variable base borrows the array for `jinja_slice_array`); **`CtxStack` frames use `ahash::AHashMap`** for faster variable lookups.

**How to apply:**

- Audit [`eval_to_value`](native/crates/runjucks-core/src/renderer.rs) and call sites for **`Expr::Variable`**, simple member/index chains, and **`is defined` / `callable`** fast paths that already only need presence or `&Value`.
- Add **`Environment::resolve_variable_ref`** (or scoped helpers) anywhere we currently do `resolve_variable` + immediate read without mutation.
- **Filters / tests:** [`filters.rs`](native/crates/runjucks-core/src/filters.rs) and `apply_is_test` in [`environment.rs`](native/crates/runjucks-core/src/environment.rs) — pass `&Value` into builtins where signatures allow; clone only when the filter mutates or returns a new value.

**Risk:** Behavior must stay byte-compatible with existing tests and Nunjucks parity.

**Verify:** `cargo test -p runjucks_core`, `node --test __test__/parity.test.mjs`, Criterion before/after.

---

### 2. (Shipped) `Arc<Value>` frames, `Node::Text` `Arc<str>`, literal filter fast path

**Done:** `CtxStack` uses [`HashMap<String, Arc<Value>>`](native/crates/runjucks-core/src/renderer.rs); `Node::Text` is [`Arc<str>`](native/crates/runjucks-core/src/ast.rs); `upper` / `lower` on string literals bypass full filter dispatch when no custom filter is registered.

**Optional later:** single `Arc<str>` pool per file for fragments; more literal specializations; sorted-key cache for object `for` (usually **P2**).

---

### 3. Reduce `flatten()` allocation for extensions

**Problem:** [`Node::ExtensionTag`](native/crates/runjucks-core/src/renderer.rs) uses `stack.flatten()` → full `Map` clone for `process(context, …)`.

**Shipped:** [`RenderState::extension_context_cache`](native/crates/runjucks-core/src/renderer.rs) reuses the merged `flatten()` context when both **`stack_identity`** (call-shape fingerprint of [`CtxStack`](native/crates/runjucks-core/src/renderer.rs)) **and** [`CtxStack::revision`](native/crates/runjucks-core/src/renderer.rs) match the cached snapshot (e.g. adjacent extension tags with no binding changes). `flatten()` pre-allocates map capacity.

**Further (optional):**

- **Lazy** snapshot: pass **`&CtxStack`** + a read-only facade — **breaking** for Rust `CustomExtensionHandler` unless versioned.
- **Breaking** for custom extension handlers in Rust — likely **P2** unless profiling shows extension-heavy workloads dominate.

---

## P2 — Larger or situational wins

| Idea | Notes |
|------|--------|
| **`serde_json` alternatives** | `sonic-rs`, `simd-json` for context parsing on N-API boundary — only if JSON decode shows up in profiles |
| **PGO / BOLT** | Profile-guided optimization on `render_hotspots` binary — build pipeline complexity |
| **Smaller `loop` representation** | Store `loop.*` in [`RenderState`](native/crates/runjucks-core/src/renderer.rs) as plain `usize`/`bool` fields; build `Value` **only if** template reads `loop` (requires use-analysis or lazy proxy — hard) |
| **NAPI `RwLock<Environment>`** | Theoretical concurrent readers; **typical Node sync render is single-threaded** — **low priority**; see [perf/README.md — Mutex vs RwLock](perf/README.md) |
| **Expand perf allowlist** | More `tag_parity` / filter IDs in [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json) as parity stays green |

### P2 execution notes (in-repo)

| Track | Status |
|-------|--------|
| **Context-boundary probe** | [`perf/context-boundary.mjs`](perf/context-boundary.mjs) + `npm run perf:context` — same template, small vs large nested context (isolates N-API + JSON materialization vs Rust render). |
| **PGO** | **Documented** in [`perf/README.md`](perf/README.md) (instrument → train → `llvm-profdata` → `profile-use`); not CI-gated. |
| **Faster JSON (`sonic-rs`, etc.)** | **`simd-json`** is **default** for JSON-context ingress in `runjucks-napi`; further parsers only if profiles justify. |
| **Extension `flatten()`** | [`CtxStack::revision`](native/crates/runjucks-core/src/renderer.rs) bumps on frame/set/set_local/inject_loop; [`RenderState::extension_context_cache`](native/crates/runjucks-core/src/renderer.rs) reuses merged `Value` when **stack identity and revision** both match (adjacent extension tags with no binding change). [`CtxStack::flatten`](native/crates/runjucks-core/src/renderer.rs) uses `Map::with_capacity`. |
| **`for` + `loop`** | Criterion [`for_200_loop_index_and_item`](native/crates/runjucks-core/benches/render_hotspots.rs) for baseline; **skipping** `inject_loop` when the body does not reference `loop` is **not** shipped (nested `loop` / shadowing makes static analysis error-prone). |

### Tier D / P3 decision gate

Do **not** start a **full** [`serde_json::Value`](https://docs.rs/serde_json) replacement or **zero-copy** JS context (P3) until:

1. **Ingress** is shown to dominate: [`perf/context-boundary.mjs`](perf/context-boundary.mjs) reports a large **large − small** mean ms delta **and** Criterion microbenches indicate the Rust core is **not** the main cost, **or**
2. P2 ingress/extension work is **done** and profiling still shows **allocation / clone** hot spots **inside** `runjucks_core`.

Until then, treat P3 rows as **research** only. See [`Non-goals`](#non-goals-same-spirit-as-parity-doc) on replacing `Value` across the engine.

---

## P3 — Experimental / product-level

- **Zero-copy or pooled context** from JS into Rust (custom N-API types, not plain `serde_json::Value`).
- **Background compilation** of templates (async — conflicts with current sync contract).
- **Embedding a second “fast” subset** of the language for known-safe templates.

---

## Non-goals (same spirit as parity doc)

- **Nunjucks-style async baseline in `perf/run.mjs`** — upstream’s async API differs from sync `renderString`; async throughput is measured separately ([`perf/run-async.mjs`](perf/run-async.mjs), `npm run perf:async`). True parallel `asyncAll` / worker-thread template execution — not a goal (Runjucks runs async tags **sequentially** for deterministic output).
- **Multithreaded rendering** of a **single** template instance — correctness and Nunjucks semantics are inherently sequential for sync templates.
- **Browser precompile / bytecode bundle** — Runjucks is Node-native N-API today.
- **Replacing `serde_json::Value` entirely** across the engine — only as a last resort after measured plateau on P1–P2.

---

## Measurement playbook

**Interpreting `npm run perf`:** The harness reports **mean latency** per case (no p95 / allocation stats). Rows with `SKIP` are excluded from **`avg nj/rj`**, which is a simple arithmetic mean of `nunjucks_ms / runjucks_ms` over non-skipped cases only (ratios can hide bimodal rows). Skip reasons include parity mismatch, render errors, fixtures with `compareWithNunjucks: false`, and missing allowlist fixtures.

**Machine-readable output:** `node perf/run.mjs --json` (or `npm run perf:json`) writes [`perf/last-run.json`](perf/last-run.json) with per-row timings and `summary.avg_nj_over_rj`.

**FFI / napi-rs focus:** When isolating `serde_json::Value::from_napi_value` vs Rust render cost, use **`perf:context`** and read [FFI_OVERHEAD_PROFILE_2026-04-11.md](FFI_OVERHEAD_PROFILE_2026-04-11.md) (local snapshots, named-template path, research synthesis with external links).

**Coverage:** Default throughput cases are [`perf/synthetic.mjs`](perf/synthetic.mjs) (including optional `renderMode: 'template'` + `templateName` for `renderTemplate` / Nunjucks `env.render`) plus allowlisted conformance vectors. This does not stress `setLoaderRoot` / `setLoaderCallback`; use app-level profiling for those.

| Step | Command / artifact |
|------|---------------------|
| **Release native addon** | From `runjucks/`: `npm run build` (not `build:debug`) |
| **Node vs Nunjucks** | `npm run perf` — uses [`harness-env.mjs`](perf/harness-env.mjs); optional `npm run perf:cold` |
| **N-API context sizing** | `npm run perf:context` — [`context-boundary.mjs`](perf/context-boundary.mjs) |
| **Async render (Runjucks only)** | `npm run perf:async` — [`perf/run-async.mjs`](perf/run-async.mjs) benches `renderStringAsync` / `renderTemplateAsync`; cases in [`asyncSyntheticCases`](perf/synthetic.mjs) / [`asyncSyncParityCases`](perf/synthetic.mjs). **No Nunjucks row** — compare sync vs async overhead on the same template via `*_async_over_sync` ratio rows, or track regressions on `async_synth_*` rows. Optional `npm run perf:async:json` → [`perf/last-run-async.json`](perf/last-run-async.json). |
| **Rust-only render cost** | `cd native && cargo bench -p runjucks_core --bench render_hotspots` or `npm run bench:rust` from package root |
| **Rust-only parse cost** | `cd native && cargo bench -p runjucks_core --bench parse_hotspots` or `npm run bench:rust:parse` |
| **Flamegraph (Linux)** | `cargo install flamegraph && cd native && cargo flamegraph --bench render_hotspots -p runjucks_core` (same for `parse_hotspots`) |
| **Regression gate** | `npm test`, `cargo test --manifest-path native/Cargo.toml -p runjucks_core`, `node --test __test__/parity.test.mjs` |
| **Fast JSON parse (maintainers)** | Default: `simd-json` via `fast-json`. To force **`serde_json`** only: `cargo build -p runjucks-napi --no-default-features` |

**Interpretation:** `nj/rj` in [perf/run.mjs](perf/run.mjs) is **Nunjucks mean / Runjucks mean**; **`> 1`** means Runjucks faster. Compare **release** builds only. V8 vs Rust + FFI means **some rows will stay &lt; 1×** even when the engine is healthy.

---

## File index (quick navigation)

| Concern | Primary files |
|---------|----------------|
| Parse cache, signature | [`environment.rs`](native/crates/runjucks-core/src/environment.rs) |
| Loader cache key | [`loader.rs`](native/crates/runjucks-core/src/loader.rs) |
| Render, stack, loops, output | [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) |
| Filters | [`filters.rs`](native/crates/runjucks-core/src/filters.rs) |
| AST | [`ast.rs`](native/crates/runjucks-core/src/ast.rs) |
| Parser | [`parser/`](native/crates/runjucks-core/src/parser/) |
| NAPI, `Template`, env lock, `renderStringFromJson` | [`runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs) |
| **Node/Rust FFI notes** (ingress, callbacks, napi-rs docs) | [`FFI_OVERHEAD_PROFILE_2026-04-11.md`](FFI_OVERHEAD_PROFILE_2026-04-11.md) |
| Perf harness | [`perf/run.mjs`](perf/run.mjs), [`perf/run-async.mjs`](perf/run-async.mjs), [`perf/harness-env.mjs`](perf/harness-env.mjs), [`perf/synthetic.mjs`](perf/synthetic.mjs), [`perf/context-boundary.mjs`](perf/context-boundary.mjs) |
| P1 regression tests | [`tests/perf_regressions.rs`](native/crates/runjucks-core/tests/perf_regressions.rs) |

---

## Local measurement snapshot (2026-04-11)

**Setup:** macOS, `cargo bench --manifest-path native/Cargo.toml -p runjucks_core --bench …`, release. Criterion’s `change:` lines compare to whatever was last stored under `native/target/criterion/`; treat the **absolute `time:`** band as the comparable signal when variance or stale baselines dominate.

### `render_hotspots`

| Benchmark | Time (typical band from one run) |
|-----------|----------------------------------|
| `for_200_int_concat` | ~67–70 µs |
| `for_200_var_plus_context_int` | ~74 µs |
| `for_200_loop_index_and_item` | ~98–101 µs |
| `many_vars_80` | ~15–16 µs |
| `nested_for_small` | ~4.0 µs |
| `literal_string_upper_filter` | ~1.24–1.33 µs |
| `attr_chain_three_depth` | ~1.84–1.94 µs |
| `literal_string_length_filter` | ~1.21–1.24 µs |
| `variable_chained_upper_lower_filters` | ~1.57–1.67 µs |
| `variable_trim_upper_filters` | ~1.49–1.51 µs |
| `variable_trim_capitalize_filters` | ~1.55 µs |
| `variable_lower_title_filters` | ~1.67–1.68 µs |

### `parse_hotspots`

| Benchmark | Time (typical band) |
|-----------|---------------------|
| `tokenize_large_plain_400_lines` | ~336–353 µs |
| `tokenize_many_interpolations_80` | ~10.3–10.4 µs |
| `parse_cold_large_plain` | ~335–353 µs |
| `parse_cold_many_interpolations` | ~10.5–11.1 µs |
| `parse_cold_heavy_nested_for` | ~195–204 µs |
| `parse_vs_render_cold_render_string_for200` | ~74–74.5 µs |
| `parse_vs_render_render_only_for200` | ~77–88 µs (noisy) |

### Follow-up tries (not shipped)

- **`fill_loop_object` via `Map::get_mut`:** Measured a large regression on `for_200_loop_index_and_item` (~+30–50% vs `insert` on current `serde_json` `Map`); keep `insert` for loop fields.
- **`hash_source` with `ahash::AHasher`:** Full-bench run showed mixed Criterion `change:`; isolated `many_vars_80` did not reproduce a clear win — left on `DefaultHasher` until a dedicated microbench proves a gain.

---

## Changelog (maintainers)

When you ship a perf track, add a one-line bullet with **date + PR/ref + area** (e.g. “2025-03 — P1: `Arc` frame values for hot paths”).

- *Initial backlog authored from parse-cache + renderer micro-opts + harness work.*
- **2026-03 — P1 tranche:** `Node::Text` → `Arc<str>` (parse once, render via `to_string()` output copy); unary `+`/`-`/`not` on plain variables use `resolve_variable_ref` where possible; `|upper` / `|lower` fast path for string literals when no custom filter overrides; `CtxStack` frames use `HashMap<String, Arc<Value>>` with `Arc::make_mut` in `inject_loop`; Criterion bench `literal_string_upper_filter` in [`render_hotspots.rs`](native/crates/runjucks-core/benches/render_hotspots.rs).
- **2026-03 —** [`tests/perf_regressions.rs`](native/crates/runjucks-core/tests/perf_regressions.rs): integration tests for unicode literal filters, custom `upper` vs fast path, filter chains, unary/`{% set %}`/`loop` edge cases tied to the P1 tranche.
- **2026-03 — P1 follow-on:** `GetAttr` / `GetItem` (literal index + slice) on plain variables use `resolve_variable_ref` where possible; literal `|length` on string/array literals; `eval_for_output` fast paths for literal `upper` / `lower` / `length` (same custom-filter overrides as `eval_to_value`).
- **2026-03 — P1 follow-on (2):** `CtxStack` frame maps → `ahash::AHashMap` (faster variable lookups; see Criterion `many_vars_80`); dotted attribute chains on a plain variable root use a single `collect_attr_chain_from_getattr` walk; new Criterion cases `attr_chain_three_depth`, `literal_string_length_filter`. User-facing **Performance** guide: [`docs/src/content/docs/guides/performance.mdx`](docs/src/content/docs/guides/performance.mdx).
- **2026-03 — P2 plan:** Node [`context-boundary.mjs`](perf/context-boundary.mjs) + `npm run perf:context`; PGO steps in [`perf/README.md`](perf/README.md); `CtxStack::revision` + extension merged-context cache + `flatten` pre-capacity; Criterion `for_200_loop_index_and_item`; Tier D/P3 **decision gate** in P2 section; faster JSON deferred pending profiles.
- **2026-03 — Compile/render perf track:** Criterion [`parse_hotspots`](native/crates/runjucks-core/benches/parse_hotspots.rs) + `perf/README.md` table (parse vs render); lexer `tokenize` + template parser root `Vec` pre-capacity; `|upper` / `|lower` / `|length` on **plain variables** via `resolve_variable_ref`; `is` tests with **no args** borrow the LHS variable when it is a bare identifier; NAPI [`renderStringFromJson`](native/crates/runjucks-napi/src/lib.rs) + optional **`fast-json`** (`simd-json`) for Rust-side JSON parse; Node [`__test__/json-ingress.test.mjs`](__test__/json-ingress.test.mjs).
- **2026-03 — Perf plan execution (local):** `perf:context` on this machine showed ~0.03 ms **large − small** context delta vs ~0.05 ms small-context mean — meaningful **relative** cost but small in absolute ms; **Tier-D ingress work** (broader JSON / `sonic-rs` on main `renderString` path) **deferred** per decision gate until profiles show ingress dominates Criterion render cost. Core: fused **`upper` / `lower` / `length`** chains on variables and literals (one `resolve_variable_ref` / leaf where valid); `is` tests with **args** borrow `LHS` when it is a bare variable (`Cow` + `apply_is_test` on `&Value`); Criterion `variable_chained_upper_lower_filters`; Node harness supports `renderMode: 'template'` synthetics ([`perf/run.mjs`](perf/run.mjs)); measurement playbook clarifies `--json`, `avg nj/rj`, skip rules.
- **2026-03 — Follow-on:** Fused chains allow **`trim`** and **`capitalize`** with other string filters; **`length` must be last** in the fused chain (any interleaving of `upper`/`lower`/`trim`/`capitalize` before that — fixes `trim`→`upper` order that the old validator rejected). [`chain_capitalize_like_builtin`](native/crates/runjucks-core/src/filters.rs) dedupes `apply_builtin` `capitalize`. Criterion: `variable_trim_upper_filters`, `variable_trim_capitalize_filters`. Synthetics `synth_var_trim_upper`, `synth_var_trim_capitalize` ([`perf/synthetic.mjs`](perf/synthetic.mjs)). Single-comparison `a == b` uses `resolve_variable_ref` for a plain variable LHS with RHS evaluated first to satisfy borrow rules.
- **2026-03 — Async throughput:** [`perf/run-async.mjs`](perf/run-async.mjs) + `npm run perf:async` / `perf:async:json`; [`asyncSyntheticCases`](perf/synthetic.mjs) / [`asyncSyncParityCases`](perf/synthetic.mjs) — Runjucks-only (no Nunjucks baseline); optional sync-vs-async ratio row for the same `for` template.
- **2026-04 — Named-template cache + FFI path:** `TemplateLoader::cache_key_cow` so map loaders can return `Cow::Borrowed(name)` and avoid per-render key allocations on parse-cache hits; follow-on skips redundant reloads when named-template cache keys are stable; extension merged-context cache invalidated by **`stack_identity` + `CtxStack::revision`** (not revision alone). Details: [`FFI_OVERHEAD_PROFILE_2026-04-11.md`](FFI_OVERHEAD_PROFILE_2026-04-11.md).
- **2026-04-11 — Measurement round:** Criterion snapshot + negative experiments recorded in **Local measurement snapshot** above (`fill_loop_object` `get_mut` path; `AHasher` for `hash_source`). No engine change merged from that pass.
- **2026-04-11 — Shipped:** Inline parse-cache key hashing [`hash_source`](native/crates/runjucks-core/src/environment.rs) uses **`ahash::AHasher`** on UTF-8 bytes (still `String` equality on cache hit for correctness). `Expr::Call` **`joiner()`** handle path uses **`resolve_variable_ref`** + `parse_joiner_id` on `&Value` (sync + async). **Measured** (Criterion, release, same session): `many_vars_80` ~**1.5%** faster vs prior median (~13.49 µs → ~13.29 µs); new microbench `joiner_set_and_three_calls` ~**18–24%** faster with the borrow path (~2.29 µs → ~1.87 µs). Other rows within noise.
- **2026-04-11 — Node / FFI ingress:** [`renderStringFromJsonBuffer`](native/crates/runjucks-napi/src/lib.rs) + `Environment#renderStringFromJsonBuffer` (UTF-8 JSON as `Buffer` / `Uint8Array`). [`perf/context-boundary.mjs`](../perf/context-boundary.mjs) now benchmarks **object** vs **JSON string** vs **JSON Buffer** on the same large context; sample run: **~1.58×** vs `renderString` for JSON string, **~1.60×** for Buffer (large context ~1 KiB JSON, template unchanged).
- **2026-04-11 —** `runjucks-napi` **default** features now include **`fast-json`** (`simd-json` for JSON context parse). Use **`cargo build -p runjucks-napi --no-default-features`** for `serde_json`-only parse.
