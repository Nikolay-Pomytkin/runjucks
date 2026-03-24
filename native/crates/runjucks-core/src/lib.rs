#![deny(clippy::all)]
//! Pure Rust template engine core for **Runjucks**, a [Nunjucks](https://mozilla.github.io/nunjucks/)-oriented engine.
//!
//! This crate is the **lex → parse → render** pipeline with no Node or NAPI dependencies. The published
//! [`runjucks` npm package](https://www.npmjs.com/package/runjucks) wraps it in a thin native addon; most
//! JavaScript callers use that API instead of linking this crate directly.
//!
//! Pipeline:
//! 1. [`lexer::tokenize`] splits template source into [`lexer::Token`]s.
//! 2. For each [`lexer::Token::Tag`], [`tag_lex::tokenize_tag_body`] can split the inner string into keywords and identifiers.
//! 3. [`parser::parse`] builds an [`ast::Node`] tree; [`parser::parse_expr`] parses `{{ }}` bodies with Nunjucks-style precedence (see [`parser::expr`]).
//! 4. [`renderer::render`] walks the AST with an [`Environment`], optional [`loader::TemplateLoader`], and JSON [`serde_json::Value`] context (`{% include %}`, `{% extends %}` / `{% block %}`, macros).
//!
//! # Example
//!
//! ```
//! use runjucks_core::Environment;
//! use serde_json::json;
//!
//! let env = Environment::default();
//! let out = env
//!     .render_string("Hello, {{ name }}".into(), json!({ "name": "Ada" }))
//!     .unwrap();
//! assert_eq!(out, "Hello, Ada");
//! ```

pub mod ast;
pub mod environment;
pub mod errors;
pub mod filters;
pub mod lexer;
pub mod loader;
pub mod parser;
pub mod renderer;
pub mod tag_lex;
pub mod value;

pub use environment::Environment;
pub use errors::RunjucksError;
pub use loader::{map_loader, FnLoader, TemplateLoader};
