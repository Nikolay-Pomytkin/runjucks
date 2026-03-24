//! Builds [`crate::ast::Node`] trees from [`crate::lexer::Token`] streams.
//!
//! Variable bodies are parsed with [`parse_expr`] (currently a single identifier only).

use crate::ast::{Expr, Node};
use crate::errors::{Result, RunjucksError};
use crate::lexer::Token;

/// Parses a token stream into a single [`Node::Root`] containing child nodes.
///
/// # Errors
///
/// Returns an error if a [`Token::Tag`] is present (`{%` … `%}` is not implemented yet).
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
    let mut nodes = Vec::new();
    for t in tokens {
        match t {
            Token::Text(s) => nodes.push(Node::Text(s.clone())),
            Token::Expression(inner) => {
                let expr = parse_expr(inner)?;
                nodes.push(Node::Output(vec![expr]));
            }
            Token::Tag(_) => {
                return Err(RunjucksError::new(
                    "template tags `{% %}` are not implemented yet",
                ));
            }
        }
    }
    Ok(Node::Root(nodes))
}

/// Parses the inside of a `{{` … `}}` region into an [`Expr`].
///
/// Today only a **single identifier** is allowed (no spaces, no literals yet in this path).
///
/// # Errors
///
/// - Empty or whitespace-only body.
/// - More than one token / whitespace inside the expression.
/// - Invalid identifier characters.
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
    let s = source.trim();
    if s.is_empty() {
        return Err(RunjucksError::new(
            "empty expression inside `{{ }}` is not allowed",
        ));
    }
    if s.chars().any(char::is_whitespace) {
        return Err(RunjucksError::new(
            "expected a single identifier inside `{{ }}` (v1)",
        ));
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(RunjucksError::new(
            "identifier must start with a letter or underscore",
        ));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(RunjucksError::new("invalid character in identifier"));
    }
    Ok(Expr::Variable(s.to_owned()))
}
