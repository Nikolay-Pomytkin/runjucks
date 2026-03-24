//! JSON [`serde_json::Value`] to display string for template output.

use serde_json::Value;

/// Converts a JSON value to its default string form for template output.
///
/// # Rules
///
/// | Variant | Output |
/// |---------|--------|
/// | [`Value::Null`] | Empty string |
/// | [`Value::Bool`] | `"true"` or `"false"` |
/// | [`Value::Number`] | [`serde_json::Number`]’s default string (integer or float) |
/// | [`Value::String`] | Cloned inner string |
/// | [`Value::Array`] / [`Value::Object`] | JSON text via the value’s `Display` implementation (same idea as `serde_json::to_string`) |
///
/// # Examples
///
/// ```
/// use runjucks_core::value::value_to_string;
/// use serde_json::json;
///
/// assert_eq!(value_to_string(&json!(null)), "");
/// assert_eq!(value_to_string(&json!(true)), "true");
/// assert_eq!(value_to_string(&json!("hi")), "hi");
/// ```
pub fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}
