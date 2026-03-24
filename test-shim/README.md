# Nunjucks test harness shim (optional)

Upstream [nunjucks/tests/util.js](https://github.com/mozilla/nunjucks/blob/master/tests/util.js) builds `Environment(loader)` + `Template` + `render`. Runjucks today exposes `renderString` / `Environment` from [`index.js`](../index.js) without filesystem loaders or async `Template.render`.

## What full parity would need

- `FileSystemLoader` (or stub) and `new Template(src, env)` delegating to the native `renderString` path.
- Async `render` if you run Mocha tests that use callbacks.
- Precompile / slim mode tests are out of scope until the compiler exists in Rust.

## Using this shim

`nunjucks-compat.js` provides minimal **sync** `Template` + `Environment` wrappers for **string-only** templates. Point a **forked** `util.js` at it (or `module-alias`) when experimenting with a subset of `tests/tests.js`.

This does **not** run the real Nunjucks repo’s `npm test`; it is a starting point for `USE_RUNJUCKS=1`-style experiments.

## Limitations

- No `asyncFilters`, `extensions`, `installJinjaCompat`, or loader-backed templates.
- Line/column and error shape differ from Nunjucks.
