---
title: Jinja2 background
description: How Runjucks relates to Jinja2 and Nunjucks — familiar syntax, Node runtime, and a Rust rendering core.
---

[Mozilla Nunjucks](https://mozilla.github.io/nunjucks/) is **heavily inspired by [Jinja2](https://jinja.palletsprojects.com/)**—filters, inheritance, `macro`, `for`/`if`, and much of the expression syntax feel the same across Python, Nunjucks, and Runjucks. **Runjucks** keeps that **language shape** but implements lexing, parsing, and rendering in **Rust**, exposed to **Node.js** via N-API.

## Why Runjucks if you already like Jinja2-style templates

- **Familiar surface** — Block inheritance (`extends` / `block`), includes, imports, macros, filters, and tests match what Jinja / Nunjucks users expect; see [Template language](./syntax/).
- **Performance** — CPU-heavy templates (large loops, many interpolations) benefit from a native core and parse caching; see [Performance](./performance/) for practical tuning and a **published vs Nunjucks** snapshot versioned with the npm package.
- **Slices without a compat shim** — Nunjucks often needs **`installJinjaCompat()`** for Pythonic slice syntax in expressions; Runjucks accepts **array slices** without that extra API. Other Jinja-only shims from Nunjucks are not mirrored as one switch—see [Limitations](./limitations/#jinja-compatibility).

## How this differs from Python Jinja2

| Topic | Jinja2 (Python) | Runjucks (Node) |
|-------|------------------|-----------------|
| **Runtime** | Python objects and methods in context | **JSON-shaped** context over the FFI boundary; use **`addGlobal`** for callable injection |
| **Types** | Rich Python types (`date`, custom classes, …) | Values the engine sees are **`serde_json`-style** (null, bool, number, string, array, object) |
| **Exact semantics** | Reference implementation | Aligned with **Nunjucks 3.x** behavior and tests; small divergences are documented in [Limitations](./limitations/) |
| **Async / streaming** | Template and environment features vary | **Synchronous** render on the JS thread; no async tags |

If you port **Django / Flask / FastAPI** templates to Node, expect to **reshape context** (plain objects, ISO date strings, explicit helpers) rather than passing arbitrary Python instances through.

## Relationship to Nunjucks docs

The official [Nunjucks templating reference](https://mozilla.github.io/nunjucks/templating.html) remains a useful cross-check for **syntax**; the [API page](https://mozilla.github.io/nunjucks/api.html) documents the **full** Nunjucks product (browser, precompile, async). Runjucks documents the **Node sync subset** in [JavaScript API](./javascript-api/) and [Limitations](./limitations/).

## Next steps

- [Installation](./installation/) — Node ≥ 18, native addon from npm.
- [Migrating from Nunjucks](./migrating-from-nunjucks/) — dependency and loader swap if you currently use **`nunjucks`** on npm.
