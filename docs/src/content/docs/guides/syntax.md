---
title: Syntax and parity
description: What Runjucks supports today versus full Nunjucks, and how we test it.
---

This page summarizes **template language support** in the Rust engine (`runjucks_core`). For the full Nunjucks language, see the [upstream templating documentation](https://mozilla.github.io/nunjucks/templating.html).

## Supported in the lexer and renderer

| Feature | Notes |
|--------|--------|
| Plain text | Passed through as-is. |
| `{# … #}` comments | Removed from the token stream. |
| `{{ … }}` expressions | Literals, identifiers, `.` / `[ ]` / `( )` postfix, `[…]` / `{…}` aggregates, operators, `in` / `is` (including `equalto(…)` / `sameas(…)`), inline `if`, and `\|` filter pipelines. |
| Whitespace control | `{%-`, `-%}`, `{{-`, `-}}` and trimming between tokens (Nunjucks-style). See [lexer](../../rustdoc/runjucks_core/lexer/index.html) docs. |
| Built-in filters (subset) | `upper`, `lower`, `length`, `join`, `replace`, `round`, `escape` / `e`, `default`, `abs`, `capitalize` — see `runjucks_core::filters::apply_builtin`. |
| `{% … %}` statements | **`if` / `elif` / `elseif` / `else` / `endif`**, **`for … in …` / `else` / `endfor`** (single loop variable only), **`set name = expr`**. Other tags are rejected at parse time. |
| `{% raw %}` / `{% verbatim %}` | Lexer only (literal regions). |

## Not implemented yet

- Macros, `extends`, `include`, `import`, `block`, `call`, `filter` blocks, `switch`, async tags, and most other Nunjucks-specific tags.
- Arbitrary **function calls** in expressions (`foo()`) except as used by `is` tests (`equalto`, `sameas`).
- Filters such as **`batch`**, full **`default`** semantics for missing keys vs `null`, and “safe” / double-escaping rules matching Nunjucks exactly.
- **`sameas`** for non–object/array values follows value equality; for two objects or two arrays the engine returns `false` (no reference identity in JSON).

## Conformance tests

Rust integration tests compare rendered output to golden strings from JSON fixtures under `native/fixtures/conformance/`. A few cases stay **skipped** (e.g. `undefined`, `batch`, `set` + safe-string escaping) — see [`conformance.rs`](https://github.com/Nikolay-Pomytkin/runjucks/blob/main/native/crates/runjucks-core/tests/conformance.rs).

## Further reading

- [Development](./development/) — scripts, conformance layout, rustdoc.
- [Architecture](./architecture/) — pipeline vs Nunjucks.
- [Node.js API](../../api/) — `renderString`, `Environment`.
