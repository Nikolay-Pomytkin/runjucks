//! [`Environment`] holds render options and is the entry point for [`Environment::render_string`].
//!
//! It ties together [`crate::lexer::tokenize`], [`crate::parser::parse`], and [`crate::renderer::render`].

use crate::errors::{Result, RunjucksError};
use crate::extension::{
    register_extension_inner, remove_extension_inner, CustomExtensionHandler, ExtensionTagMeta,
};
use crate::globals::{default_globals_map, value_is_callable, RJ_CALLABLE};
use crate::lexer::{LexerOptions, Tags};
use crate::loader::TemplateLoader;
use crate::parser::is_reserved_tag_keyword;
use crate::value::{is_undefined_value, undefined_value, value_to_string};
use crate::{lexer, parser, renderer};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// User-registered filter (Nunjucks `addFilter`). Invoked as `(input, extra_args…)`.
///
/// When a custom filter has the same name as a built-in, the custom filter wins (Nunjucks behavior).
pub type CustomFilter = Arc<dyn Fn(&Value, &[Value]) -> Result<Value> + Send + Sync>;

/// User-registered `is` test (Nunjucks `addTest`). Invoked as `(value, extra_args…) -> bool`.
pub type CustomTest = Arc<dyn Fn(&Value, &[Value]) -> Result<bool> + Send + Sync>;

/// User-registered global **function** (Nunjucks `addGlobal` with a JS function in Node).
///
/// Positional args are passed in order; keyword args are passed as a single trailing object value
/// (Nunjucks keyword-argument convention), represented as `[(String, Value)]` before marshalling.
pub type CustomGlobalFn = Arc<dyn Fn(&[Value], &[(String, Value)]) -> Result<Value> + Send + Sync>;

/// Configuration and entry point for rendering templates.
///
/// # Fields
///
/// - **`autoescape`**: When `true` (the default), HTML-escapes string output from variable tags via
///   [`crate::filters::escape_html`].
/// - **`dev`**: Reserved for developer-mode behavior (e.g. richer errors); currently unused in the renderer.
/// - **`throw_on_undefined`**: When `true`, unbound variables are errors instead of the internal undefined sentinel.
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
    /// When true, an unbound variable name (not in context or globals) is a render error instead of [`crate::value::undefined_value`].
    pub throw_on_undefined: bool,
    /// When set, [`crate::filters::apply_builtin`] `random` uses this seed for reproducible output (conformance / tests).
    pub random_seed: Option<u64>,
    /// When true, the first newline after a block tag (`{% … %}`) is automatically removed (Nunjucks `trimBlocks`).
    pub trim_blocks: bool,
    /// When true, leading whitespace/tabs on a line before a block tag or comment are stripped (Nunjucks `lstripBlocks`).
    pub lstrip_blocks: bool,
    /// Custom delimiter strings (Nunjucks `tags` key in `configure`). `None` uses default delimiters.
    pub tags: Option<Tags>,
    pub(crate) custom_filters: HashMap<String, CustomFilter>,
    pub(crate) custom_tests: HashMap<String, CustomTest>,
    /// Nunjucks `addGlobal` with a callable (Node: JS function; tests: [`Environment::add_global_callable`]).
    pub(crate) custom_globals: HashMap<String, CustomGlobalFn>,
    /// Custom tag names → extension metadata (see [`Environment::register_extension`]).
    pub(crate) extension_tags: HashMap<String, ExtensionTagMeta>,
    pub(crate) extension_closing_tag_names: HashSet<String>,
    pub(crate) custom_extensions: HashMap<String, CustomExtensionHandler>,
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment")
            .field("autoescape", &self.autoescape)
            .field("dev", &self.dev)
            .field("loader", &self.loader.is_some())
            .field("globals_len", &self.globals.len())
            .field("custom_filters_len", &self.custom_filters.len())
            .field("custom_tests_len", &self.custom_tests.len())
            .field("custom_globals_len", &self.custom_globals.len())
            .field("extension_tags_len", &self.extension_tags.len())
            .field("throw_on_undefined", &self.throw_on_undefined)
            .field("random_seed", &self.random_seed)
            .finish()
    }
}

fn is_truthy_value(v: &Value) -> bool {
    if is_undefined_value(v) {
        return false;
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn as_is_test_integer(v: &Value) -> Result<i64> {
    v.as_i64()
        .or_else(|| v.as_f64().map(|x| x as i64))
        .ok_or_else(|| RunjucksError::new("test expected a number"))
}

impl Default for Environment {
    /// Returns an environment with `autoescape = true` and `dev = false`.
    fn default() -> Self {
        Self {
            autoescape: true,
            dev: false,
            loader: None,
            globals: default_globals_map(),
            throw_on_undefined: false,
            random_seed: None,
            trim_blocks: false,
            lstrip_blocks: false,
            tags: None,
            custom_filters: HashMap::new(),
            custom_tests: HashMap::new(),
            custom_globals: HashMap::new(),
            extension_tags: HashMap::new(),
            extension_closing_tag_names: HashSet::new(),
            custom_extensions: HashMap::new(),
        }
    }
}

impl Environment {
    /// Registers or replaces a global value (Nunjucks `addGlobal`). Names still lose to template context keys with the same name.
    ///
    /// Replacing a global with a JSON value clears any registered [`Environment::add_global_callable`] for that name.
    pub fn add_global(&mut self, name: impl Into<String>, value: Value) -> &mut Self {
        let name = name.into();
        self.custom_globals.remove(&name);
        self.globals.insert(name, value);
        self
    }

    /// Registers a global **function** implemented in Rust (tests / embedders). Node callers use NAPI `addGlobal` with a JS function.
    ///
    /// The template sees a [`crate::globals::RJ_CALLABLE`] marker for `is callable` / variable resolution; invocation uses `f`.
    pub fn add_global_callable(
        &mut self,
        name: impl Into<String>,
        f: CustomGlobalFn,
    ) -> &mut Self {
        let name = name.into();
        let mut m = Map::new();
        m.insert(RJ_CALLABLE.to_string(), Value::Bool(true));
        self.globals.insert(name.clone(), Value::Object(m));
        self.custom_globals.insert(name, f);
        self
    }

    /// Registers or replaces a custom filter (Nunjucks `addFilter`). Overrides a built-in with the same name.
    pub fn add_filter(&mut self, name: impl Into<String>, filter: CustomFilter) -> &mut Self {
        self.custom_filters.insert(name.into(), filter);
        self
    }

    /// Registers or replaces a custom `is` test (Nunjucks `addTest`). Used by `x is name` and by `select` / `reject`.
    pub fn add_test(&mut self, name: impl Into<String>, test: CustomTest) -> &mut Self {
        self.custom_tests.insert(name.into(), test);
        self
    }

    /// Registers a custom tag extension (Nunjucks `addExtension`): `tag_specs` lists `(opening_tag, optional_end_tag)`.
    pub fn register_extension(
        &mut self,
        extension_name: impl Into<String>,
        tag_specs: Vec<(String, Option<String>)>,
        handler: CustomExtensionHandler,
    ) -> Result<()> {
        let extension_name = extension_name.into();
        register_extension_inner(
            &mut self.extension_tags,
            &mut self.extension_closing_tag_names,
            &mut self.custom_extensions,
            extension_name,
            tag_specs,
            handler,
            is_reserved_tag_keyword,
        )
    }

    /// Returns whether a custom extension with this name is registered (Nunjucks `hasExtension`).
    pub fn has_extension(&self, name: &str) -> bool {
        self.custom_extensions.contains_key(name)
    }

    /// Unregisters a custom extension by name (Nunjucks `removeExtension`). Returns `true` if it existed.
    pub fn remove_extension(&mut self, name: &str) -> bool {
        remove_extension_inner(
            &mut self.extension_tags,
            &mut self.extension_closing_tag_names,
            &mut self.custom_extensions,
            name,
        )
    }

    /// Lexes and parses `src` with this environment’s extension tags (for eager-compile validation).
    pub fn validate_lex_parse(&self, src: &str) -> Result<()> {
        let tokens = lexer::tokenize_with_options(src, self.lexer_options())?;
        let _ = parser::parse_with_env(
            &tokens,
            &self.extension_tags,
            &self.extension_closing_tag_names,
        )?;
        Ok(())
    }

    pub(crate) fn eval_user_is_test(
        &self,
        name: &str,
        value: &Value,
        args: &[Value],
    ) -> Result<bool> {
        match self.custom_tests.get(name) {
            Some(t) => t(value, args),
            None => Err(RunjucksError::new(format!("unknown test: `{name}`"))),
        }
    }

    /// Built-in and user-registered `is` tests (`x is name`, `select` / `reject`). Argument values are already evaluated.
    pub(crate) fn apply_is_test(
        &self,
        test_name: &str,
        value: &Value,
        arg_vals: &[Value],
    ) -> Result<bool> {
        match test_name {
            "equalto" => Ok(arg_vals.first().map(|a| a == value).unwrap_or(false)),
            "sameas" => Ok(match arg_vals.first() {
                Some(a) => match (value, a) {
                    (Value::Object(_), Value::Object(_)) | (Value::Array(_), Value::Array(_)) => {
                        false
                    }
                    _ => a == value,
                },
                None => false,
            }),
            "null" | "none" => Ok(value.is_null()),
            "falsy" => Ok(!is_truthy_value(value)),
            "truthy" => Ok(is_truthy_value(value)),
            "number" => Ok(value.is_number()),
            "string" => Ok(value.is_string()),
            "lower" => Ok(match value {
                Value::String(s) => s.chars().all(|c| !c.is_uppercase()),
                _ => false,
            }),
            "upper" => Ok(match value {
                Value::String(s) => s.chars().all(|c| !c.is_lowercase()),
                _ => false,
            }),
            "callable" => Ok(value_is_callable(value)),
            "defined" => Ok(!is_undefined_value(value)),
            "odd" => {
                let n = as_is_test_integer(value)?;
                Ok(n.rem_euclid(2) != 0)
            }
            "even" => {
                let n = as_is_test_integer(value)?;
                Ok(n.rem_euclid(2) == 0)
            }
            "divisibleby" => {
                let denom = arg_vals
                    .first()
                    .and_then(|a| {
                        a.as_i64()
                            .or_else(|| a.as_f64().map(|x| x as i64))
                            .or_else(|| value_to_string(a).parse().ok())
                    })
                    .ok_or_else(|| RunjucksError::new("`divisibleby` test expects a divisor"))?;
                if denom == 0 {
                    return Ok(false);
                }
                let n = as_is_test_integer(value)?;
                Ok(n.rem_euclid(denom) == 0)
            }
            _ => self.eval_user_is_test(test_name, value, arg_vals),
        }
    }

    /// Resolves a name: template context first (any frame), then [`Environment::globals`].
    ///
    /// Unbound names yield [`crate::value::undefined_value`] unless [`Environment::throw_on_undefined`] is set.
    pub fn resolve_variable(&self, stack: &renderer::CtxStack, name: &str) -> Result<Value> {
        if stack.defined(name) {
            Ok(stack.get(name))
        } else if let Some(v) = self.globals.get(name) {
            Ok(v.clone())
        } else if self.throw_on_undefined {
            Err(RunjucksError::new(format!("undefined variable: `{name}`")))
        } else {
            Ok(undefined_value())
        }
    }

    /// Returns the [`LexerOptions`] derived from this environment's configuration.
    pub fn lexer_options(&self) -> LexerOptions {
        LexerOptions {
            trim_blocks: self.trim_blocks,
            lstrip_blocks: self.lstrip_blocks,
            tags: self.tags.clone(),
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
        let tokens = lexer::tokenize_with_options(&template, self.lexer_options())?;
        let ast = parser::parse_with_env(
            &tokens,
            &self.extension_tags,
            &self.extension_closing_tag_names,
        )?;
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
        let tokens = lexer::tokenize_with_options(&src, self.lexer_options())?;
        let ast = parser::parse_with_env(
            &tokens,
            &self.extension_tags,
            &self.extension_closing_tag_names,
        )?;
        let root = match context {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        let mut stack = renderer::CtxStack::from_root(root);
        let mut state = renderer::RenderState::new(Some(loader.as_ref()), self.random_seed);
        state.push_template(name)?;
        let out = renderer::render_entry(self, &mut state, &ast, &mut stack)?;
        state.pop_template();
        Ok(out)
    }
}
