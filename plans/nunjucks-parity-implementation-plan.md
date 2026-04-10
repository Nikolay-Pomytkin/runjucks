# Nunjucks parity implementation plan (next 3 PRs)

This plan turns the current parity backlog into concrete, shippable work items with clear acceptance criteria.

## Goals

- Raise migration confidence for `nunjucks@3.2.4` users by closing high-impact behavioral gaps.
- Keep parity measurable via conformance IDs + allowlist + parity summary output.
- Avoid mixing in P3 product tracks (browser/precompile/callback-only async core).

## Scope boundaries

### In scope (P1/P2)

1. Safe-string / copySafeness edge-case hardening.
2. `include` / `extends` edge-behavior parity for stock-valid syntax.
3. Expression/runtime corner-case parity for coercion and undefined behavior.
4. Loader/Express migration polish driven by real app scenarios.

### Out of scope for this plan (P3)

- `precompile` / `precompileString` / `PrecompiledLoader`
- Browser/WASM runtime support
- Callback-only async core API and parallel `asyncAll`

---

## PR-1: copySafeness completion sprint

### Objective

Close remaining copySafeness mismatches in common filter chains and template composition paths.

### Files to touch

- `native/fixtures/conformance/filter_cases.json`
- `__test__/upstream/filters-ported.test.mjs`
- `native/crates/runjucks-core/src/filters.rs` (only if new fixtures expose mismatches)
- `native/crates/runjucks-core/tests/filters.rs` (targeted Rust regression tests as needed)
- `perf/conformance-allowlist.json`
- `NUNJUCKS_PARITY.md` (update status notes)

### Tasks

1. Add 6–10 upstream-derived vectors covering:
   - `safe|escape|e|forceescape` with `set` + output,
   - macro return + `safe`/`escape` interactions,
   - `caller()` / `super()` output through filter chains,
   - regex `replace` safeness interactions.
2. Run JS parity + Rust conformance tests.
3. Fix engine behavior only where fixtures fail.
4. Add all green non-skipped fixture IDs to allowlist.
5. Update parity markdown with “moved to green” notes.

### Done criteria

- New vectors are green in both Node parity and Rust conformance.
- No new allowlist drift.
- CopySafeness “known mismatch” list is reduced (or eliminated for this wave).

---

## PR-2: include/extends parity hardening

### Objective

Stabilize inheritance/include behavior for stock-valid Nunjucks scenarios.

### Files to touch

- `native/fixtures/conformance/tag_parity_cases.json`
- `native/crates/runjucks-core/tests/tag_parity.rs`
- `native/crates/runjucks-core/src/renderer.rs` (if fixes needed)
- `__test__/parity.test.mjs` (only if harness handling is required)
- `perf/conformance-allowlist.json`
- `NUNJUCKS_PARITY.md`

### Tasks

1. Add 5–8 fixtures for:
   - dynamic parent resolution behavior,
   - include with nested templates and context propagation,
   - cycle/error surface behavior that matches upstream outputs/errors where comparable.
2. Mark true divergences with `compareWithNunjucks: false` + `divergenceNote`.
3. Fix renderer behavior for any stock-valid mismatch.
4. Update allowlist and parity notes.

### Done criteria

- New stock-valid fixtures compare green against Nunjucks.
- Any intentional divergence is explicitly documented and justified.

---

## PR-3: expression/runtime + Express migration polish

### Objective

Address practical migration blockers in runtime semantics and Express integration.

### Files to touch

- `native/fixtures/conformance/render_cases.json`
- `__test__/express.test.mjs`
- `native/crates/runjucks-core/src/renderer.rs` (if expression fixes needed)
- `index.js` / `express.js` (if API-level adjustments are needed)
- `perf/conformance-allowlist.json`
- `NUNJUCKS_PARITY.md`

### Tasks

1. Add 5–8 expression cases focused on:
   - mixed numeric/string relational coercion,
   - undefined propagation in nested lookups/calls,
   - `is`-test edge behavior.
2. Add 3–5 Express migration scenarios:
   - multi-root + relative include behavior,
   - cache invalidation expectations,
   - error middleware propagation.
3. Fix behavior where mismatches are confirmed.
4. Update allowlist and parity docs.

### Done criteria

- Expression and Express additions are green and stable.
- No regressions in existing parity suite.

---

## Standard validation checklist (run every PR)

1. `npm run build`
2. `npm test`
3. `npm run test:rust`
4. `npm run check:conformance-allowlist`

If a PR is fixture-only and build-heavy checks are intentionally skipped, explicitly document why in the PR description.

## Tracking template (copy into each PR body)

- **Track:** (PR-1 copySafeness / PR-2 include+extends / PR-3 expression+express)
- **New fixture IDs:**
- **Behavior fixes made:**
- **Intentional divergences added:**
- **Parity summary before/after:**
- **Commands run + result:**
