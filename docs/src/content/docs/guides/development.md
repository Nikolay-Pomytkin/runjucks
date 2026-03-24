---
title: Development
description: Tests, conformance vectors, and project layout for contributors.
---

## Scripts (package root)

| Command | Purpose |
|--------|---------|
| `npm run build` | Release native addon + generated JS/TS |
| `npm run build:debug` | Debug native build |
| `npm test` | Node tests (`__test__/*.test.mjs`; build first) |
| `npm run test:rust` | Rust integration tests (`native/src/tests/`) |
| `npm run test:rust:green` | Smaller subset of Rust tests |
| `npm run test:conformance:rust` / `test:conformance:node` | JSON goldens under `native/fixtures/conformance/` |

## Layout

- **Node package** — `package.json`, `index.js`, `index.d.ts`, `__test__/`, generated `*.node`
- **`native/`** — Rust crate (`Cargo.toml`, `src/`, integration tests in `native/src/tests/`)

Internal engine modules used only from the crate are `#[doc(hidden)]` in `lib.rs` unless you are contributing to the Rust side.

## Optional harness

For notes on running a Nunjucks-style Mocha harness against `renderString`, see `test-shim/README.md` in the repo.
