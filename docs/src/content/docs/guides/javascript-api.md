---
title: JavaScript API
description: configure, Environment, templates, maps, and extensions — how to use Runjucks from Node.js.
---

Types ship in `index.d.ts`; the [generated API reference](../../api/) is the source for signatures. Below is a **usage-oriented** overview without repeating every parameter.

## Top-level helpers

| Export | Role |
|--------|------|
| `renderString(template, context)` | Render a string with the **default** environment (see `configure`). |
| `configure(options?)` | Create or update the module default `Environment` (Nunjucks-style). Returns that environment. |
| `render(name, context)` | Render a **named** template from the default env’s template map. |
| `reset()` | Clear the module default environment (mainly for tests). |
| `compile(src, env?, path?, eagerCompile?)` | Build a `Template` from source; optional env and eager validation. |
| `serializeContextForRender` | From **`@zneep/runjucks/serialize-context`** — converts `Map` / `Set` (and nested values) to JSON-friendly data for `context`. |
| `fetchTemplateMap` | From **`@zneep/runjucks/fetch-template-map`** — `async` helper: given `{ name, url }[]`, returns a **`name → source`** object for **`setTemplateMap`** after **`fetch`**. Use when loading templates over HTTP(S); render stays sync after sources are loaded. |
| `installJinjaCompat` | From **`@zneep/runjucks/install-jinja-compat`** — **no-op**; Nunjucks needs this call for slice syntax; Runjucks does not. Safe to import when porting apps that still call it. |
| `renderWithCallback` / `renderWithCallbackAsync` | From **`@zneep/runjucks/render-with-callback`** — invoke **`(err, html)`** callbacks wrapping **`renderTemplate`** / **`renderTemplateAsync`** (optional migration sugar; core APIs remain sync / Promise). |

## `Environment`

Create with `new Environment()` or use the instance returned by `configure()`.

### Rendering

- **`renderString(template, context)`** — Render inline source. Context is a plain object; values should be JSON-serializable for predictable behavior. The environment **caches parsed templates** when the same source string is rendered again with unchanged lexer/parser settings (custom delimiters, `trimBlocks`, registered extensions, etc.).
- **`setTemplateMap({ name: source, … })`** — Provide in-memory templates for `{% include %}`, `{% extends %}`, `{% import %}`, `{% from %}`, `renderTemplate`, and `getTemplate`. Replacing the map clears the named-template parse cache for that environment.
- **`setLoaderRoot(path)`** — Load named templates from a directory on disk. `path` should be absolute (or resolve it before calling). Template names use forward slashes; paths must stay under the root (no `..` escape). Replaces any previous loader (`setTemplateMap` or an earlier `setLoaderRoot`). See [Limitations](./limitations/) for Express and security notes.
- **`setLoaderCallback(fn)`** — Sync **`(name: string) => string | null | { src: string }`** (main thread). `null` means missing template. Replaces any previous loader. Does not use per-name parse caching (always loads fresh source from JS). Useful for custom resolution or wrapping HTTP fetches in user code.
- **`invalidateCache()`** — Clears **named** and **inline** parse caches (Nunjucks-style `invalidateCache`). Replacing the loader via `setTemplateMap` / `setLoaderRoot` / `setLoaderCallback` still clears the named cache.
- **`renderTemplate(name, context)`** — Render a template from the active loader (map or disk).
- **`getTemplate(name, eagerCompile?)`** — Obtain a `Template` instance; with `eagerCompile`, invalid source fails early.

### Options

- **`setAutoescape` / `setDev` / `setRandomSeed`** — Autoescape HTML in outputs, dev flag (reserved), and a fixed seed for `| random` in tests. **`setAutoescape` accepts only a boolean.**
- **`configure({ autoescape?, dev?, throwOnUndefined?, trimBlocks?, lstripBlocks?, tags? })`** — Instance method; mirrors Nunjucks’ `configure` for these flags. **`autoescape`** accepts **boolean, string, number, or `null`** and is normalized with **JavaScript-like truthiness** (same idea as Nunjucks’ `suppressValue`): `false`, `0`, `""`, and `null` turn escaping **off**; other values (including non-empty strings such as `"html"`) turn it **on**. The Rust engine still stores a **single boolean** — there is no per-file extension autoescape (see [Limitations](./limitations/)). **`tags`** sets custom delimiters (`blockStart`, `blockEnd`, `variableStart`, `variableEnd`, `commentStart`, `commentEnd`).

### Globals, filters, tests

- **`addGlobal(name, value)`** — JSON-serializable values **or** a **JavaScript function** invoked when the template calls `name(…)` (keyword arguments follow Nunjucks: trailing object). See [Limitations](./limitations/) for context vs globals.
- **`addFilter(name, fn)`** — `(input, ...args) => any`. Overrides a built-in filter with the same name. Runs **synchronously** during render.
- **`addTest(name, fn)`** — `(value, ...args) => boolean` (truthy return). Used for `is` tests and for `select` / `reject`. Built-in test names still use built-in implementations.

### Async rendering {#async-rendering}

Runjucks supports Promise-based async rendering, matching the Nunjucks async API surface:

- **`renderStringAsync(template, context)`** — Returns `Promise<string>`. Supports all sync features plus async-only tags (`asyncEach`, `asyncAll`, `ifAsync`) and async filters/globals.
- **`renderTemplateAsync(name, context)`** — Same as above for named templates from the active loader.
- **`addAsyncFilter(name, fn)`** — Register an async filter `(input, ...args) => any`. Available in both sync and async render paths.
- **`addAsyncGlobal(name, fn)`** — Register an async global callable `(...args) => any`. Available in both sync and async render paths.

```js
const env = new Environment();
env.setTemplateMap({ 'page.html': '{% asyncEach item in items %}{{ item | shout }}{% endeach %}' });

env.addAsyncFilter('shout', (val) => String(val).toUpperCase());

const result = await env.renderTemplateAsync('page.html', { items: ['a', 'b', 'c'] });
// => "ABC"
```

> **Note:** Rendering executes synchronously on the calling thread; the result is wrapped in an already-resolved Promise. This matches the Nunjucks callback API surface but does not yield the event loop during render. JS callbacks registered via `addAsyncFilter` / `addAsyncGlobal` must be **synchronous** functions (not `async` functions) — the "async" refers to the render mode, not the callback. Passing an `async` function will produce a clear error at render time.

### Custom tags (`addExtension`) {#custom-tags-addextension}

```js
env.addExtension(
  'myExtensionName',
  ['tagOne', 'tagTwo'],
  { tagOne: 'endtagOne' }, // optional: opening tag → closing tag name
  (context, argsString, body) => {
    // body is null for simple tags; a string for block tags
    return '…'
  }
)
```

Tag **parsing** is fixed in the engine; your callback only **produces output** from the already-parsed arguments and optional inner body. This differs from Nunjucks’ JavaScript `parse()` hook for extensions.

### `getExtension` (introspection)

- **`getExtension(name)`** returns **`{ name, tags, blocks }`** when a custom extension is registered, or **`null`** if not. **`tags`** lists opening tag names (**sorted** for stable output); **`blocks`** maps an opening tag name to its closing tag name (for block-style tags only). This object is for **tooling and tests** — it does not expose the underlying Rust or JavaScript `process` callback. Nunjucks’ **`getExtension`** returns the **live extension object** (including any `parse` hook); Runjucks only exposes this **descriptor**, not parser hooks or the same object identity as upstream.

## Express (optional)

```js
const express = require('express')
const { expressEngine } = require('@zneep/runjucks/express')

const app = express()
app.set('views', '/path/to/views')
app.set('view engine', 'njk')
expressEngine(app, { ext: 'njk' })

app.get('/', (req, res) => {
  res.render('home', { title: 'Hi' })
})
```

`expressEngine` creates a new `Environment`, applies optional `configure(opts)`, calls `setLoaderRoot` from `opts.views` or `app.get('views')`, and registers the view engine. View locals are merged with `res.locals` and passed through **`JSON.parse(JSON.stringify(…))`** so only JSON-compatible data reaches the engine (Express often attaches functions; those are dropped). When **`app.get('view cache') === false`** (common in development), by default **`invalidateCache()`** runs before each render so the Rust parse cache does not hide edits to files on disk; set **`invalidateOnViewCacheOff: false`** on `expressEngine` opts to disable. Rendering is **synchronous**; there is no promise-based API.

## `Template`

`new Template(src, env?, path?, eagerCompile?)` or `compile(…)` returns an object with **`render(context)`**. The Rust engine **parses once per `Template` instance** (lazy on first `render`, or immediately when **`eagerCompile`** is true) and reuses the parsed AST on later renders — there is no separate JavaScript bytecode cache, but inline templates do not re-lex/re-parse on every `.render()` call. **Runtime options** such as **`setAutoescape`** still apply on each render.

## Context and `throwOnUndefined`

By default, missing variables render as empty. With **`throwOnUndefined: true`** (via `configure`), accessing an undefined name is an error. **`undefined` and `null`** from JavaScript both participate in the same JSON-style model the engine uses — treat them with the same expectations as in [Limitations](./limitations/).

## Performance

For practical guidance (caching, release builds, measuring), see **[Performance](./performance/)**. The engine caches parsed templates and applies Rust-side optimizations automatically; the page explains what that means for Node apps.
