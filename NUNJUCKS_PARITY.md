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
| **2** | **`{% import %}` / `{% from %}` exports** — multi-target `{% set %}`, block `{% set %}…{% endset %}` as exports | **Shipped** — [`eval_exported_top_level_sets`](native/crates/runjucks-core/src/renderer.rs) / [`collect_top_level_set_exports`](native/crates/runjucks-core/src/renderer.rs); Rust tests in [`import_from.rs`](native/crates/runjucks-core/tests/import_from.rs); **conformance** IDs `tag_import_multi_target_export`, `tag_import_block_set_export`, `tag_from_import_multi_and_block`, `tag_import_chained_top_level_sets` in [`tag_parity_cases.json`](native/fixtures/conformance/tag_parity_cases.json) (also on [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json)). |
| **3 — P2** | Loaders, Express, optional `getExtension` | **Partial** — **`setLoaderRoot`** (filesystem), optional **`runjucks/express`**, **`getExtension`** introspection stub shipped; URL loaders / async loaders remain open. |
| **Defer** | **ECMAScript `Map` / `Set`** in context, **full RegExp parity**, remaining **copySafeness** edge cases, **include** quirks vs nunjucks 3.x | Pursue when a **concrete** template or conformance ID demands it — often avoidable at the app layer. |
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
- **`.test(str)`** and regex-backed `replace(r/.../, "...")` are supported on regex values for common Nunjucks cases. Other `RegExp` methods are not implemented.

### Map / Set / `length` (documented [for](https://mozilla.github.io/nunjucks/templating.html#for) / [length](https://mozilla.github.io/nunjucks/templating.html))

- In core, context values are JSON: **`length`** on **`Object`** is the key count; **`for`** over plain objects uses **sorted keys** for stable output (see user guides).
- **ECMAScript `Map` / `Set`:** not represented in the Rust engine; pass **objects** or **arrays of pairs** from Node if you need structured data. NAPI continues to use JSON-shaped values (no native Map/Set bridge).

### Test strategy (maintainers)

**Layers (bottom = most localized, top = broadest regression signal):**

1. **Rust unit / integration** — [`native/crates/runjucks-core/tests/`](native/crates/runjucks-core/tests/) by feature (`filters.rs`, `import_from.rs`, `is_tests.rs`, …): fast, no NAPI; use for edge cases not worth a JSON golden.
2. **Shared JSON goldens** — [`native/fixtures/conformance/*.json`](native/fixtures/conformance/): schema in [`README.md`](native/fixtures/conformance/README.md); `expected` is Nunjucks output; use `skip: true` until Runjucks matches.
3. **Rust reads goldens** — [`tests/conformance.rs`](native/crates/runjucks-core/tests/conformance.rs), [`tests/tag_parity.rs`](native/crates/runjucks-core/tests/tag_parity.rs) assert core output vs fixtures.
4. **Node vs Nunjucks** — [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json) drives [`parity.test.mjs`](__test__/parity.test.mjs) (same `id`s as allowlisted); proves the **npm** addon matches **nunjucks** for cases with **`compareWithNunjucks` ≠ false**, and runjucks matches **`expected`** for all allowlisted rows (including Runjucks-only goldens).
5. **Node conformance** — [`__test__/conformance/run.mjs`](__test__/conformance/run.mjs) runs fixtures through NAPI without comparing to Nunjucks.
6. **Perf** — [`perf/run.mjs`](perf/run.mjs) / [`perf/synthetic.mjs`](perf/synthetic.mjs): throughput only, not correctness. Correctness vs Nunjucks is **[`__test__/parity.test.mjs`](__test__/parity.test.mjs)**, not the perf harness (see [RUNJUCKS_PERF.md](RUNJUCKS_PERF.md)).

### Testing model: full parity vs partial vs Runjucks-only goldens

| Tier | When to use | Gates |
|------|----------------|--------|
| **Full parity** | Template is valid **nunjucks 3.2.4** syntax; output should match upstream | JSON `expected` + allowlist + [`parity.test.mjs`](__test__/parity.test.mjs) (runjucks ≡ nunjucks ≡ golden) |
| **Intentional divergence** | Runjucks accepts different syntax or semantics (e.g. `{% include %}` `without context`) | Rust / Node tests + **Tags → Partial** / [limitations](docs/src/content/docs/guides/limitations.md) — **not** compared to nunjucks in [`parity.test.mjs`](__test__/parity.test.mjs) |
| **Runjucks-only golden** | Freeze stable output for divergent syntax; still want a JSON vector | Set **`compareWithNunjucks: false`** + **`divergenceNote`** on the fixture ([conformance README](native/fixtures/conformance/README.md)); [`parity.test.mjs`](__test__/parity.test.mjs) asserts runjucks ≡ `expected` only. Perf harness skips these for nunjucks baseline ([`perf/run.mjs`](perf/run.mjs)). |

**Pointers:** API surface smoke: [`__test__/napi-surface.test.mjs`](__test__/napi-surface.test.mjs). Runjucks-only **error** substrings: [`__test__/error-cases.test.mjs`](__test__/error-cases.test.mjs). JSON ingress parity: [`json-ingress.test.mjs`](__test__/json-ingress.test.mjs).

**Upstream-ported cases (maintainers):** [`__test__/upstream/README.md`](__test__/upstream/README.md) — scenarios inspired by vendored [`nunjucks/tests/`](../nunjucks/tests/) (`node:test`, not Mocha). Use for extra regression signal beyond JSON goldens; skipped tests document known partials (`is` tests not yet in core, `sameas` object identity, `int` vs `"3.5"` truncation, …). Does **not** replace the conformance allowlist or parity gate.

**Follow-on epics (test-driven):**

| Epic | Notes |
|------|--------|
| **SafeString / copySafeness edge cases** | Common `escape` / `safe` / `e` / `forceescape` chains plus macro / `caller()` / `super()` output are covered; keep extending [`filters-ported.test.mjs`](__test__/upstream/filters-ported.test.mjs) + Rust tests when a new mismatch shows up. |
| **`Map` / `Set` in context** | Explicit Node cases with `skip` until NAPI/model supports or product stays JSON-only. |
| **Include `with` / `without` context** | Port only what `setTemplateMap` can express; compare to `nunjucks` in harness. |
| **Extends / dynamic parent cycles** | Targeted `templateMap` scenarios; see **Tags → Partial**. |
| **Regex** | Port string-only snippets from upstream; ECMAScript parity is non-goal except documented flags. |
| **Conformance allowlist** | All current fixture IDs are allowlisted; for **new** vectors, append the `id` once green. Avoid duplicating the same scenario in three places — prefer one canonical home. |
| **Nunjucks `parse(parser, nodes)` extensions** | No upstream port; Runjucks [`addExtension`](__test__/extensions.test.mjs) stays declarative. |

---

## At a glance: remaining work vs Nunjucks

| Area | Still open / partial | Notes |
|------|----------------------|--------|
| **Tags** | `{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}` | **P3** — need async render pipeline or stay documented non-goal. |
| **Expressions** | `addGlobal` with **live JS callables** (`{{ fn(…) }}`) | **Shipped** (Node); context fields remain JSON — see **Roadmap → P1 spec**. |
| **Filters** | `length` on ECMAScript `Map`/`Set` in Node (not JSON-shaped) | Partial — core **`length`** on objects is key count; see **Map / Set / length**. **`safe`/`escape` chains** aligned for common cases — see **Filters → Partial**. |
| **Node API** | **URL / async loaders** | **P2** — **`setTemplateMap`** + **`setLoaderRoot`** (sync disk); **`runjucks/express`** optional helper; no `http(s):` URL loader. |
| **Node API** | **Async** `render` / `renderString` | **P3**. |
| **Node API** | **`precompile` / `precompileString`**, **browser bundle** | **P3** — different product shape; runjucks is Node-native. |
| **Node API** | **`installJinjaCompat()`**-style shim | **P3** — slices already native; shim would be migration sugar only. |
| **Extensions** | Nunjucks **`parse(parser, nodes)`** extension API | Not planned; Runjucks uses **declarative** `addExtension` + `process(…)` (shipped). |
| **Conformance** | **Perf parity allowlist** | All fixture IDs on [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json) are tracked; **`npm run check:conformance-allowlist`** fails if a non-skipped JSON `id` is missing. Perf harness: [`perf/run.mjs`](perf/run.mjs). |

---

## Nunjucks API surface (high-level matrix)

How Runjucks compares to the [documented Nunjucks API](https://mozilla.github.io/nunjucks/api.html) and the [`Environment` implementation](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/environment.js) in upstream.

| Nunjucks concept | Runjucks today |
|------------------|----------------|
| `configure(path?, opts?)` / `new Environment(loaders?, opts)` | `configure(opts)` / `new Environment()`; templates via **`setTemplateMap`** (name → source) and/or **`setLoaderRoot(path)`** (sync filesystem under `path`). |
| Loaders (`FileSystemLoader`, `WebLoader`, `PrecompiledLoader`, …) | **Disk:** **`setLoaderRoot`** (Rust [`FileSystemLoader`](native/crates/runjucks-core/src/loader.rs)). **URL / async / pluggable JS loaders** — not exposed. |
| `render` / `renderString` (sync + **callback**) | **Sync only** in NAPI; no promise/callback render path. |
| `compile` / `Template` / `getTemplate(name, eagerCompile?, …)` | **Shipped** — Rust **AST** is cached (per `Template` for inline source; per environment for named templates from the map). No Nunjucks-style **JavaScript** bytecode cache. |
| `addFilter` / `addTest` / `addGlobal` | **Shipped** — `addGlobal` JSON values + **P1: JS functions** (see **Roadmap → P1 spec**). |
| `addExtension` — JS object with **`parse`** | **Shipped** — different model: tag names + optional block ends + **`process(context, args, body)`**. |
| `hasExtension` / `removeExtension` | **Shipped** (Rust: [`Environment::has_extension`](native/crates/runjucks-core/src/environment.rs) / [`remove_extension`](native/crates/runjucks-core/src/environment.rs); NAPI: [`hasExtension`](native/crates/runjucks-napi/src/lib.rs) / [`removeExtension`](native/crates/runjucks-napi/src/lib.rs)). |
| `getExtension` | **Shipped as a stub descriptor** — NAPI returns `{ name, tags, blocks }` for introspection (or `null` when missing); it does not expose the Rust-side handler object. |
| `express(app)` | **Optional** — `require('runjucks/express').expressEngine(app, opts?)` registers a sync view engine (see docs); not identical to upstream’s `nunjucks.express`. |
| `invalidateCache` | **`invalidateCache()`** clears **named** and **inline** parse caches (NAPI). **`setTemplateMap`** / **`setLoaderRoot`** / **`setLoaderCallback`** still clear the named cache when replacing the loader. |
| `precompile` / precompiled loader | **Not implemented**. |
| `installJinjaCompat()` | **Not implemented** as an API; slice syntax works without it. |

---

## Tags

### Implemented (parser + renderer)

`if`/`elif`/`else`/`endif`, `for`/`else`/`endfor` (single, multi-var, k/v, `loop.*`), `switch`/`case`/`default`/`endswitch`, `set`/`endset` (multi-target, block capture, frame scoping), `include` (expression, `ignore missing`, `without context` / `with context`), `import`/`from` (top-level macros + top-level `{% set %}` exports: single-target, multi-target same-value assign, and block capture — `with context` / `without context` for module scope), `extends` (expression, evaluated at render; see Partial), `block`/`endblock`, `macro`/`endmacro` (defaults + call kwargs), `{{ super() }}`, `{% call %}…{% endcall %}` / `caller()`, `{% filter %}…{% endfilter %}`, `{% raw %}`/`{% endraw %}`, `{% verbatim %}`/`{% endverbatim %}`.

### Remaining

- [ ] **`{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}`** — **P3**
  Async iteration / conditional. Keywords exist in the lexer vocabulary; no parser/renderer/runtime. Requires async render pipeline or explicit non-goal.

### Partial

- **`{% include %}`**: **`without context`** supported; **`with context`** parses as no-op vs default. Stock **nunjucks 3.x** does not parse these on `include` (not on npm parity allowlist).
- **`{% extends %}`**: dynamic parent at render time. **Literal-only** `{% extends "…" %}` chains are **pre-checked** for cycles via [`scan_literal_extends_graph`](native/crates/runjucks-core/src/renderer.rs) before render; dynamic `{% extends expr %}` still relies on runtime resolution (and runtime errors for bad chains).

---

## Expressions & runtime

### Implemented

`Literal`, `Variable`, `GetAttr`, `GetItem` (including Jinja-style `arr[start:stop:step]`) — **missing** keys / out-of-bounds indices yield the internal **undefined** sentinel (not JSON `null`) so `is defined` matches Nunjucks for `{{ o.missing }}`, `{{ arr[99] }}`, and **import namespaces** (`{{ lib.nope }}`). `Call` (macros, `super()`, `caller()`, built-in globals), `Filter`, `List`, `Dict`, `InlineIf`, `Unary`, `Binary`, `Compare`, `is` tests (`defined`, `equalto`, `sameas`, `null`/`none`, `falsy`, `truthy`, `number`, `string`, `lower`, `upper`, `callable`, `odd`, `even`, `divisibleby`, plus `add_test` / `addTest`). **`throwOnUndefined`**, unknown custom `is` tests throw, **`addGlobal`** with JSON values, default globals (`range`, `cycler`, `joiner`). **Import namespaces:** `lib.mac` as a value uses a `__runjucks_callable` marker so `is callable` / `is defined` work without calling the macro.

### Partial

- **User callables from context:** **template context** (`renderString(…, ctx)`) remains JSON-serializable only (no live JS/Rust closures in `ctx`). Use **`addGlobal` with a function** (Node) or **`add_global_callable`** (Rust embedders) for invocable globals.
- **`is callable`:** filter names are not template values; Nunjucks **3.2.4** and Runjucks both return `false` / `undefined` semantics for `{{ upper is callable }}` / `{{ upper is defined }}`.
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
- **Safe-string chaining:** `escape` preserves marked-safe input; `safe | escape`, `escape | safe`, `safe | e`, `safe | forceescape`, regex-backed `replace`, and macro / `caller()` / `super()` render output match Nunjucks for common HTML cases; other filter combinations may still differ from upstream `copySafeness`.

### Remaining

- None tracked as missing filter **names** — gaps are **behavioral** (partials above).

---

## Node / NAPI API

### Implemented

`renderString` (top-level + `Environment`), `Environment` (`setAutoescape`, `setDev`, `setRandomSeed`, `setTemplateMap`, **`setLoaderRoot`** (sync disk loader), **`setLoaderCallback`** (sync JS `getSource(name)` → source / `null`), **`invalidateCache`**, `renderTemplate`, `getTemplate`, `addFilter`, `addTest`, `addExtension`, `hasExtension`, `getExtension`, `removeExtension`, `addGlobal`, `configure`), module-level `configure` / `render` / `reset`, `compile`, `Template` (`.render(ctx)`). Options: `autoescape`, `dev`, `throwOnUndefined`, `trimBlocks`, `lstripBlocks`, `tags` (custom delimiters). Optional **`@zneep/runjucks/express`** (`expressEngine`), **`@zneep/runjucks/serialize-context`** (`serializeContextForRender`).

### Remaining

- [ ] **Built-in URL / `http(s):` loader** — **P2** — use **`setLoaderCallback`** or app-layer fetch + `setTemplateMap`; no first-class WebLoader in-box.
- [ ] **Async render** — **P3** (callback / promise `render`, `renderString`).
- [ ] **`precompile` / `precompileString`** — **P3**.
- [ ] **Browser build** (UMD / ESM / WASM) — **P3**.
- [ ] **`installJinjaCompat()`** API — **P3** (migration shim; slices already native).

---

## Conformance & perf

### Current state

- **128** JSON vectors (non-skipped): [`render_cases.json`](native/fixtures/conformance/render_cases.json) (46) + [`filter_cases.json`](native/fixtures/conformance/filter_cases.json) (26) + [`tag_parity_cases.json`](native/fixtures/conformance/tag_parity_cases.json) (56).
- **Allowlist hygiene** — run **`npm run check:conformance-allowlist`** so every non-skipped fixture `id` appears in [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json).
- **Parity gate** — [`__test__/parity.test.mjs`](__test__/parity.test.mjs) vs `nunjucks` npm using that allowlist. Env fixtures in [`__test__/conformance/run.mjs`](__test__/conformance/run.mjs).

### Next steps

- [ ] **New conformance vectors** — when adding rows to the JSON fixtures, append the new `id` to [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json) once Runjucks matches Nunjucks; `npm run perf` can include them for trends.

---

## Out of scope / non-goals (current)

- Full **async render pipeline** (**P3**; intentional for now) — see [`P3_ROADMAP.md`](P3_ROADMAP.md).
- **Nunjucks browser precompile** workflow as a supported product path (runjucks is Node-native).
- Exact **`undefined` vs `null`** JS semantics in Rust’s JSON model (both collapse similarly; templates should not rely on distinction).

---

## References

- **P3 deferred tracks (async, precompile, browser):** [`P3_ROADMAP.md`](P3_ROADMAP.md)
- **User docs (this repo):** [`docs/src/content/docs/guides/`](docs/src/content/docs/guides/)
- **Vendored Nunjucks:** [`../nunjucks/nunjucks/src/`](../nunjucks/nunjucks/src/)
- **Conformance fixtures:** [`native/fixtures/conformance/`](native/fixtures/conformance/)
- **Perf harness:** [`perf/README.md`](perf/README.md)
- **Rust core:** [`native/crates/runjucks-core/src/`](native/crates/runjucks-core/src/)
- **NAPI bindings:** [`native/crates/runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs)
