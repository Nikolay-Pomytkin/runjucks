# P3 deferred product tracks

Runjucks is **Node-first** with a **synchronous** Rust core for the default path. The following items are **out of scope** for routine P1/P2 parity work unless a product decision commits engineering time. See [NUNJUCKS_PARITY.md](NUNJUCKS_PARITY.md) for the full matrix.

## Async rendering

**Shipped (Node NAPI + `runjucks_core` with `async` feature):**

- **API:** `renderStringAsync`, `renderTemplateAsync`; `addAsyncFilter`, `addAsyncGlobal` for Promise-returning JS callables used from async templates.
- **Tags:** `{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}` — parsed and rendered on the async pipeline (`async_renderer` in Rust).
- **Semantics:** `asyncAll` executes iterations **sequentially** (same observable order as input; not parallel workers). See tests in [`__test__/async-render.test.mjs`](__test__/async-render.test.mjs) and [`native/crates/runjucks-core/tests/async_renderer.rs`](native/crates/runjucks-core/tests/async_renderer.rs).

**Still deferred / different from upstream Nunjucks:**

- **Callback-style** `render(name, ctx, cb)` without Promises — Runjucks exposes **async functions** returning Promises instead.
- **True parallel** `asyncAll` — not planned; sequential semantics are intentional.
- **Perf:** [`perf/run.mjs`](perf/run.mjs) remains sync-only vs Nunjucks; async throughput is [`perf/run-async.mjs`](perf/run-async.mjs) (`npm run perf:async`) — no Nunjucks baseline row for async-only templates.

## Precompile / precompiled loader

- **Upstream:** `precompile` / `precompileString` emit JavaScript that runs against Nunjucks’ JS runtime.
- **Runjucks:** Core parses to a Rust AST — a precompile story would imply **WASM**, **embedded bytecode**, or **codegen**, not a thin NAPI wrapper.

## Browser / WASM

- **WASM target** for `runjucks-core` + minimal JS glue (`renderString`-style).
- **Bundle size**, **startup**, and **loader** story (fetch vs embedded strings) need a dedicated spike before any “browser support” claim.

## `installJinjaCompat()`-style API

- **Optional** no-op or thin re-export for migration from apps that call Nunjucks’s shim.
- **Slices** and most Jinja-like syntax already work without a compat install in Runjucks.

## When to revisit

Revisit remaining P3 tracks when there is a **concrete consumer** (e.g. “must run templates in the browser”, **precompiled** bundles, or **callback**-only Nunjucks APIs) and capacity for multi-sprint work.
