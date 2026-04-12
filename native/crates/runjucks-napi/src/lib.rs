#![deny(clippy::all)]

use napi::bindgen_prelude::ToNapiValue;
use napi::bindgen_prelude::{FromNapiValue, JsValue, Uint8Array, Unknown};
use napi::{check_pending_exception, check_status, sys, Env, Error, Result, Status, ValueType};
use napi_derive::napi;
use runjucks_core::ast::Node;
use runjucks_core::value::value_to_string;
use runjucks_core::{
    file_system_loader, loader::TemplateLoader, map_loader, AsyncCustomFilter, AsyncCustomGlobalFn,
    CustomFilter, CustomGlobalFn, CustomTest, Environment, RunjucksError, Tags,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;
#[allow(unused_imports)]
use std::pin::Pin;
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
            // Detect Promise returns from async JS functions and reject with a
            // clear error — we cannot await Promises on the synchronous render thread.
            if Self::is_promise(self.env, ret) {
                return Err(Error::from_reason(
                    "async filter/global returned a Promise. Callbacks registered via \
                     addAsyncFilter/addAsyncGlobal must be synchronous functions, not \
                     async functions. The 'async' in the API name refers to the async \
                     rendering mode, not the callback itself.",
                ));
            }
            serde_json::Value::from_napi_value(self.env, ret)
        }
    }

    /// Check whether a napi_value is a Promise (object with a callable `then` property).
    unsafe fn is_promise(env: sys::napi_env, value: sys::napi_value) -> bool {
        let mut vtype = 0i32;
        if sys::napi_typeof(env, value, &mut vtype) != sys::Status::napi_ok {
            return false;
        }
        // napi_object = 6
        if vtype != 6 {
            return false;
        }
        let key = "then\0";
        let mut key_val = ptr::null_mut();
        if sys::napi_create_string_utf8(env, key.as_ptr().cast(), 4isize, &mut key_val)
            != sys::Status::napi_ok
        {
            return false;
        }
        let mut has = false;
        if sys::napi_has_property(env, value, key_val, &mut has) != sys::Status::napi_ok {
            return false;
        }
        if !has {
            return false;
        }
        let mut then_val = ptr::null_mut();
        if sys::napi_get_property(env, value, key_val, &mut then_val) != sys::Status::napi_ok {
            return false;
        }
        let mut then_type = 0i32;
        if sys::napi_typeof(env, then_val, &mut then_type) != sys::Status::napi_ok {
            return false;
        }
        // napi_function = 7
        then_type == 7
    }
}

impl Drop for JsFnRef {
    fn drop(&mut self) {
        unsafe {
            let _ = sys::napi_delete_reference(self.env, self.inner);
        }
    }
}

/// Nunjucks-style sync loader: JS `(name) => string | null | { src: string }` (main thread only).
struct JsTemplateLoader {
    get_source: Arc<JsFnRef>,
}

impl TemplateLoader for JsTemplateLoader {
    fn load(&self, name: &str) -> runjucks_core::errors::Result<String> {
        let v = self
            .get_source
            .call(&[serde_json::json!(name)])
            .map_err(|e| RunjucksError::new(e.to_string()))?;
        match v {
            serde_json::Value::Null => {
                Err(RunjucksError::new(format!("template not found: {name}")))
            }
            serde_json::Value::String(s) => Ok(s),
            serde_json::Value::Object(o) => match o.get("src") {
                Some(serde_json::Value::String(s)) => Ok(s.clone()),
                _ => Err(RunjucksError::new(
                    "loader callback must return a string, null, or { src: string }",
                )),
            },
            _ => Err(RunjucksError::new(
                "loader callback must return a string, null, or { src: string }",
            )),
        }
    }

    fn cache_key(&self, _name: &str) -> Option<String> {
        None
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

/// Creates an [`AsyncCustomFilter`] backed by a synchronous JS function reference.
///
/// The JS function is called synchronously on the Node main thread (same as sync filters).
/// This bridges `addAsyncFilter` so that the async renderer can invoke these filters.
fn napi_async_custom_filter_from_sync(js: Arc<JsFnRef>) -> AsyncCustomFilter {
    Arc::new(
        move |input: &serde_json::Value, args: &[serde_json::Value]| {
            let js = js.clone();
            let input = input.clone();
            let args = args.to_vec();
            Box::pin(async move {
                let active = RENDER_NAPI_ENV.with(|c| c.get()).ok_or_else(|| {
                    RunjucksError::new(
                        "async custom filter invoked without an active Node N-API render context",
                    )
                })?;
                if active != js.env {
                    return Err(RunjucksError::new(
                        "N-API environment mismatch during async custom filter call",
                    ));
                }
                let mut call_args: Vec<serde_json::Value> = Vec::with_capacity(1 + args.len());
                call_args.push(input);
                call_args.extend(args);
                js.call(&call_args)
                    .map_err(|e: Error| RunjucksError::new(e.to_string()))
            })
                as Pin<
                    Box<
                        dyn std::future::Future<
                                Output = runjucks_core::errors::Result<serde_json::Value>,
                            > + Send,
                    >,
                >
        },
    )
}

/// Creates an [`AsyncCustomGlobalFn`] backed by a synchronous JS function reference.
fn napi_async_custom_global_from_sync(js: Arc<JsFnRef>) -> AsyncCustomGlobalFn {
    Arc::new(
        move |args: &[serde_json::Value], kwargs: &[(String, serde_json::Value)]| {
            let js = js.clone();
            let args = args.to_vec();
            let kwargs = kwargs.to_vec();
            Box::pin(async move {
                let active = RENDER_NAPI_ENV.with(|c| c.get()).ok_or_else(|| {
                    RunjucksError::new(
                        "async custom global invoked without an active Node N-API render context",
                    )
                })?;
                if active != js.env {
                    return Err(RunjucksError::new(
                        "N-API environment mismatch during async custom global call",
                    ));
                }
                let mut call_args: Vec<serde_json::Value> = args;
                if !kwargs.is_empty() {
                    let m: serde_json::Map<String, serde_json::Value> =
                        kwargs.into_iter().collect();
                    call_args.push(serde_json::Value::Object(m));
                }
                js.call(&call_args)
                    .map_err(|e: Error| RunjucksError::new(e.to_string()))
            })
                as Pin<
                    Box<
                        dyn std::future::Future<
                                Output = runjucks_core::errors::Result<serde_json::Value>,
                            > + Send,
                    >,
                >
        },
    )
}

/// Wraps a `Result<String>` as a resolved/rejected JS `Promise`.
fn wrap_result_as_promise(
    napi_env: sys::napi_env,
    result: Result<String>,
) -> Result<Unknown<'static>> {
    unsafe {
        let mut deferred = ptr::null_mut();
        let mut promise = ptr::null_mut();
        check_status!(
            sys::napi_create_promise(napi_env, &mut deferred, &mut promise),
            "create promise"
        )?;
        match result {
            Ok(s) => {
                let mut value = ptr::null_mut();
                check_status!(
                    sys::napi_create_string_utf8(
                        napi_env,
                        s.as_ptr().cast(),
                        s.len() as isize,
                        &mut value,
                    ),
                    "create string"
                )?;
                check_status!(
                    sys::napi_resolve_deferred(napi_env, deferred, value),
                    "resolve"
                )?;
            }
            Err(e) => {
                let msg = e.to_string();
                let mut err_msg = ptr::null_mut();
                check_status!(
                    sys::napi_create_string_utf8(
                        napi_env,
                        msg.as_ptr().cast(),
                        msg.len() as isize,
                        &mut err_msg,
                    ),
                    "create error message"
                )?;
                let mut error_obj = ptr::null_mut();
                check_status!(
                    sys::napi_create_error(napi_env, ptr::null_mut(), err_msg, &mut error_obj),
                    "create error"
                )?;
                check_status!(
                    sys::napi_reject_deferred(napi_env, deferred, error_obj),
                    "reject"
                )?;
            }
        }
        Unknown::from_napi_value(napi_env, promise)
    }
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

/// Parses JSON bytes to [`serde_json::Value`]. By default the **`fast-json`** crate feature is enabled
/// and this uses **`simd-json`** (same semantic result for valid JSON; invalid JSON errors may differ
/// slightly in message text vs `serde_json`). Build with `--no-default-features` to use `serde_json`
/// only.
fn parse_json_context_bytes(bytes: Vec<u8>) -> napi::Result<serde_json::Value> {
    #[cfg(feature = "fast-json")]
    {
        let mut bytes = bytes;
        simd_json::serde::from_slice::<serde_json::Value>(&mut bytes)
            .map_err(|e| Error::from_reason(format!("JSON parse: {e}")))
    }
    #[cfg(not(feature = "fast-json"))]
    {
        serde_json::from_slice(&bytes).map_err(|e| Error::from_reason(e.to_string()))
    }
}

fn parse_json_context_string(ctx: String) -> napi::Result<serde_json::Value> {
    parse_json_context_bytes(ctx.into_bytes())
}

/// Like [`render_string`], but the context is **JSON text** (e.g. from `JSON.stringify(ctx)` in JS).
/// Skips N-API object→JSON conversion on the JS side when you already have a string; Rust parses to
/// `serde_json::Value` before render (**`simd-json`** by default; use `--no-default-features` for
/// `serde_json`-only parse).
#[napi(js_name = "renderStringFromJson")]
pub fn render_string_from_json(template: String, context_json: String) -> napi::Result<String> {
    let ctx = parse_json_context_string(context_json)?;
    render_with_env(&Environment::default(), template, ctx)
}

/// Same as [`render_string_from_json`], but context is **UTF-8 JSON bytes** (e.g. `Buffer` /
/// `Uint8Array`). Avoids an extra Rust `String` allocation when the payload is already binary;
/// still parses to `serde_json::Value` before render.
#[napi(js_name = "renderStringFromJsonBuffer")]
pub fn render_string_from_json_buffer(
    template: String,
    context_json: Uint8Array,
) -> napi::Result<String> {
    let ctx = parse_json_context_bytes(context_json.as_ref().to_vec())?;
    render_with_env(&Environment::default(), template, ctx)
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

#[napi(object)]
pub struct ConfigureOptions {
    /// Nunjucks accepts truthy/falsy values; normalized to a Rust `bool` for the engine (see `coerce_autoescape_value`).
    #[napi(ts_type = "boolean | string | number | null | undefined")]
    pub autoescape: Option<Unknown<'static>>,
    pub dev: Option<bool>,
    pub throw_on_undefined: Option<bool>,
    pub trim_blocks: Option<bool>,
    pub lstrip_blocks: Option<bool>,
    pub tags: Option<TagsOptions>,
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct ExtensionDescriptor {
    pub name: String,
    pub tags: Vec<String>,
    pub blocks: HashMap<String, String>,
}

fn validate_parse(env: &Environment, src: &str) -> std::result::Result<(), RunjucksError> {
    env.validate_lex_parse(src)
}

/// Match Nunjucks `suppressValue` / JS truthiness for `opts.autoescape`: `false`, `0`, `""`, `null`, `undefined` → off; other truthy values (including non-empty strings like `"html"`) → on.
fn coerce_autoescape_value(u: &Unknown) -> Result<bool> {
    match u.get_type()? {
        ValueType::Undefined | ValueType::Null => Ok(false),
        ValueType::Boolean => FromNapiValue::from_unknown(u.clone()),
        ValueType::Number => {
            let n: f64 = FromNapiValue::from_unknown(u.clone())?;
            Ok(n != 0.0 && !n.is_nan())
        }
        ValueType::String => {
            let s: String = FromNapiValue::from_unknown(u.clone())?;
            Ok(!s.is_empty())
        }
        _ => Ok(true),
    }
}

fn apply_configure_opts(env: &mut Environment, opts: &ConfigureOptions) -> Result<()> {
    if let Some(ref a) = opts.autoescape {
        env.autoescape = coerce_autoescape_value(a)?;
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
    Ok(())
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

    /// Same as [`render_string`], but context is a JSON string (see [`render_string_from_json`]).
    #[napi(js_name = "renderStringFromJson")]
    pub fn render_string_from_json_env(
        &self,
        env: Env,
        template: String,
        context_json: String,
    ) -> Result<String> {
        let ctx = parse_json_context_string(context_json)?;
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        with_render_napi_env(env.raw(), || render_with_env(&inner, template, ctx))
    }

    /// Same as [`render_string_from_json_buffer`] for this environment.
    #[napi(js_name = "renderStringFromJsonBuffer")]
    pub fn render_string_from_json_buffer_env(
        &self,
        env: Env,
        template: String,
        context_json: Uint8Array,
    ) -> Result<String> {
        let ctx = parse_json_context_bytes(context_json.as_ref().to_vec())?;
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        with_render_napi_env(env.raw(), || render_with_env(&inner, template, ctx))
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

    /// Returns an introspection-only descriptor for a registered extension (tags + block end tags).
    #[napi(js_name = "getExtension")]
    pub fn get_extension(&self, name: String) -> Result<Option<ExtensionDescriptor>> {
        let env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(env
            .get_extension_descriptor(&name)
            .map(|desc| ExtensionDescriptor {
                name: desc.name,
                tags: desc.tags,
                blocks: desc.blocks,
            }))
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

    /// Registers a global: JSON-serializable value, or a **JavaScript function** invoked for `{{ name(...) }}` (Nunjucks-style keyword args as a trailing object). See `ai_docs/NUNJUCKS_PARITY.md` (P1).
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
            let v = unsafe { serde_json::Value::from_napi_value(env.raw(), value.value().value)? };
            inner.add_global(name, v);
        }
        Ok(())
    }

    /// Subset of Nunjucks `configure`: `autoescape` (truthy/falsy coercion like Nunjucks), `dev`, `throwOnUndefined`, `trimBlocks`, `lstripBlocks`, and `tags` are applied.
    #[napi]
    pub fn configure(&self, opts: ConfigureOptions) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        apply_configure_opts(&mut env, &opts)?;
        Ok(())
    }

    /// Sync callback `(name: string) => string | null | { src: string }`. `null` / `undefined` JSON as
    /// `null` means template not found. Replaces any previous loader (same as `setTemplateMap` /
    /// `setLoaderRoot`). Does not use the named parse cache per key (sources may change arbitrarily).
    #[napi(js_name = "setLoaderCallback")]
    pub fn set_loader_callback(&self, env: Env, callback: Unknown) -> Result<()> {
        let js = Arc::new(JsFnRef::new(&env, &callback)?);
        let loader: Arc<dyn TemplateLoader + Send + Sync> =
            Arc::new(JsTemplateLoader { get_source: js });
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        inner.loader = Some(loader);
        inner.clear_named_parse_cache();
        Ok(())
    }

    /// Clears parse caches for named templates and inline `renderString` / `Template` sources (Nunjucks `invalidateCache`).
    #[napi(js_name = "invalidateCache")]
    pub fn invalidate_cache_js(&self) -> Result<()> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        inner.invalidate_cache();
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
            Error::from_reason(
                "no template loader configured (use setTemplateMap, setLoaderRoot, or setLoaderCallback first)",
            )
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

    /// Loads named templates from a directory on disk (relative paths under `root`). Replaces any
    /// previous loader. See [`runjucks_core::FileSystemLoader`].
    #[napi(js_name = "setLoaderRoot")]
    pub fn set_loader_root(&self, path: String) -> Result<()> {
        let loader =
            file_system_loader(Path::new(&path)).map_err(|e| Error::from_reason(e.to_string()))?;
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        env.loader = Some(loader);
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

    /// Registers an async filter `(input, ...args) => any`.
    /// The function is called synchronously on the main thread during render, but registered
    /// as an async filter so it's available in `renderStringAsync` / `renderTemplateAsync`.
    #[napi(js_name = "addAsyncFilter")]
    pub fn add_async_filter(&self, env: Env, name: String, func: Unknown) -> Result<()> {
        let js = Arc::new(JsFnRef::new(&env, &func)?);
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        inner.add_async_filter(name, napi_async_custom_filter_from_sync(js));
        Ok(())
    }

    /// Registers an async global callable `(...args) => any`.
    #[napi(js_name = "addAsyncGlobal")]
    pub fn add_async_global(&self, env: Env, name: String, func: Unknown) -> Result<()> {
        let js = Arc::new(JsFnRef::new(&env, &func)?);
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        inner.add_async_global_callable(name, napi_async_custom_global_from_sync(js));
        Ok(())
    }

    /// Async render of an inline template string. Returns a `Promise<string>`.
    ///
    /// Async render of an inline template string. Returns a `Promise<string>`.
    ///
    /// Supports async-only tags (`asyncEach`, `asyncAll`, `ifAsync`) and async filters/globals.
    ///
    /// **Implementation note:** Rendering executes synchronously on the calling thread
    /// (the Node.js main thread) via a current-thread tokio runtime. The result is wrapped
    /// in an already-resolved/rejected Promise. This matches the Nunjucks `renderString`
    /// callback API surface but does **not** yield the event loop during render. This is
    /// an intentional trade-off: the async renderer's future holds `&mut` borrows that are
    /// `!Send`, preventing off-thread execution. True non-blocking rendering would require
    /// an `Arc<Mutex<...>>`-based state design in a future major version.
    #[napi(js_name = "renderStringAsync", ts_return_type = "Promise<string>")]
    pub fn render_string_async(
        &self,
        env: Env,
        template: String,
        context: serde_json::Value,
    ) -> Result<Unknown<'static>> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let result = with_render_napi_env(env.raw(), || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| Error::from_reason(e.to_string()))?;
            rt.block_on(inner.render_string_async(template, context))
                .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
        });
        wrap_result_as_promise(env.raw(), result)
    }

    /// Async render of a named template. Returns a `Promise<string>`.
    /// Same blocking-then-Promise-wrap behavior as [`render_string_async`].
    #[napi(js_name = "renderTemplateAsync", ts_return_type = "Promise<string>")]
    pub fn render_template_async(
        &self,
        env: Env,
        name: String,
        context: serde_json::Value,
    ) -> Result<Unknown<'static>> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let result = with_render_napi_env(env.raw(), || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| Error::from_reason(e.to_string()))?;
            rt.block_on(inner.render_template_async(&name, context))
                .map_err(|e: RunjucksError| Error::from_reason(e.to_string()))
        });
        wrap_result_as_promise(env.raw(), result)
    }
}

/// Nunjucks-style module `configure(opts?)` — sets the default environment used by [`render`].
#[napi(js_name = "configure")]
pub fn configure_default(opts: Option<ConfigureOptions>) -> Result<JsEnvironment> {
    let env = Arc::new(Mutex::new(Environment::default()));
    if let Some(o) = opts {
        let mut inner = env.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        apply_configure_opts(&mut inner, &o)?;
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
