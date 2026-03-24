//! Walks [`crate::ast::Node`] trees and produces output strings using an [`crate::Environment`] and JSON context.

use crate::ast::{BinOp, CompareOp, Expr, Node, UnaryOp};
use crate::environment::Environment;
use crate::errors::{Result, RunjucksError};
use serde_json::{json, Map, Value};

/// Renders `root` to a string using `env` and `ctx`.
///
/// For whole templates, `root` is typically a [`Node::Root`] from [`crate::parser::parse`].
///
/// # Errors
///
/// Propagates errors from expression evaluation if that path gains fallible operations; today lookups
/// use Nunjucks-style defaults (missing keys → empty string).
///
/// # Examples
///
/// ```
/// use runjucks_core::Environment;
/// use runjucks_core::lexer::tokenize;
/// use runjucks_core::parser::parse;
/// use runjucks_core::renderer::render;
/// use serde_json::json;
///
/// let env = Environment::default();
/// let ast = parse(&tokenize("{{ x }}").unwrap()).unwrap();
/// let mut ctx = json!({"x": "y"});
/// let s = render(&env, &ast, &mut ctx).unwrap();
/// assert_eq!(s, "y");
/// ```
pub fn render(env: &Environment, root: &Node, ctx: &mut Value) -> Result<String> {
    render_node(env, root, ctx)
}

fn render_node(env: &Environment, n: &Node, ctx: &mut Value) -> Result<String> {
    match n {
        Node::Root(nodes) => {
            let mut out = String::new();
            for child in nodes {
                out.push_str(&render_node(env, child, ctx)?);
            }
            Ok(out)
        }
        Node::Text(s) => Ok(s.clone()),
        Node::Output(exprs) => render_output(env, exprs, ctx),
        Node::If { branches } => {
            for br in branches {
                if let Some(cond) = &br.cond {
                    if !is_truthy(&eval_to_value(env, cond, ctx)?) {
                        continue;
                    }
                }
                return render_children(env, &br.body, ctx);
            }
            Ok(String::new())
        }
        Node::For {
            var,
            iter,
            body,
            else_body,
        } => {
            let v = eval_to_value(env, iter, ctx)?;
            let empty = match &v {
                Value::Array(a) => a.is_empty(),
                _ => true,
            };
            if empty {
                return if let Some(eb) = else_body {
                    render_children(env, eb, ctx)
                } else {
                    Ok(String::new())
                };
            }
            let Value::Array(items) = v else {
                return if let Some(eb) = else_body {
                    render_children(env, eb, ctx)
                } else {
                    Ok(String::new())
                };
            };
            let base = ctx_as_map(ctx)?.clone();
            let mut acc = String::new();
            for item in items {
                let mut scope = base.clone();
                scope.insert(var.clone(), item);
                let mut child = Value::Object(scope);
                acc.push_str(&render_children(env, body, &mut child)?);
            }
            Ok(acc)
        }
        Node::Set { name, value } => {
            let v = eval_to_value(env, value, ctx)?;
            ctx_as_map_mut(ctx)?.insert(name.clone(), v);
            Ok(String::new())
        }
    }
}

fn render_children(env: &Environment, nodes: &[Node], ctx: &mut Value) -> Result<String> {
    let mut out = String::new();
    for child in nodes {
        out.push_str(&render_node(env, child, ctx)?);
    }
    Ok(out)
}

fn ctx_as_map(ctx: &Value) -> Result<&Map<String, Value>> {
    match ctx {
        Value::Object(m) => Ok(m),
        _ => Err(RunjucksError::new(
            "template context must be a JSON object for `for` / `set`",
        )),
    }
}

fn ctx_as_map_mut(ctx: &mut Value) -> Result<&mut Map<String, Value>> {
    match ctx {
        Value::Object(m) => Ok(m),
        _ => Err(RunjucksError::new(
            "template context must be a JSON object for `for` / `set`",
        )),
    }
}

fn render_output(env: &Environment, exprs: &[Expr], ctx: &mut Value) -> Result<String> {
    let mut out = String::new();
    for e in exprs {
        out.push_str(&eval_for_output(env, e, ctx)?);
    }
    Ok(out)
}

/// Template output for `{{ expr }}`: literals are not auto-escaped; other values respect [`Environment::autoescape`].
fn eval_for_output(env: &Environment, e: &Expr, ctx: &Value) -> Result<String> {
    match e {
        Expr::Literal(v) => Ok(crate::value::value_to_string(v)),
        _ => {
            let v = eval_to_value(env, e, ctx)?;
            let s = crate::value::value_to_string(&v);
            if env.autoescape {
                Ok(crate::filters::escape_html(&s))
            } else {
                Ok(s)
            }
        }
    }
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::Number(n) => n
            .as_f64()
            .map(|x| x != 0.0 && !x.is_nan())
            .unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn lookup_variable(ctx: &Value, name: &str) -> Result<Value> {
    match ctx.get(name) {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::Null),
    }
}

fn compare_values(left: &Value, op: CompareOp, right: &Value) -> bool {
    match op {
        CompareOp::Eq | CompareOp::StrictEq => left == right,
        CompareOp::Ne | CompareOp::StrictNe => left != right,
        CompareOp::Lt => json_partial_cmp(left, right) == Some(std::cmp::Ordering::Less),
        CompareOp::Gt => json_partial_cmp(left, right) == Some(std::cmp::Ordering::Greater),
        CompareOp::Le => matches!(
            json_partial_cmp(left, right),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        CompareOp::Ge => matches!(
            json_partial_cmp(left, right),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
    }
}

fn json_partial_cmp(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let xf = x.as_f64()?;
            let yf = y.as_f64()?;
            xf.partial_cmp(&yf)
        }
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

fn eval_in(key: &Value, container: &Value) -> Result<bool> {
    match container {
        Value::Array(a) => Ok(a.iter().any(|v| v == key)),
        Value::String(s) => {
            let frag = match key {
                Value::String(k) => k.as_str(),
                _ => return Ok(false),
            };
            Ok(s.contains(frag))
        }
        Value::Object(o) => match key {
            Value::String(k) => Ok(o.contains_key(k)),
            _ => Ok(false),
        },
        _ => Err(RunjucksError::new(
            "Cannot use \"in\" operator to search in unexpected type",
        )),
    }
}

/// Right-hand side of `is`: identifier, string/null literal, or call (`equalto(3)`).
fn is_test_parts(e: &Expr) -> Option<(&str, &[Expr])> {
    match e {
        Expr::Variable(n) => Some((n.as_str(), &[])),
        Expr::Literal(Value::String(s)) => Some((s.as_str(), &[])),
        Expr::Literal(Value::Null) => Some(("null", &[])),
        Expr::Call { callee, args } => {
            if let Expr::Variable(n) = callee.as_ref() {
                Some((n.as_str(), args.as_slice()))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn eval_is_test(
    env: &Environment,
    test_name: &str,
    value: &Value,
    arg_exprs: &[Expr],
    ctx: &Value,
) -> Result<bool> {
    let arg_vals: Vec<Value> = arg_exprs
        .iter()
        .map(|e| eval_to_value(env, e, ctx))
        .collect::<Result<_>>()?;
    Ok(match test_name {
        "equalto" => arg_vals.first().map(|a| a == value).unwrap_or(false),
        "sameas" => match arg_vals.first() {
            Some(a) => match (value, a) {
                (Value::Object(_), Value::Object(_)) | (Value::Array(_), Value::Array(_)) => false,
                _ => a == value,
            },
            None => false,
        },
        "null" | "none" => value.is_null(),
        "falsy" => !is_truthy(value),
        "truthy" => is_truthy(value),
        "number" => value.is_number(),
        "string" => value.is_string(),
        "lower" => match value {
            Value::String(s) => s.chars().all(|c| !c.is_uppercase()),
            _ => false,
        },
        "upper" => match value {
            Value::String(s) => s.chars().all(|c| !c.is_lowercase()),
            _ => false,
        },
        "callable" => false,
        "defined" => !value.is_null(),
        _ => false,
    })
}

fn as_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn add_like_js(a: &Value, b: &Value) -> Value {
    if let (Some(x), Some(y)) = (as_number(a), as_number(b)) {
        json_num(x + y)
    } else {
        Value::String(format!(
            "{}{}",
            crate::value::value_to_string(a),
            crate::value::value_to_string(b)
        ))
    }
}

fn json_num(x: f64) -> Value {
    if x.fract() == 0.0 && x >= i64::MIN as f64 && x <= i64::MAX as f64 {
        json!(x as i64)
    } else {
        json!(x)
    }
}

fn eval_to_value(env: &Environment, e: &Expr, ctx: &Value) -> Result<Value> {
    match e {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Variable(name) => lookup_variable(ctx, name),
        Expr::Unary { op, expr } => {
            let v = eval_to_value(env, expr, ctx)?;
            Ok(match op {
                UnaryOp::Not => Value::Bool(!is_truthy(&v)),
                UnaryOp::Neg => {
                    let n = as_number(&v).ok_or_else(|| {
                        RunjucksError::new("unary '-' expects a numeric value")
                    })?;
                    json_num(-n)
                }
                UnaryOp::Pos => v,
            })
        }
        Expr::Binary { op, left, right } => match op {
            BinOp::Add => Ok(add_like_js(
                &eval_to_value(env, left, ctx)?,
                &eval_to_value(env, right, ctx)?,
            )),
            BinOp::Concat => Ok(Value::String(format!(
                "{}{}",
                eval_for_output(env, left, ctx)?,
                eval_for_output(env, right, ctx)?
            ))),
            BinOp::Sub => {
                let a = eval_to_value(env, left, ctx)?;
                let b = eval_to_value(env, right, ctx)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`-` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`-` expects numbers"))?;
                Ok(json_num(x - y))
            }
            BinOp::Mul => {
                let a = eval_to_value(env, left, ctx)?;
                let b = eval_to_value(env, right, ctx)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`*` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`*` expects numbers"))?;
                Ok(json_num(x * y))
            }
            BinOp::Div => {
                let a = eval_to_value(env, left, ctx)?;
                let b = eval_to_value(env, right, ctx)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`/` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`/` expects numbers"))?;
                Ok(json!(x / y))
            }
            BinOp::FloorDiv => {
                let a = eval_to_value(env, left, ctx)?;
                let b = eval_to_value(env, right, ctx)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`//` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`//` expects numbers"))?;
                if y == 0.0 {
                    return Err(RunjucksError::new("division by zero"));
                }
                Ok(json_num((x / y).floor()))
            }
            BinOp::Mod => {
                let a = eval_to_value(env, left, ctx)?;
                let b = eval_to_value(env, right, ctx)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`%` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`%` expects numbers"))?;
                Ok(json_num(x % y))
            }
            BinOp::Pow => {
                let a = eval_to_value(env, left, ctx)?;
                let b = eval_to_value(env, right, ctx)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`**` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`**` expects numbers"))?;
                Ok(json!(x.powf(y)))
            }
            BinOp::And => {
                let l = eval_to_value(env, left, ctx)?;
                if !is_truthy(&l) {
                    return Ok(l);
                }
                eval_to_value(env, right, ctx)
            }
            BinOp::Or => {
                let l = eval_to_value(env, left, ctx)?;
                if is_truthy(&l) {
                    return Ok(l);
                }
                eval_to_value(env, right, ctx)
            }
            BinOp::In => {
                let key = eval_to_value(env, left, ctx)?;
                let container = eval_to_value(env, right, ctx)?;
                Ok(Value::Bool(eval_in(&key, &container)?))
            }
            BinOp::Is => {
                let (test_name, arg_exprs) = is_test_parts(right).ok_or_else(|| {
                    RunjucksError::new("`is` test must be an identifier, call, string, or null")
                })?;
                if test_name == "defined" {
                    if let Expr::Variable(n) = &**left {
                        return Ok(Value::Bool(ctx.get(n).is_some()));
                    }
                }
                let v = eval_to_value(env, left, ctx)?;
                Ok(Value::Bool(eval_is_test(
                    env, test_name, &v, arg_exprs, ctx,
                )?))
            }
        },
        Expr::Compare { head, rest } => {
            let mut acc = eval_to_value(env, head, ctx)?;
            for (op, rhs_e) in rest.iter() {
                let r = eval_to_value(env, rhs_e, ctx)?;
                let ok = compare_values(&acc, *op, &r);
                acc = Value::Bool(ok);
            }
            Ok(acc)
        }
        Expr::InlineIf {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = eval_to_value(env, cond, ctx)?;
            if is_truthy(&c) {
                eval_to_value(env, then_expr, ctx)
            } else if let Some(els) = else_expr {
                eval_to_value(env, els, ctx)
            } else {
                Ok(Value::Null)
            }
        }
        Expr::GetAttr { base, attr } => {
            let b = eval_to_value(env, base, ctx)?;
            match b {
                Value::Object(o) => Ok(o.get(attr).cloned().unwrap_or(Value::Null)),
                _ => Ok(Value::Null),
            }
        }
        Expr::GetItem { base, index } => {
            let b = eval_to_value(env, base, ctx)?;
            let i = eval_to_value(env, index, ctx)?;
            match (&b, &i) {
                (Value::Array(a), Value::Number(n)) => {
                    let idx = n
                        .as_u64()
                        .or_else(|| n.as_f64().map(|x| x as u64))
                        .unwrap_or(0) as usize;
                    Ok(a.get(idx).cloned().unwrap_or(Value::Null))
                }
                (Value::Object(o), Value::String(k)) => Ok(o.get(k).cloned().unwrap_or(Value::Null)),
                _ => Ok(Value::Null),
            }
        }
        Expr::Call { .. } => Err(RunjucksError::new(
            "function calls are not supported in expressions yet",
        )),
        Expr::Filter { name, input, args } => {
            let input_v = eval_to_value(env, input, ctx)?;
            let arg_vals: Vec<Value> = args
                .iter()
                .map(|a| eval_to_value(env, a, ctx))
                .collect::<Result<_>>()?;
            crate::filters::apply_builtin(env, name, &input_v, &arg_vals)
        }
        Expr::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(eval_to_value(env, it, ctx)?);
            }
            Ok(Value::Array(out))
        }
        Expr::Dict(pairs) => {
            use serde_json::Map;
            let mut m = Map::new();
            for (k, v) in pairs {
                let key_v = eval_to_value(env, k, ctx)?;
                let key = match key_v {
                    Value::String(s) => s,
                    _ => crate::value::value_to_string(&key_v),
                };
                m.insert(key, eval_to_value(env, v, ctx)?);
            }
            Ok(Value::Object(m))
        }
    }
}
