//! Builds [`crate::ast::Node`] trees from [`crate::lexer::Token`] streams.
//!
//! `{{ … }}` bodies are parsed with [`parse_expr`] (nom-driven, Nunjucks-style precedence; see [`expr`]).
//! For `{%` … `%}` tags, use [`crate::tag_lex::tokenize_tag_body`] on [`crate::lexer::Token::Tag`] inner strings;
//! control-flow AST and statement parsing are not implemented yet (tags still error in [`parse`]).

pub mod expr;

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
                let e = parse_expr(inner)?;
                nodes.push(Node::Output(vec![e]));
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
