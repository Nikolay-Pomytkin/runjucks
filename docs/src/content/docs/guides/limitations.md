---
title: Limitations and differences
description: Where Runjucks intentionally differs from Nunjucks or does not yet match it.
---

Runjucks targets **Node.js** and **synchronous** rendering. This page lists **product-level** gaps and quirks. For a maintainer-facing checklist, see **`NUNJUCKS_PARITY.md`** in the repository (including **Testing model**: full parity vs partial vs Runjucks-only JSON goldens). For throughput and caching (what is *not* a limitation), see [Performance](./performance/).

## Node.js and loaders

- **`autoescape` option** — Nunjucks `configure({ autoescape: … })` accepts **`true`**, **`false`**, or a **string** (extension-based escaping, e.g. only `*.html`). Runjucks supports **boolean only** (`setAutoescape` / `configure({ autoescape })`): either all normal variable output is escaped or none is, per environment. There is no per-filename extension switch.
- **Filesystem templates** — call **`setLoaderRoot(absolutePath)`** on an [`Environment`](./javascript-api/) so named templates load from disk (relative paths under that root; `..` traversal is rejected). Alternatively use **`setTemplateMap`** with an object of name → source strings, **`setLoaderCallback`** for a sync JS `getSource(name)` (no built-in **`http(s):`** loader — fetch in JS and return source, or use disk/map).
- **Express** — optional helper **`require('@zneep/runjucks/express').expressEngine(app, opts?)`** registers `app.engine` for `.njk` (or your chosen `ext`) using `setLoaderRoot` from `app.get('views')` or `opts.views`. Rendering is **synchronous**; there is no async `render` callback like some Nunjucks setups.
- **No browser / UMD bundle** as a first-class artifact — the runtime is a native addon for Node.

## Async and precompile

- **No async `render` / `renderString`** — templates run to completion on the calling thread; async tags (`asyncEach`, `asyncAll`, `ifAsync`) are not supported.
- **No `precompile` / `precompileString`** emitting JavaScript — the Rust engine parses templates to an internal AST and **caches** parses per environment / `Template` (see [JavaScript API](./javascript-api/)); there is no Nunjucks-style **JS** precompile artifact or browser bundle workflow.

## Globals and callables

- **`addGlobal(name, value)`** accepts **JSON-serializable** values **or** a **JavaScript function** for Nunjucks-style `{{ fn(…) }}` calls (same thread as `render`; keyword arguments become a trailing plain object). See **`NUNJUCKS_PARITY.md`** (P1).
- **Render context** (`renderString(…, ctx)`) is still **JSON-shaped** — you cannot pass live functions inside `ctx` and expect them to be invoked from templates (use **`addGlobal`** on the environment instead).

## Custom extensions

- **`addExtension`** uses a **declarative** model: tag names, optional block end names, and a **`process`** callback. Nunjucks’ **parser hook** (`parse(parser, nodes)`) for custom AST nodes is **not** exposed.

## Import / include / extends (nuances)

- **`import` / `from`**: only **top-level macros** are collected; side effects from running imported templates are not the same as Nunjucks in every edge case. Modifiers like `with context` on imports are parsed but not always equivalent to upstream.
- **`include`**: Runjucks parses **`without context`** and **`with context`** on `{% include %}` (see `native/crates/runjucks-core/tests/composition.rs` and `__test__/tags-extended.test.mjs` in the repo). Stock **nunjucks 3.2.4** does **not** accept those modifiers on **`include`** (it will parse-error). JSON conformance cases that must match **nunjucks** line-for-line (`__test__/parity.test.mjs`) therefore use plain includes and nested includes; behavior for **`include` + context modifiers** is covered by Rust tests and Node tests, not by the npm parity gate.
- **`extends`**: dynamic parent names resolve at render time. A **literal-only** `{% extends "…" %}` chain is checked for cycles before render (in addition to runtime resolution); dynamic `{% extends expr %}` is not analyzed statically.

## Filters and types

- Some filters differ in edge cases (e.g. **`length`** on non-array objects, **safe-string** chaining vs Nunjucks). Prefer conformance tests or side-by-side checks for critical templates.
- **`undefined` vs `null`** from JavaScript both map into the engine’s JSON-style value model — do not rely on distinct runtime behavior between them in templates.

## Map, Set, RegExp, and incremental parity

- **`Map` / `Set`** in render context are not automatically expanded into JSON objects — pass plain objects/arrays or use **`require('@zneep/runjucks/serialize-context').serializeContextForRender(obj)`** for an explicit `Map`/`Set` → JSON conversion at the boundary.
- **Regular expressions** in templates use the Rust engine’s regex support, not full **ECMAScript** `RegExp` semantics; see **`NUNJUCKS_PARITY.md`** for flags and limitations.
- **Filter safeness** (`escape`, `safe`, `forceescape`) and copy-on-escape behavior: tighten with targeted tests when a real template shows a gap; the repo does not guarantee bit-for-bit Nunjucks output for every edge chain.

## Jinja compatibility

- Runjucks accepts **array slice** syntax without requiring a separate **`installJinjaCompat()`**-style shim (Nunjucks needs that for slices). A dedicated Jinja-compat API flag is not required for slices; other Jinja shims from Nunjucks are not mirrored as a single API.
