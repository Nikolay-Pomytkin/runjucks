use runjucks_core::environment::Environment;
use runjucks_core::filters::{apply_builtin, escape_html};
use runjucks_core::value::is_marked_safe;
use serde_json::{json, Value};

#[test]
fn builtin_upper_lower_length() {
    let env = Environment::default();
    assert_eq!(
        apply_builtin(&env, "upper", &json!("ab"), &[]).unwrap(),
        json!("AB")
    );
    assert_eq!(
        apply_builtin(&env, "lower", &json!("AB"), &[]).unwrap(),
        json!("ab")
    );
    assert_eq!(
        apply_builtin(&env, "length", &json!("hello"), &[]).unwrap(),
        json!(5)
    );
}

#[test]
fn builtin_join_round_replace() {
    let env = Environment::default();
    assert_eq!(
        apply_builtin(&env, "join", &json!(["a", "b"]), &[json!(",")])
            .unwrap(),
        json!("a,b")
    );
    assert_eq!(
        apply_builtin(&env, "round", &json!(4.5667), &[]).unwrap(),
        json!(5)
    );
    assert_eq!(
        apply_builtin(
            &env,
            "replace",
            &json!("foofoo"),
            &[json!("foo"), json!("bar")]
        )
        .unwrap(),
        json!("barbar")
    );
}

#[test]
fn builtin_abs_capitalize() {
    let env = Environment::default();
    assert_eq!(apply_builtin(&env, "abs", &json!(-3), &[]).unwrap(), json!(3));
    assert_eq!(
        apply_builtin(&env, "capitalize", &json!("foo"), &[]).unwrap(),
        json!("Foo")
    );
}

#[test]
fn escape_html_escapes_special_chars() {
    assert_eq!(escape_html(r#"&<>"'"#), "&amp;&lt;&gt;&quot;&#39;");
}

#[test]
fn escape_html_preserves_safe_text() {
    assert_eq!(escape_html("hello world 123"), "hello world 123");
}

#[test]
fn escape_html_empty() {
    assert_eq!(escape_html(""), "");
}

#[test]
fn safe_and_escape_filter_mark_safe() {
    let env = Environment::default();
    let raw = apply_builtin(&env, "safe", &json!("<b>"), &[]).unwrap();
    assert!(is_marked_safe(&raw));
    let esc = apply_builtin(&env, "escape", &raw, &[]).unwrap();
    assert_eq!(esc, raw);
    let doubled = apply_builtin(&env, "escape", &json!("<"), &[]).unwrap();
    assert!(is_marked_safe(&doubled));
}

#[test]
fn default_undefined_and_boolean_mode() {
    let env = Environment::default();
    use runjucks_core::value::undefined_value;
    assert_eq!(
        apply_builtin(&env, "default", &undefined_value(), &[json!("foo")]).unwrap(),
        json!("foo")
    );
    assert_eq!(
        apply_builtin(
            &env,
            "default",
            &Value::Null,
            &[json!("foo")]
        )
        .unwrap(),
        Value::Null
    );
    assert_eq!(
        apply_builtin(
            &env,
            "default",
            &json!(false),
            &[json!("foo"), json!(true)]
        )
        .unwrap(),
        json!("foo")
    );
    assert_eq!(
        apply_builtin(&env, "d", &undefined_value(), &[json!("z")]).unwrap(),
        json!("z")
    );
}

#[test]
fn batch_and_join_attr() {
    let env = Environment::default();
    let out = apply_builtin(
        &env,
        "batch",
        &json!([1, 2, 3, 4, 5, 6]),
        &[json!(2)],
    )
    .unwrap();
    assert_eq!(out, json!([[1, 2], [3, 4], [5, 6]]));
    let with_fill = apply_builtin(
        &env,
        "batch",
        &json!([1, 2, 3]),
        &[json!(2), json!(0)],
    )
    .unwrap();
    assert_eq!(with_fill, json!([[1, 2], [3, 0]]));
    let joined = apply_builtin(
        &env,
        "join",
        &json!([{"x": "a"}, {"x": "b"}]),
        &[json!(","), json!("x")],
    )
    .unwrap();
    assert_eq!(joined, json!("a,b"));
}

#[test]
fn round_ceil_floor() {
    let env = Environment::default();
    assert_eq!(
        apply_builtin(&env, "round", &json!(1.5), &[json!(0), json!("floor")]).unwrap(),
        json!(1)
    );
    assert_eq!(
        apply_builtin(&env, "round", &json!(1.3), &[json!(0), json!("ceil")]).unwrap(),
        json!(2)
    );
}

#[test]
fn selectattr_rejectattr_and_sort() {
    let env = Environment::default();
    let arr = json!([{"a": 1}, {"a": 0}, {"a": 2}]);
    let sel = apply_builtin(&env, "selectattr", &arr, &[json!("a")]).unwrap();
    assert_eq!(sel, json!([{"a": 1}, {"a": 2}]));
    let rej = apply_builtin(&env, "rejectattr", &arr, &[json!("a")]).unwrap();
    assert_eq!(rej, json!([{"a": 0}]));
    let sorted = apply_builtin(&env, "sort", &arr, &[]).unwrap();
    assert_eq!(
        sorted,
        json!([{"a": 0}, {"a": 1}, {"a": 2}])
    );
}

#[test]
fn template_safe_outputs_raw_html_when_autoescape_on() {
    let env = Environment::default();
    assert!(env.autoescape);
    let out = env
        .render_string(r#"{{ "<b>" | safe }}"#.into(), json!({}))
        .unwrap();
    assert_eq!(out, "<b>");
}

#[test]
fn forceescape_is_safe_wrapped() {
    let env = Environment::default();
    let v = apply_builtin(&env, "forceescape", &json!("<"), &[]).unwrap();
    assert!(is_marked_safe(&v));
}
