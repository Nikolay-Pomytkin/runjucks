# FFI overhead profile and follow-on ideas (2026-04-11)

## Scope

This note tracks a focused pass on Node↔Rust boundary overhead affecting `npm run perf`, plus the immediate low-risk optimization shipped in this change set.

Inputs reviewed:

- `plans/PERF_PARITY_ACTION_PLAN_2026-04-11.md`
- `plans/NEXT_WORK.md`
- `ai_docs/RUNJUCKS_PERF.md`

## Why this was the next perf target

From the current perf plan, the clearest remaining loser is `synth_named_template_interp`, which exercises `renderTemplate(name, ctx)` and therefore includes named-loader/cache-key work on the hot path.

The render core is already fast for most rows; this points to template-name path overhead (cache lookup key construction, loader path bookkeeping, and boundary cost) as the most actionable next target.

## Profiling snapshot (local machine)

Command:

- `npm run perf:context -- --json`

Observed output:

- `small`: `0.0898 ms`
- `large`: `0.1598 ms`
- delta (`large - small`): `0.0700 ms`

Interpretation:

- FFI+context materialization overhead is measurable and non-trivial for tiny templates.
- For named-template micro-cases, shaving small per-call allocations can move ratios meaningfully.

## Optimization implemented in this pass

### Borrowed cache key path for named templates

Problem before:

- `TemplateLoader::cache_key(&str) -> Option<String>` forced a fresh `String` allocation per render for stable name-based loaders (e.g. map loader / `HashMap` loader), even when the parse cache hit.

Change:

- Added `TemplateLoader::cache_key_cow(&str) -> Option<Cow<'_, str>>` with default behavior delegating to `cache_key`.
- Updated named-template cache lookups in `Environment` to use borrowed keys on lookup.
- Implemented `cache_key_cow` in the built-in `HashMap` loader as `Cow::Borrowed(name)`.

Expected effect:

- Removes a per-render allocation on named-template cache-hit paths.
- Most relevant to `renderTemplate` rows (including `synth_named_template_interp`) and include/extends chains that repeatedly resolve stable names.

## Follow-on FFI improvements worth testing next

1. **Add a dedicated FFI microbench script** for fixed template + tiny context:
   - compare `renderString` vs `renderTemplate`
   - report ops/s and allocation-sensitive deltas across 1M iterations.
2. **Optional interned-template-name handle API** in NAPI for repeated named renders:
   - compile/register once, render by integer handle.
   - avoids repeated JS string marshaling for hot loops.
3. **Fast ingress path expansion**:
   - extend `renderStringFromJson`-style fast-json option to more call sites only if `perf:context` + flamegraphs show ingress dominating.

## Guardrails

- Keep parity and public API behavior unchanged.
- Prefer reversible micro-optimizations first; defer broader API shape changes (handles/ingress redesign) until measured need is clear.
