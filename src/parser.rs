//! Recursive-descent parser. Currently maps [`crate::lexer::Token`] to [`crate::ast::Node`].

use crate::ast::{Expr, Node};
use crate::errors::{Result, RunjucksError};
use crate::lexer::Token;

/// Parse a token stream into a template AST (root node).
pub fn parse(tokens: &[Token]) -> Result<Node> {
    let mut nodes = Vec::new();
    for t in tokens {
        match t {
            Token::Text(s) => nodes.push(Node::Text(s.clone())),
        }
    }
    Ok(Node::Root(nodes))
}

/// Placeholder for expression parsing inside `{{ }}` (not yet wired from the lexer).
#[allow(dead_code)]
pub fn parse_expr(_source: &str) -> Result<Expr> {
    Err(RunjucksError::new(
        "expression parsing is not implemented yet (lexer/parser for {{ }} in progress)",
    ))
}
