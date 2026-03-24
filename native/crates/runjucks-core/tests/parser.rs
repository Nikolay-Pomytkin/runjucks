use runjucks_core::ast::{Expr, Node};
use runjucks_core::lexer::Token;
use runjucks_core::parser::{parse, parse_expr};

#[test]
fn parse_empty_token_list_yields_empty_root() {
    let ast = parse(&[]).unwrap();
    match ast {
        Node::Root(nodes) => assert!(nodes.is_empty()),
        _ => panic!("expected Root"),
    }
}

#[test]
fn parse_concatenates_adjacent_text_tokens_into_sequential_nodes() {
    let ast = parse(&[
        Token::Text("a".into()),
        Token::Text("b".into()),
        Token::Text("c".into()),
    ])
    .unwrap();
    match ast {
        Node::Root(nodes) => {
            assert_eq!(nodes.len(), 3);
            assert!(matches!(&nodes[0], Node::Text(s) if s == "a"));
            assert!(matches!(&nodes[1], Node::Text(s) if s == "b"));
            assert!(matches!(&nodes[2], Node::Text(s) if s == "c"));
        }
        _ => panic!("expected Root"),
    }
}

#[test]
fn parse_expr_single_identifier() {
    match parse_expr("  name  ").unwrap() {
        Expr::Variable(s) => assert_eq!(s, "name"),
        _ => panic!("expected Variable"),
    }
}

#[test]
fn parse_expr_rejects_multiple_tokens() {
    let err = parse_expr("not parsed yet").unwrap_err();
    assert!(err.to_string().contains("single identifier"), "{}", err);
}
