//! `is defined` / `is callable` parity with Nunjucks for objects, arrays, and `{% import %}` namespaces.

use runjucks_core::loader::map_loader;
use runjucks_core::Environment;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn is_defined_missing_object_key_is_false() {
    let env = Environment::default();
    let out = env
        .render_string(r#"{{ o.a is defined }}"#.into(), json!({ "o": {} }))
        .unwrap();
    assert_eq!(out, "false");
}

#[test]
fn is_defined_null_object_property_is_true() {
    let env = Environment::default();
    let out = env
        .render_string(
            r#"{{ o.a is defined }}"#.into(),
            json!({ "o": { "a": null } }),
        )
        .unwrap();
    assert_eq!(out, "true");
}

#[test]
fn is_defined_array_oob_is_false() {
    let env = Environment::default();
    let out = env
        .render_string(
            r#"{{ items[5] is defined }}"#.into(),
            json!({ "items": [1, 2] }),
        )
        .unwrap();
    assert_eq!(out, "false");
}

#[test]
fn import_namespace_macro_is_callable_and_defined() {
    let mut m = HashMap::new();
    m.insert(
        "lib.html".into(),
        r#"{% macro m() %}x{% endmacro %}"#.into(),
    );
    m.insert(
        "main.html".into(),
        r#"{% import "lib.html" as lib %}{{ lib.m is callable }} {{ lib.m is defined }}"#.into(),
    );
    let mut env = Environment::default();
    env.loader = Some(map_loader(m));
    let out = env.render_template("main.html", json!({})).unwrap();
    assert_eq!(out, "true true");
}

#[test]
fn import_namespace_missing_export_is_not_defined() {
    let mut m = HashMap::new();
    m.insert("lib.html".into(), r#"{% set v = 1 %}"#.into());
    m.insert(
        "main.html".into(),
        r#"{% import "lib.html" as lib %}{{ lib.nope is defined }}"#.into(),
    );
    let mut env = Environment::default();
    env.loader = Some(map_loader(m));
    let out = env.render_template("main.html", json!({})).unwrap();
    assert_eq!(out, "false");
}
