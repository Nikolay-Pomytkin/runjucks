# P3 deferred product tracks

Runjucks is **Node-first** with a **synchronous** Rust core. The following items are **out of scope** for routine P1/P2 parity work unless a product decision commits engineering time. See [NUNJUCKS_PARITY.md](NUNJUCKS_PARITY.md) for the full matrix.

## Async rendering

- **API:** Nunjucks-style `render` / `renderString` with callbacks or Promises.
- **Tags:** `{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}` (keywords may exist in the lexer only).
- **Requirements:** An async-capable pipeline (or worker offload) and clear semantics for JS `addFilter` / `addGlobal` callables with async.

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

Revisit P3 when there is a **concrete consumer** (e.g. “must run templates in the browser” or “must match Nunjucks async CMS”) and capacity for multi-sprint work.
