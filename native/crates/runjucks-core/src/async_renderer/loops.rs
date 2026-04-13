//! Async loop and switch rendering.

use crate::ast::{Expr, ForVars, Node, SwitchCase};
use crate::environment::Environment;
use crate::errors::Result;
use crate::render_common::{
    iterable_empty, iterable_from_value, resolve_plain_value_or_attr_chain_ref, Iterable,
};
use crate::renderer::{CtxStack, RenderState};
use serde_json::Value;
use std::borrow::Cow;

use super::eval::eval_to_value_async;
use super::nodes::render_children_into_async;

fn inject_loop(stack: &mut CtxStack, i: usize, len: usize) {
    crate::render_common::inject_loop(&mut stack.frames, i, len);
    stack.bump_revision();
}

pub(super) async fn render_for_async(
    env: &Environment,
    state: &mut RenderState<'_>,
    vars: &ForVars,
    iter_expr: &Expr,
    body: &[Node],
    else_body: Option<&[Node]>,
    stack: &mut CtxStack,
) -> Result<String> {
    let mut out = String::new();
    render_for_async_into(
        env, state, vars, iter_expr, body, else_body, stack, &mut out,
    )
    .await?;
    Ok(out)
}

pub(super) async fn render_for_async_into(
    env: &Environment,
    state: &mut RenderState<'_>,
    vars: &ForVars,
    iter_expr: &Expr,
    body: &[Node],
    else_body: Option<&[Node]>,
    stack: &mut CtxStack,
    out: &mut String,
) -> Result<()> {
    let v = eval_to_value_async(env, state, iter_expr, stack).await?;
    let it = iterable_from_value(v);
    if iterable_empty(&it) {
        return if let Some(eb) = else_body {
            render_children_into_async(env, state, eb, stack, out).await
        } else {
            Ok(())
        };
    }

    stack.push_frame();

    match (vars, it) {
        (ForVars::Single(x), Iterable::Rows(items)) => {
            let len = items.len();
            out.reserve(len.saturating_mul(16));
            for (i, item) in items.into_iter().enumerate() {
                inject_loop(stack, i, len);
                stack.set_local(x, item);
                render_children_into_async(env, state, body, stack, out).await?;
            }
        }
        (ForVars::Multi(names), Iterable::Rows(rows)) if names.len() >= 2 => {
            let len = rows.len();
            out.reserve(len.saturating_mul(16));
            for (i, row) in rows.into_iter().enumerate() {
                inject_loop(stack, i, len);
                if let Value::Array(cols) = row {
                    for (u, name) in names.iter().enumerate() {
                        let cell = cols.get(u).cloned().unwrap_or(Value::Null);
                        stack.set_local(name, cell);
                    }
                } else {
                    for name in names {
                        stack.set_local(name, Value::Null);
                    }
                }
                render_children_into_async(env, state, body, stack, out).await?;
            }
        }
        (ForVars::Multi(names), Iterable::Pairs(pairs)) if names.len() == 2 => {
            let len = pairs.len();
            out.reserve(len.saturating_mul(16));
            for (i, (k, v)) in pairs.into_iter().enumerate() {
                inject_loop(stack, i, len);
                stack.set_local(&names[0], Value::String(k));
                stack.set_local(&names[1], v);
                render_children_into_async(env, state, body, stack, out).await?;
            }
        }
        (ForVars::Single(_), _) | (ForVars::Multi(_), _) => {
            stack.pop_frame();
            return if let Some(eb) = else_body {
                render_children_into_async(env, state, eb, stack, out).await
            } else {
                Ok(())
            };
        }
    }

    stack.pop_frame();
    Ok(())
}

pub(super) async fn render_switch_async_into(
    env: &Environment,
    state: &mut RenderState<'_>,
    disc_expr: &Expr,
    cases: &[SwitchCase],
    default_body: Option<&[Node]>,
    stack: &mut CtxStack,
    out: &mut String,
) -> Result<()> {
    let start = {
        let skip_root = |root_name: &str| {
            state.macro_namespaces.contains_key(root_name)
                || state.macro_namespace_values.contains_key(root_name)
        };
        let mut disc = if let Some(v) =
            resolve_plain_value_or_attr_chain_ref(env, stack, disc_expr, skip_root)?
        {
            v
        } else {
            Cow::Owned(eval_to_value_async(env, state, disc_expr, stack).await?)
        };
        let mut start = None;
        for (i, c) in cases.iter().enumerate() {
            let case_val = match &c.cond {
                Expr::Literal(v) => Cow::Borrowed(v),
                _ => {
                    let skip_root = |root_name: &str| {
                        state.macro_namespaces.contains_key(root_name)
                            || state.macro_namespace_values.contains_key(root_name)
                    };
                    if let Some(v) =
                        resolve_plain_value_or_attr_chain_ref(env, stack, &c.cond, skip_root)?
                    {
                        v
                    } else {
                        disc = Cow::Owned(disc.into_owned());
                        Cow::Owned(eval_to_value_async(env, state, &c.cond, stack).await?)
                    }
                }
            };
            if case_val.as_ref() == disc.as_ref() {
                start = Some(i);
                break;
            }
        }
        start
    };
    if let Some(mut idx) = start {
        loop {
            let body = &cases[idx].body;
            render_children_into_async(env, state, body, stack, out).await?;
            if !body.is_empty() {
                return Ok(());
            }
            idx += 1;
            if idx >= cases.len() {
                break;
            }
        }
    }
    if let Some(db) = default_body {
        render_children_into_async(env, state, db, stack, out).await?;
    }
    Ok(())
}

/// `{% asyncEach %}` — sequential async iteration (same as `for` in async mode).
pub(super) async fn render_async_each(
    env: &Environment,
    state: &mut RenderState<'_>,
    vars: &ForVars,
    iter_expr: &Expr,
    body: &[Node],
    else_body: Option<&[Node]>,
    stack: &mut CtxStack,
) -> Result<String> {
    render_for_async(env, state, vars, iter_expr, body, else_body, stack).await
}

/// `{% asyncAll %}` — in Nunjucks this signals parallel intent, but since we hold `&mut RenderState`
/// we execute sequentially (same observable result for deterministic templates).
pub(super) async fn render_async_all(
    env: &Environment,
    state: &mut RenderState<'_>,
    vars: &ForVars,
    iter_expr: &Expr,
    body: &[Node],
    else_body: Option<&[Node]>,
    stack: &mut CtxStack,
) -> Result<String> {
    render_for_async(env, state, vars, iter_expr, body, else_body, stack).await
}
