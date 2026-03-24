//! Output filters applied during render (e.g. HTML escaping when [`crate::Environment::autoescape`] is on).

use crate::environment::Environment;
use crate::errors::{Result, RunjucksError};
use crate::value::value_to_string;
use serde_json::{json, Value};

/// Escapes a string for safe insertion into HTML text.
///
/// Escapes `&`, `<`, `>`, `"`, and `'` as entities.
///
/// # Examples
///
/// ```
/// use runjucks_core::filters::escape_html;
///
/// assert_eq!(escape_html("<a>"), "&lt;a&gt;");
/// assert_eq!(escape_html("ok"), "ok");
/// ```
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Nunjucks-style `| escape`: HTML entities plus backslash as `&#92;`.
pub fn escape_filter_value(v: &Value) -> Value {
    let s = value_to_string(v);
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\\' => out.push_str("&#92;"),
            _ => out.push(c),
        }
    }
    Value::String(out)
}

fn js_style_round(x: f64, precision: i64) -> f64 {
    if precision == 0 {
        if x.is_sign_negative() {
            -((-x) + 0.5).floor()
        } else {
            (x + 0.5).floor()
        }
    } else {
        let m = 10_f64.powi(precision as i32);
        js_style_round(x * m, 0) / m
    }
}

/// Applies a built-in filter (`name`) to `input` with extra `args` (Nunjucks-style).
pub fn apply_builtin(
    _env: &Environment,
    name: &str,
    input: &Value,
    args: &[Value],
) -> Result<Value> {
    match name {
        "upper" => Ok(Value::String(value_to_string(input).to_uppercase())),
        "lower" => Ok(Value::String(value_to_string(input).to_lowercase())),
        "length" => match input {
            Value::String(s) => Ok(json!(s.chars().count())),
            Value::Array(a) => Ok(json!(a.len())),
            Value::Object(o) => Ok(json!(o.len())),
            _ => Ok(json!(0)),
        },
        "join" => {
            let sep = args
                .first()
                .map(value_to_string)
                .unwrap_or_else(|| ",".to_string());
            let Value::Array(items) = input else {
                return Ok(Value::String(String::new()));
            };
            let mut s = String::new();
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    s.push_str(&sep);
                }
                s.push_str(&value_to_string(it));
            }
            Ok(Value::String(s))
        }
        "replace" => {
            let from = args
                .first()
                .map(value_to_string)
                .ok_or_else(|| RunjucksError::new("replace filter needs from string"))?;
            let to = args
                .get(1)
                .map(value_to_string)
                .unwrap_or_default();
            let hay = value_to_string(input);
            Ok(Value::String(hay.replace(&from, &to)))
        }
        "round" => {
            let n = input
                .as_f64()
                .or_else(|| value_to_string(input).parse().ok())
                .ok_or_else(|| RunjucksError::new("round filter expects a number"))?;
            let prec = args
                .first()
                .and_then(|a| a.as_i64().or_else(|| value_to_string(a).parse().ok()))
                .unwrap_or(0_i64);
            let r = js_style_round(n, prec);
            Ok(if r.fract() == 0.0 && r >= i64::MIN as f64 && r <= i64::MAX as f64 {
                json!(r as i64)
            } else {
                json!(r)
            })
        }
        "escape" | "e" => Ok(escape_filter_value(input)),
        "default" => {
            let d = args.first().cloned().unwrap_or(Value::Null);
            if input.is_null() {
                Ok(d)
            } else {
                Ok(input.clone())
            }
        }
        "abs" => {
            let n = input
                .as_f64()
                .or_else(|| value_to_string(input).parse().ok())
                .ok_or_else(|| RunjucksError::new("abs filter expects a number"))?;
            let a = n.abs();
            Ok(if a.fract() == 0.0 && a >= i64::MIN as f64 && a <= i64::MAX as f64 {
                json!(a as i64)
            } else {
                json!(a)
            })
        }
        "capitalize" => {
            let s = value_to_string(input);
            let mut it = s.chars();
            if let Some(c) = it.next() {
                let head: String = c.to_uppercase().collect();
                let tail = it.as_str().to_lowercase();
                Ok(Value::String(format!("{head}{tail}")))
            } else {
                Ok(Value::String(String::new()))
            }
        }
        _ => Err(RunjucksError::new(format!("unknown filter `{name}`"))),
    }
}
