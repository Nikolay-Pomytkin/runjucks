# Conformance fixtures (Nunjucks parity)

JSON files are arrays of cases consumed by Rust (`native/crates/runjucks-core/tests/conformance.rs`) and optionally by Node (`__test__/conformance/`).

## Schema (per object)

| Field | Required | Description |
|-------|----------|-------------|
| `id` | yes | Stable identifier for failures. |
| `source` | no | Citation, e.g. `nunjucks/tests/tests.js` and line range. |
| `template` | yes | Template string. |
| `context` | no | JSON object (default `{}`). |
| `env` | no | `{ "autoescape": bool, "dev": bool }` — partial ok, defaults match `Environment::default()`. |
| `expected` | yes | Exact string output Nunjucks produces (golden). |

Scenarios are BSD-2-Clause Nunjucks test vectors; `source` should point to upstream for traceability.
