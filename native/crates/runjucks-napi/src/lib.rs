#![deny(clippy::all)]

use napi::bindgen_prelude::{FromNapiValue, JsValue, Unknown};
use napi::bindgen_prelude::ToNapiValue;
use napi::{check_pending_exception, check_status, sys, Env, Error, Result, Status, ValueType};
use napi_derive::napi;
use runjucks_core::{map_loader, CustomFilter, Environment, RunjucksError};
use std::cell::Cell;
use std::collections::HashMap;
use std::ptr;
use std::sync::{Arc, Mutex};

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

fn napi_custom_filter(js: Arc<JsFnRef>) -> CustomFilter {
    Arc::new(move |input, args| {
        let active = RENDER_NAPI_ENV.with(|c| c.get()).ok_or_else(|| {
            RunjucksError::new(
                "custom filter invoked without an active Node N-API render context",
            )
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
pub struct ConfigureOptions {
    pub autoescape: Option<bool>,
    pub dev: Option<bool>,
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

    /// JSON-serializable globals only; JavaScript functions are rejected by conversion (see parity doc).
    #[napi(js_name = "addGlobal")]
    pub fn add_global(&self, name: String, value: serde_json::Value) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        env.add_global(name, value);
        Ok(())
    }

    /// Subset of Nunjucks `configure`: `autoescape` and `dev` are applied; other keys are not supported yet.
    #[napi]
    pub fn configure(&self, opts: ConfigureOptions) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if let Some(a) = opts.autoescape {
            env.autoescape = a;
        }
        if let Some(d) = opts.dev {
            env.dev = d;
        }
        Ok(())
    }

    /// Sets an in-memory template map (`name` → source). Enables `renderTemplate`, `{% include %}`, `{% extends %}`, etc.
    #[napi]
    pub fn set_template_map(&self, map: HashMap<String, String>) -> Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        env.loader = Some(map_loader(map));
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
