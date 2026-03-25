---
title: Template language
description: What Runjucks supports in templates — tags, expressions, filters, and whitespace — versus full Nunjucks.
---

Runjucks aims to match [Nunjucks templating](https://mozilla.github.io/nunjucks/templating.html) closely. This page summarizes **user-visible** behavior. For exact Node entrypoints, see [JavaScript API](./javascript-api/).

## Text and comments

| Feature | Notes |
|--------|--------|
| Plain text | Output as-is. |
| `{# … #}` | Comments removed from output. |

## `{{ … }}` expressions

- **Literals** — strings, numbers, booleans, `null`.
- **Variables** — dotted and bracket paths; `in` and chained comparisons.
- **Operators** — arithmetic, logic, `not`, comparisons.
- **Aggregates** — `[ … ]` lists and `{ … }` objects.
- **Inline conditionals** — `a if cond else b`.
- **Filters** — `value \| filter` and `value \| filter(args)`; pipelines chain left-to-right.
- **`is` tests** — `value is name` and call forms such as `equalto(…)` / `sameas(…)` where supported.
- **Calls** — macros, `super()`, `caller()`, and built-in globals such as `range`, `cycler`, `joiner` (see [JavaScript API](./javascript-api/) for globals and custom behavior).

**Slices:** Jinja-style array slices (e.g. `arr[1:4]`, `arr[::2]`) are accepted without a separate “compat” install — unlike stock `nunjucks`, which needs `installJinjaCompat()` for that syntax.

Built-in **filter** names and behavior are listed in the [Node.js API reference](../../api/) (TypeDoc). The set is large and growing; if something differs from Nunjucks, check [Limitations](./limitations/).

## `{% … %}` tags

Supported constructs include:

- **`if` / `elif` / `else` / `endif`** — including `elseif` alias.
- **`for` / `else` / `endfor`** — single or multiple loop variables, tuple unpack, `key, value` over objects (stable key order), `loop.*` (`index`, `index0`, `first`, `last`, `length`, `revindex`, `revindex0`), optional `{% else %}` when the sequence is empty.
- **`switch` / `case` / `default` / `endswitch`** — including fall-through behavior for empty `case` bodies (JavaScript-style).
- **`set`** — `{% set x = expr %}`, multi-target `{% set a, b = expr %}`, and block capture `{% set x %}…{% endset %}` with frame rules aligned to Nunjucks.
- **`include`** — expression template name; `ignore missing`; optional `without context` / `with context` (see [Limitations](./limitations/) for nuances vs upstream).
- **`extends`**, **`block` / `endblock`**, **`{{ super() }}`** — multi-level inheritance.
- **`macro` / `endmacro`** — defaults and keyword arguments at call sites.
- **`import` / `from`** — namespaces and imported macros (top-level macros from imported templates).
- **`call` / `endcall`**, **`caller()`** — optional caller argument lists on the opening `{% call %}` tag.
- **`filter` / `endfilter`** — block filtered as a whole.
- **`raw` / `endraw`**, **`verbatim` / `endverbatim`** — literal regions; nesting is balanced like Nunjucks.

Async-only tags (`asyncEach`, `asyncAll`, `ifAsync`) are **not** supported — see [Limitations](./limitations/).

## Whitespace control

Nunjucks-style trim markers work: `{%-`, `-%}`, `{{-`, `-}}`, and the interaction of **`trimBlocks`** / **`lstripBlocks`** with the lexer (configured on the environment — see [JavaScript API](./javascript-api/)). Block-tag trimming does not apply the same rules to comment-only lines as to `{% %}` tags; `endraw` / `endverbatim` closing tags follow upstream newline behavior.

## Custom tag extensions

Register opening tag names (and optional closing names) on the environment; your **`process(context, args, body)`** runs when those tags appear. This is a **declarative** model — not the Nunjucks extension API that customizes parsing in JavaScript. See [JavaScript API](./javascript-api/#custom-tags-addextension).

## When in doubt

Use the [Nunjucks templating docs](https://mozilla.github.io/nunjucks/templating.html) for language concepts, then verify against Runjucks behavior or the [Limitations](./limitations/) page. A detailed parity backlog for maintainers lives in the GitHub repo (`NUNJUCKS_PARITY.md`).
