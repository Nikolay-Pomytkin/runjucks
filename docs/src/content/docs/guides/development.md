---
title: Development
description: Tests, conformance vectors, and project layout for contributors.
---

## Scripts (package root)

| Command | Purpose |
|--------|---------|
| `npm run build` | Release native addon + generated JS/TS |
| `npm run build:debug` | Debug native build |
| `npm test` | Node tests: `__test__/*.test.mjs`, JSON conformance, parity vs `nunjucks` (build first) |
| `npm run test:rust` | Rust integration tests (`native/crates/runjucks-core/tests/`) |
| `npm run test:rust:green` | Smaller subset of Rust tests |
| `npm run test:conformance:rust` / `test:conformance:node` | JSON goldens (subset of `npm test`) |

## Layout

- **Node package** — `package.json`, `index.js`, `index.d.ts`, `__test__/`, generated `*.node`
- **`native/`** — Cargo workspace (`Cargo.toml`, `crates/runjucks-core/`, `crates/runjucks-napi/`, fixtures under `native/fixtures/`)

The **`runjucks_core`** crate documents the engine publicly; browse it on the docs site (**Rust crate (rustdoc)**) or with `cargo doc -p runjucks_core`.

## Optional harness

For notes on running a Nunjucks-style Mocha harness against `renderString`, see `test-shim/README.md` in the repo.
