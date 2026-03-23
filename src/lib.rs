#![deny(clippy::all)]

// `lexer`, `parser`, `renderer`, etc. are public so `tests/*.rs` integration tests can exercise them.
// The stable surface for embedders is `Environment`, `RunjucksError`, and the N-API exports below.
#[doc(hidden)]
pub mod ast;
pub mod environment;
pub mod errors;
#[doc(hidden)]
pub mod filters;
#[doc(hidden)]
pub mod lexer;
#[doc(hidden)]
pub mod parser;
#[doc(hidden)]
pub mod renderer;
#[doc(hidden)]
pub mod value;

pub use environment::Environment;
pub use errors::RunjucksError;

use napi::bindgen_prelude::Unknown;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

/// Render a template string with a JSON context (default environment).
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

/// Nunjucks-compatible `Environment` exposed to JavaScript.
#[napi(js_name = "Environment")]
pub struct JsEnvironment {
    inner: Arc<Mutex<Environment>>,
}

#[napi]
#[allow(clippy::new_without_default)] // N-API class uses `new()` as the JS constructor, not `Default`.
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

    /// Reserved for custom JS filters; implementation deferred until the runtime supports callbacks.
    #[napi]
    pub fn add_filter(&mut self, _name: String, _func: Unknown) -> napi::Result<()> {
        let _ = _func;
        Ok(())
    }
}
