#![deny(clippy::all)]

use napi::bindgen_prelude::Unknown;
use napi_derive::napi;
use runjucks_core::{Environment, RunjucksError};
use std::sync::{Arc, Mutex};

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
        .map_err(|e: RunjucksError| napi::Error::from_reason(e.to_string()))
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

    #[napi]
    pub fn render_string(
        &self,
        template: String,
        context: serde_json::Value,
    ) -> napi::Result<String> {
        let env = self
            .inner
            .lock()
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        render_with_env(&env, template, context)
    }

    #[napi]
    pub fn set_autoescape(&self, enabled: bool) -> napi::Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        env.autoescape = enabled;
        Ok(())
    }

    #[napi]
    pub fn set_dev(&self, enabled: bool) -> napi::Result<()> {
        let mut env = self
            .inner
            .lock()
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        env.dev = enabled;
        Ok(())
    }

    #[napi]
    pub fn add_filter(&mut self, _name: String, _func: Unknown) -> napi::Result<()> {
        let _ = _func;
        Ok(())
    }
}
