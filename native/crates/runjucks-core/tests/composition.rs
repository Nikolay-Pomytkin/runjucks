//! Template composition: `include`, `extends` / `block`, macros.

use runjucks_core::loader::map_loader;
use runjucks_core::Environment;
use serde_json::json;
use std::collections::HashMap;

fn env_with_map(templates: HashMap<String, String>) -> Environment {
    let mut env = Environment::default();
    env.loader = Some(map_loader(templates));
    env
}

#[test]
fn include_renders_subtemplate() {
    let mut m = HashMap::new();
    m.insert("part.html".into(), "Hello {{ name }}".into());
    m.insert(
        "main.html".into(),
        r#"{% include "part.html" %}!"#.into(),
    );
    let env = env_with_map(m);
    let out = env
        .render_template("main.html", json!({ "name": "Ada" }))
        .unwrap();
    assert_eq!(out, "Hello Ada!");
}

#[test]
fn include_cycle_errors() {
    let mut m = HashMap::new();
    m.insert("a.html".into(), r#"{% include "b.html" %}"#.into());
    m.insert("b.html".into(), r#"{% include "a.html" %}"#.into());
    let env = env_with_map(m);
    let err = env.render_template("a.html", json!({})).unwrap_err();
    assert!(err.to_string().contains("circular"));
}

#[test]
fn extends_block_override() {
    let mut m = HashMap::new();
    m.insert(
        "base.html".into(),
        r#"<!doctype><title>{% block title %}T{% endblock %}</title><body>{% block body %}{% endblock %}</body>"#
            .into(),
    );
    m.insert(
        "child.html".into(),
        r#"{% extends "base.html" %}{% block title %}Hi{% endblock %}{% block body %}B{% endblock %}"#
            .into(),
    );
    let env = env_with_map(m);
    let out = env.render_template("child.html", json!({})).unwrap();
    assert_eq!(
        out,
        "<!doctype><title>Hi</title><body>B</body>"
    );
}

#[test]
fn macro_call_in_output() {
    let env = Environment::default();
    let out = env
        .render_string(
            r#"{% macro greet(n) %}Hello {{ n }}{% endmacro %}{{ greet("x") }}"#.into(),
            json!({}),
        )
        .unwrap();
    assert_eq!(out, "Hello x");
}

#[test]
fn render_string_with_loader_include() {
    let mut env = Environment::default();
    let mut m = HashMap::new();
    m.insert("x.html".into(), "ok".into());
    env.loader = Some(map_loader(m));
    let out = env
        .render_string(r#"{% include "x.html" %}!"#.into(), json!({}))
        .unwrap();
    assert_eq!(out, "ok!");
}

#[test]
fn no_loader_include_errors() {
    let env = Environment::default();
    let err = env
        .render_string(r#"{% include "x.html" %}"#.into(), json!({}))
        .unwrap_err();
    assert!(err.to_string().contains("include"));
}
