# Nunjucks parity backlog

Living checklist of **what runjucks still needs** to match Nunjucks behavior. Pull work items from here; mark `[x]` when shipped. See [`README.md`](README.md) Status for the high-level summary.

**How to read priorities:**

| Tier | Meaning |
|------|---------|
| **P0** | Unblocks real-world templates; ship first |
| **P1** | Common Nunjucks usage; important for adoption |
| **P2** | Ecosystem / DX; nice-to-have for parity |
| **P3** | Advanced / async; explicitly deferred |

---

## Summary

| Category | P0 | P1 | P2 | P3 |
|----------|----|----|----|----|
| **Tags** | — | — | — | `asyncEach`, `asyncAll`, `ifAsync` |
| **Expressions** | `addFilter` wiring | `addGlobal` + `range`/`cycler`/`joiner`, `callable` test | `throwOnUndefined`, Jinja slices | |
| **Filters** | `safe`, `default` (full), `batch` | `first`, `last`, `sort`, `reverse`, `trim`, `int`, `float`, `string`, `title`, `truncate`, `striptags`, `urlencode`, `indent`, `nl2br`, `sum`, `wordcount` | `groupby`, `dictsort`, `select`/`reject`/`*attr`, `center`, `dump`, `forceescape`, `list`, `random`, `slice`, `urlize` | |
| **Node API** | `addFilter` functional | `addGlobal`, `configure` basics | `compile`, `getTemplate`, `render(name)`, filesystem loader, Express | async `render`, `addExtension`, precompile, browser build |
| **Conformance** | fix 3 skipped render cases | grow `tag_parity_cases.json` | expand perf allowlist | |

---

## Tags

### Implemented (parser + renderer)

`if`/`elif`/`else`/`endif`, `for`/`else`/`endfor` (single, multi-var, k/v, `loop.*`), `switch`/`case`/`default`/`endswitch`, `set`/`endset` (multi-target, block capture, frame scoping), `include` (expression, `ignore missing`), `import`/`from` (top-level macros; see Partial), `extends` (full expression, evaluated at render; see Partial for static cycle tracing), `block`/`endblock`, `macro`/`endmacro`, `{{ super() }}` (multi-level `extends`), `{% call %}…{% endcall %}` / `caller()`, `{% filter %}…{% endfilter %}`.

### Not yet implemented

- [x] **`{% raw %}`/`{% endraw %}`, `{% verbatim %}`/`{% endverbatim %}`** — **P0** (shipped)
  Lexer balances nested raw/verbatim like Nunjucks; parser maps opening/`Text`/closing to `Node::Text`. Ref: [`lexer.rs` `LexerMode::Raw`](native/crates/runjucks-core/src/lexer.rs), [`template.rs` `parse_raw_block`](native/crates/runjucks-core/src/parser/template.rs).

- [x] **`{% import "x" as ns %}`, `{% from "x" import a, b %}`** — **P0** (shipped)
  Top-level `{% macro %}` only; `ns.macro()` via [`renderer.rs` `macro_namespaces`](native/crates/runjucks-core/src/renderer.rs). Literal import-graph cycle detection; dynamic template paths work for load but are not traced for cycles. Ref: [`template.rs`](native/crates/runjucks-core/src/parser/template.rs) `parse_import_stmt` / `parse_from_stmt`.

- [x] **`{{ super() }}`** — **P0** (shipped)
  Per-block body chains built in [`renderer.rs` `build_block_chains`](native/crates/runjucks-core/src/renderer.rs); `RenderState::super_context` + `Expr::Call` `super()` render the next layer. Intermediate layout roots skip `{% extends %}` during render so multi-level inheritance works. Ref: [`super_call_filter.rs`](native/crates/runjucks-core/tests/super_call_filter.rs).

- [x] **`{% call %}…{% endcall %}` / `caller`** — **P1** (shipped)
  [`Node::CallBlock`](native/crates/runjucks-core/src/ast.rs); macro invocation parsed as `Expr::Call`; `caller_stack` + `caller()` in [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs). **Partial:** Nunjucks `{% call(a, b) macro(x) %}` caller parameters are not supported (phase 2).

- [x] **`{% filter name %}…{% endfilter %}`** — **P1** (shipped)
  [`Node::FilterBlock`](native/crates/runjucks-core/src/ast.rs); body rendered to string then [`filters::apply_builtin`](native/crates/runjucks-core/src/filters.rs) like `Expr::Filter`. Filter name plus optional parenthesized arguments in the opening tag.

- [x] **Dynamic `{% extends expr %}`** — **P2** (shipped)
  [`Node::Extends`](native/crates/runjucks-core/src/ast.rs) carries [`Expr`](native/crates/runjucks-core/src/ast.rs); [`render_entry`](native/crates/runjucks-core/src/renderer.rs) and [`build_block_chains`](native/crates/runjucks-core/src/renderer.rs) evaluate it with the current context (same idea as dynamic `include`). Quoted paths remain valid as string literals. **Partial:** unlike `{% import %}`/`{% from %}`, dynamic extends targets are not traced for literal-only cycle analysis before render.

- [ ] **`{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}`** — **P3**
  Async iteration / conditional. Keywords exist; no parser, renderer, or runtime model. Would require async render pipeline or documented non-goal.

### Partial

- **`{% macro %}`**: header parsing does not support **default argument values** or keyword args (`bar="default"`). Macro body renders to string; no `SafeString` handling.
- **`{% include %}`**: no `with context` / `without context` modifiers (Nunjucks `parseFrom` supports these for `{% from %}` and docs mention them for include in some builds).
- **`{% import %}` / `{% from %}`**: `with context` / `without context` are parsed and stored but **not** applied to macro extraction (Nunjucks runs `getExported` with merged context); only top-level macros are collected, not side effects from running the imported template.
- **`{% extends %}`**: dynamic parent names are resolved at render time only; **literal-only** static cycle detection is not extended to extends (cycles are still caught when the resolved name repeats during `build_block_chains`).

---

## Expressions & runtime

### Implemented

`Literal`, `Variable`, `GetAttr`, `GetItem`, `Call` (macros, `super()`, `caller()`), `Filter`, `List`, `Dict`, `InlineIf`, `Unary`, `Binary`, `Compare`, `is` tests (`defined`, `equalto`, `sameas`, `null`/`none`, `falsy`, `truthy`, `number`, `string`, `lower`, `upper`).

### Not yet implemented

- [ ] **`Expr::Call` for non-macro callees** — **P1**
  Currently errors with `"only template macro calls are supported"`. Needed for **globals** (`range()`, `cycler()`, `joiner()`) and any user-registered callables. Requires a function registry on `Environment` or `RenderState`.

- [ ] **`addGlobal` / default globals (`range`, `cycler`, `joiner`)** — **P1**
  Nunjucks [`globals.js`](../nunjucks/nunjucks/src/globals.js) provides `range(start, stop?, step?)`, `cycler(...)`, `joiner(sep?)`. No support today; needs `Environment`-level global map + `Expr::Call` generalization.

- [ ] **`is callable` test** — **P1** (Partial)
  Always returns `false`. Should return `true` for macros, global functions, and (if `addFilter` is wired) JS callbacks.

- [ ] **Unknown `is` tests silently return `false`** — **P2**
  Nunjucks throws on unknown tests. Consider erroring or adding `addTest` API.

- [ ] **`throwOnUndefined`** — **P2**
  Nunjucks `Environment` option; undefined variables throw instead of rendering empty. Not surfaced on runjucks `Environment`.

- [ ] **Jinja-compat array slices** — **P2**
  `arr[1:4]`, `arr[::2]`, etc. Tested in Nunjucks `jinja-compat.js`; not parsed in [`expr.rs`](native/crates/runjucks-core/src/parser/expr.rs).

---

## Filters

### Implemented (11 + 1 alias)

`upper`, `lower`, `length`, `join`, `replace`, `round`, `escape` / `e`, `default`, `abs`, `capitalize`.

### Not yet implemented

**P0:**
- [ ] **`safe`** — marks output as not-to-be-escaped; needs a `SafeString` or equivalent in the value model.
- [ ] **`default`** (full semantics) — Nunjucks also triggers on `false` and `""` with `boolean=true` second arg. Current impl is null-only. Conformance case `tests_js_filter_default_undefined` is skipped because of this.
- [ ] **`batch`** — splits an array into chunks; conformance case `tests_js_for_batch` is skipped.

**P1:**
- [ ] `first`
- [ ] `last`
- [ ] `sort`
- [ ] `reverse`
- [ ] `trim`
- [ ] `int`
- [ ] `float`
- [ ] `string`
- [ ] `title`
- [ ] `truncate`
- [ ] `striptags`
- [ ] `urlencode`
- [ ] `indent`
- [ ] `nl2br`
- [ ] `sum`
- [ ] `wordcount`
- [ ] `d` (alias of `default` — trivial once `default` is full)

**P2:**
- [ ] `groupby`
- [ ] `dictsort`
- [ ] `select` / `selectattr`
- [ ] `reject` / `rejectattr`
- [ ] `center`
- [ ] `dump`
- [ ] `forceescape`
- [ ] `list`
- [ ] `random`
- [ ] `slice`
- [ ] `urlize`

---

## Node / NAPI API

### Implemented

`renderString` (top-level + `Environment`), `Environment` class (`setAutoescape`, `setDev`, `addFilter` stub, `setTemplateMap`, `renderTemplate`).

### Not yet implemented

**P0:**
- [ ] **`addFilter` functional** — currently a no-op in [`runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs). Needs a way to call a JS function from Rust render pipeline (NAPI `ThreadsafeFunction` or sync callback). This is the single biggest JS API gap for real-world adoption.

**P1:**
- [ ] **`addGlobal(name, value)`** — register a value or function accessible as a template variable.
- [ ] **`configure(opts)` basics** — at minimum `autoescape`, `throwOnUndefined`, `tags` (custom delimiters), `trimBlocks`, `lstripBlocks`.

**P2:**
- [ ] **`compile(src)` / `getTemplate(name)`** — return a compiled template object with `.render(ctx)` for caching.
- [ ] **`render(name, ctx)`** — top-level name-based render (sugar over `getTemplate` + `.render`).
- [ ] **Filesystem loader** — `FileSystemLoader` or equivalent; today only `setTemplateMap` (in-memory).
- [ ] **Express integration** — `env.express(app)` / `app.set('view engine', 'njk')`.

**P3:**
- [ ] **Async render** — callback / promise-based `render`, `renderString`.
- [ ] **`addExtension`** — custom tag extensions.
- [ ] **`precompile` / `precompileString`** — ahead-of-time compilation.
- [ ] **Browser build** — UMD / ESM bundle for browser use.
- [ ] **`installJinjaCompat()`** — Jinja2 compatibility shim.

---

## Conformance & perf

### Current state

- **34** conformance cases across `render_cases.json` (28) + `filter_cases.json` (6).
- **3** render cases skipped in [`conformance.rs`](native/crates/runjucks-core/tests/conformance.rs):
  - `tests_js_filter_default_undefined` — needs full `default` filter semantics.
  - `tests_js_for_batch` — needs `batch` filter.
  - `tests_js_set_and_output` — needs `set` + `escape` + autoescape interplay.
- **4** additional cases in [`tag_parity_cases.json`](native/fixtures/conformance/tag_parity_cases.json), run by `tag_parity.rs`.
- **31** cases in the [perf allowlist](perf/conformance-allowlist.json) (matches the non-skipped conformance set).

### Next steps

- [ ] **Fix 3 skipped render cases** — **P0**: implement `default` (full), `batch`, fix `set`+escape. Un-skip and add to perf allowlist.
- [ ] **Grow `tag_parity_cases.json`** — **P1**: add vectors for `switch` fall-through, `for`+`loop.*`, block `set`, `include ignore missing`, nested scoping.
- [ ] **Expand perf allowlist** — **P2**: as new conformance cases go green, append IDs and verify perf harness still runs.

---

## Out of scope / non-goals (current)

- Full **async render pipeline** (P3; document as intentional limitation).
- **Nunjucks browser precompile** workflow (different target; runjucks is Node-native).
- Exact **`undefined` vs `null`** JS semantics in Rust's `serde_json::Value` (both map to `Value::Null`; document the trade-off).

---

## References

- Vendored Nunjucks: [`../nunjucks/nunjucks/src/`](../nunjucks/nunjucks/src/) (compiler, parser, filters, globals, runtime).
- Nunjucks upstream tests: [`../nunjucks/tests/compiler.js`](../nunjucks/tests/compiler.js).
- Conformance fixtures: [`native/fixtures/conformance/`](native/fixtures/conformance/).
- Perf harness: [`perf/README.md`](perf/README.md).
- Runjucks Rust core: [`native/crates/runjucks-core/src/`](native/crates/runjucks-core/src/).
- NAPI bindings: [`native/crates/runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs).
