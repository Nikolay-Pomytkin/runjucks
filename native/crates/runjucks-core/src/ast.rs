//! Abstract syntax tree for templates after [`crate::parser::parse`].
//!
//! Today this is a flat list of children under [`Node::Root`]: plain [`Node::Text`] and
//! [`Node::Output`] nodes for `{{ … }}` regions.

use serde_json::Value;

/// Template structure produced by the parser.
///
/// - [`Node::Root`]: sequence of top-level fragments (text + outputs).
/// - [`Node::Text`]: literal characters from the template.
/// - [`Node::Output`]: one or more [`Expr`] values to evaluate and concatenate (filters etc. are future work).
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Container for sibling template fragments.
    Root(Vec<Node>),
    /// Raw text between (or outside) template tags.
    Text(String),
    /// Content inside `{{` … `}}` after expression parsing.
    Output(Vec<Expr>),
}

/// Expression inside a variable tag.
///
/// - [`Expr::Literal`]: JSON literal (reserved for richer expressions later).
/// - [`Expr::Variable`]: single identifier looked up in the render context.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Embedded JSON value.
    Literal(Value),
    /// Context key; non-existent keys render as empty (Nunjucks-style).
    Variable(String),
}
