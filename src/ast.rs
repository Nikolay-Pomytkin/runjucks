//! Abstract syntax tree for Runjucks templates.

use serde_json::Value;

/// A template node (statements and raw text).
#[allow(dead_code)] // `Output` and expression nodes will be constructed once `{{ }}` parsing lands.
#[derive(Debug, Clone)]
pub enum Node {
    /// Root template body.
    Root(Vec<Node>),
    /// Literal text outside of `{{ }}` / `{% %}`.
    Text(String),
    /// `{{ ... }}` — one or more expressions (filters chain on an expression).
    Output(Vec<Expr>),
}

/// Expression AST (variables, literals, operators, filters).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Value),
    /// Simple variable: `foo` (dotted paths and subscripts come later).
    Variable(String),
}
