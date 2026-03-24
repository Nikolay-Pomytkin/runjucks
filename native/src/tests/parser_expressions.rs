//! Parser parity with nunjucks expression / output parsing.
//! See nunjucks/tests/parser.js (`should parse basic types`, …).

use runjucks::ast::{Expr, Node};
use runjucks::lexer::tokenize;
use runjucks::parser::parse;
use serde_json::{json, Value};

#[test]
fn parse_output_integer_literal() {
    let tokens = tokenize("{{ 1 }}").unwrap();
    let ast = parse(&tokens).unwrap();
    assert_eq!(
        ast,
        Node::Root(vec![Node::Output(vec![Expr::Literal(json!(1))])])
    );
}

#[test]
fn parse_output_float_literal() {
    let tokens = tokenize("{{ 4.567 }}").unwrap();
    let ast = parse(&tokens).unwrap();
    assert_eq!(
        ast,
        Node::Root(vec![Node::Output(vec![Expr::Literal(json!(4.567))])])
    );
}

#[test]
fn parse_output_string_double_quotes() {
    let tokens = tokenize("{{ \"foo\" }}").unwrap();
    let ast = parse(&tokens).unwrap();
    assert_eq!(
        ast,
        Node::Root(vec![Node::Output(vec![Expr::Literal(json!("foo"))])])
    );
}

#[test]
fn parse_output_bool_true() {
    let tokens = tokenize("{{ true }}").unwrap();
    let ast = parse(&tokens).unwrap();
    assert_eq!(
        ast,
        Node::Root(vec![Node::Output(vec![Expr::Literal(json!(true))])])
    );
}

#[test]
fn parse_output_none_as_null() {
    let tokens = tokenize("{{ none }}").unwrap();
    let ast = parse(&tokens).unwrap();
    assert_eq!(
        ast,
        Node::Root(vec![Node::Output(vec![Expr::Literal(Value::Null)])])
    );
}

#[test]
fn parse_output_binary_addition() {
    let tokens = tokenize("{{ 2 + 3 }}").unwrap();
    let _ast = parse(&tokens).unwrap();
}
