use runjucks_core::environment::Environment;
use runjucks_core::filters::{apply_builtin, escape_html};
use serde_json::json;

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
