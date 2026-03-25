---
title: Limitations and differences
description: Where Runjucks intentionally differs from Nunjucks or does not yet match it.
---

Runjucks targets **Node.js** and **synchronous** rendering. This page lists **product-level** gaps and quirks. For a maintainer-facing checklist, see **`NUNJUCKS_PARITY.md`** in the repository.

## Node.js and loaders

- **No filesystem loader** in the package — use **`setTemplateMap`** with an object of name → source strings, or build your own loader that populates the map.
- **No Express** view-engine helper — wire `render` / `renderString` yourself.
- **No browser / UMD bundle** as a first-class artifact — the runtime is a native addon for Node.

## Async and precompile

- **No async `render` / `renderString`** — templates run to completion on the calling thread; async tags (`asyncEach`, `asyncAll`, `ifAsync`) are not supported.
- **No `precompile` / `precompileString`** producing cached bytecode — templates are parsed when you render (the Rust side is built for speed, but there is no Nunjucks-style JS precompile workflow).

## Globals and callables

- **`addGlobal(name, value)`** accepts **JSON-serializable** values **or** a **JavaScript function** for Nunjucks-style `{{ fn(…) }}` calls (same thread as `render`; keyword arguments become a trailing plain object). See **`NUNJUCKS_PARITY.md`** (P1).
- **Render context** (`renderString(…, ctx)`) is still **JSON-shaped** — you cannot pass live functions inside `ctx` and expect them to be invoked from templates (use **`addGlobal`** on the environment instead).

## Custom extensions

- **`addExtension`** uses a **declarative** model: tag names, optional block end names, and a **`process`** callback. Nunjucks’ **parser hook** (`parse(parser, nodes)`) for custom AST nodes is **not** exposed.

## Import / include / extends (nuances)

- **`import` / `from`**: only **top-level macros** are collected; side effects from running imported templates are not the same as Nunjucks in every edge case. Modifiers like `with context` on imports are parsed but not always equivalent to upstream.
- **`include`**: `with context` / `without context` may differ from stock **nunjucks 3.x** parsing in subtle ways; Runjucks documents behavior via tests and parity where applicable.
- **`extends`**: dynamic parent names resolve at render time; static cycle analysis may not cover every dynamic path ahead of time.

## Filters and types

- Some filters differ in edge cases (e.g. **`length`** on non-array objects, **safe-string** chaining vs Nunjucks). Prefer conformance tests or side-by-side checks for critical templates.
- **`undefined` vs `null`** from JavaScript both map into the engine’s JSON-style value model — do not rely on distinct runtime behavior between them in templates.

## Jinja compatibility

- Runjucks accepts **array slice** syntax without requiring a separate **`installJinjaCompat()`**-style shim (Nunjucks needs that for slices). A dedicated Jinja-compat API flag is not required for slices; other Jinja shims from Nunjucks are not mirrored as a single API.
