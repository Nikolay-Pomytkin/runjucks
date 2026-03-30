# Async Rendering Design

**Status:** Proposal (P3 — deferred until a concrete consumer exists)
**Audience:** Maintainers evaluating the engineering cost of async rendering support.
**See also:** [P3_ROADMAP.md](../../P3_ROADMAP.md), [NUNJUCKS_PARITY.md](../../NUNJUCKS_PARITY.md)

---

## 1. Motivation

Nunjucks supports async rendering: filters and globals that return Promises, template loaders that fetch from databases or APIs, and dedicated tags (`asyncEach`, `asyncAll`) for async iteration. Runjucks today is entirely synchronous — this blocks adoption in codebases that rely on async data fetching during template rendering.

**Concrete use cases:**

- **Async filters/globals** — a `translate` filter that calls an i18n service, a `fetchUser` global that queries a database.
- **Async template loaders** — templates stored in a CMS, S3, or database where the load call is inherently async.
- **Async iteration** — `{% asyncEach item in fetchItems() %}` where the iterable is a Promise; `{% asyncAll %}` for parallel rendering of independent items.

## 2. Scope

Full Nunjucks async parity:

| Feature | Nunjucks API | Target |
|---------|-------------|--------|
| Promise-based render | `env.render(name, ctx, callback)` | `env.renderStringAsync()` / `env.renderTemplateAsync()` returning `Promise<string>` |
| Async filters | `env.addFilter('name', asyncFn, true)` | `env.addAsyncFilter(name, fn)` |
| Async globals | `env.addGlobal('name', asyncFn)` | `env.addAsyncGlobal(name, fn)` |
| `{% asyncEach %}` | Sequential async iteration | Parse + async render |
| `{% asyncAll %}` | Parallel async iteration | Parse + async render |
| `{% ifAsync %}` | Async condition evaluation | Parse + async render |
| Async loaders | `getSource(name, callback)` | `env.setAsyncLoaderCallback(fn)` |

**Non-goals:** async `{% macro %}` definitions, WASM/browser async, async `precompile`.

## 3. Background: Current Architecture

### 3.1 Renderer pipeline

```
lex → parse → tree-walk render (all in Rust, synchronous)
```

The renderer ([`renderer.rs`](../../native/crates/runjucks-core/src/renderer.rs)) is a recursive tree-walk interpreter. Every function returns `Result<String>` or `Result<Value>` — concrete values, not futures.

Key entry point:

```rust
pub fn render(
    env: &Environment,
    loader: Option<&(dyn TemplateLoader + Send + Sync)>,
    root: &Node,
    ctx_stack: &mut CtxStack,
) -> Result<String>
```

**`CtxStack`** is a stack of `AHashMap<String, Arc<Value>>` frames, passed as `&mut` through the entire call chain. **`RenderState`** holds per-render mutable state (macro scopes, block chains, cyclers, etc.).

### 3.2 NAPI binding model

JS callbacks are invoked synchronously via `napi_call_function` through persistent `napi_ref` handles ([`lib.rs`](../../native/crates/runjucks-napi/src/lib.rs)). A thread-local `RENDER_NAPI_ENV` stores the active N-API environment handle during render, enforcing main-thread execution.

Custom filter/test/global type aliases are sync closures:

```rust
pub type CustomFilter = Arc<dyn Fn(&Value, &[Value]) -> Result<Value> + Send + Sync>;
pub type CustomTest = Arc<dyn Fn(&Value, &[Value]) -> Result<bool> + Send + Sync>;
pub type CustomGlobalFn = Arc<dyn Fn(&[Value], &[(String, Value)]) -> Result<Value> + Send + Sync>;
```

### 3.3 Existing async vocabulary

The lexer ([`tag_lex.rs`](../../native/crates/runjucks-core/src/tag_lex.rs)) already tokenizes `asyncEach`, `asyncAll`, `endeach`, `endall`, and `ifAsync` keywords. There is no parser, AST, or renderer support — using these tags produces a parse error today.

### 3.4 What makes async hard

| Constraint | Detail |
|-----------|--------|
| **Concrete return types** | All render/eval functions return `Result<String/Value>`, not `Future`s |
| **`&mut` across await** | `&mut CtxStack` and `&mut RenderState` cannot be held across `.await` boundaries |
| **Sync JS callbacks** | `napi_call_function` blocks until the JS function returns; no Promise awaiting |
| **Thread-local NAPI env** | Assumes single-threaded sync execution |
| **No async runtime** | No tokio dependency; napi v3 with `serde-json` only |
| **Sync loader trait** | `TemplateLoader::load()` returns `Result<String>`, not a future |

## 4. Approach Evaluation

### 4.1 Approach A: Dual-mode renderer

Keep the sync renderer (`renderer.rs`) **completely untouched**. Add a parallel async renderer (`async_renderer.rs`) with `async fn` variants of every render/eval function.

**Pros:** Zero regression risk for sync users. Sync path stays fast. Clear separation.
**Cons:** Code duplication between the two renderers (~2000 lines).

### 4.2 Approach B: Single async renderer with sync wrapper

Replace the renderer with an async-first implementation. Provide `render()` as `block_on(render_async())`.

**Pros:** Single implementation to maintain.
**Cons:** Contaminates every function signature. `block_on` inside NAPI risks deadlocking the Node event loop. Performance regression for the sync hot path (async overhead on every call, interior mutability everywhere).

### 4.3 Approach C: Cooperative / generator-style renderer

Use Rust coroutines (nightly) or a hand-rolled state machine to suspend/resume the renderer when an async operation is needed.

**Pros:** Elegant single implementation.
**Cons:** Rust coroutines are unstable (nightly-only). A hand-rolled state machine is effectively reimplementing async/await manually, with no ecosystem benefit.

### 4.4 Decision: Approach A (Dual-Mode)

Approach A is recommended. The duplication cost is manageable: shared pure-computation helpers (arithmetic, comparison, value coercion, built-in filter logic) can be factored into a `render_common.rs` module. The divergence is in orchestration — how filters/globals are called, how iteration proceeds, and how templates are loaded.

## 5. Phased Rollout

### Phase 1: Promise-based API + async filters/globals

**Goal:** `env.renderStringAsync(template, context)` returns a JS `Promise<string>`. Async JS functions work as filters and globals.

**Rust core changes:**

1. New file `async_renderer.rs` — async tree-walk interpreter, structured parallel to `renderer.rs`:
   - `async fn render_node_async(...)` → `Result<String>`
   - `async fn eval_to_value_async(...)` → `Result<Value>`
   - Uses `Rc<RefCell<CtxStack>>` instead of `&mut CtxStack` (the async renderer is `!Send` by design — it runs on the Node main thread)
   - `RenderState` gets the same `Rc<RefCell<...>>` treatment

2. New type aliases in [`environment.rs`](../../native/crates/runjucks-core/src/environment.rs):
   ```rust
   pub type AsyncCustomFilter = Arc<
       dyn Fn(&Value, &[Value]) -> Pin<Box<dyn Future<Output = Result<Value>>>>
       + Send + Sync
   >;

   pub type AsyncCustomGlobalFn = Arc<
       dyn Fn(&[Value], &[(String, Value)]) -> Pin<Box<dyn Future<Output = Result<Value>>>>
       + Send + Sync
   >;
   ```

3. New `Environment` fields and methods:
   - `async_custom_filters: HashMap<String, AsyncCustomFilter>`
   - `async_custom_globals: HashMap<String, AsyncCustomGlobalFn>`
   - `render_string_async()`, `render_template_async()`
   - `add_async_filter()`, `add_async_global_callable()`

4. Feature-gated: `#[cfg(feature = "async")]` on all async code. The core crate stays runtime-agnostic (no tokio dependency) — uses only `std::future` / `Pin<Box<dyn Future>>`.

**NAPI changes:**

1. Enable `napi/tokio_rt` feature in [`Cargo.toml`](../../native/crates/runjucks-napi/Cargo.toml) — provides tokio runtime for executing Rust futures.
2. Enable `runjucks_core/async` feature.
3. New `JsFnRef::call_async()` using `ThreadsafeFunction` (TSFN):
   - TSFN schedules the JS call on the Node event loop from within an async context.
   - If the JS function returns a Promise, the TSFN tracks settlement.
   - A `tokio::sync::oneshot` channel bridges the result back to the awaiting Rust future.
4. New NAPI exports on `JsEnvironment`:
   - `renderStringAsync(template, context) → Promise<string>`
   - `renderTemplateAsync(name, context) → Promise<string>`
   - `addAsyncFilter(name, func)`
   - `addAsyncGlobal(name, func)`
5. The async path does **not** use `RENDER_NAPI_ENV` thread-local. TSFN handles thread safety.
6. Sync filters called from async mode are wrapped transparently: `async { sync_filter(input, args) }`.

**Node.js API additions (`index.d.ts`):**

```typescript
export class Environment {
  renderStringAsync(template: string, context: object): Promise<string>;
  renderTemplateAsync(name: string, context: object): Promise<string>;
  addAsyncFilter(name: string, func: (...args: any[]) => Promise<any>): void;
  addAsyncGlobal(name: string, func: (...args: any[]) => Promise<any>): void;
}
```

### Phase 2: Async template tags

**Goal:** Parse and render `{% asyncEach %}`, `{% asyncAll %}`, and `{% ifAsync %}`.

**AST additions** in [`ast.rs`](../../native/crates/runjucks-core/src/ast.rs):

```rust
AsyncEach {
    vars: ForVars,
    iter: Expr,
    body: Vec<Node>,
    else_body: Option<Vec<Node>>,
},
AsyncAll {
    vars: ForVars,
    iter: Expr,
    body: Vec<Node>,
    else_body: Option<Vec<Node>>,
},
IfAsync {
    branches: Vec<IfBranch>,
},
```

**Parser changes** in `parser/template.rs`:
- Add parsing branches for `asyncEach`, `asyncAll`, `ifAsync` using the same expression grammar as `for` and `if`.

**Async renderer additions:**

| Function | Behavior |
|----------|----------|
| `render_async_each` | Like `render_for` but each iteration body is `.await`ed sequentially. Async filters/globals within the body can suspend. |
| `render_async_all` | Collects all iteration items, then renders all bodies concurrently via `futures::join_all`. Results are concatenated in iteration order. |
| `render_if_async` | Evaluates the condition via `eval_to_value_async` (can await async expressions), then renders the matching branch. |

**Sync renderer behavior:** `render_node` in `renderer.rs` returns a clear error if it encounters `AsyncEach`, `AsyncAll`, or `IfAsync`:
```
"{% asyncEach %} requires async render mode; use renderStringAsync()"
```

### Phase 3: Async loaders

**Goal:** Template loading can be asynchronous.

**New trait** in [`loader.rs`](../../native/crates/runjucks-core/src/loader.rs):

```rust
pub trait AsyncTemplateLoader: Send + Sync {
    fn load_async<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + 'a>>;

    fn cache_key(&self, name: &str) -> Option<String> {
        let _ = name;
        None
    }
}
```

**New `Environment` field:** `pub async_loader: Option<Arc<dyn AsyncTemplateLoader>>`

**Async renderer integration:** `render_extends_async`, `render_include_async`, and `render_import_async` use `async_loader.load_async()` when available, falling back to the sync loader.

**NAPI:** `setAsyncLoaderCallback(fn)` registers a JS function `(name: string) => Promise<string | null | {src: string}>` via TSFN, wrapped in an `AsyncTemplateLoader` implementation.

## 6. Key Design Decisions

### Feature gating

All async code in `runjucks-core` lives behind `#[cfg(feature = "async")]`. The default build (sync-only) is unchanged in size and compile time. AST node variants (`AsyncEach`, etc.) are unconditional since they are data-only and needed by the parser regardless.

### `Rc<RefCell<>>` vs `Arc<Mutex<>>` for async state

The async renderer is `!Send` — it runs on the Node main thread's tokio local-set runtime. This means `Rc<RefCell<>>` is safe and avoids the overhead of `Arc<Mutex<>>`. This is an intentional design choice: async rendering happens on a single thread (the Node.js event loop), never across threads.

### TSFN for JS callbacks

`ThreadsafeFunction` is the standard napi-rs mechanism for calling JS from async Rust. Each async filter/global gets a TSFN created at registration time. A `tokio::sync::oneshot` channel bridges the TSFN callback result back to the awaiting future.

### Shared helpers

Factor pure-computation code out of `renderer.rs` into `render_common.rs`:
- Arithmetic / comparison / coercion helpers
- Built-in filter implementations (upper, lower, join, replace, etc.)
- `loop` variable injection
- Block chain resolution

Both `renderer.rs` and `async_renderer.rs` import these. This minimizes duplication to ~orchestration code only.

## 7. Backward Compatibility

- The sync `render` / `renderString` / `renderTemplate` API is **completely unchanged**.
- Existing `addFilter` / `addGlobal` / `addTest` continue to work identically.
- Sync filters called from async mode are wrapped transparently — no re-registration needed.
- Async tags in templates rendered via the sync API produce a clear error message.
- The `async` feature flag is opt-in; the default build does not pull in tokio or increase binary size.

## 8. Testing Strategy

### Unit tests (Rust)

- `native/crates/runjucks-core/tests/async_renderer.rs` — async rendering with mock async filters/globals (using `async { ... }` closures).
- `native/crates/runjucks-core/tests/async_tags.rs` — `asyncEach`, `asyncAll`, `ifAsync` parsing and rendering.
- `native/crates/runjucks-core/tests/async_loader.rs` — async template loading in extends/include/import.
- Sync-mode error tests: verify that async tags produce clear errors when rendered synchronously.

### Integration tests (Node.js)

- `__test__/async-render.test.mjs` — end-to-end async rendering with real async JS filters/globals.
- `__test__/async-tags.test.mjs` — `asyncEach` / `asyncAll` with async data sources.
- Regression: all existing sync tests must continue passing with the `async` feature enabled.

### Performance

- `perf/` benchmarks should be extended with an async variant to track TSFN overhead.
- Sync path benchmarks must show zero regression when the `async` feature is compiled in but not used.

## 9. Risks and Open Questions

| Risk | Mitigation |
|------|-----------|
| **Code duplication** between sync and async renderers (~2000 lines) | Factor shared helpers into `render_common.rs`; keep divergence to orchestration only |
| **TSFN overhead** — each async filter call round-trips through the event loop | Async path is opt-in; sync users are unaffected. Document the performance characteristics. |
| **`asyncAll` nondeterminism** — parallel iterations that mutate the same variable via `{% set %}` | Matches Nunjucks behavior; document the race condition. Consider giving each iteration its own CtxStack frame. |
| **tokio dependency** in NAPI crate | Acceptable — NAPI crate is already Node-specific; napi-rs tokio integration is mature |
| **Promise rejection propagation** | TSFN callback must map JS rejection reasons to `RunjucksError`; needs careful error conversion |
| **Cancellation** | If the caller drops the Promise, in-flight TSFN calls should be cancellable. Investigate napi-rs TSFN abort semantics. |
| **Maintenance burden** of two renderers | Track with CI — any change to `renderer.rs` logic should prompt a check of `async_renderer.rs`. Consider a lint or codegen approach long-term. |

## Appendix A: File Change Summary

| File | Phase | Change |
|------|-------|--------|
| `native/crates/runjucks-core/Cargo.toml` | 1 | Add `async` feature flag |
| `native/crates/runjucks-core/src/lib.rs` | 1 | Export `async_renderer` and `render_common` modules |
| `native/crates/runjucks-core/src/render_common.rs` | 1 | **New** — shared pure helpers extracted from `renderer.rs` |
| `native/crates/runjucks-core/src/async_renderer.rs` | 1 | **New** — async tree-walk interpreter |
| `native/crates/runjucks-core/src/environment.rs` | 1 | Async type aliases, fields, methods |
| `native/crates/runjucks-core/src/ast.rs` | 2 | `AsyncEach`, `AsyncAll`, `IfAsync` node variants |
| `native/crates/runjucks-core/src/parser/template.rs` | 2 | Parse async tags |
| `native/crates/runjucks-core/src/renderer.rs` | 2 | Error arms for async nodes in sync mode |
| `native/crates/runjucks-core/src/loader.rs` | 3 | `AsyncTemplateLoader` trait |
| `native/crates/runjucks-napi/Cargo.toml` | 1 | Enable `napi/tokio_rt`, `runjucks_core/async` |
| `native/crates/runjucks-napi/src/lib.rs` | 1 | TSFN infrastructure, async render/filter/global exports |
| `index.d.ts` | 1 | Async method type signatures |
| `__test__/async-*.test.mjs` | 1-3 | **New** — async integration tests |
