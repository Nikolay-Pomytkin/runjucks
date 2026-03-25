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
| **Expressions** | — | — | `throwOnUndefined`, Jinja slices | |
| **Filters** | — | — | `select` / `reject` (dynamic tests), `random` | |
| **Node API** | — | — | `compile`, `getTemplate`, `render(name)`, filesystem loader, Express | async `render`, `addExtension`, precompile, browser build |
| **Conformance** | — | more vectors from upstream tests | optional perf CI artifact | |

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

`Literal`, `Variable`, `GetAttr`, `GetItem`, `Call` (macros, `super()`, `caller()`, built-in globals below), `Filter`, `List`, `Dict`, `InlineIf`, `Unary`, `Binary`, `Compare`, `is` tests (`defined`, `equalto`, `sameas`, `null`/`none`, `falsy`, `truthy`, `number`, `string`, `lower`, `upper`, `callable`).

- [x] **`addGlobal` / default globals (`range`, `cycler`, `joiner`)** — **P1** (shipped)
  [`Environment::globals`](native/crates/runjucks-core/src/environment.rs) + [`Environment::add_global`](native/crates/runjucks-core/src/environment.rs); [`Environment::resolve_variable`](native/crates/runjucks-core/src/environment.rs) matches Nunjucks context-over-globals lookup. Defaults in [`globals::default_globals_map`](native/crates/runjucks-core/src/globals.rs). **`range`**: [`globals::builtin_range`](native/crates/runjucks-core/src/globals.rs). **`cycler` / `joiner`**: opaque handles + [`RenderState::cyclers` / `joiners`](native/crates/runjucks-core/src/renderer.rs); `c.next()`, `j()` dispatch in [`Expr::Call`](native/crates/runjucks-core/src/renderer.rs).

- [x] **`Expr::Call` for non-macro callees (built-in globals)** — **P1** (shipped)
  Same paths as above; arbitrary user callables from JSON are not invoked—only markers (`__runjucks_builtin`, `__runjucks_callable`) for `is callable` / registry. **Partial:** Nunjucks-style user-defined functions from the npm/NAPI layer are a follow-up.

- [x] **`is callable` test** — **P1** (shipped, partial)
  [`eval_is_test`](native/crates/runjucks-core/src/renderer.rs) + [`BinOp::Is`](native/crates/runjucks-core/src/renderer.rs): `true` for template macros when the left side is a bare `Variable`, built-in global markers, and objects tagged with `__runjucks_callable`. **Partial:** `ns.mac is callable` (namespaced macro) not special-cased; filter *names* are not treated as callables in `is callable` (Nunjucks differs here too for user filters).

### Not yet implemented

- [ ] **Unknown `is` tests silently return `false`** — **P2**
  Nunjucks throws on unknown tests. Consider erroring or adding `addTest` API.

- [ ] **`throwOnUndefined`** — **P2**
  Nunjucks `Environment` option; undefined variables throw instead of rendering empty. Not surfaced on runjucks `Environment`.

- [ ] **Jinja-compat array slices** — **P2**
  `arr[1:4]`, `arr[::2]`, etc. Tested in Nunjucks `jinja-compat.js`; not parsed in [`expr.rs`](native/crates/runjucks-core/src/parser/expr.rs).

---

## Filters

### Implemented

Built-ins in [`filters::apply_builtin`](native/crates/runjucks-core/src/filters.rs): `upper`, `lower`, `length`, `join` (optional `attr`), `replace`, `round` (precision + optional `ceil`/`floor`), `escape` / `e`, `safe`, `forceescape`, `default` / `d` (two-arg undefined-only vs three-arg `boolean` “or” mode), `batch`, `abs`, `capitalize`, `first`, `last`, `sort` (reverse, case-sensitive, attribute), `reverse`, `trim`, `int`, `float`, `string`, `title`, `truncate`, `striptags`, `urlencode`, `indent`, `nl2br`, `sum`, `wordcount`, `dictsort`, `center`, `dump`, `list`, `slice`, `urlize`, `selectattr`, `rejectattr`, `groupby`.

- **Safe strings:** [`value::RJ_SAFE`](native/crates/runjucks-core/src/value.rs) / [`mark_safe`](native/crates/runjucks-core/src/value.rs); unbound names use [`value::RJ_UNDEFINED`](native/crates/runjucks-core/src/value.rs) for Nunjucks `default` / `defined` parity with [`Environment::resolve_variable`](native/crates/runjucks-core/src/environment.rs).

### Partial

- **`replace`:** no `maxCount` / empty-`from` Python-style parity from Nunjucks.
- **`length`:** no ECMAScript `Map`/`Set` size.
- **`select` / `reject`:** require `Environment` test registry (`getTest`); not implemented.
- **`random`:** nondeterministic; not implemented.
- **`striptags`:** `preserveLinebreaks` branch is simplified vs Nunjucks.
- **Filter safeness chaining:** Nunjucks `copySafeness` across many filters is only partially mirrored (`string` / `escape` / `safe` paths).

### Not yet implemented

- [ ] **`select` / `reject`** — **P2** (needs `addTest` / built-in test lookup like Nunjucks).

- [ ] **`random`** — **P2** (nondeterministic; omit or add seeded test hook).

---

## Node / NAPI API

### Implemented

`renderString` (top-level + `Environment`), `Environment` class (`setAutoescape`, `setDev`, `setTemplateMap`, `renderTemplate`).

- [x] **`addFilter(name, fn)`** — JS `(input, ...args) => any` registered on [`Environment::add_filter`](native/crates/runjucks-core/src/environment.rs); overrides built-ins with the same name. Invoked synchronously during render via a persistent [`napi_ref`](https://nodejs.org/api/n-api.html#napi_create_reference) (main-thread / sync render only).

- [x] **`addGlobal(name, value)`** — JSON-serializable values via [`Environment::add_global`](native/crates/runjucks-core/src/environment.rs). Plain JS functions are rejected at conversion; use an object tagged with `__runjucks_callable` for `is callable` parity (see [`globals.rs`](native/crates/runjucks-core/src/globals.rs)).

- [x] **`configure({ autoescape?, dev? })`** — maps to `Environment` flags; other Nunjucks `configure` keys remain future work until core supports them.

### Not yet implemented

**P1 (partial / follow-up):**
- [ ] **`addGlobal` with live JS functions** — invoke user callbacks from templates (beyond JSON + `__runjucks_callable` markers).
- [ ] **`configure(opts)` full parity** — `throwOnUndefined`, `tags` (custom delimiters), `trimBlocks`, `lstripBlocks` (require lexer/parser/renderer support).

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

- **46** JSON vectors: `render_cases.json` (29) + `filter_cases.json` (6) + [`tag_parity_cases.json`](native/fixtures/conformance/tag_parity_cases.json) (11).
- **3** render cases marked `"skip": true` (Rust + Node skip until implemented):
  - `tests_js_filter_default_undefined` — needs full `default` filter semantics.
  - `tests_js_for_batch` — needs `batch` filter.
  - `tests_js_set_and_output` — needs `set` + `escape` + autoescape interplay.
- **Parity gate** — [`__test__/parity.test.mjs`](__test__/parity.test.mjs) compares runjucks vs the `nunjucks` npm package for every ID in [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json) (non-skipped fixtures + tag parity subset).
- **Perf allowlist** — grows with green cases; includes `render_cases`, `filter_cases`, and `tag_parity_cases` keys (see file).

### Next steps

- [ ] **Fix 3 skipped render cases** — **P0**: implement `default` (full), `batch`, fix `set`+escape. Remove `"skip": true` from JSON and keep IDs on the perf allowlist.
- [ ] **Grow `tag_parity_cases.json`** — **P1**: add vectors for composition-only cases (`include`/`extends` with loader — often covered in Rust + Node integration tests), more edge cases from upstream.
- [ ] **Expand perf allowlist** — **P2**: as new conformance cases go green, append IDs under the right key and run `npm run perf`.

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
