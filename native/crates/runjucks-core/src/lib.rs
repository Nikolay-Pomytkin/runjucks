#![deny(clippy::all)]
//! Pure Rust template engine core for **Runjucks**, a [Nunjucks](https://mozilla.github.io/nunjucks/)-oriented engine.
//!
//! This crate is the **lex → parse → render** pipeline with no Node or NAPI dependencies. The published
//! [`runjucks` npm package](https://www.npmjs.com/package/runjucks) wraps it in a thin native addon; most
//! JavaScript callers use that API instead of linking this crate directly.
//!
//! Pipeline:
//! 1. [`lexer::tokenize`] splits template source into [`lexer::Token`]s.
//! 2. [`parser::parse`] builds an [`ast::Node`] tree (and [`parser::parse_expr`] for `{{ }}` bodies).
//! 3. [`renderer::render`] walks the AST with an [`Environment`] and JSON [`serde_json::Value`] context.
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
pub mod parser;
pub mod renderer;
pub mod value;

pub use environment::Environment;
pub use errors::RunjucksError;
