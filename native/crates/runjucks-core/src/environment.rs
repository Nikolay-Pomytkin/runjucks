//! [`Environment`] holds render options and is the entry point for [`Environment::render_string`].
//!
//! It ties together [`crate::lexer::tokenize`], [`crate::parser::parse`], and [`crate::renderer::render`].

use crate::errors::Result;
use crate::{lexer, parser, renderer};
use serde_json::Value;

/// Configuration and entry point for rendering templates.
///
/// # Fields
///
/// - **`autoescape`**: When `true` (the default), HTML-escapes string output from variable tags via
///   [`crate::filters::escape_html`].
/// - **`dev`**: Reserved for developer-mode behavior (e.g. richer errors); currently unused in the renderer.
///
/// # Default
///
/// [`Environment::default`] sets `autoescape = true` and `dev = false`.
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
#[derive(Debug)]
pub struct Environment {
    /// When true, variable output is passed through [`crate::filters::escape_html`].
    pub autoescape: bool,
    /// Developer mode flag (reserved).
    pub dev: bool,
}

impl Default for Environment {
    /// Returns an environment with `autoescape = true` and `dev = false`.
    fn default() -> Self {
        Self {
            autoescape: true,
            dev: false,
        }
    }
}

impl Environment {
    /// Lexes `template`, parses it to an AST, and renders it with `context`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::errors::RunjucksError`] when:
    ///
    /// - The [`crate::lexer`] finds malformed delimiters (e.g. unclosed `{{`).
    /// - The [`crate::parser`] hits unsupported syntax (e.g. `{%` tags are not implemented yet).
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
        renderer::render(self, &ast, &context)
    }
}
