# Upstream Nunjucks-ported tests

This folder **mirrors** scenarios from the vendored [Nunjucks Mocha suite](https://github.com/mozilla/nunjucks/tree/master/tests) (`nunjucks/tests/*.js`) using Node’s built-in test runner (`node:test`), **not** by executing Mocha against Runjucks. Nunjucks’ helpers assume `FileSystemLoader`, async `render`, the JS compiler, `installJinjaCompat`, and `addExtension({ parse })` — none of which map 1:1 to Runjucks.

## Layout

| File | Source |
|------|--------|
| [`upstream-harness.mjs`](upstream-harness.mjs) | Shared env (`dev: true` like `tests/util.js`), optional `templateMap` from disk, optional reference check vs the `nunjucks` npm package. |
| [`tests-ported.test.mjs`](tests-ported.test.mjs) | `nunjucks/tests/tests.js` — `is` tests (skipped items are unknown tests or known partials). |
| [`filters-ported.test.mjs`](filters-ported.test.mjs) | Cherry-picked sync cases from `nunjucks/tests/filters.js`. |

## Running

From the `runjucks` package root:

```bash
npm run test:upstream
```

Requires a successful `npm run build` so the native addon matches the Rust core (same as `npm test`).

## Vendored tree path

By default, helpers look for `nunjucks/tests` at `../../nunjucks/tests` relative to this package (sibling folder in the monorepo). Override with:

```bash
export RUNJUCKS_NUNJUCKS_TESTS=/absolute/path/to/nunjucks/tests
```

## Relationship to other gates

- **JSON goldens** ([`native/fixtures/conformance/`](../../native/fixtures/conformance/)) remain the canonical **bit-identical** vectors for Rust + allowlisted Node parity.
- **`npm test`** / [`parity.test.mjs`](../parity.test.mjs) — allowlisted IDs vs `nunjucks` npm.
- **This suite** — broader **upstream-inspired** coverage and **skipped** inventory for parity gaps (`is` tests, `sameas` identity, `int` vs float strings, …).

## Non-goals

- Do not run vendored `nunjucks` Mocha verbatim inside this repo without a full Nunjucks API shim.
- Do not port `compiler.js`, `express.js`, or `precompile.js` unless product scope expands; those target Nunjucks’ JS compiler or server integration.

See [`ai_docs/NUNJUCKS_PARITY.md`](../../ai_docs/NUNJUCKS_PARITY.md) for maintainer-facing strategy and follow-on epics.
