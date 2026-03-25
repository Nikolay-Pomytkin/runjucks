//! [`Environment`] holds render options and is the entry point for [`Environment::render_string`].
//!
//! It ties together [`crate::lexer::tokenize`], [`crate::parser::parse`], and [`crate::renderer::render`].

use crate::errors::{Result, RunjucksError};
use crate::globals::default_globals_map;
use crate::loader::TemplateLoader;
use crate::value::undefined_value;
use crate::{lexer, parser, renderer};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// User-registered filter (Nunjucks `addFilter`). Invoked as `(input, extra_args…)`.
///
/// When a custom filter has the same name as a built-in, the custom filter wins (Nunjucks behavior).
pub type CustomFilter = Arc<dyn Fn(&Value, &[Value]) -> Result<Value> + Send + Sync>;

/// Configuration and entry point for rendering templates.
///
/// # Fields
///
/// - **`autoescape`**: When `true` (the default), HTML-escapes string output from variable tags via
///   [`crate::filters::escape_html`].
/// - **`dev`**: Reserved for developer-mode behavior (e.g. richer errors); currently unused in the renderer.
/// - **`loader`**: Optional [`TemplateLoader`] for [`Environment::render_template`], `{% include %}`, `{% import %}`, `{% from %}`, and `{% extends %}`.
///
/// # Default
///
/// [`Environment::default`] sets `autoescape = true`, `dev = false`, and `loader = None`.
///
/// # Examples
///
/// ```
/// use runjucks_core::Environment;
/// use serde_json::json;
///
/// let mut env = Environment::default();
/// env.autoescape = false;
/// let out = env.render_string("<{{ x }}>".into(), json!({ "x": "b" })).unwrap();
/// assert_eq!(out, "<b>");
/// ```
#[derive(Clone)]
pub struct Environment {
    /// When true, variable output is passed through [`crate::filters::escape_html`].
    pub autoescape: bool,
    /// Developer mode flag (reserved).
    pub dev: bool,
    /// Resolves template names for [`Environment::render_template`], `include`, and `extends`.
    pub loader: Option<Arc<dyn TemplateLoader + Send + Sync>>,
    /// Nunjucks-style globals: used when a name is not bound in the template context (context wins if the key exists, including `null`).
    pub globals: HashMap<String, Value>,
    pub(crate) custom_filters: HashMap<String, CustomFilter>,
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment")
            .field("autoescape", &self.autoescape)
            .field("dev", &self.dev)
            .field("loader", &self.loader.is_some())
            .field("globals_len", &self.globals.len())
            .field("custom_filters_len", &self.custom_filters.len())
            .finish()
    }
}

impl Default for Environment {
    /// Returns an environment with `autoescape = true` and `dev = false`.
    fn default() -> Self {
        Self {
            autoescape: true,
            dev: false,
            loader: None,
            globals: default_globals_map(),
            custom_filters: HashMap::new(),
        }
    }
}

impl Environment {
    /// Registers or replaces a global value (Nunjucks `addGlobal`). Names still lose to template context keys with the same name.
    pub fn add_global(&mut self, name: impl Into<String>, value: Value) -> &mut Self {
        self.globals.insert(name.into(), value);
        self
    }

    /// Registers or replaces a custom filter (Nunjucks `addFilter`). Overrides a built-in with the same name.
    pub fn add_filter(&mut self, name: impl Into<String>, filter: CustomFilter) -> &mut Self {
        self.custom_filters.insert(name.into(), filter);
        self
    }

    /// Resolves a name: template context first (any frame), then [`Environment::globals`].
    pub fn resolve_variable(&self, stack: &renderer::CtxStack, name: &str) -> Value {
        if stack.defined(name) {
            stack.get(name)
        } else {
            self.globals.get(name).cloned().unwrap_or_else(undefined_value)
        }
    }

    /// Lexes `template`, parses it to an AST, and renders it with `context`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::errors::RunjucksError`] when:
    ///
    /// - The [`crate::lexer`] finds malformed delimiters (e.g. unclosed `{{`).
    /// - The [`crate::parser`] hits unsupported tag syntax.
    /// - Rendering fails (currently rare; lookup errors use Nunjucks-style defaults).
    ///
    /// # Examples
    ///
    /// ```
    /// use runjucks_core::Environment;
    /// use serde_json::json;
    ///
    /// let env = Environment::default();
    /// let html = env
    ///     .render_string("{{ msg }}".into(), json!({ "msg": "<ok>" }))
    ///     .unwrap();
    /// assert_eq!(html, "&lt;ok&gt;");
    /// ```
    pub fn render_string(&self, template: String, context: Value) -> Result<String> {
        let tokens = lexer::tokenize(&template)?;
        let ast = parser::parse(&tokens)?;
        let root = match context {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        let mut stack = renderer::CtxStack::from_root(root);
        let loader = self.loader.as_ref().map(|arc| arc.as_ref());
        renderer::render(self, loader, &ast, &mut stack)
    }

    /// Renders a named template using the configured [`TemplateLoader`].
    ///
    /// Supports `{% extends %}`, `{% include %}`, `{% import %}`, `{% from %}`, and `{% macro %}` across files.
    pub fn render_template(&self, name: &str, context: Value) -> Result<String> {
        let loader = self
            .loader
            .as_ref()
            .ok_or_else(|| RunjucksError::new("no template loader configured"))?;
        let src = loader.load(name)?;
        let tokens = lexer::tokenize(&src)?;
        let ast = parser::parse(&tokens)?;
        let root = match context {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        let mut stack = renderer::CtxStack::from_root(root);
        let mut state = renderer::RenderState::new(Some(loader.as_ref()));
        state.push_template(name)?;
        let out = renderer::render_entry(self, &mut state, &ast, &mut stack)?;
        state.pop_template();
        Ok(out)
    }
}
