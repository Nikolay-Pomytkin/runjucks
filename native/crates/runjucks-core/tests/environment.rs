use runjucks_core::Environment;
use serde_json::json;

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
