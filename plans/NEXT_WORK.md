# Next work plan (derived from `ai_docs/`)

## Source note

`ai_docs/README.md` marks these planning docs as potentially stale maintainer notes, so treat this as a **best-current synthesis** and verify against current failing tests/issues before implementation.

## What appears to be next (in priority order)

### 1) Parity hardening: SafeString / copySafeness edge cases (P1)

Focus first on additional upstream-derived vectors for:

- `safe`, `escape`/`e`, `forceescape`, `replace`
- macro return values, `caller()`, and `super()` chains

Suggested workflow from the parity backlog:

1. Add 3–5 failing conformance vectors in one category.
2. Add non-skipped IDs to `perf/conformance-allowlist.json`.
3. Fix behavior in `native/crates/runjucks-core/src/filters.rs`.
4. Run build + JS tests + Rust tests + allowlist check.

## 2) Template composition parity: `include` / `extends` edge behavior (P1)

Add targeted fixtures for Nunjucks-valid tricky cases:

- dynamic parent expressions
- include context passing
- ignore-missing combinations

Use `compareWithNunjucks: false` + `divergenceNote` only for intentional, documented divergences.

## 3) Expression/runtime cleanup (P1)

Continue parity cleanup for comparison and `is` behavior with JSON-shaped data:

- mixed numeric/string relational cases
- undefined propagation paths

Prefer conformance JSON fixtures so Rust + Node stay aligned automatically.

## 4) Loader/Express migration polish (P2)

Expand parity-style test coverage around:

- multi-root resolution
- relative includes
- cache invalidation
- error surfaces in Express integration

Prioritize real migration pain points over broad theoretical API parity.

## 5) Performance: small, measured follow-on only (P1/P2)

Most P1 perf work is marked shipped. Remaining guidance is to:

- keep extending borrow/reference paths only where profiling proves value
- avoid large ingress/Value-model rewrites until `perf:context` + Criterion show ingress dominates
- treat P3 perf ideas (zero-copy context, broader architecture changes) as research only

## Explicitly deferred / separate tracks (P3)

Do **not** prioritize these unless product direction changes:

- callback-only `render(name, ctx, cb)` as the core async model
- true parallel `asyncAll`
- precompile/precompiled loader
- browser/WASM distribution work

## Practical “next PR” candidate

A high-confidence next PR is:

- Add 3–5 upstream-derived SafeString/copySafeness fixtures
- Fix the smallest set of filter/runtime behavior needed to pass
- Update allowlist + parity notes

This matches the parity backlog’s own suggested next PR checklist and keeps scope tight.
