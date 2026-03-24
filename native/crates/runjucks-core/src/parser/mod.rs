//! Builds [`crate::ast::Node`] trees from [`crate::lexer::Token`] streams.
//!
//! `{{ … }}` bodies are parsed with [`parse_expr`] (nom-driven, Nunjucks-style precedence; see [`expr`]).
//! `{% … %}` supports `if` / `elif` / `else` / `endif`, `for` / `else` / `endfor`, and `set` (see [`template`]).
//! For other tags, use [`crate::tag_lex::tokenize_tag_body`] for tokenization only.

pub mod expr;
mod template;

use crate::ast::{Expr, Node};
use crate::errors::Result;
use crate::lexer::Token;

/// Parses a token stream into a single [`Node::Root`] containing child nodes.
///
/// # Errors
///
/// Returns an error on malformed `{% %}` blocks, unknown tag keywords, or invalid expressions.
///
/// # Examples
///
/// ```
/// use runjucks_core::lexer::tokenize;
/// use runjucks_core::parser::parse;
///
/// let tokens = tokenize("a{{x}}b").unwrap();
/// let root = parse(&tokens).unwrap();
/// ```
pub fn parse(tokens: &[Token]) -> Result<Node> {
    template::parse_template_tokens(tokens)
}

/// Parses the inside of a `{{` … `}}` region into an [`Expr`].
///
/// # Errors
///
/// Invalid syntax or trailing garbage (after trim) yields [`RunjucksError`].
///
/// # Examples
///
/// ```
/// use runjucks_core::parser::parse_expr;
/// use runjucks_core::ast::Expr;
///
/// match parse_expr("foo").unwrap() {
///     Expr::Variable(name) => assert_eq!(name, "foo"),
///     _ => panic!("expected variable"),
/// }
/// ```
pub fn parse_expr(source: &str) -> Result<Expr> {
    expr::parse_expression(source)
}
