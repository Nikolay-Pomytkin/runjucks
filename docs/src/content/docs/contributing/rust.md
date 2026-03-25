---
title: Rust API (rustdoc)
description: How to browse the Runjucks engine crate documentation and repository layout.
---

The npm package is a thin binding layer over the **`runjucks_core`** crate in `native/crates/runjucks-core/`. Most users only need the [Node.js API](../../api/) and the [JavaScript API guide](../guides/javascript-api/).

## Workspace layout (contributors)

| Piece | Role |
|--------|------|
| **`runjucks_core`** | Template engine: lexer, parser, AST, renderer, environment, filters |
| **`runjucks-napi`** | Node native addon that exposes the JavaScript API |
| **`native/fixtures/`** | Conformance and test data |

Expression parsing, tag parsing, and built-in filters live inside **`runjucks_core`**. Use **rustdoc** (below) for module-level detail rather than duplicating it in the Starlight guides.

## On this site

Each docs build runs `cargo doc` on `runjucks_core` and copies the HTML into **`/rustdoc/`** (under your [base path](https://docs.astro.build/en/reference/configuration-reference/#base) if you use one for GitHub Pages). The crate root is:

**`/rustdoc/runjucks_core/`** (e.g. `https://<owner>.github.io/<repo>/rustdoc/runjucks_core/` for a project site).

Use the sidebar link **Rust crate (rustdoc)** to open it.

## Local rustdoc

From the repository root (`runjucks/`):

```bash
cargo doc --manifest-path native/Cargo.toml --no-deps -p runjucks_core --open
```

This documents the **`runjucks_core`** package only (`--no-deps`). All engine modules are public in that crate.

## docs.rs

If **`runjucks_core`** is published to [crates.io](https://crates.io), documentation will appear at `https://docs.rs/runjucks_core` automatically. Until then, use the hosted rustdoc on this site or local `cargo doc` as above.
