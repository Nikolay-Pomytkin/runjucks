//! Walks [`crate::ast::Node`] trees and produces output strings using an [`crate::Environment`] and JSON context.

use crate::ast::{Expr, Node};
use crate::environment::Environment;
use crate::errors::Result;
use serde_json::Value;

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
/// let s = render(&env, &ast, &json!({"x": "y"})).unwrap();
/// assert_eq!(s, "y");
/// ```
pub fn render(env: &Environment, root: &Node, ctx: &Value) -> Result<String> {
    match root {
        Node::Root(nodes) => {
            let mut out = String::new();
            for n in nodes {
                out.push_str(&render_node(env, n, ctx)?);
            }
            Ok(out)
        }
        Node::Text(s) => Ok(s.clone()),
        Node::Output(exprs) => render_output(env, exprs, ctx),
    }
}

fn render_node(env: &Environment, n: &Node, ctx: &Value) -> Result<String> {
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
    }
}

fn render_output(env: &Environment, exprs: &[Expr], ctx: &Value) -> Result<String> {
    let mut out = String::new();
    for e in exprs {
        out.push_str(&eval_expr(env, e, ctx)?);
    }
    Ok(out)
}

fn eval_expr(env: &Environment, e: &Expr, ctx: &Value) -> Result<String> {
    match e {
        Expr::Literal(v) => Ok(crate::value::value_to_string(v)),
        Expr::Variable(name) => {
            let v = lookup_variable(ctx, name)?;
            let s = crate::value::value_to_string(&v);
            if env.autoescape {
                Ok(crate::filters::escape_html(&s))
            } else {
                Ok(s)
            }
        }
    }
}

fn lookup_variable(ctx: &Value, name: &str) -> Result<Value> {
    match ctx.get(name) {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::Null),
    }
}
