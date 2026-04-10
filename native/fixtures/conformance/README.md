# Conformance fixtures (Nunjucks parity)

JSON files are arrays of cases consumed by Rust (`native/crates/runjucks-core/tests/conformance.rs`) and optionally by Node (`__test__/conformance/`). Additional tag-focused vectors live in [`tag_parity_cases.json`](tag_parity_cases.json) and are run by `tests/tag_parity.rs`.

## Schema (per object)

| Field | Required | Description |
|-------|----------|-------------|
| `id` | yes | Stable identifier for failures. |
| `source` | no | Citation, e.g. `nunjucks/tests/tests.js` and line range. |
| `template` | yes | Template string. |
| `context` | no | JSON object (default `{}`). |
| `env` | no | `{ "autoescape": bool, "dev": bool }` — partial ok, defaults match `Environment::default()`. |
| `expected` | yes | Exact string output Nunjucks produces (golden). |
| `skip` | no | If `true`, Rust (`conformance` / `tag_parity` tests) and Node conformance skip the case until the engine matches (see [`ai_docs/NUNJUCKS_PARITY.md`](../../../ai_docs/NUNJUCKS_PARITY.md)). |
| `compareWithNunjucks` | no | Default `true`. If `false`, [`__test__/parity.test.mjs`](../../../__test__/parity.test.mjs) checks runjucks output against `expected` only (must not compare to nunjucks 3.2.4). Requires **`divergenceNote`**. |
| `divergenceNote` | when `compareWithNunjucks` is false | Human-readable reason (and pointer to `ai_docs/NUNJUCKS_PARITY.md`); enforced by **`npm run check:conformance-allowlist`**. |

Scenarios are BSD-2-Clause Nunjucks test vectors; `source` should point to upstream for traceability.

### Error fixtures (`error_cases.json`)

Separate array for **Node-only** tests ([`__test__/error-cases.test.mjs`](../../../__test__/error-cases.test.mjs)): templates that must **throw**; each row has `errorContains` (substring of the Runjucks error message). Not on the **perf allowlist** and not compared to Nunjucks error text.

When adding a case, append its `id` to [`perf/conformance-allowlist.json`](../../perf/conformance-allowlist.json) (unless `skip: true`). Run **`npm run check:conformance-allowlist`** from the package root to verify.
