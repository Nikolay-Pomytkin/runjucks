use crate::ast::{Expr, Node};
use crate::errors::{Result, RunjucksError};
use crate::lexer::Token;

pub fn parse(tokens: &[Token]) -> Result<Node> {
    let mut nodes = Vec::new();
    for t in tokens {
        match t {
            Token::Text(s) => nodes.push(Node::Text(s.clone())),
            Token::Expression(inner) => {
                let expr = parse_expr(inner)?;
                nodes.push(Node::Output(vec![expr]));
            }
        }
    }
    Ok(Node::Root(nodes))
}

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
