# Rendering performance continuation plan (conditionals, macros, iterators, switch, filters)

Date: 2026-04-12

This plan consolidates current perf documentation and proposes a measurable implementation sequence focused on rendering hot paths for templates that use:

- conditional rendering (`if`, inline-if)
- macros / `call`
- list iterators (`for`, tuple unpacking, loop metadata)
- `switch`
- filters (single and chained)

## 1) What is already landed (from current perf docs)

Based on `ai_docs/RUNJUCKS_PERF.md`, `plans/PERF_PARITY_ACTION_PLAN_2026-04-11.md`, docs/perf guide, and `perf/README.md`:

- Parse/cache baseline is in place (inline + named parse caches, `Template` AST caching).
- Renderer already includes several hot-path optimizations:
  - loop object reuse in `for`
  - string reserve heuristics
  - variable/filter borrow fast paths
  - dotted attr-chain optimization
  - extension context flatten cache
- Harnesses are available and complementary:
  - Node end-to-end: `npm run perf`, `npm run perf:cold`, `npm run perf:json`
  - Runjucks-only context-boundary probe: `npm run perf:context`
  - Rust-only Criterion: `npm run bench:rust`, `npm run bench:rust:parse`
- Current published snapshot (`docs/src/data/perf/reports/0.1.9.json`) shows strong average speedup but uneven row-level performance remains a stated target in docs.

## 2) Gap statement for target constructs

While many rows for `if`, `for`, `switch`, `macro`, and filters are already >1x vs Nunjucks, there is still opportunity in **render-only CPU cost** and **allocation behavior** for complex templates with mixed constructs.

Main likely gaps to attack next:

1. Repeated expression-tree evaluation inside branches and loops.
2. Macro invocation overhead (argument binding + frame creation + repeated lookup).
3. Per-iteration loop metadata work when only a subset of `loop.*` is read.
4. Filter dispatch overhead for common builtins in chained pipelines.
5. `switch` branch selection and repeated equality/coercion work.

## 3) Measurable targets (explicit)

Track success with existing harnesses plus targeted cases to be added:

- Node harness (`perf/run.mjs`) improvements on focused synthetic rows:
  - `synth_if_nested`
  - `synth_deep_if_chain`
  - `synth_for_medium`
  - `synth_filters_chain`
  - new rows proposed below for macro/switch/iterator-heavy combinations
- Rust Criterion (`render_hotspots`) median/mean improvements for equivalent workloads.
- No parity regressions in allowlisted conformance rows.

Acceptance bar for each PR in this wave:

- At least one focused perf row improves by >= 8% (or a clear documented tradeoff if neutral).
- No statistically meaningful regression (>3%) on unrelated top synthetic rows.

## 4) Proposed implementation sequence

### PR-1: Add missing targeted perf coverage (measurement-first)

Purpose: ensure the harness directly stresses the requested constructs in combination.

Changes:

- Add synthetic Node perf cases in `perf/synthetic.mjs`:
  1. `synth_conditional_macro_iter_switch_filters`
  2. `synth_macro_call_with_filter_chain_in_loop`
  3. `synth_switch_in_for_with_attr_filters`
  4. `synth_inline_if_filter_chain_dense`
- Add Rust Criterion analogs in `native/crates/runjucks-core/benches/render_hotspots.rs`.
- Keep templates deterministic and parity-safe (no random/time).

Why first:

- Prevent optimization-by-guessing.
- Gives stable baselines for subsequent PRs.

### PR-2: Branch + switch evaluation fast path

Hypothesis:

- `if` / `switch` workloads repeatedly evaluate branch expressions with avoidable allocations/clones.

Refactor candidates:

- In renderer expression evaluation, add borrow-oriented helpers for branch predicates where only truthiness / comparison is required.
- For `switch`, pre-evaluate selector once and compare against case literals via lightweight borrowed path when possible.
- Avoid reconstructing small temporary values during repeated case checks.

Measure:

- New `synth_*if*` and `synth_*switch*` rows.
- Existing `synth_if_nested`, `synth_deep_if_chain`, `conf:tag_switch_*` rows.

### PR-3: Macro invocation + frame management optimization

Hypothesis:

- Macro-heavy templates spend significant time in frame setup, argument mapping/default resolution, and repeated symbol lookup.

Refactor candidates:

- Introduce a reusable macro call frame/binding structure for stable macro signatures.
- Cache macro argument name/index mapping at parse time or first render.
- Reduce temporary map/object creation for `caller` and macro-local scopes when values are unchanged.

Measure:

- `synth_macro_call_with_filter_chain_in_loop` (new)
- Existing macro rows (`conf:tag_macro_*`, `conf:tag_call_macro_caller`)

### PR-4: Loop + filter-chain specialization

Hypothesis:

- In iterator-heavy templates, combined `for` + filter chain costs are dominated by repeated generic filter dispatch and avoidable loop metadata writes.

Refactor candidates:

- Add additional filter-chain specialization for common builtins (`trim`, `upper`, `lower`, `length`, selected safe-path combos) when no custom filter overrides exist.
- For loops, lazily update heavy `loop.*` fields only if read in body (or use a cheap “needed-fields” bitmap computed at parse stage).
- Keep correctness for nested loops and shadowed `loop` intact.

Measure:

- `synth_filters_chain`, `synth_for_medium`, new combined synthetic rows.
- Criterion loop/filter benches.

## 5) Guardrails and correctness strategy

For each PR:

- Run parity + conformance checks first, then perf.
- Add/extend regression tests in `native/crates/runjucks-core/tests/perf_regressions.rs` for new fast paths.
- Keep optimizations gated by clear preconditions (no custom filter override, no side effects, literal/index constraints, etc.).
- Preserve semantics over speed if there is conflict.

## 6) Suggested runbook per PR

1. `npm run build`
2. `npm test`
3. `npm run test:rust`
4. `npm run perf:json`
5. `npm run bench:rust`
6. Compare deltas in:
   - `perf/last-run.json`
   - Criterion output (baseline vs candidate)

Optional deep profiling when a row stalls:

- `npm run perf:context` (separate boundary vs core concerns)
- `cargo flamegraph --bench render_hotspots -p runjucks_core`

## 7) Exit criteria for this optimization wave

- Added benchmark rows that directly represent requested construct combinations.
- At least two rendering-focused PRs merged with measurable improvements in those rows.
- No new unexplained perf skip reasons for allowlisted conformance IDs.
- Short write-up appended to `ai_docs/RUNJUCKS_PERF.md` changelog with before/after numbers.
