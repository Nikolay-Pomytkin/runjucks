# Nunjucks parity backlog

**Audience:** maintainers porting behavior and tracking gaps vs [Nunjucks](https://mozilla.github.io/nunjucks/). For **users**, see the Starlight site ([`docs/src/content/docs/guides/`](docs/src/content/docs/guides/)) — **Template language**, **JavaScript API**, **Limitations** — and the repo **README** for install/build. This file keeps **implementation references** and **checkboxes**; it is not the public product doc.

**External references (language & API):**

- [Templating](https://mozilla.github.io/nunjucks/templating.html) — tag/expression syntax.
- [API](https://mozilla.github.io/nunjucks/api.html) — `configure`, `Environment`, loaders, `compile`, async rendering, extensions, precompile, `installJinjaCompat`.

**Priorities:**

| Tier | Meaning |
|------|---------|
| **P0** | Unblocks real-world templates; ship first |
| **P1** | Common Nunjucks usage; important for adoption |
| **P2** | Ecosystem / DX; nice-to-have for parity |
| **P3** | Advanced / async; explicitly deferred |

---

## Roadmap: “partial parity” (prioritized)

Features that are **mostly** implemented but differ from Nunjucks in edge cases — or need deliberate sequencing. **Do not** tackle all of these at once; pick **one track** until shipped.

| Wave | Focus | Rationale |
|------|--------|-----------|
| **1 — P1** | **Live `addGlobal` callables** (JS functions from Node; Rust `add_global_callable` for tests) | **Shipped** — unblocks migrations (`{{ fn(…) }}`); see **P1 spec** below. |
| **2** | **`{% import %}` / `{% from %}` exports** — multi-target `{% set %}`, block `{% set %}…{% endset %}` as exports | Library-style shared templates; subtle breakage when missing. |
| **3 — P2** | Loaders, Express, perf allowlist expansion, optional `getExtension` | Ecosystem / DX, not language core. |
| **Defer** | **ECMAScript `Map` / `Set`** in context, **full RegExp parity**, **macro SafeString** polish, **include** quirks vs nunjucks 3.x | Pursue when a **concrete** template or conformance ID demands it — often avoidable at the app layer. |
| **Separate** | **`asyncEach` / `asyncAll` / `ifAsync`**, async `render`, precompile, browser bundle | **P3** — requires async pipeline or product decision; do not mix with wave 1–2 unless committing to async. |

### P1 spec: live globals (`addGlobal` + template calls)

**Goal:** Match Nunjucks’ pattern: `env.addGlobal('fn', function (...) { ... })` and `{{ fn(1, 2) }}` / keyword args per [keyword arguments](https://mozilla.github.io/nunjucks/templating.html#keyword-arguments) (`foo(1, 2, bar=3)` → last argument is a plain object in JS).

**Scope (shipped):**

- **Node (NAPI):** `addGlobal(name, value)` accepts a **JavaScript function**; synchronous call from the Rust renderer into that function during `render` / `renderTemplate` / `Template#render` (same thread as today — no async).
- **Rust core:** `Environment` holds `custom_globals: HashMap<…, Arc<dyn Fn(&[Value], &[(String, Value)]) -> Result<Value>>>` (or equivalent). **`add_global` with JSON** removes any registered callable for that name. Public **`add_global_callable`** (or similar) for integration tests without Node.
- **Calling convention:** Positional arguments map to JS arguments in order; non-empty keyword map is appended as a **single** trailing `serde_json::Value::Object` (Nunjucks-style hash), matching how custom filters/tests bridge kwargs elsewhere.
- **`is callable`:** Globals registered as functions resolve like other callable markers (`__runjucks_callable` / existing rules).
- **Output:** Interpolating a bare global function reference (`{{ fn }}`) should not dump noisy JSON — align with **empty string** for pure callable markers where that matches expectations (document in tests).

**Non-goals (this wave):** Calling **methods** on arbitrary objects (`obj.method()` unless `obj` is already supported); async functions; passing non-JSON-safe exotic values beyond what `serde_json` can round-trip for args/results.

---

## Documented language ([templating.html](https://mozilla.github.io/nunjucks/templating.html)) vs Runjucks

Cross-check the official [Nunjucks templating reference](https://mozilla.github.io/nunjucks/templating.html) (tags, expressions, filters, globals). This table is the **language** layer; the [API](https://mozilla.github.io/nunjucks/api.html) matrix is below.

| Topic | Status | Notes |
|-------|--------|--------|
| **Tags** `if`, `for`, `macro`, `set`, `extends`, `block`, `include`, `import`, `raw`, `verbatim`, `filter`, `call` | Shipped | See **Tags → Implemented**; `switch` also supported. |
| **`{% asyncEach %}`, `{% asyncAll %}`** (and `ifAsync` in lexer) | Missing | **P3** — requires async pipeline; see **Tags → Remaining**. |
| **Expressions:** literals, math, comparisons, inline `if`, calls | Mostly shipped | **Partial:** arbitrary **function calls** from **context** (still JSON-only); **global** callables via **`addGlobal` (Node)** / [`add_global_callable`](native/crates/runjucks-core/src/environment.rs) (Rust) — see **Roadmap → P1 spec**. |
| **Regex literals** `r/pattern/flags` | Shipped | Rust `regex` crate; flags **`g`** (find), **`i`**, **`m`**, **`y`** — not full ECMAScript semantics. See **Expressions → Regex**. |
| **`for` over Map / Set / iterables** | Partial | Core uses JSON values; **object** iteration uses sorted keys. **`length`** on JSON objects counts keys. ECMAScript **`Map`/`Set`** in Node context are not first-class (serialize to JSON or use objects/arrays) — see **Map / Set / length** below. |
| **`is` tests (`defined`, `callable`, …)** | Shipped | **Dotted lookups** (`o.a`, `items[0]`) use Nunjucks-style **missing → undefined** so `is defined` matches upstream; **`lib.mac` is callable** for import namespaces uses the same rules. See **Expressions → Implemented**. |
| **Builtin filters / globals** | Mostly shipped | Behavioral gaps — **Filters → Partial**; `range`, `cycler`, `joiner` shipped. |
| **`installJinjaCompat()`** (Pythonic APIs, etc.) | Not an API | Slices work without shim; see **Node / NAPI API → Remaining**. |

### Regex literals (implementation)

- Parsed in [`parser/expr.rs`](native/crates/runjucks-core/src/parser/expr.rs); evaluated in [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs).
- **`.test(str)`** is supported on regex values for Nunjucks-style checks. Other `RegExp` methods are not implemented.

### Map / Set / `length` (documented [for](https://mozilla.github.io/nunjucks/templating.html#for) / [length](https://mozilla.github.io/nunjucks/templating.html))

- In core, context values are JSON: **`length`** on **`Object`** is the key count; **`for`** over plain objects uses **sorted keys** for stable output (see user guides).
- **ECMAScript `Map` / `Set`:** not represented in the Rust engine; pass **objects** or **arrays of pairs** from Node if you need structured data. NAPI continues to use JSON-shaped values (no native Map/Set bridge).

### Test strategy (maintainers)

- **Goldens:** [`native/fixtures/conformance/*.json`](native/fixtures/conformance/) — schema in [`README.md`](native/fixtures/conformance/README.md); use `skip: true` until Runjucks matches Nunjucks output.
- **Rust:** [`native/crates/runjucks-core/tests/`](native/crates/runjucks-core/tests/) — integration tests by feature (e.g. [`is_tests.rs`](native/crates/runjucks-core/tests/is_tests.rs) for `is defined` / `is callable` on objects, arrays, imports).
- **Node:** [`__test__/`](runjucks/__test__/) conformance + [`parity.test.mjs`](runjucks/__test__/parity.test.mjs); allowlist [`perf/conformance-allowlist.json`](runjucks/perf/conformance-allowlist.json) for npm vs runjucks gates.

---

## At a glance: remaining work vs Nunjucks

| Area | Still open / partial | Notes |
|------|----------------------|--------|
| **Tags** | `{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}` | **P3** — need async render pipeline or stay documented non-goal. |
| **Expressions** | `addGlobal` with **live JS callables** (`{{ fn(…) }}`) | **Shipped** (Node); context fields remain JSON — see **Roadmap → P1 spec**. |
| **Expressions** | Filter names as `callable` in `is` tests | Minor — see **Expressions & runtime → Partial**. |
| **Filters** | `length` on ECMAScript `Map`/`Set` in Node (not JSON-shaped) | Partial — core **`length`** on objects is key count; see **Map / Set / length**. **`safe`/`escape` chains** aligned for common cases — see **Filters → Partial**. |
| **Node API** | **Filesystem / URL loaders**, **Express** helper | **P2** — today: `setTemplateMap` only. |
| **Node API** | **Async** `render` / `renderString` | **P3**. |
| **Node API** | **`precompile` / `precompileString`**, **browser bundle** | **P3** — different product shape; runjucks is Node-native. |
| **Node API** | **`installJinjaCompat()`**-style shim | **P3** — slices already native; shim would be migration sugar only. |
| **Extensions** | Nunjucks **`parse(parser, nodes)`** extension API | Not planned; Runjucks uses **declarative** `addExtension` + `process(…)` (shipped). |
| **Conformance** | Expand **perf parity allowlist** | **P2** — as more cases go green; local perf harness uses a **warm** environment (parsed-template cache), comparable to Nunjucks’ compiled-template reuse. |

---

## Nunjucks API surface (high-level matrix)

How Runjucks compares to the [documented Nunjucks API](https://mozilla.github.io/nunjucks/api.html) and the [`Environment` implementation](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/environment.js) in upstream.

| Nunjucks concept | Runjucks today |
|------------------|----------------|
| `configure(path?, opts?)` / `new Environment(loaders?, opts)` | `configure(opts)` / `new Environment()`; templates via **`setTemplateMap`** (name → source), not filesystem paths by default. |
| Loaders (`FileSystemLoader`, `WebLoader`, `PrecompiledLoader`, …) | **Not exposed** — use in-memory map or wrap your own loader that fills the map. |
| `render` / `renderString` (sync + **callback**) | **Sync only** in NAPI; no promise/callback render path. |
| `compile` / `Template` / `getTemplate(name, eagerCompile?, …)` | **Shipped** — Rust **AST** is cached (per `Template` for inline source; per environment for named templates from the map). No Nunjucks-style **JavaScript** bytecode cache. |
| `addFilter` / `addTest` / `addGlobal` | **Shipped** — `addGlobal` JSON values + **P1: JS functions** (see **Roadmap → P1 spec**). |
| `addExtension` — JS object with **`parse`** | **Shipped** — different model: tag names + optional block ends + **`process(context, args, body)`**. |
| `hasExtension` / `removeExtension` | **Shipped** (Rust: [`Environment::has_extension`](native/crates/runjucks-core/src/environment.rs) / [`remove_extension`](native/crates/runjucks-core/src/environment.rs); NAPI: [`hasExtension`](native/crates/runjucks-napi/src/lib.rs) / [`removeExtension`](native/crates/runjucks-napi/src/lib.rs)). |
| `getExtension` | **Not exposed** — Nunjucks returns the registered extension object; runjucks keeps Rust-side handlers only. |
| `express(app)` | **Not implemented**. |
| `invalidateCache` | **Not exposed** — parse entries are invalidated when lexer/parser-related settings change; **`setTemplateMap`** clears the named-template parse cache. |
| `precompile` / precompiled loader | **Not implemented**. |
| `installJinjaCompat()` | **Not implemented** as an API; slice syntax works without it. |

---

## Tags

### Implemented (parser + renderer)

`if`/`elif`/`else`/`endif`, `for`/`else`/`endfor` (single, multi-var, k/v, `loop.*`), `switch`/`case`/`default`/`endswitch`, `set`/`endset` (multi-target, block capture, frame scoping), `include` (expression, `ignore missing`, `without context` / `with context`), `import`/`from` (top-level macros + top-level `{% set %}` exports, `with context` / `without context` for module scope), `extends` (expression, evaluated at render; see Partial), `block`/`endblock`, `macro`/`endmacro` (defaults + call kwargs), `{{ super() }}`, `{% call %}…{% endcall %}` / `caller()`, `{% filter %}…{% endfilter %}`, `{% raw %}`/`{% endraw %}`, `{% verbatim %}`/`{% endverbatim %}`.

### Remaining

- [ ] **`{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}`** — **P3**
  Async iteration / conditional. Keywords exist in the lexer vocabulary; no parser/renderer/runtime. Requires async render pipeline or explicit non-goal.

### Partial

- **`{% macro %}`**: macro body renders to string; no full `SafeString` handling. Default parameters and call-site keyword args are shipped.
- **`{% include %}`**: **`without context`** supported; **`with context`** parses as no-op vs default. Stock **nunjucks 3.x** does not parse these on `include` (not on npm parity allowlist).
- **`{% import %}` / `{% from %}`**: multi-target **`{% set a, b = … %}`** and **block** `{% set %}…{% endset %}` are not lifted into exports (only simple top-level `{% set name = expr %}`).
- **`{% extends %}`**: dynamic parent at render time; literal-only static cycle detection not extended to extends (runtime still catches repeating resolved parents).

---

## Expressions & runtime

### Implemented

`Literal`, `Variable`, `GetAttr`, `GetItem` (including Jinja-style `arr[start:stop:step]`) — **missing** keys / out-of-bounds indices yield the internal **undefined** sentinel (not JSON `null`) so `is defined` matches Nunjucks for `{{ o.missing }}`, `{{ arr[99] }}`, and **import namespaces** (`{{ lib.nope }}`). `Call` (macros, `super()`, `caller()`, built-in globals), `Filter`, `List`, `Dict`, `InlineIf`, `Unary`, `Binary`, `Compare`, `is` tests (`defined`, `equalto`, `sameas`, `null`/`none`, `falsy`, `truthy`, `number`, `string`, `lower`, `upper`, `callable`, `odd`, `even`, `divisibleby`, plus `add_test` / `addTest`). **`throwOnUndefined`**, unknown custom `is` tests throw, **`addGlobal`** with JSON values, default globals (`range`, `cycler`, `joiner`). **Import namespaces:** `lib.mac` as a value uses a `__runjucks_callable` marker so `is callable` / `is defined` work without calling the macro.

### Partial

- **User callables from context:** **template context** (`renderString(…, ctx)`) remains JSON-serializable only (no live JS/Rust closures in `ctx`). Use **`addGlobal` with a function** (Node) or **`add_global_callable`** (Rust embedders) for invocable globals.
- **`is callable`:** user-registered **filter** names are not treated as callables in `is` tests (Nunjucks behavior varies by version).
- **Slices:** runjucks accepts slice syntax **without** `installJinjaCompat()`; upstream needs compat for the same syntax in vanilla Nunjucks.

### Remaining

- [x] **`addGlobal` with live JS functions** — callable from template expressions (**P1**); see **Roadmap → P1 spec**, [`runjucks-napi` `addGlobal`](native/crates/runjucks-napi/src/lib.rs), [`Environment::add_global_callable`](native/crates/runjucks-core/src/environment.rs).

---

## Filters

### Implemented

Built-ins in [`filters::apply_builtin`](native/crates/runjucks-core/src/filters.rs) include: `upper`, `lower`, `length`, `join` (optional `attr`), `replace`, `random` (seeded via `setRandomSeed` / `random_seed`), `round`, `escape` / `e`, `safe`, `forceescape`, `default` / `d`, `batch`, `abs`, `capitalize`, `first`, `last`, `sort`, `reverse`, `trim`, `int`, `float`, `string`, `title`, `truncate`, `striptags`, `urlencode`, `indent`, `nl2br`, `sum`, `wordcount`, `dictsort`, `center`, `dump`, `list`, `slice`, `urlize`, `selectattr`, `rejectattr`, `select`, `reject`, `groupby`.

Upstream Nunjucks built-ins live in [`nunjucks/src/filters.js`](../nunjucks/nunjucks/src/filters.js); compare exports when adding or auditing filters.

### Partial

- **`length`:** JSON **object** key count is supported; ECMAScript **`Map`/`Set`** (non-JSON context values) are not.
- **`striptags`:** `preserveLinebreaks` aligned with Nunjucks behavior (line-edge trim, CRLF, newline caps).
- **Safe-string chaining:** `escape` preserves marked-safe input; `safe | escape` and `escape | safe` match Nunjucks for typical HTML output; other filter combinations may not propagate `copySafeness` like upstream.

### Remaining

- None tracked as missing filter **names** — gaps are **behavioral** (partials above).

---

## Node / NAPI API

### Implemented

`renderString` (top-level + `Environment`), `Environment` (`setAutoescape`, `setDev`, `setRandomSeed`, `setTemplateMap`, `renderTemplate`, `getTemplate`, `addFilter`, `addTest`, `addExtension`, `hasExtension`, `removeExtension`, `addGlobal`, `configure`), module-level `configure` / `render` / `reset`, `compile`, `Template` (`.render(ctx)`). Options: `autoescape`, `dev`, `throwOnUndefined`, `trimBlocks`, `lstripBlocks`, `tags` (custom delimiters).

### Remaining

- [ ] **Filesystem / URL loader** — **P2** (`FileSystemLoader`-style or equivalent); today **`setTemplateMap`** only.
- [ ] **Express integration** — **P2** (`env.express(app)` / view engine).
- [ ] **Async render** — **P3** (callback / promise `render`, `renderString`).
- [ ] **`precompile` / `precompileString`** — **P3**.
- [ ] **Browser build** (UMD / ESM) — **P3**.
- [ ] **`installJinjaCompat()`** API — **P3** (migration shim; slices already native).

---

## Conformance & perf

### Current state

- **117** JSON vectors: [`render_cases.json`](native/fixtures/conformance/render_cases.json) (44) + [`filter_cases.json`](native/fixtures/conformance/filter_cases.json) (24) + [`tag_parity_cases.json`](native/fixtures/conformance/tag_parity_cases.json) (49).
- **Parity gate** — [`__test__/parity.test.mjs`](__test__/parity.test.mjs) vs `nunjucks` npm using [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json); env fixtures in [`__test__/conformance/run.mjs`](__test__/conformance/run.mjs).

### Next steps

- [ ] **Expand perf allowlist** — **P2**: append IDs as cases go green; run `npm run perf` (recent adds: `is_defined_*`, `tag_import_macro_is_callable`).
- [ ] **`getExtension`** — optional **P2** if callers need introspection (return a stub descriptor from NAPI).

---

## Out of scope / non-goals (current)

- Full **async render pipeline** (**P3**; intentional for now).
- **Nunjucks browser precompile** workflow as a supported product path (runjucks is Node-native).
- Exact **`undefined` vs `null`** JS semantics in Rust’s JSON model (both collapse similarly; templates should not rely on distinction).

---

## References

- **User docs (this repo):** [`docs/src/content/docs/guides/`](docs/src/content/docs/guides/)
- **Vendored Nunjucks:** [`../nunjucks/nunjucks/src/`](../nunjucks/nunjucks/src/)
- **Conformance fixtures:** [`native/fixtures/conformance/`](native/fixtures/conformance/)
- **Perf harness:** [`perf/README.md`](perf/README.md)
- **Rust core:** [`native/crates/runjucks-core/src/`](native/crates/runjucks-core/src/)
- **NAPI bindings:** [`native/crates/runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs)
