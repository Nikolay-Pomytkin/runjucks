# Nunjucks parity backlog

**Audience:** maintainers porting behavior and tracking gaps vs [Nunjucks](https://mozilla.github.io/nunjucks/). For **users**, see the Starlight site ([`docs/src/content/docs/guides/`](docs/src/content/docs/guides/)) — **Template language**, **JavaScript API**, **Limitations** — and the repo **README** for install/build. This file keeps **implementation references** and **checkboxes**; it is not the public product doc.

**Scope vs mozilla.io docs:** The official site documents the **full Nunjucks product** — async tags, callback/promise APIs, **precompile**, **browser** bundles, and extension `parse()` hooks. Runjucks is **Node-native** with a **sync-first** API and an optional **Promise-based async path** (`renderStringAsync`, `renderTemplateAsync`, `addAsyncFilter` / `addAsyncGlobal`, tags `asyncEach` / `asyncAll` / `ifAsync`). **Precompile**, **browser** bundles, and Nunjucks’ **callback-style** async `render` (no Promises) remain **P3** / non-goals unless the roadmap changes ([P3_ROADMAP.md](P3_ROADMAP.md)). Near–**100% parity** below means **that sync language + builtins + documented Node API subset**, not every feature on [templating.html](https://mozilla.github.io/nunjucks/templating.html) and [api.html](https://mozilla.github.io/nunjucks/api.html).

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

## Prioritized Nunjucks API parity queue

**Maintainer action order** for closing gaps vs the [Nunjucks API](https://mozilla.github.io/nunjucks/api.html) and loader ecosystem. Tiers match **Priorities** above. The **API surface matrix** is in [Nunjucks API surface](#nunjucks-api-surface-high-level-matrix); deferred product tracks are in [P3_ROADMAP.md](P3_ROADMAP.md).

### P2 — Ecosystem / drop-in DX (highest ROI for “more like Nunjucks npm”)

1. **HTTP(S) / URL loading** — **Done (recipe + helper):** [Limitations](docs/src/content/docs/guides/limitations.md) documents fetch → map → `setTemplateMap` / `setLoaderCallback`; **`@zneep/runjucks/fetch-template-map`** implements `fetchTemplateMap`; tests in [`__test__/loader-url-pattern.test.mjs`](__test__/loader-url-pattern.test.mjs). No native HTTP loader in Rust (by design).
2. **`configure` / `autoescape` depth** — **Done (JS truthiness):** `configure({ autoescape })` accepts boolean / string / number / `null` and normalizes to one engine boolean (Nunjucks-style truthiness); **`setAutoescape`** stays boolean-only. Documented in [limitations](docs/src/content/docs/guides/limitations.md) and [JavaScript API](docs/src/content/docs/guides/javascript-api.md); tests in [`__test__/configure-autoescape.test.mjs`](__test__/configure-autoescape.test.mjs). **Not done:** per-filename extension autoescape (still a single global flag).
3. **`runjucks/express` vs `nunjucks.express`** — **Partial:** [`__test__/express.test.mjs`](__test__/express.test.mjs) covers missing view → 500, `configure.trimBlocks`, `view cache` off + parse invalidation, **`res.locals` JSON round-trip** (functions stripped), **error middleware** receiving render errors, and **multi-root `views`**: **`setLoaderCallback`** searches all roots; relative template names match the root Express resolved for the file. Further gaps only as reported.
4. **`getExtension`** — **Stub by design** (`name`, `tags`, sorted `tags`, `blocks`); the Rust registry does not store **priority** / arbitrary ids — nothing to expose beyond [`ExtensionDescriptor`](native/crates/runjucks-core/src/environment.rs). Docs: [JavaScript API](docs/src/content/docs/guides/javascript-api.md). **`parse()`-style objects** remain out of scope.

### P3 — Large effort or different product shape

1. **Callback-only async `render(name, ctx, cb)`** — Core exposes **Promise** APIs; optional JS adapters [`render-with-callback.js`](render-with-callback.js) wrap **`renderTemplate`** / **`renderTemplateAsync`** with **`(err, html)`** ([P3_ROADMAP.md](P3_ROADMAP.md)).
2. **`precompile` / `precompileString` / `PrecompiledLoader`** — Upstream emits JS for Nunjucks’ runtime; Runjucks uses a Rust AST — needs a **codegen or WASM** story, not a small NAPI tweak.
3. **Browser bundle (UMD / ESM) / WASM target** — Distribution and loader story for non-Node hosts.
4. **`installJinjaCompat()`** — **Shipped (no-op):** [`install-jinja-compat.js`](install-jinja-compat.js); slices and most Jinja-like syntax already work without calling it.
5. **`addExtension` with `parse(parser, nodes)`** — Nunjucks parser-hook extensions; Runjucks uses **declarative** `addExtension` + `process(…)` instead (no plan to clone the JS parser API).
6. **Parallel `asyncAll`** — Runjucks runs **`asyncAll` sequentially** for deterministic output; true overlap would be new semantics.

### Ongoing / test-driven (not one API ticket)

- **ECMAScript `Map` / `Set` in context** — Core is JSON-shaped through NAPI; use objects/arrays or [`serialize-context`](serialize-context.js) until a bounded bridge is a product priority.
- **Safe-string / `copySafeness` and filter chains** — Extend [upstream-ported tests](__test__/upstream/) and conformance as real templates surface mismatches.
- **RegExp** — Rust `regex` with documented flags; full ECMAScript `RegExp` parity is a non-goal except where tests require it.
- **`include` / `import` / `extends` edge cases** — Add goldens when stock **nunjucks@3.2.4** accepts the same syntax; see **Tags → Partial**.

---

## Roadmap: “partial parity” (prioritized)

Features that are **mostly** implemented but differ from Nunjucks in edge cases — or need deliberate sequencing. **Do not** tackle all of these at once; pick **one track** until shipped.

| Wave | Focus | Rationale |
|------|--------|-----------|
| **1 — P1** | **Live `addGlobal` callables** (JS functions from Node; Rust `add_global_callable` for tests) | **Shipped** — unblocks migrations (`{{ fn(…) }}`); see **P1 spec** below. |
| **2** | **`{% import %}` / `{% from %}` exports** — multi-target `{% set %}`, block `{% set %}…{% endset %}` as exports | **Shipped** — [`eval_exported_top_level_sets`](native/crates/runjucks-core/src/renderer.rs) / [`collect_top_level_set_exports`](native/crates/runjucks-core/src/renderer.rs); Rust tests in [`import_from.rs`](native/crates/runjucks-core/tests/import_from.rs); **conformance** IDs `tag_import_multi_target_export`, `tag_import_block_set_export`, `tag_from_import_multi_and_block`, `tag_import_chained_top_level_sets` in [`tag_parity_cases.json`](native/fixtures/conformance/tag_parity_cases.json) (also on [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json)). |
| **3 — P2** | Loaders, Express, optional `getExtension` | **Partial** — **`setLoaderRoot`** (filesystem), **`setTemplateMap`**, **`setLoaderCallback`** (sync JS), optional **`runjucks/express`**, **`getExtension`** descriptor shipped. **HTTP(S):** no Rust `http:` loader — use **`fetch`** + **`setTemplateMap`** / [`fetch-template-map`](fetch-template-map.js) ([`__test__/loader-url-pattern.test.mjs`](__test__/loader-url-pattern.test.mjs)). **No** upstream-style async loader or **`PrecompiledLoader`**. |
| **Defer** | **ECMAScript `Map` / `Set`** in context, **full RegExp parity**, remaining **copySafeness** edge cases, **include** quirks vs nunjucks 3.x | Pursue when a **concrete** template or conformance ID demands it — often avoidable at the app layer. |
| **Separate** | **Nunjucks callback `render`**, **true parallel `asyncAll`**, precompile, browser bundle | **P3** — Runjucks async uses **Promises** and **sequential** `asyncAll`; see [P3_ROADMAP.md](P3_ROADMAP.md). |

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
| **`{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}`** | Shipped (async API only) | Use `renderStringAsync` / `renderTemplateAsync`. `asyncAll` is **sequential** (not parallel); see **Tags → Async**. |
| **Expressions:** literals, math, comparisons, inline `if`, calls | Mostly shipped | **Partial:** arbitrary **function calls** from **context** (still JSON-only); **global** callables via **`addGlobal` (Node)** / [`add_global_callable`](native/crates/runjucks-core/src/environment.rs) (Rust) — see **Roadmap → P1 spec**. |
| **Regex literals** `r/pattern/flags` | Shipped | Rust `regex` crate; flags **`g`** (find), **`i`**, **`m`**, **`y`** — not full ECMAScript semantics. See **Expressions → Regex**. |
| **`for` over Map / Set / iterables** | Partial | Core uses JSON values; **object** iteration uses sorted keys. **`length`** on JSON objects counts keys. ECMAScript **`Map`/`Set`** in Node context are not first-class (serialize to JSON or use objects/arrays) — see **Map / Set / length** below. |
| **`is` tests (`defined`, `callable`, `gt`, `mapping`, …)** | Shipped | **Dotted lookups** (`o.a`, `items[0]`) use Nunjucks-style **missing → undefined** so `is defined` matches upstream; **`lib.mac` is callable** for import namespaces uses the same rules. Builtin list: **Expressions → Builtin `is` tests**. |
| **Builtin filters / globals** | Mostly shipped | Behavioral gaps — **Filters → Partial**; `range`, `cycler`, `joiner` shipped. |
| **`installJinjaCompat()`** (Pythonic APIs, etc.) | Optional no-op | [`install-jinja-compat.js`](install-jinja-compat.js) for migrations; slices work without calling it — see [limitations](docs/src/content/docs/guides/limitations.md#jinja-compatibility). |

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
| **Tags (async)** | `{% asyncEach %}`, `{% asyncAll %}`, `{% ifAsync %}` | **Shipped** when using **`renderStringAsync`** / **`renderTemplateAsync`**. **`asyncAll`** is **sequential** (deterministic order), not worker-parallel — see **Tags → Async**. |
| **Expressions** | `addGlobal` with **live JS callables** (`{{ fn(…) }}`) | **Shipped** (Node); context fields remain JSON — see **Roadmap → P1 spec**. |
| **Expressions** | Builtin **`is`** tests (`gt`, `ne`, `escaped`, `mapping`, …) | **Shipped** — see **Expressions → Builtin `is` tests**; **`equalto` / `sameas`** use `===`-style rules for JSON context (same-variable vs distinct bindings). |
| **Filters** | `length` on ECMAScript `Map`/`Set` in Node (not JSON-shaped) | Partial — core **`length`** on objects is key count; see **Map / Set / length**. **`safe`/`escape` chains** aligned for common cases — see **Filters → Partial**. |
| **Node API** | **URL / remote templates** | **Partial** — **`setTemplateMap`**, **`setLoaderRoot`**, **`setLoaderCallback`**; prefetch over HTTP with **`fetch`** / [`fetch-template-map`](fetch-template-map.js) (no **Rust** `http:` loader). **`runjucks/express`** optional. **No** `PrecompiledLoader` / upstream **`WebLoader`** class. |
| **Node API** | **Async** `renderString` / `render` | **Partial** — **`renderStringAsync`**, **`renderTemplateAsync`**, **`addAsyncFilter`**, **`addAsyncGlobal`** shipped. Optional **`render-with-callback.js`** for **`(err, html)`**; core remains Promise-based — see **Deferred spikes**. |
| **Node API** | **`precompile` / `precompileString`**, **browser bundle** | **P3** — different product shape; runjucks is Node-native. |
| **Node API** | **`installJinjaCompat()`**-style shim | **Shipped (no-op)** — [`install-jinja-compat.js`](install-jinja-compat.js); slices already native. |
| **Extensions** | Nunjucks **`parse(parser, nodes)`** extension API | Not planned; Runjucks uses **declarative** `addExtension` + `process(…)` (shipped). |
| **Conformance** | **Perf parity allowlist** | All fixture IDs on [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json) are tracked; **`npm run check:conformance-allowlist`** fails if a non-skipped JSON `id` is missing. Perf harness: [`perf/run.mjs`](perf/run.mjs). |

---

## Path to near-100% parity (concrete next milestones)

If we optimize strictly for **Nunjucks 3.2.4 migration confidence** (not P3 browser/precompile), the highest-value path is to close behavioral gaps and expand parity coverage in this order:

1. **Safe-string/copySafeness hardening (P1)**
   - Port additional upstream edge cases for `safe`, `escape/e`, `forceescape`, `replace`, macro returns, `caller()`, and `super()` chains.
   - Land failing vectors first in [`__test__/upstream/filters-ported.test.mjs`](__test__/upstream/filters-ported.test.mjs) and/or [`native/fixtures/conformance/`](native/fixtures/conformance/), then fix in [`native/crates/runjucks-core/src/filters.rs`](native/crates/runjucks-core/src/filters.rs).
   - **Done when:** no known copySafeness mismatches remain in upstream-ported coverage.

2. **`include`/`extends` edge-behavior parity (P1)**
   - Add targeted fixtures for tricky inheritance and include interactions (dynamic parent expressions, context passing, ignore-missing combinations) that are valid in stock Nunjucks.
   - Keep runjucks-only divergences explicitly tagged with `compareWithNunjucks: false` + `divergenceNote`.
   - **Done when:** all stock-valid scenarios in this area are either parity-green or explicitly documented as intentional divergence.

3. **Expression/runtime parity cleanup (P1)**
   - Continue closing subtle `is`/comparison/coercion differences with upstream tests (especially mixed numeric/string relational cases and undefined propagation paths).
   - Prefer adding cases to JSON conformance so Rust + Node stay aligned automatically.
   - **Done when:** no open mismatches in `is` tests and comparison behavior for JSON-shaped data.

4. **Loader/Express migration polish (P2)**
   - Expand parity-style tests around multi-root resolution, relative includes, cache invalidation, and error surfaces in [`__test__/express.test.mjs`](__test__/express.test.mjs) and loader suites.
   - Focus on “drop-in app migration” failures reported by users, not theoretical API coverage.
   - **Done when:** common `nunjucks.configure(...).express(app)` usage patterns are covered by green tests with documented deltas.

5. **Coverage accounting + release gate (P1/P2)**
   - **Shipped:** [`scripts/check-conformance-allowlist.mjs`](scripts/check-conformance-allowlist.mjs) now prints parity summary counts (`total`, `comparedWithNunjucks`, `runjucksOnlyGoldens`, `skipped`) plus per-suite breakdown.
   - Keep `perf/conformance-allowlist.json` synchronized as new IDs go green.
   - **Done when:** maintainers can quote parity progress from CI without manual counting.

### Suggested “next PR” checklist

- Add 3–5 new upstream-derived conformance vectors focused on one gap category (recommended: copySafeness).
- Ensure each new non-skipped fixture ID is added to [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json).
- Run: `npm run build`, `npm test`, `npm run test:rust`, `npm run check:conformance-allowlist`.
- Update this document’s relevant “Partial/Remaining” bullets as cases move to green.

---

## Nunjucks API surface (high-level matrix)

How Runjucks compares to the [documented Nunjucks API](https://mozilla.github.io/nunjucks/api.html) and the [`Environment` implementation](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/environment.js) in upstream.

| Nunjucks concept | Runjucks today |
|------------------|----------------|
| `configure(path?, opts?)` / `new Environment(loaders?, opts)` | `configure(opts)` / `new Environment()`; templates via **`setTemplateMap`** (name → source) and/or **`setLoaderRoot(path)`** (sync filesystem under `path`). |
| Loaders (`FileSystemLoader`, `WebLoader`, `PrecompiledLoader`, …) | **Disk:** **`setLoaderRoot`** ([`FileSystemLoader`](native/crates/runjucks-core/src/loader.rs)). **In-memory / callback:** **`setTemplateMap`**, **`setLoaderCallback`**. **HTTP(S):** prefetch in JS (`fetch` / [`fetch-template-map`](fetch-template-map.js)) then register sources — **no** built-in Rust URL loader, **no** `PrecompiledLoader`, **no** npm `WebLoader` equivalent. |
| `render` / `renderString` (sync + **callback**) | **Sync** `renderString` / `render` / `renderTemplate`; **async** via **`renderStringAsync`**, **`renderTemplateAsync`** (return Promises). Optional JS **`renderWithCallback`** / **`renderWithCallbackAsync`** ([`render-with-callback.js`](render-with-callback.js)) for **`(err, html)`**; no callback-only core API. |
| `compile` / `Template` / `getTemplate(name, eagerCompile?, …)` | **Shipped** — Rust **AST** is cached (per `Template` for inline source; per environment for named templates from the map). No Nunjucks-style **JavaScript** bytecode cache. |
| `addFilter` / `addTest` / `addGlobal` | **Shipped** — `addGlobal` JSON values + **P1: JS functions** (see **Roadmap → P1 spec**). **`addAsyncFilter` / `addAsyncGlobal`** for async templates (Promise callables). |
| `addExtension` — JS object with **`parse`** | **Shipped** — different model: tag names + optional block ends + **`process(context, args, body)`**. |
| `hasExtension` / `removeExtension` | **Shipped** (Rust: [`Environment::has_extension`](native/crates/runjucks-core/src/environment.rs) / [`remove_extension`](native/crates/runjucks-core/src/environment.rs); NAPI: [`hasExtension`](native/crates/runjucks-napi/src/lib.rs) / [`removeExtension`](native/crates/runjucks-napi/src/lib.rs)). |
| `getExtension` | **Shipped as a stub descriptor** — NAPI returns `{ name, tags, blocks }` for introspection (or `null` when missing); it does not expose the Rust-side handler object. |
| `express(app)` | **Optional** — `require('runjucks/express').expressEngine(app, opts?)` registers a sync view engine (see docs); not identical to upstream’s `nunjucks.express`. |
| `invalidateCache` | **`invalidateCache()`** clears **named** and **inline** parse caches (NAPI). **`setTemplateMap`** / **`setLoaderRoot`** / **`setLoaderCallback`** still clear the named cache when replacing the loader. |
| `precompile` / precompiled loader | **Not implemented**. |
| `installJinjaCompat()` | **Optional no-op** — [`install-jinja-compat.js`](install-jinja-compat.js) for migrations; slice syntax already works without calling it — see [limitations](docs/src/content/docs/guides/limitations.md). |

---

## Tags

### Implemented (parser + renderer)

`if`/`elif`/`else`/`endif`, `for`/`else`/`endfor` (single, multi-var, k/v, `loop.*`), `switch`/`case`/`default`/`endswitch`, `set`/`endset` (multi-target, block capture, frame scoping), `include` (expression, `ignore missing`, `without context` / `with context`), `import`/`from` (top-level macros + top-level `{% set %}` exports: single-target, multi-target same-value assign, and block capture — `with context` / `without context` for module scope), `extends` (expression, evaluated at render; see Partial), `block`/`endblock`, `macro`/`endmacro` (defaults + call kwargs), `{{ super() }}`, `{% call %}…{% endcall %}` / `caller()`, `{% filter %}…{% endfilter %}`, `{% raw %}`/`{% endraw %}`, `{% verbatim %}`/`{% endverbatim %}`.

### Async (requires `renderStringAsync` / `renderTemplateAsync`)

- [x] **`{% asyncEach %}…{% endeach %}`**, **`{% asyncAll %}…{% endall %}`**, **`{% ifAsync %}…{% endif %}`** — **Shipped** on the async renderer ([`async_renderer`](native/crates/runjucks-core/src/async_renderer/)). **`asyncAll`** runs body iterations **in sequence** (deterministic; not worker-parallel). Tests: [`__test__/async-render.test.mjs`](__test__/async-render.test.mjs), [`tests/async_renderer.rs`](native/crates/runjucks-core/tests/async_renderer.rs) (with `--features async`).

### Partial

- **`{% include %}`**: **`without context`** supported; **`with context`** parses as no-op vs default. Stock **nunjucks 3.x** does not parse these on `include` (not on npm parity allowlist).
- **`{% extends %}`**: dynamic parent at render time. **Literal-only** `{% extends "…" %}` chains are **pre-checked** for cycles via [`scan_literal_extends_graph`](native/crates/runjucks-core/src/renderer.rs) before render; dynamic `{% extends expr %}` still relies on runtime resolution (and runtime errors for bad chains).

---

## Expressions & runtime

### Implemented

`Literal`, `Variable`, `GetAttr`, `GetItem` (including Jinja-style `arr[start:stop:step]`) — **missing** keys / out-of-bounds indices yield the internal **undefined** sentinel (not JSON `null`) so `is defined` matches Nunjucks for `{{ o.missing }}`, `{{ arr[99] }}`, and **import namespaces** (`{{ lib.nope }}`). `Call` (macros, `super()`, `caller()`, built-in globals), `Filter`, `List`, `Dict`, `InlineIf`, `Unary`, `Binary`, `Compare`, builtin `is` tests (see table below), plus `add_test` / `addTest`. **`throwOnUndefined`**, unknown custom `is` tests throw, **`addGlobal`** with JSON values, default globals (`range`, `cycler`, `joiner`). **Import namespaces:** `lib.mac` as a value uses a `__runjucks_callable` marker so `is callable` / `is defined` work without calling the macro.

### Builtin `is` tests (vs [nunjucks `tests.js`](../nunjucks/nunjucks/src/tests.js))

| Test names | Status | Notes |
|------------|--------|--------|
| `defined`, `callable`, `null` / `none`, `undefined` | Shipped | `undefined` uses internal sentinel; JSON `null` is not undefined. |
| `equalto`, **`eq`** (alias), **`sameas`** (alias of `equalto`) | Shipped | **`===` semantics** for values from **distinct** bindings: two objects/arrays from different context keys are **never** equal, even if structurally identical. **`{{ o is sameas(o) }}`** / **`{{ o is equalto(o) }}`** is **true** (same template variable). `select` / `reject` always use the “distinct binding” path. |
| `truthy`, `falsy`, `number`, `string`, `lower`, `upper` | Shipped | |
| `odd`, `even`, `divisibleby` | Shipped | |
| **`greaterthan`**, **`gt`**, **`lessthan`**, **`lt`**, **`ge`**, **`le`**, **`ne`** | Shipped | Relational ops use JS-like numeric coercion where both sides parse as numbers; otherwise two **string** operands compare lexicographically; **`ne()`** with no arg → `!== undefined` (Nunjucks). |
| **`escaped`** | Shipped | True for **`safe`** / marked-safe wrapper values. |
| **`iterable`**, **`mapping`** | Shipped | JSON model: strings + arrays **iterable**; plain objects **mapping**; excludes undefined/safe/regexp markers. Not ECMAScript **`Map`/`Set`** (see below). |

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

`renderString` (top-level + `Environment`), **`renderStringAsync`**, `Environment` (`setAutoescape`, `setDev`, `setRandomSeed`, `setTemplateMap`, **`setLoaderRoot`** (sync disk loader), **`setLoaderCallback`** (sync JS `getSource(name)` → source / `null`), **`invalidateCache`**, `renderTemplate`, **`renderTemplateAsync`**, `getTemplate`, `addFilter`, **`addAsyncFilter`**, `addTest`, `addExtension`, `hasExtension`, `getExtension`, `removeExtension`, `addGlobal`, **`addAsyncGlobal`**, `configure`), module-level `configure` / `render` / `reset`, `compile`, `Template` (`.render(ctx)`). Options: `autoescape`, `dev`, `throwOnUndefined`, `trimBlocks`, `lstripBlocks`, `tags` (custom delimiters). Optional **`@zneep/runjucks/express`** (`expressEngine`), **`@zneep/runjucks/serialize-context`** (`serializeContextForRender`), **`@zneep/runjucks/install-jinja-compat`** (`installJinjaCompat` no-op), **`@zneep/runjucks/render-with-callback`** (`renderWithCallback` / `renderWithCallbackAsync` — thin JS adapters over sync / Promise APIs).

### Remaining

- [x] **Built-in URL / `http(s):` loader** — **By design:** no Rust `http:` loader in the addon (avoids blocking the event loop). Use **`setLoaderCallback`**, app-layer **`fetch`** + **`setTemplateMap`**, or **`@zneep/runjucks/fetch-template-map`** — same story as **P2 queue → HTTP(S) / URL loading** (done).
- [x] **Nunjucks-style `render` with callback** — optional JS adapters [`render-with-callback.js`](render-with-callback.js) (`renderWithCallback`, `renderWithCallbackAsync`); core remains Promise-based.
- [ ] **`precompile` / `precompileString`** — **P3**.
- [ ] **Browser build** (UMD / ESM / WASM) — **P3**.
- [x] **`installJinjaCompat()`** — no-op in [`install-jinja-compat.js`](install-jinja-compat.js) (slices already native).

---

## Deferred spikes (P3 — separate efforts)

Do **not** batch these into a single parity sprint; each needs its own design note or explicit product decision. Minimum bar before implementation: see [P3_ROADMAP.md](P3_ROADMAP.md) and [`PER_TEMPLATE_AUTOESCAPE.md`](PER_TEMPLATE_AUTOESCAPE.md) (proof + mismatch tests).

| Item | Defer reason | Spike / proof |
|------|----------------|---------------|
| **Per-template / extension autoescape** | `RenderState` must track **current template** and apply rules in [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) (sync + async). | [`PER_TEMPLATE_AUTOESCAPE.md`](PER_TEMPLATE_AUTOESCAPE.md) + spike tests in [`__test__/per-template-autoescape-spike.test.mjs`](__test__/per-template-autoescape-spike.test.mjs) |
| **`precompile` / `PrecompiledLoader`** | Rust AST ≠ Nunjucks JS bytecode. | Codegen or WASM story; see **P3_ROADMAP** |
| **Browser bundle** | Native addon is Node-first. | Distribution plan |
| **`parse(parser, nodes)` extensions** | Conflicts with Rust lexer/parser. | Document “won’t implement” |
| **Parallel `asyncAll`** | Would break sequential semantics and tests. | Product decision |
| **Callback-only core `render`** | Duplicates Promise path. | Use [`render-with-callback.js`](render-with-callback.js) |

---

## Conformance & perf

### Current state

- **142** allowlisted JSON vectors (non-skipped): [`render_cases.json`](native/fixtures/conformance/render_cases.json) (60) + [`filter_cases.json`](native/fixtures/conformance/filter_cases.json) (26) + [`tag_parity_cases.json`](native/fixtures/conformance/tag_parity_cases.json) (56).
- **Allowlist hygiene** — run **`npm run check:conformance-allowlist`** so every non-skipped fixture `id` appears in [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json).
- **Parity gate** — [`__test__/parity.test.mjs`](__test__/parity.test.mjs) vs `nunjucks` npm using that allowlist. Env fixtures in [`__test__/conformance/run.mjs`](__test__/conformance/run.mjs).

### Next steps

- [ ] **New conformance vectors** — when adding rows to the JSON fixtures, append the new `id` to [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json) once Runjucks matches Nunjucks; `npm run perf` can include them for trends.

### Maintainer workflow (allowlist + upstream tests)

1. After editing [`native/fixtures/conformance/*.json`](native/fixtures/conformance/) (non-`skip` rows), run **`npm run check:conformance-allowlist`** — every fixture `id` must appear in [`perf/conformance-allowlist.json`](perf/conformance-allowlist.json).
2. **Parity vs npm Nunjucks:** [`__test__/parity.test.mjs`](__test__/parity.test.mjs) uses that allowlist; add **`compareWithNunjucks: false`** + **`divergenceNote`** when syntax or semantics intentionally differ ([Testing model](#testing-model-full-parity-vs-partial-vs-runjucks-only-goldens)).
3. **Extra regressions** (not a substitute for allowlist): extend [`__test__/upstream/filters-ported.test.mjs`](__test__/upstream/filters-ported.test.mjs) when a **copySafeness** / filter-chain mismatch appears; file an issue or add a minimal case before widening scope.

### Behavioral follow-ups (incremental)

- **Safe-string / `copySafeness`:** extend [`__test__/upstream/filters-ported.test.mjs`](__test__/upstream/filters-ported.test.mjs) and/or filter conformance rows when a real template finds a new chain; no blanket guarantee vs Nunjucks for every filter ordering.
- **`Map` / `Set`:** policy unchanged — use [`serialize-context.js`](serialize-context.js) / **`serializeContextForRender`** or plain objects; see **Map / Set / length** above.
- **Include / import / extends:** keep exercising edge cases in Rust ([`composition.rs`](native/crates/runjucks-core/tests/composition.rs)) and Node (`__test__/tags-extended.test.mjs`); promote to **`parity.test.mjs`** only when stock **nunjucks 3.2.4** accepts the same syntax.

---

## Out of scope / non-goals (current)

- **Nunjucks callback-only async in core**, **true parallel `asyncAll`**, **browser precompile** as a supported path — see [`P3_ROADMAP.md`](P3_ROADMAP.md) (Promise-based async templates are shipped; optional [`render-with-callback.js`](render-with-callback.js) for `(err, html)` ergonomics).
- **Nunjucks browser precompile** workflow as a supported product path (runjucks is Node-native).
- Exact **`undefined` vs `null`** JS semantics in Rust’s JSON model (both collapse similarly; templates should not rely on distinction).

---

## References

- **Per-template / extension autoescape spike:** [`PER_TEMPLATE_AUTOESCAPE.md`](PER_TEMPLATE_AUTOESCAPE.md)
- **P3 deferred tracks (precompile, browser, callback-async):** [`P3_ROADMAP.md`](P3_ROADMAP.md)
- **User docs (this repo):** [`docs/src/content/docs/guides/`](docs/src/content/docs/guides/)
- **Vendored Nunjucks:** [`../nunjucks/nunjucks/src/`](../nunjucks/nunjucks/src/)
- **Conformance fixtures:** [`native/fixtures/conformance/`](native/fixtures/conformance/)
- **Perf harness:** [`perf/README.md`](perf/README.md)
- **Rust core:** [`native/crates/runjucks-core/src/`](native/crates/runjucks-core/src/)
- **NAPI bindings:** [`native/crates/runjucks-napi/src/lib.rs`](native/crates/runjucks-napi/src/lib.rs)
