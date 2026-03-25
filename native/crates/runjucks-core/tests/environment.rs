use runjucks_core::Environment;
use runjucks_core::value::value_to_string;
use serde_json::{json, Value};
use std::sync::Arc;

#[test]
fn plain_text_round_trip() {
    let out = Environment::default()
        .render_string("hello".to_string(), json!({}))
        .unwrap();
    assert_eq!(out, "hello");
}

#[test]
fn render_string_empty_template() {
    let env = Environment::default();
    assert_eq!(env.render_string(String::new(), json!({})).unwrap(), "");
}

#[test]
fn render_string_multiline_plain_text() {
    let env = Environment::default();
    let tpl = "line1\n\nline3".to_string();
    assert_eq!(env.render_string(tpl.clone(), json!({})).unwrap(), tpl);
}

#[test]
fn render_string_comment_only_produces_empty_output() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{# only #}".to_string(), json!({}))
            .unwrap(),
        ""
    );
}

#[test]
fn render_string_strips_comments() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("a {# x #} b".to_string(), json!({}))
            .unwrap(),
        "a  b"
    );
}

#[test]
fn default_environment_renders_plain_text() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("Hello, world.".to_string(), json!({}))
            .unwrap(),
        "Hello, world."
    );
}

#[test]
fn render_string_accepts_nested_context() {
    let env = Environment::default();
    let ctx = json!({ "outer": { "inner": 7 } });
    let out = env.render_string("static".to_string(), ctx).unwrap();
    assert_eq!(out, "static");
}

#[test]
fn custom_filter_overrides_builtin_upper() {
    let mut env = Environment::default();
    env.add_filter(
        "upper",
        Arc::new(|input, _args| Ok(Value::String(format!("x{}", value_to_string(input))))),
    );
    let out = env
        .render_string("{{ x | upper }}".into(), json!({ "x": "a" }))
        .unwrap();
    assert_eq!(out, "xa");
}

#[test]
fn custom_filter_with_extra_args() {
    let mut env = Environment::default();
    env.add_filter(
        "twice",
        Arc::new(|input, args| {
            let sep = args.first().map(value_to_string).unwrap_or_default();
            let s = value_to_string(input);
            Ok(Value::String(format!("{s}{sep}{s}")))
        }),
    );
    let out = env
        .render_string(r#"{{ "a" | twice("-") }}"#.into(), json!({}))
        .unwrap();
    assert_eq!(out, "a-a");
}
