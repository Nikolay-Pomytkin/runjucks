# FFI overhead profile and follow-on ideas (2026-04-11)

## Scope

This note tracks a focused pass on Node↔Rust boundary overhead affecting `npm run perf`, plus the immediate low-risk optimization shipped in this change set. For the full perf backlog, tiered priorities (P0–P3), and harness commands, see [`RUNJUCKS_PERF.md`](RUNJUCKS_PERF.md).

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

## Subsequent landings on `main` (same theme)

After this note, related commits tightened the same hot path further (see `git log`):

- **Stable named-template cache keys:** skip redundant reloads when the loader reports a stable borrowed key (`cache_key_cow`).
- **Named-template allocations:** fewer cache-key string allocations on the NAPI / loader boundary.
- **Extension context cache:** merged `flatten()` snapshot is reused only when both **stack identity** and **`CtxStack::revision`** match, so different call shapes cannot share a stale context.

For maintainer-facing narrative and updated goals, see [`plans/PERF_PARITY_ACTION_PLAN_2026-04-11.md`](../plans/PERF_PARITY_ACTION_PLAN_2026-04-11.md) and the **2026-04** bullet in [`RUNJUCKS_PERF.md`](RUNJUCKS_PERF.md).

---

## Research synthesis: napi-rs, N-API, and ecosystem patterns

This section ties **public napi-rs documentation**, common **Node native-addon** advice, and **this repo’s actual binding shape** (`native/crates/runjucks-napi/src/lib.rs`) so follow-on work stays evidence-based.

### How Runjucks crosses the boundary today (relevant to cost)

| Mechanism | Role | FFI / conversion cost |
|-----------|------|------------------------|
| **`context: serde_json::Value`** on `renderString` / `renderTemplate` / `Template#render` | napi-rs converts the JS object graph into `serde_json::Value` **before** Rust render runs | **High**: full traversal + allocation; scales with object size (see [`perf/context-boundary.mjs`](../perf/context-boundary.mjs) comment: dominated by `serde_json::Value::from_napi_value`). |
| **`renderStringFromJson` / env variant** | JS passes **JSON text**; Rust parses with **`simd-json`** by default (`fast-json` in default features; `--no-default-features` → `serde_json`) | **Medium**: one UTF-8 parse in Rust; avoids per-property N-API `get` dispatch. Still copies string bytes. |
| **`renderStringFromJsonBuffer` / env variant** | JS passes **`Buffer` / `Uint8Array`** (UTF-8 JSON); same Rust parse as string path without an extra Rust `String` wrapper for the payload | **Same order as JSON string**; use when the context is already bytes (e.g. from a socket or `fs.readFile`). |
| **`JsFnRef::call`** for filters, globals, tests, extensions, `setLoaderCallback` | Each invocation: `serde_json::Value` → `to_napi_value` per arg, `napi_call_function`, then `from_napi_value` on return | **Per callback**: multiple conversions + JS call; hot templates with many custom filters amplify this. |
| **`Arc<Mutex<Environment>>`** | Serializes access to the Rust `Environment` from NAPI | **Lock contention**: unlikely on typical single-threaded Node sync render, but every public method pays lock acquire (see [`RUNJUCKS_PERF.md`](RUNJUCKS_PERF.md) P2 note on theoretical `RwLock`). |
| **String template name** on `renderTemplate` / `render` | UTF-16/UTF-8 string into Rust on every call | **Small but non-zero**; named-template microbenches (`synth_named_template_interp`) still lose partly for this + loader path (addressed in part by `cache_key_cow`). |

### What napi-rs documents explicitly

The [Values](https://napi.rs/docs/concepts/values) guide states:

- **Object**: converting or probing anonymous objects is **more expensive than primitives**; each `Object.get` is a dispatch to Node plus JS→Rust conversion.
- **Array**: same underlying costs as objects; **Array ↔ `Vec` is O(n)** and “even heavier.”
- **`#[napi(object)]`**: values are **cloned** from JS; not a zero-copy view into the heap.
- **TypedArray**: passed as a **reference**; mutations can reflect without cloning the elements.
- **Buffer**: typical pattern for **byte payloads** without building a full JS object tree on the Rust side for input.

Implication for Runjucks: taking **`serde_json::Value` from a JS `object`** is the **ergonomic default** but is inherently the **heavy path** for large contexts. The docs support the existing direction: **JSON string** or **buffer of UTF-8 JSON** as a faster ingress when profiling demands it.

### Broader ecosystem themes (blogs, issues, tooling)

Common advice across **napi-rs** posts, **N-API** articles, and Rust/JS bindings (e.g. [HackerNoon N-API image diff](https://hackernoon.com/beating-javascript-performance-limits-with-rust-and-n-api-building-a-faster-image-diff-tool), general Medium/Dev.to napi-rs summaries):

1. **Minimize crossings** — Do one native call per unit of work; avoid “walk AST in JS and call Rust per node” patterns. Runjucks already does **whole-template render** in Rust (good).
2. **Prefer bulk / binary interfaces for large data** — Pass **bytes** (`Buffer`, `Uint8Array`) or compact encodings when the payload is big; deserialize once in Rust. Aligns with [napi-rs Buffer docs](https://napi.rs/docs/concepts/values#buffer).
3. **Keep hot data in Rust** — Expose handles (opaque IDs or wrapper types) so repeated operations do not re-send full state. Matches the **“interned template handle”** idea in [Follow-on FFI improvements](#follow-on-ffi-improvements-worth-testing-next) above.
4. **Async / threads** — `ThreadsafeFunction` and async napi patterns help **event-loop** behavior; they do not remove **sync** conversion cost on the main thread. Runjucks async path still uses **`serde_json::Value`** context today.

The napi-rs issue **[#1502 — fast Rust → JS for complex structs](https://github.com/napi-rs/napi-rs/issues/1502)** is representative: maintainers and users discuss **layout/serialization** trade-offs for large structured results. Runjucks’ hot path is mostly **string out**, so the **inbound context** and **filter round-trips** matter more than return struct shape.

High-throughput tools (**SWC**, **Rspack**, **Oxc**, etc.) are often cited as **napi-rs** success stories: they batch work, minimize per-operation JS object churn, and use **release binaries** per platform (same model as `@zneep/runjucks-*` optional deps). See [Announcing NAPI-RS v3](https://napi.rs/blog/announce-v3) for the project’s own positioning around **binding performance and DX**.

### Prioritized ideas mapped to Runjucks (cost vs effort)

| Idea | Rationale | Effort / risk |
|------|-----------|----------------|
| **Promote `renderStringFromJson` / `renderStringFromJsonBuffer`** where apps control serialization | Avoids property-by-property N-API object walk; **`simd-json`** parse is **default** in `runjucks-napi` | Document in app integration guides; optional CI slice with `--no-default-features` if needed. |
| **Accept `Buffer` / `Uint8Array` for JSON context** (new overload or method) | Same as string path but avoids extra JS string copy in some pipelines; parse `&[u8]` in Rust | Medium: API + types + tests; same semantics as JSON string. |
| **Template handle API** (register name → `u32` / opaque id; render by id) | Cuts repeated **template name** string marshaling and map lookups on **tight loops** | Medium–high: new public API, must preserve loader/map semantics. |
| **Reduce filter/global FFI churn** | Today each `JsFnRef::call` allocates `Vec<napi_value>` and converts args with **`to_napi_value`**. Batching is **hard** without template compile-time knowledge; micro-opts (reuse buffers, small-string paths) are **low yield** unless profiling shows dominance | Low–medium per experiment. |
| **`RwLock` for read-mostly `Environment`** | Theoretical gain if many concurrent readers; Node **sync** render is usually **single-threaded** | Low code change; **validate** with benchmarks — may be noise. |
| **Lazier context** (proxy / arena / “read only keys template touches”) | Would require **engine** or **template analysis** changes; aligns with P3 “zero-copy context” in [`RUNJUCKS_PERF.md`](RUNJUCKS_PERF.md) | High; research-only until ingress dominates Criterion + `perf:context`. |

### Measurement guardrails (from this repo)

- **`npm run perf:context`** — isolates **large vs small** context delta on the **real** `from_napi_value` path ([`perf/context-boundary.mjs`](../perf/context-boundary.mjs)).
- **`npm run perf` / `perf:json`** — end-to-end vs **nunjucks**; use to validate that FFI tweaks do not regress parity-backed rows.
- **Flamegraphs / samply on Linux** — see [`RUNJUCKS_PERF.md`](RUNJUCKS_PERF.md) playbook; attribute time to **`from_napi_value`**, **`to_napi_value`**, **`napi_call_function`**, vs Rust core.

### Reference links (external)

- [napi.rs — Values (Object, Array, Buffer, TypedArray)](https://napi.rs/docs/concepts/values)
- [napi.rs — Object concept](https://napi.rs/docs/concepts/object)
- [napi-rs GitHub — issue #1502 (complex struct transfer)](https://github.com/napi-rs/napi-rs/issues/1502)
- [napi.rs — Announcing v3](https://napi.rs/blog/announce-v3)
- [Node.js N-API documentation](https://nodejs.org/api/n-api.html) (underlying ABI; napi-rs wraps it)
