# Perf + Nunjucks parity action plan (2026-04-11)

**Revision:** Updated to match current fixture IDs, perf harness behavior, and post–April 2026 landings on `main` (named-template cache path, extension context cache). The numeric perf snapshot below is still a **point-in-time** maintainer paste — re-run `npm run build && npm run perf` for current ratios.

## Context snapshot

Inputs used:

- `plans/NEXT_WORK.md`
- `ai_docs/NUNJUCKS_PARITY.md`
- `ai_docs/RUNJUCKS_PERF.md`
- `ai_docs/P3_ROADMAP.md`
- Local `npm run perf` output (historical paste)

**Historical** macro signal from that perf run:

- Average non-skipped speed ratio was **~4.4×** in favor of Runjucks.
- One recurring underperformer where Runjucks loses: **`synth_named_template_interp`** (order **~0.3×** `nj/rj` in published snapshots such as `docs/src/data/perf/reports/0.1.9.json` — meaning Nunjucks faster on that microbench).

**Conformance rows vs perf skips (clarified):**

| Fixture id | Role in perf harness |
|------------|----------------------|
| `tests_js_is_gt_mixed_string_numeric` | Parity case; on [`perf/conformance-allowlist.json`](../perf/conformance-allowlist.json). **Not** a “pending parity mismatch” skip by id. |
| `tag_include_nested_context_override` | Valid **nunjucks 3.2.4** include/stacking behavior; allowlisted; should **not** be confused with a divergent-syntax golden. |
| `tag_include_without_context_divergent` | **`compareWithNunjucks: false`** — Runjucks-only golden (stock nunjucks does not parse `without context` on `{% include %}`). Perf skips with **“no nunjucks baseline”**, not because runjucks and nunjucks disagree on output. |

So the old framing “three conformance-backed rows skipped due to parity mismatches” was **wrong**: one row is intentional divergence, and the other ids were misnamed.

**Already landed (named-template / FFI path):** borrowed loader cache keys (`cache_key_cow`), stable-key reload skips, extension merged-context cache keyed by **stack identity** + revision — see [`ai_docs/FFI_OVERHEAD_PROFILE_2026-04-11.md`](../ai_docs/FFI_OVERHEAD_PROFILE_2026-04-11.md) and [`ai_docs/RUNJUCKS_PERF.md`](../ai_docs/RUNJUCKS_PERF.md) changelog. Re-measure `synth_named_template_interp` after these; it may still be &lt; 1×.

---

## Goals (next 2–3 PRs)

1. **Perf:** drive **`synth_named_template_interp`** toward **≥ 1.0×** (`nj/rj`) if profiling still shows headroom (NAPI, loader, cache path), without correctness regressions.
2. **Parity / quality:** continue **SafeString / copySafeness** in small batches (upstream-derived fixtures + allowlist hygiene).
3. **Safety:** keep current fast-path wins (string filters, loops, attr chains, extension cache) guarded by existing tests + Criterion where applicable.

---

## Priority queue

### P0: Measurement hygiene

- When quoting perf skips, distinguish **`compareWithNunjucks: false`** (expected perf skip) from **`parity mismatch`** (unexpected — fix or narrow fixture).
- Use canonical fixture **`id`** values from [`native/fixtures/conformance/`](../native/fixtures/conformance/) (harness row names prefix them with `conf:`).

### P1: Named-template path (`synth_named_template_interp`)

Hypothesis:

- Render-by-name still pays vs inline `renderString` (boundary, loader bookkeeping, cache lookup), even after `cache_key_cow` / reload optimizations.

Plan:

1. Reproduce in isolation (Node harness + optional Criterion case mirroring the synthetic).
2. Profile `renderString` vs `renderTemplate` for the same body and template map.
3. Apply small, reversible optimizations; avoid API churn unless a handle-based API is clearly justified by profiles.
4. Guard with regression tests / bench notes.

Success target:

- **`synth_named_template_interp` ≥ 1.0×** with no parity regressions (re-verify with `npm run perf`).

### P1: SafeString / copySafeness (small batch)

Per [`ai_docs/NUNJUCKS_PARITY.md`](../ai_docs/NUNJUCKS_PARITY.md):3–5 upstream-derived vectors per PR around `safe`, `escape`/`e`, `forceescape`, `replace`, macro / `caller()` / `super()` chains.

### P2: Measurement hygiene

1. Report arithmetic **and** geometric mean ratio (optional; reduces outlier dominance).
2. Emit explicit counts: total rows, skipped by reason, parity-compared rows.
3. Optional CI guard when a previously compared row starts skipping for parity mismatch.

---

## Suggested execution sequence

### PR A — Named-template perf (if still &lt; 1× after re-measure)

- Profile → minimal hot-path change → `npm run perf` + parity/conformance.

### PR B — SafeString edge batch

- New JSON / upstream tests → allowlist updates → filter/renderer fixes.

### PR C — Harness reporting (optional)

- Richer summary output from `perf/run.mjs` (counts, geo mean).

---

## Definition of done (for this wave)

- `synth_named_template_interp` re-benchmarked; either **≥ 1.0×** or documented remaining bottleneck with profile citation.
- No new unexplained **parity mismatch** skips for allowlisted `compareWithNunjucks !== false` rows.
- SafeString batch merged with allowlist + tests green (`npm test`, `npm run test:rust`, `npm run check:conformance-allowlist` as applicable).

---

## Risks / watchouts

- Include / `without context` cases: treat **divergent-syntax** goldens as documentation + perf exclusions, not as Nunjucks parity bugs.
- Overfitting one synthetic perf row; confirm with a second case or Criterion microbench.
- SafeString changes can ripple through macros/extensions; keep tests tight around the diff.
