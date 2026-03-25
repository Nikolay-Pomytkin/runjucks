#![deny(clippy::all)]

use napi::bindgen_prelude::ToNapiValue;
use napi::bindgen_prelude::{FromNapiValue, JsValue, Unknown};
use napi::{check_pending_exception, check_status, sys, Env, Error, Result, Status, ValueType};
use napi_derive::napi;
use runjucks_core::ast::Node;
use runjucks_core::value::value_to_string;
use runjucks_core::{
    map_loader, CustomFilter, CustomGlobalFn, CustomTest, Environment, RunjucksError, Tags,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};

thread_local! {
    static RENDER_NAPI_ENV: Cell<Option<sys::napi_env>> = const { Cell::new(None) };
}

struct RenderEnvGuard;
impl Drop for RenderEnvGuard {
    fn drop(&mut self) {
        RENDER_NAPI_ENV.with(|c| c.set(None));
    }
}

fn with_render_napi_env<T>(napi_env: sys::napi_env, f: impl FnOnce() -> T) -> T {
    RENDER_NAPI_ENV.with(|c| c.set(Some(napi_env)));
    let _g = RenderEnvGuard;
    f()
}

/// Holds a persistent [`sys::napi_ref`] to a JS function for synchronous calls during render.
///
/// # Safety
///
/// N-API handles are only exercised from the Node main thread during synchronous render (same contract as Nunjucks sync templates).
struct JsFnRef {
    inner: sys::napi_ref,
    env: sys::napi_env,
}

unsafe impl Send for JsFnRef {}
unsafe impl Sync for JsFnRef {}

impl JsFnRef {
    fn new(env: &Env, func: &Unknown) -> Result<Self> {
        if func.get_type()? != ValueType::Function {
            return Err(Error::new(
                Status::InvalidArg,
                "addFilter expects a JavaScript function",
            ));
        }
        let mut reference = ptr::null_mut();
        check_status!(
            unsafe { sys::napi_create_reference(env.raw(), func.value().value, 1, &mut reference) },
            "create function reference"
        )?;
        Ok(Self {
            inner: reference,
            env: env.raw(),
        })
    }

    fn call(&self, args: &[serde_json::Value]) -> Result<serde_json::Value> {
        unsafe {
            let mut func = ptr::null_mut();
            check_status!(
                sys::napi_get_reference_value(self.env, self.inner, &mut func),
                "get function from reference"
            )?;
            let mut raw_this = ptr::null_mut();
            check_status!(
                sys::napi_get_undefined(self.env, &mut raw_this),
                "get undefined"
            )?;
            let mut raw_args: Vec<sys::napi_value> = Vec::with_capacity(args.len());
            for a in args {
                raw_args.push(serde_json::Value::to_napi_value(self.env, a.clone())?);
            }
            let mut ret = ptr::null_mut();
            check_pending_exception!(
                self.env,
                sys::napi_call_function(
                    self.env,
                    raw_this,
                    func,
                    raw_args.len(),
                    raw_args.as_ptr(),
                    &mut ret,
                )
            )?;
            serde_json::Value::from_napi_value(self.env, ret)
        }
    }
}

impl Drop for JsFnRef {
    fn drop(&mut self) {
        unsafe {
            let _ = sys::napi_delete_reference(self.env, self.inner);
        }
    }
}

fn json_value_is_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null | serde_json::Value::Bool(false) => false,
        serde_json::Value::Bool(true) => true,
        serde_json::Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(true),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => {
            if o.get("__runjucks_undefined") == Some(&serde_json::Value::Bool(true)) {
                return false;
            }
            true
        }
    }
}

fn napi_custom_test(js: Arc<JsFnRef>) -> CustomTest {
    Arc::new(move |value, args| {
        let active = RENDER_NAPI_ENV.with(|c| c.get()).ok_or_else(|| {
            RunjucksError::new("custom test invoked without an active Node N-API render context")
        })?;
        if active != js.env {
            return Err(RunjucksError::new(
                "N-API environment mismatch during custom test call",
            ));
        }
        let mut call_args: Vec<serde_json::Value> = Vec::with_capacity(1 + args.len());
        call_args.push(value.clone());
        call_args.extend_from_slice(args);
        let out = js
            .call(&call_args)
            .map_err(|e: Error| RunjucksError::new(e.to_string()))?;
        Ok(json_value_is_truthy(&out))
    })
}

fn napi_extension_process(js: Arc<JsFnRef>) -> runjucks_core::extension::CustomExtensionHandler {
    Arc::new(move |ctx, args, body| {
        let active = RENDER_NAPI_ENV.with(|c| c.get()).ok_or_else(|| {
            RunjucksError::new(
                "extension process invoked without an active Node N-API render context",
            )
        })?;
        if active != js.env {
            return Err(RunjucksError::new(
                "N-API environment mismatch during extension process call",
            ));
        }
        let call_args = vec![
            ctx.clone(),
            serde_json::Value::String(args.to_string()),
            match body {
                Some(s) => serde_json::Value::String(s),
                None => serde_json::Value::Null,
            },
        ];
        let out = js
            .call(&call_args)
            .map_err(|e: Error| RunjucksError::new(e.to_string()))?;
        Ok(value_to_string(&out))
    })
}

fn napi_custom_global(js: Arc<JsFnRef>) -> CustomGlobalFn {
    Arc::new(move |args, kwargs| {
        let active = RENDER_NAPI_ENV.with(|c| c.get()).ok_or_else(|| {
            RunjucksError::new("custom global invoked without an active Node N-API render context")
        })?;
        if active != js.env {
            return Err(RunjucksError::new(
                "N-API environment mismatch during custom global call",
            ));
        }
        let mut call_args: Vec<serde_json::Value> = args.to_vec();
        if !kwargs.is_empty() {
            let m: serde_json::Map<String, serde_json::Value> = kwargs.iter().cloned().collect();
            call_args.push(serde_json::Value::Object(m));
        }
        js.call(&call_args)
            .map_err(|e: Error| RunjucksError::new(e.to_string()))
    })
}

fn napi_custom_filter(js: Arc<JsFnRef>) -> CustomFilter {
    Arc::new(move |input, args| {
        let active = RENDER_NAPI_ENV.with(|c| c.get()).ok_or_else(|| {
            RunjucksError::new("custom filter invoked without an active Node N-API render context")
        })?;
        if active != js.env {
            return Err(RunjucksError::new(
                "N-API environment mismatch during custom filter call",
            ));
        }
        let mut call_args: Vec<serde_json::Value> = Vec::with_capacity(1 + args.len());
        call_args.push(input.clone());
        call_args.extend_from_slice(args);
        js.call(&call_args)
            .map_err(|e: Error| RunjucksError::new(e.to_string()))
    })
}

#[napi]
pub fn render_string(template: String, context: serde_json::Value) -> napi::Result<String> {
    render_with_env(&Environment::default(), template, context)
}

fn render_with_env(
    env: &Environment,
    template: String,
    context: serde_json::Value,
) -> napi::Result<String> {
    env.render_string(template, context)
        .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct TagsOptions {
    pub block_start: Option<String>,
    pub block_end: Option<String>,
    pub variable_start: Option<String>,
    pub variable_end: Option<String>,
    pub comment_start: Option<String>,
    pub comment_end: Option<String>,
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct ConfigureOptions {
    pub autoescape: Option<bool>,
    pub dev: Option<bool>,
    pub throw_on_undefined: Option<bool>,
    pub trim_blocks: Option<bool>,
    pub lstrip_blocks: Option<bool>,
    pub tags: Option<TagsOptions>,
}

fn validate_parse(env: &Environment, src: &str) -> std::result::Result<(), RunjucksError> {
    env.validate_lex_parse(src)
}

fn apply_configure_opts(env: &mut Environment, opts: &ConfigureOptions) {
    if let Some(a) = opts.autoescape {
        env.autoescape = a;
    }
    if let Some(d) = opts.dev {
        env.dev = d;
    }
    if let Some(t) = opts.throw_on_undefined {
        env.throw_on_undefined = t;
    }
    if let Some(t) = opts.trim_blocks {
        env.trim_blocks = t;
    }
    if let Some(l) = opts.lstrip_blocks {
        env.lstrip_blocks = l;
    }
    if let Some(t) = &opts.tags {
        let defaults = Tags::default();
        env.tags = Some(Tags {
            block_start: t.block_start.clone().unwrap_or(defaults.block_start),
            block_end: t.block_end.clone().unwrap_or(defaults.block_end),
            variable_start: t.variable_start.clone().unwrap_or(defaults.variable_start),
            variable_end: t.variable_end.clone().unwrap_or(defaults.variable_end),
            comment_start: t.comment_start.clone().unwrap_or(defaults.comment_start),
            comment_end: t.comment_end.clone().unwrap_or(defaults.comment_end),
        });
    }
}

/// Module-level default environment for Nunjucks-style [`configure`] / [`render`].
static GLOBAL_ENV: OnceLock<Mutex<Option<Arc<Mutex<Environment>>>>> = OnceLock::new();

fn global_env_mutex() -> &'static Mutex<Option<Arc<Mutex<Environment>>>> {
    GLOBAL_ENV.get_or_init(|| Mutex::new(None))
}

fn global_env() -> Arc<Mutex<Environment>> {
    let mut g = global_env_mutex().lock().unwrap();
    if g.is_none() {
        *g = Some(Arc::new(Mutex::new(Environment::default())));
    }
    g.as_ref().unwrap().clone()
}

fn set_global_env(env: Arc<Mutex<Environment>>) {
    *global_env_mutex().lock().unwrap() = Some(env);
}

/// Nunjucks-compatible compiled template: inline source or a named template from the environment loader.
#[napi(js_name = "Template")]
pub struct JsTemplate {
    env: Arc<Mutex<Environment>>,
    src: Option<String>,
    name: Option<String>,
    path: Option<String>,
    /// Parsed AST for inline `compile()` templates; filled on first render or by eager compile.
    cached_ast: Mutex<Option<Arc<Node>>>,
}

#[napi]
impl JsTemplate {
    /// `new Template(src, env?, path?, eagerCompile?)` — mirror [`compile`].
    #[napi(constructor)]
    pub fn new(
        src: String,
        env: Option<&JsEnvironment>,
        path: Option<String>,
        eager_compile: Option<bool>,
    ) -> Result<Self> {
        let _ = global_env();
        let env_arc = if let Some(e) = env {
            e.inner.clone()
        } else {
            Arc::new(Mutex::new(Environment::default()))
        };
        let cached_ast = Mutex::new(if eager_compile.unwrap_or(false) {
            let lock = env_arc
                .lock()
                .map_err(|e| Error::from_reason(e.to_string()))?;
            Some(
                lock.parse_or_cached_inline(&src)
                    .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))?,
            )
        } else {
            None
        });
        Ok(Self {
            env: env_arc,
            src: Some(src),
            name: None,
            path,
            cached_ast,
        })
    }

    #[napi]
    pub fn render(&self, env: Env, context: serde_json::Value) -> Result<String> {
        let inner = self
            .env
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if let Some(src) = &self.src {
            let ast = {
                let mut g = self
                    .cached_ast
                    .lock()
                    .map_err(|e| Error::from_reason(e.to_string()))?;
                if let Some(ref a) = *g {
                    Arc::clone(a)
                } else {
                    let a = inner
                        .parse_or_cached_inline(src)
                        .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))?;
                    *g = Some(Arc::clone(&a));
                    a
                }
            };
            return with_render_napi_env(env.raw(), || {
                inner
                    .render_parsed(ast.as_ref(), context)
                    .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
            });
        }
        if let Some(name) = &self.name {
            return with_render_napi_env(env.raw(), || {
                inner
                    .render_template(name, context)
                    .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
            });
        }
        Err(Error::from_reason("invalid Template (no source or name)"))
    }

    /// Optional path for this template (Nunjucks uses it for errors; inline [`compile`] / `new Template` only).
    #[napi(getter, js_name = "path")]
    pub fn get_path(&self) -> Option<String> {
        self.path.clone()
    }
}

#[napi(js_name = "Environment")]
pub struct JsEnvironment {
    inner: Arc<Mutex<Environment>>,
}

#[napi]
#[allow(clippy::new_without_default)]
impl JsEnvironment {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Environment::default())),
        }
    }

    /// Renders with this environment. `Env` is required so custom filters can call back into JavaScript synchronously.
    #[napi]
    pub fn render_string(
        &self,
        env: Env,
        template: String,
        context: serde_json::Value,
    ) -> Result<String> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        with_render_napi_env(env.raw(), || render_with_env(&inner, template, context))
    }

    #[napi]
    pub fn set_autoescape(&self, enabled: bool) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        env.autoescape = enabled;
        Ok(())
    }

    #[napi]
    pub fn set_dev(&self, enabled: bool) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        env.dev = enabled;
        Ok(())
    }

    /// Fixes the PRNG used by `| random` for reproducible tests (omit / pass `undefined` to use a fresh non-deterministic seed per render).
    #[napi(js_name = "setRandomSeed")]
    pub fn set_random_seed(&self, seed: Option<u32>) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        env.random_seed = seed.map(u64::from);
        Ok(())
    }

    /// Registers `(input, ...args) => any`. Overrides a built-in filter with the same name.
    #[napi]
    pub fn add_filter(&self, env: Env, name: String, func: Unknown) -> Result<()> {
        let js = Arc::new(JsFnRef::new(&env, &func)?);
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        inner.add_filter(name, napi_custom_filter(js));
        Ok(())
    }

    /// Registers `(value, ...args) => boolean` (truthy return) for `is` tests and for `select` / `reject`. Built-in tests (`odd`, `even`, …) take precedence over the same name.
    #[napi(js_name = "addTest")]
    pub fn add_test(&self, env: Env, name: String, func: Unknown) -> Result<()> {
        let js = Arc::new(JsFnRef::new(&env, &func)?);
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        inner.add_test(name, napi_custom_test(js));
        Ok(())
    }

    /// Custom tag extension (Nunjucks `addExtension`): `tags`, optional `blocks` map (opening tag → end tag name), and `process(context, args, body)` — `body` is `null` for simple tags.
    #[napi(js_name = "addExtension")]
    pub fn add_extension(
        &self,
        env: Env,
        extension_name: String,
        tags: Vec<String>,
        blocks: Option<HashMap<String, String>>,
        process: Unknown,
    ) -> Result<()> {
        if tags.is_empty() {
            return Err(Error::from_reason(
                "addExtension: `tags` must list at least one tag name",
            ));
        }
        let js = Arc::new(JsFnRef::new(&env, &process)?);
        let mut tag_specs: Vec<(String, Option<String>)> = Vec::new();
        for t in &tags {
            let end = blocks.as_ref().and_then(|b| b.get(t).cloned());
            tag_specs.push((t.clone(), end));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        inner
            .register_extension(extension_name, tag_specs, napi_extension_process(js))
            .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
    }

    /// Returns whether a custom extension with this name is registered (Nunjucks `hasExtension`).
    #[napi(js_name = "hasExtension")]
    pub fn has_extension(&self, name: String) -> Result<bool> {
        let env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(env.has_extension(&name))
    }

    /// Unregisters a custom extension by name (Nunjucks `removeExtension`). Returns `true` if it existed.
    #[napi(js_name = "removeExtension")]
    pub fn remove_extension(&self, name: String) -> Result<bool> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(env.remove_extension(&name))
    }

    /// Registers a global: JSON-serializable value, or a **JavaScript function** invoked for `{{ name(...) }}` (Nunjucks-style keyword args as a trailing object). See `NUNJUCKS_PARITY.md` (P1).
    #[napi(js_name = "addGlobal")]
    pub fn add_global(&self, env: Env, name: String, value: Unknown) -> Result<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if value.get_type()? == ValueType::Function {
            let js = Arc::new(JsFnRef::new(&env, &value)?);
            inner.add_global_callable(name, napi_custom_global(js));
        } else {
            let v = unsafe {
                serde_json::Value::from_napi_value(env.raw(), value.value().value)?
            };
            inner.add_global(name, v);
        }
        Ok(())
    }

    /// Subset of Nunjucks `configure`: `autoescape`, `dev`, `throwOnUndefined`, `trimBlocks`, `lstripBlocks`, and `tags` are applied.
    #[napi]
    pub fn configure(&self, opts: ConfigureOptions) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        apply_configure_opts(&mut env, &opts);
        Ok(())
    }

    /// Loads a named template from this environment’s loader (same idea as Nunjucks `getTemplate`).
    #[napi(js_name = "getTemplate")]
    pub fn get_template(&self, name: String, eager_compile: Option<bool>) -> Result<JsTemplate> {
        let _ = global_env();
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let loader = inner.loader.as_ref().ok_or_else(|| {
            Error::from_reason("no template loader configured (use setTemplateMap first)")
        })?;
        if eager_compile.unwrap_or(false) {
            let src = loader
                .load(&name)
                .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))?;
            validate_parse(&inner, &src).map_err(|e| Error::from_reason(e.to_string()))?;
        }
        drop(inner);
        Ok(JsTemplate {
            env: self.inner.clone(),
            src: None,
            name: Some(name),
            path: None,
            cached_ast: Mutex::new(None),
        })
    }

    /// Sets an in-memory template map (`name` → source). Enables `renderTemplate`, `{% include %}`, `{% extends %}`, etc.
    #[napi]
    pub fn set_template_map(&self, map: HashMap<String, String>) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        env.loader = Some(map_loader(map));
        env.clear_named_parse_cache();
        Ok(())
    }

    /// Renders a named template from the map set via [`set_template_map`].
    #[napi(js_name = "renderTemplate")]
    pub fn render_template(
        &self,
        env: Env,
        name: String,
        context: serde_json::Value,
    ) -> Result<String> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        with_render_napi_env(env.raw(), || {
            inner
                .render_template(&name, context)
                .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
        })
    }
}

/// Nunjucks-style module `configure(opts?)` — sets the default environment used by [`render`].
#[napi(js_name = "configure")]
pub fn configure_default(opts: Option<ConfigureOptions>) -> Result<JsEnvironment> {
    let env = Arc::new(Mutex::new(Environment::default()));
    if let Some(o) = opts {
        let mut inner = env.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        apply_configure_opts(&mut inner, &o);
    }
    set_global_env(env.clone());
    Ok(JsEnvironment { inner: env })
}

/// `compile(src, env?, path?, eagerCompile?)` — Nunjucks-compatible factory for [`Template`].
#[napi]
pub fn compile(
    src: String,
    env: Option<&JsEnvironment>,
    path: Option<String>,
    eager_compile: Option<bool>,
) -> Result<JsTemplate> {
    let _ = global_env();
    JsTemplate::new(src, env, path, eager_compile)
}

/// Top-level `render(name, ctx)` using the environment from [`configure_default`].
#[napi(js_name = "render")]
pub fn render_named_template(name: String, context: serde_json::Value, env: Env) -> Result<String> {
    let env_arc = global_env();
    let inner = env_arc
        .lock()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    with_render_napi_env(env.raw(), || {
        inner
            .render_template(&name, context)
            .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
    })
}

/// Clears the module-level default environment (for tests; matches Nunjucks `reset`).
#[napi]
pub fn reset() {
    *global_env_mutex().lock().unwrap() = None;
}
