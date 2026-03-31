---
title: Migrating from Nunjucks
description: Swap nunjucks for @zneep/runjucks on Node.js — what maps cleanly, what to check first, and where behavior differs.
---

[Runjucks](https://github.com/Nikolay-Pomytkin/runjucks) targets the **same mental model** as [Nunjucks](https://mozilla.github.io/nunjucks/): familiar `{{ }}` / `{% %}` syntax, `Environment`, `configure`, `renderString`, and composition with `extends`, `include`, `import`, and macros. The engine is a **Rust** tree-walker behind N-API instead of compile-to-JavaScript + `eval`.

**”Drop-in” here means:** typical Node servers and CLIs that render strings or disk-backed templates—**not** every feature on the [Nunjucks API](https://mozilla.github.io/nunjucks/api.html) page (precompile, browser bundles, and some loaders are out of scope today). Async rendering (`renderStringAsync`, `renderTemplateAsync`) is supported — see [JavaScript API](./javascript-api/#async-rendering). See [Limitations](./limitations/) and the repo’s [`NUNJUCKS_PARITY.md`](https://github.com/Nikolay-Pomytkin/runjucks/blob/main/NUNJUCKS_PARITY.md) for the full matrix.

## When migration is usually straightforward

- **`renderString` / `render` with string templates** — same overall pattern; context remains a plain object (JSON-shaped going into Rust).
- **`configure`, `Environment`, `compile`, `Template`** — same ideas; reuse `Template` instances across renders for steady-state performance.
- **In-memory template graphs** — `setTemplateMap({ name: source, … })` replaces loader-only setups that used a custom loader returning map-backed sources.
- **Filesystem templates** — use **`setLoaderRoot(absolutePath)`** instead of `FileSystemLoader` + `Environment` with a path loader (paths must stay under the root; `..` is rejected).
- **Express** — `require('@zneep/runjucks/express').expressEngine(app, opts?)` registers **`app.engine`** with disk-backed templates; see [JavaScript API](./javascript-api/#express-optional).
- **Globals and callables** — `addGlobal(name, fn)` supports **JavaScript functions** for `{{ myFn(…) }}` (keyword args follow Nunjucks conventions). Pass callables on the **environment**, not inside ad-hoc context objects, for predictable bridging.

## Step-by-step

1. **Install** — Remove `nunjucks` and add **`@zneep/runjucks`** (scoped package name; npm rejects the unscoped name `runjucks` as too close to `nunjucks`).
2. **Update imports** — Point `require` / `import` at `@zneep/runjucks` (and `@zneep/runjucks/express` for Express).

   ```js
   // ESM
   import { renderString, Environment, configure } from '@zneep/runjucks'

   // CommonJS
   const { renderString, Environment, configure } = require('@zneep/runjucks')
   ```

3. **Replace loaders** — If you used `new nunjucks.Environment([loader], opts)`, switch to `new Environment()` (or `configure`) plus **`setTemplateMap`** or **`setLoaderRoot`** / **`setLoaderCallback`** as needed. There is no built-in `http(s):` loader; fetch in JS inside **`setLoaderCallback`** if you must resolve remote templates synchronously.
4. **Express** — Replace `nunjucks.configure` + `app.engine` wiring with **`expressEngine`** (`express.js` entry); merge any `configure` options into **`expressEngine(app, { configure: { … } })`**.
5. **Build** — Consumers need a **release** native addon in production (`npm run build` from source when developing the library; published installs ship per-platform binaries). Run your template smoke tests after the swap.

## API mapping (Nunjucks → Runjucks)

| Nunjucks idea | Runjucks |
|---------------|----------|
| `nunjucks.renderString(str, ctx)` | `renderString(str, ctx)` or `env.renderString(str, ctx)` |
| `nunjucks.configure(opts)` | `configure(opts)` — same pattern for default env |
| `new nunjucks.Environment(loaders?, opts)` | `new Environment()` then `setTemplateMap` / `setLoaderRoot` / `setLoaderCallback` |
| `env.renderString`, `env.render` | `env.renderString`, `env.renderTemplate` (named templates) |
| `env.render` (async callback) | `env.renderTemplateAsync(name, ctx)` → `Promise<string>` |
| `env.addFilter` (async) | `env.addAsyncFilter(name, fn)` |
| `asyncEach` / `asyncAll` / `ifAsync` | Supported in `renderStringAsync` / `renderTemplateAsync` |
| `FileSystemLoader` path | `setLoaderRoot(absoluteDir)` |
| Custom sync loader | `setLoaderCallback((name) => src or null)` |
| `env.addFilter`, `addGlobal`, `addExtension` | Same names on `Environment` ([details](./javascript-api/)) |
| `compile` / `Template` | Supported; `Template` parses once per instance |
| `installJinjaCompat()` for slices | **Not required** for array slices in Runjucks ([limitations](./limitations/#jinja-compatibility)) |

For the full surface (including `throwOnUndefined`, `trimBlocks`, custom tags, and extension **declarative** model), see [JavaScript API](./javascript-api/).

## Pre-flight checklist (before production)

Work through these with your real templates and tests:

- **Async rendering** — `renderStringAsync` and `renderTemplateAsync` return `Promise<string>` and support `asyncEach`, `asyncAll`, `ifAsync`, `addAsyncFilter`, and `addAsyncGlobal`. JS callbacks currently run synchronously; see [JavaScript API](./javascript-api/#async-rendering).
- **No precompile / browser UMD** — Runjucks is a **Node native addon**. There is no Nunjucks-style JS precompile artifact.
- **Autoescape** — Only **boolean** global autoescape per environment; Nunjucks’ string form (extension-based) is not implemented ([limitations](./limitations/#nodejs-and-loaders)).
- **Context shape** — Render context crosses the boundary as **JSON-compatible** data. Use **`addGlobal`** for injectable functions; use [`serialize-context`](./javascript-api/) if you relied on `Map` / `Set` in context.
- **Custom extensions** — Tag bodies are parsed by the engine; your extension supplies a **`process`** callback, not Nunjucks’ `parse(parser, nodes)` hook ([limitations](./limitations/#custom-extensions)).
- **Errors** — Stack traces and message shapes differ from Nunjucks; adjust logging and any brittle string assertions.

Golden / parity tests in the repo run **`nunjucks` 3.2.4** against allowlisted fixtures ([`__test__/parity.test.mjs`](https://github.com/Nikolay-Pomytkin/runjucks/blob/main/__test__/parity.test.mjs)); treat that as regression signal, not a promise about every edge case.

## Optional: string-only test shim

For experiments against a **forked** Nunjucks test `util.js`, the repo provides a minimal sync **`Environment` + `Template`** wrapper in [`test-shim/nunjucks-compat.js`](https://github.com/Nikolay-Pomytkin/runjucks/blob/main/test-shim/nunjucks-compat.js). It does **not** run upstream’s full Mocha suite; read [`test-shim/README.md`](https://github.com/Nikolay-Pomytkin/runjucks/blob/main/test-shim/README.md) for limits.

## Next steps

- [Template language](./syntax/) — syntax reference for tags and expressions.
- [Performance](./performance/) — caching, release builds, and published vs-Nunjucks numbers.
- [Jinja2 background](./jinja2-background/) — if your team thinks in **Jinja2** terms more than Nunjucks internals.
