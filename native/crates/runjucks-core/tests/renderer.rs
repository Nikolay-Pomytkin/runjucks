use runjucks_core::ast::{Expr, Node};
use runjucks_core::environment::Environment;
use runjucks_core::renderer::render;
use serde_json::json;

#[test]
fn render_nested_root_flattens_text() {
    let env = Environment::default();
    let inner = Node::Root(vec![Node::Text("a".into()), Node::Text("b".into())]);
    let root = Node::Root(vec![inner]);
    let mut ctx = json!({});
    let out = render(&env, &root, &mut ctx).unwrap();
    assert_eq!(out, "ab");
}

#[test]
fn render_output_literal() {
    let env = Environment::default();
    let root = Node::Root(vec![Node::Output(vec![Expr::Literal(json!("hi"))])]);
    let mut ctx = json!({});
    let out = render(&env, &root, &mut ctx).unwrap();
    assert_eq!(out, "hi");
}

#[test]
fn render_output_variable_missing_is_empty() {
    let env = Environment::default();
    let root = Node::Root(vec![Node::Output(vec![Expr::Variable("missing".into())])]);
    let mut ctx = json!({});
    let out = render(&env, &root, &mut ctx).unwrap();
    assert_eq!(out, "");
}

#[test]
fn render_output_variable_present() {
    let env = Environment::default();
    let root = Node::Root(vec![Node::Output(vec![Expr::Variable("name".into())])]);
    let mut ctx = json!({ "name": "Ada" });
    let out = render(&env, &root, &mut ctx).unwrap();
    assert_eq!(out, "Ada");
}

#[test]
fn render_variable_autoescapes_html_when_enabled() {
    let env = Environment {
        autoescape: true,
        dev: false,
    };
    let root = Node::Root(vec![Node::Output(vec![Expr::Variable("x".into())])]);
    let mut ctx = json!({ "x": "<script>" });
    let out = render(&env, &root, &mut ctx).unwrap();
    assert_eq!(out, "&lt;script&gt;");
}

#[test]
fn render_variable_no_escape_when_autoescape_off() {
    let env = Environment {
        autoescape: false,
        dev: false,
    };
    let root = Node::Root(vec![Node::Output(vec![Expr::Variable("x".into())])]);
    let mut ctx = json!({ "x": "<b>" });
    let out = render(&env, &root, &mut ctx).unwrap();
    assert_eq!(out, "<b>");
}
