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

## `Environment`

Create with `new Environment()` or use the instance returned by `configure()`.

### Rendering

- **`renderString(template, context)`** — Render inline source. Context is a plain object; values should be JSON-serializable for predictable behavior. The environment **caches parsed templates** when the same source string is rendered again with unchanged lexer/parser settings (custom delimiters, `trimBlocks`, registered extensions, etc.).
- **`setTemplateMap({ name: source, … })`** — Provide in-memory templates for `{% include %}`, `{% extends %}`, `{% import %}`, `{% from %}`, `renderTemplate`, and `getTemplate`. There is **no** built-in filesystem loader yet — see [Limitations](./limitations/). Replacing the map clears the named-template parse cache for that environment.
- **`renderTemplate(name, context)`** — Render a template from the map. Named templates are cached by name when the loader is stable (the default in-memory map); repeated calls reuse the parsed AST if the source and parse settings are unchanged.
- **`getTemplate(name, eagerCompile?)`** — Obtain a `Template` instance; with `eagerCompile`, invalid source fails early.

### Options

- **`setAutoescape` / `setDev` / `setRandomSeed`** — Autoescape HTML in outputs, dev flag (reserved), and a fixed seed for `| random` in tests.
- **`configure({ autoescape?, dev?, throwOnUndefined?, trimBlocks?, lstripBlocks?, tags? })`** — Instance method; same flags as Nunjucks’ `configure`. **`tags`** sets custom delimiters (`blockStart`, `blockEnd`, `variableStart`, `variableEnd`, `commentStart`, `commentEnd`).

### Globals, filters, tests

- **`addGlobal(name, value)`** — JSON-serializable values **or** a **JavaScript function** invoked when the template calls `name(…)` (keyword arguments follow Nunjucks: trailing object). See [Limitations](./limitations/) for context vs globals.
- **`addFilter(name, fn)`** — `(input, ...args) => any`. Overrides a built-in filter with the same name. Runs **synchronously** during render.
- **`addTest(name, fn)`** — `(value, ...args) => boolean` (truthy return). Used for `is` tests and for `select` / `reject`. Built-in test names still use built-in implementations.

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

## `Template`

`new Template(src, env?, path?, eagerCompile?)` or `compile(…)` returns an object with **`render(context)`**. The Rust engine **parses once per `Template` instance** (lazy on first `render`, or immediately when **`eagerCompile`** is true) and reuses the parsed AST on later renders — there is no separate JavaScript bytecode cache, but inline templates do not re-lex/re-parse on every `.render()` call. **Runtime options** such as **`setAutoescape`** still apply on each render.

## Context and `throwOnUndefined`

By default, missing variables render as empty. With **`throwOnUndefined: true`** (via `configure`), accessing an undefined name is an error. **`undefined` and `null`** from JavaScript both participate in the same JSON-style model the engine uses — treat them with the same expectations as in [Limitations](./limitations/).

## Performance

For practical guidance (caching, release builds, measuring), see **[Performance](./performance/)**. The engine caches parsed templates and applies Rust-side optimizations automatically; the page explains what that means for Node apps.
