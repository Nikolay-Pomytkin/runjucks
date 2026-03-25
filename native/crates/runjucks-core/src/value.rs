//! JSON [`serde_json::Value`] to display string for template output.
//!
//! Also defines internal runtime markers: Nunjucks-style **safe** strings
//! ([`RJ_SAFE`]) and **undefined** ([`RJ_UNDEFINED`]) for lookup and `default` filter parity.

use serde_json::{json, Map, Value};
use std::borrow::Cow;

/// Object key for HTML-safe output: not re-escaped when [`crate::Environment::autoescape`] is on.
pub const RJ_SAFE: &str = "__runjucks_safe";

/// Sentinel for “JavaScript `undefined`” (distinct from JSON `null`). Used when a name is not bound
/// in context or globals, and for `default` filter two-argument semantics.
pub const RJ_UNDEFINED: &str = "__runjucks_undefined";

/// `true` if `v` is a [`mark_safe`] wrapper.
pub fn is_marked_safe(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(o) if o.get(RJ_SAFE).and_then(|x| x.as_str()).is_some()
    )
}

/// `true` if `v` is the internal undefined sentinel ([`undefined_value`]).
pub fn is_undefined_value(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(o) if o.get(RJ_UNDEFINED) == Some(&Value::Bool(true))
    )
}

/// Nunjucks `undefined`-like value for unbound names.
pub fn undefined_value() -> Value {
    json!({ RJ_UNDEFINED: true })
}

fn safe_payload(v: &Value) -> Option<&str> {
    match v {
        Value::Object(o) => o.get(RJ_SAFE).and_then(|x| x.as_str()),
        _ => None,
    }
}

/// Wrap a string so autoescape does not re-encode it (Nunjucks `markSafe`).
pub fn mark_safe(s: String) -> Value {
    let mut m = Map::new();
    m.insert(RJ_SAFE.to_string(), Value::String(s));
    Value::Object(m)
}

/// Converts a JSON value to its default string form for template output.
///
/// | Variant | Output |
/// |---------|--------|
/// | [`RJ_UNDEFINED`] sentinel | Empty string |
/// | [`RJ_SAFE`] wrapper | Inner string |
/// | [`Value::Null`] | Empty string |
/// | [`Value::Bool`] | `"true"` or `"false"` |
/// | [`Value::Number`] | Default numeric string |
/// | [`Value::String`] | Cloned |
/// | [`Value::Array`] / plain [`Value::Object`] | JSON `Display` |
pub fn value_to_string(v: &Value) -> String {
    if is_undefined_value(v) {
        return String::new();
    }
    if let Some(s) = safe_payload(v) {
        return s.to_string();
    }
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

/// Raw string content for escaping (unwraps safe; undefined → empty).
pub fn value_to_string_raw(v: &Value) -> Cow<'_, str> {
    if is_undefined_value(v) {
        return Cow::Borrowed("");
    }
    if let Some(s) = safe_payload(v) {
        return Cow::Borrowed(s);
    }
    match v {
        Value::Null => Cow::Borrowed(""),
        Value::Bool(b) => Cow::Owned(b.to_string()),
        Value::Number(n) => Cow::Owned(n.to_string()),
        Value::String(s) => Cow::Borrowed(s.as_str()),
        Value::Array(_) | Value::Object(_) => Cow::Owned(v.to_string()),
    }
}
