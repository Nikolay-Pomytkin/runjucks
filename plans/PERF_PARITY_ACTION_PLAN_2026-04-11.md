# Perf + Nunjucks parity action plan (2026-04-11)

## Context snapshot

Inputs used:

- `plans/NEXT_WORK.md`
- `ai_docs/NUNJUCKS_PARITY.md`
- `ai_docs/RUNJUCKS_PERF.md`
- `ai_docs/P3_ROADMAP.md`
- latest local `npm run perf` output pasted by maintainer

Current macro signal from the perf run:

- Average non-skipped speed ratio is **4.39x** in favor of Runjucks.
- There is a single clear underperformer where Runjucks loses: **`synth_named_template_interp` (0.30x)**.
- Three conformance-backed perf rows are skipped due to parity mismatches:
  - `conf:tests_js_is_gt_mixed_string_num`
  - `conf:tag_include_without_context_div`
  - `conf:tag_include_nested_context_over`

This suggests biggest near-term ROI is **parity closure first** and **targeted template-path perf fix** for named-template rendering.

---

## Goals (next 2–3 PRs)

1. **Parity:** remove at least 2 of 3 perf skips by fixing behavior and promoting rows into allowlist parity.
2. **Perf:** lift `synth_named_template_interp` from 0.30x toward >=1.0x without regressing correctness.
3. **Safety:** keep current fast-path wins intact (string filters, loops, attr chains) and guard with tests.

---

## Priority queue

## P0: Unskip parity rows currently hiding regressions

### 1) `tests_js_is_gt_mixed_string_num`

- Add/confirm focused conformance vectors for mixed numeric/string relational comparisons under `is gt/ge/lt/le`.
- Align coercion semantics to Nunjucks behavior in expression evaluator / is-test plumbing.
- Promote case into `perf/conformance-allowlist.json` when green in parity test.

Expected impact:

- Better migration fidelity for real templates using heterogeneous values.
- Removes one perf skip and improves trust in perf summary.

### 2) include context behavior: `without context` and nested override

Rows:

- `tag_include_without_context_div`
- `tag_include_nested_context_over`

Actions:

- Add explicit fixture pairs showing context visibility and override order for include chains.
- Match Nunjucks behavior where syntax is valid upstream; if intentionally divergent, mark with `compareWithNunjucks:false` + divergence note and exclude from speed-ratio interpretation.
- Re-enable whichever row can be made fully parity-compatible.

Expected impact:

- Closes high-friction migration area (`include`/composition).
- Reduces “silent unknowns” in benchmark set.

---

## P1: Fix the one obvious perf loser (`synth_named_template_interp`)

Hypothesis from docs + perf shape:

- Render-by-name path likely pays extra overhead vs inline renderString path (loader/cache lookup + additional JS/Rust boundary work).

Plan:

1. **Reproduce in isolation**
   - Add a dedicated microbench in `native/crates/runjucks-core/benches/render_hotspots.rs` and/or Node harness mode mirroring `synth_named_template_interp`.
2. **Profile both modes**
   - Compare `renderString` vs template-name rendering under same template body.
   - Validate whether overhead is in loader resolution, cache key construction, or template object lifecycle.
3. **Optimize hot path** (small, reversible)
   - Prefer already-cached parsed template retrieval for named renders.
   - Minimize repeated key normalization / string allocation in loader+cache path.
   - Avoid extra conversion hops in NAPI when template name is static.
4. **Guard**
   - Add benchmark expectation note + regression test where practical.

Success target:

- `synth_named_template_interp` >= 1.0x (Runjucks at least parity with Nunjucks) with no parity regressions.

---

## P1: Continue SafeString / copySafeness hardening (small batch)

Per backlog direction, run a focused 3–5-case batch on:

- `safe` + `escape/e` + `forceescape`
- `replace` interactions
- macro return / `caller()` / `super()` chains

Why now:

- This area has repeated subtle mismatches and high user-visible escaping correctness risk.
- Good candidate for small, deterministic PRs that reduce future regressions.

---

## P2: Measurement hygiene improvements

1. Report both:
   - arithmetic mean ratio (current), and
   - geometric mean ratio (less dominated by extreme outliers).
2. Emit explicit counts in perf summary:
   - total rows, skipped rows by reason, parity-compared rows.
3. Fail CI (optional) when a previously unskipped parity row becomes skipped.

This improves confidence when average ratio changes.

---

## Suggested execution sequence

### PR 1 — Parity unskip batch

- Fix `tests_js_is_gt_mixed_string_num`.
- Fix one include-context case (prefer upstream-compatible first).
- Update allowlist + docs notes.
- Run: build, parity test, conformance, perf.

### PR 2 — Named-template perf track

- Add reproducer bench.
- Implement minimal cache/lookup/path optimization.
- Validate no parity drift; compare before/after perf JSON.

### PR 3 — SafeString edge batch

- Add 3–5 upstream-derived fixtures.
- Minimal runtime/filter changes to pass.
- Keep perf neutral/positive.

---

## Definition of done (for this wave)

- Parity skips reduced from 3 to <=1.
- `synth_named_template_interp` no longer <1x.
- No regressions in:
  - `__test__/parity.test.mjs`
  - Rust conformance/tag parity tests
  - existing perf synthetic winners.

---

## Risks / watchouts

- Include-context behavior may intentionally diverge in specific syntax; document explicitly to avoid churn.
- Overfitting one synthetic perf case can hurt real templates; require multi-case confirmation before landing.
- SafeString fixes can unintentionally change escaping in extension/macro paths; keep targeted tests close to changes.
