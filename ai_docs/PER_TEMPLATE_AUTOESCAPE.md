# Spike: per-template and per-extension autoescape

This note supports the **Deferred spikes** row in [`NUNJUCKS_PARITY.md`](NUNJUCKS_PARITY.md) (“per-template / extension autoescape”). It does **not** implement the feature; it records **upstream behavior** and **Runjucks behavior** so a future implementation can be scoped against proof.

## Nunjucks 3.2.4 (npm / upstream sources)

- **Global flag:** `Environment` stores `opts.autoescape` (truthy/falsy in JS). Output uses `runtime.suppressValue(val, autoescape)` where `autoescape` reflects global settings for normal interpolations.
- **Extension tags:** For `CallExtension`, the compiler emits `suppressValue(…, ext.autoescape && env.opts.autoescape)` — see [`compiler.js` (`compileCallExtension`)](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/compiler.js) and [`nodes.js` (`CallExtension`)](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/nodes.js) (`this.autoescape = ext.autoescape`). An extension can set **`this.autoescape = false`** so its **returned string is not escaped** even when `env.opts.autoescape` is true.
- **Upstream test:** `nunjucks/tests/compiler.js` — *“should not autoescape when extension set false”* — expects raw `<b>Foo</b>` with global autoescape on.

So “per-extension autoescape” in Nunjucks is **not** per filename; it is a property on **JS extension objects** used with the **`parse()`** pipeline. Runjucks does not expose that pipeline.

## Runjucks today

- **Single global boolean** in Rust: [`Environment::autoescape`](native/crates/runjucks-core/src/environment.rs). NAPI maps JS `configure({ autoescape })` truthiness to that bool.
- **Custom extension tags:** [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) (`Node::ExtensionTag`) applies **`escape_html` to handler output whenever `env.autoescape` is true** — there is **no** per-extension `autoescape` flag (the declarative `addExtension` API does not carry one).

## Observable mismatch (proof)

With global autoescape **on**:

- Nunjucks + extension `autoescape: false` → raw HTML from extension (see spike test below).
- Runjucks + `addExtension` → extension output is **always** escaped when `setAutoescape(true)` (see [`__test__/extensions.test.mjs`](__test__/extensions.test.mjs) and [`__test__/per-template-autoescape-spike.test.mjs`](__test__/per-template-autoescape-spike.test.mjs)).

## Future implementation (outline only)

1. Decide whether to support **per-extension** escape opt-out (closer to Nunjucks) vs **per-template file extension** rules (often described in mozilla.io docs — verify against **nunjucks@3.2.4** before coding).
2. If per-extension: extend the declarative registration API (e.g. optional boolean on `addExtension`) and thread through `Node::ExtensionTag` / async renderer **without** exposing `parse()`.
3. If per-template filename rules: track **current template name** in render state and evaluate rules in [`renderer.rs`](native/crates/runjucks-core/src/renderer.rs) / [`async_renderer`](native/crates/runjucks-core/src/async_renderer/) — larger change; add **failing golden** vs Nunjucks first.
