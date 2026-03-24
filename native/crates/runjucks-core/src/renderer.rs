//! Walks [`crate::ast::Node`] trees and produces output strings using an [`crate::Environment`] and JSON context.

use crate::ast::{BinOp, CompareOp, Expr, ForVars, MacroDef, Node, SwitchCase, UnaryOp};
use crate::environment::Environment;
use crate::errors::{Result, RunjucksError};
use crate::loader::TemplateLoader;
use crate::{lexer, parser};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// Nunjucks-style frame stack: inner frames shadow outer; `set` updates the innermost existing binding.
#[derive(Debug, Clone)]
pub struct CtxStack {
    frames: Vec<Map<String, Value>>,
}

impl CtxStack {
    pub fn from_root(root: Map<String, Value>) -> Self {
        Self {
            frames: vec![root],
        }
    }

    pub fn push_frame(&mut self) {
        self.frames.push(Map::new());
    }

    pub fn pop_frame(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    pub fn get(&self, name: &str) -> Value {
        for f in self.frames.iter().rev() {
            if let Some(v) = f.get(name) {
                return v.clone();
            }
        }
        Value::Null
    }

    pub fn defined(&self, name: &str) -> bool {
        self.frames.iter().rev().any(|f| f.contains_key(name))
    }

    pub fn set(&mut self, name: &str, value: Value) {
        for f in self.frames.iter_mut().rev() {
            if f.contains_key(name) {
                f.insert(name.to_string(), value);
                return;
            }
        }
        if let Some(inner) = self.frames.last_mut() {
            inner.insert(name.to_string(), value);
        }
    }

    /// Assign in the innermost frame only (for `for` / `loop.*` bindings so inner loops can shadow).
    pub fn set_local(&mut self, name: &str, value: Value) {
        if let Some(inner) = self.frames.last_mut() {
            inner.insert(name.to_string(), value);
        }
    }

    /// Outer frames first, then inner overwrites — snapshot for macro bodies.
    pub fn flatten(&self) -> Map<String, Value> {
        let mut m = Map::new();
        for f in &self.frames {
            for (k, v) in f {
                m.insert(k.clone(), v.clone());
            }
        }
        m
    }
}

/// Per-render state: optional loader, include cycle stack, macro scopes, and block overrides for `extends`.
pub struct RenderState<'a> {
    pub loader: Option<&'a (dyn TemplateLoader + Send + Sync)>,
    pub stack: Vec<String>,
    pub macro_scopes: Vec<HashMap<String, MacroDef>>,
    pub block_overrides: Option<HashMap<String, Vec<Node>>>,
}

impl<'a> RenderState<'a> {
    pub fn new(loader: Option<&'a (dyn TemplateLoader + Send + Sync)>) -> Self {
        Self {
            loader,
            stack: Vec::new(),
            macro_scopes: Vec::new(),
            block_overrides: None,
        }
    }

    pub fn push_template(&mut self, name: &str) -> Result<()> {
        if self.stack.iter().any(|s| s == name) {
            return Err(RunjucksError::new(format!(
                "circular template reference: {name}"
            )));
        }
        self.stack.push(name.to_string());
        Ok(())
    }

    pub fn pop_template(&mut self) {
        self.stack.pop();
    }

    pub fn push_macros(&mut self, defs: HashMap<String, MacroDef>) {
        self.macro_scopes.push(defs);
    }

    pub fn pop_macros(&mut self) {
        self.macro_scopes.pop();
    }

    pub fn lookup_macro(&self, name: &str) -> Option<&MacroDef> {
        for scope in self.macro_scopes.iter().rev() {
            if let Some(m) = scope.get(name) {
                return Some(m);
            }
        }
        None
    }
}

/// Renders `root` to a string using `env` and `ctx_stack`.
pub fn render(
    env: &Environment,
    loader: Option<&(dyn TemplateLoader + Send + Sync)>,
    root: &Node,
    ctx_stack: &mut CtxStack,
) -> Result<String> {
    let mut state = RenderState::new(loader);
    render_entry(env, &mut state, root, ctx_stack)
}

/// Entry: handle `{% extends %}` child templates, otherwise normal render.
pub fn render_entry(
    env: &Environment,
    state: &mut RenderState<'_>,
    root: &Node,
    ctx_stack: &mut CtxStack,
) -> Result<String> {
    if let Some((parent, blocks)) = extract_layout_if_any(root)? {
        render_extends(env, state, &parent, blocks, ctx_stack)
    } else {
        render_with_state(env, state, root, ctx_stack)
    }
}

fn extract_layout_if_any(root: &Node) -> Result<Option<(String, HashMap<String, Vec<Node>>)>> {
    let Node::Root(children) = root else {
        return Ok(None);
    };
    let mut idx = 0usize;
    while idx < children.len() {
        match &children[idx] {
            Node::Text(s) if s.trim().is_empty() => idx += 1,
            Node::Extends { parent } => {
                let parent = parent.clone();
                let mut blocks = HashMap::new();
                for n in children.iter().skip(idx + 1) {
                    match n {
                        Node::Block { name, body } => {
                            blocks.insert(name.clone(), body.clone());
                        }
                        Node::Text(s) if s.chars().all(|c| c.is_whitespace()) => {}
                        Node::MacroDef(_) => {}
                        _ => {
                            return Err(RunjucksError::new(
                                "invalid content in template with `extends` (only `block` allowed)",
                            ));
                        }
                    }
                }
                return Ok(Some((parent, blocks)));
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
}

fn render_extends(
    env: &Environment,
    state: &mut RenderState<'_>,
    parent_name: &str,
    blocks: HashMap<String, Vec<Node>>,
    ctx_stack: &mut CtxStack,
) -> Result<String> {
    let loader = state
        .loader
        .ok_or_else(|| RunjucksError::new("`extends` requires a template loader"))?;
    let src = loader.load(parent_name)?;
    let tokens = lexer::tokenize(&src)?;
    let parent_ast = parser::parse(&tokens)?;
    state.push_template(parent_name)?;
    let prev = state.block_overrides.replace(blocks);
    let out = render_with_state(env, state, &parent_ast, ctx_stack)?;
    state.block_overrides = prev;
    state.pop_template();
    Ok(out)
}

fn render_with_state(
    env: &Environment,
    state: &mut RenderState<'_>,
    root: &Node,
    ctx_stack: &mut CtxStack,
) -> Result<String> {
    render_node(env, state, root, ctx_stack)
}

fn render_node(
    env: &Environment,
    state: &mut RenderState<'_>,
    n: &Node,
    stack: &mut CtxStack,
) -> Result<String> {
    match n {
        Node::Root(nodes) => {
            let mut defs = HashMap::new();
            for n in nodes.iter() {
                if let Node::MacroDef(m) = n {
                    defs.insert(m.name.clone(), m.clone());
                }
            }
            let had_macros = !defs.is_empty();
            if had_macros {
                state.push_macros(defs);
            }
            let mut out = String::new();
            for child in nodes.iter() {
                if matches!(child, Node::MacroDef(_)) {
                    continue;
                }
                out.push_str(&render_node(env, state, child, stack)?);
            }
            if had_macros {
                state.pop_macros();
            }
            Ok(out)
        }
        Node::Text(s) => Ok(s.clone()),
        Node::Output(exprs) => render_output(env, state, exprs, stack),
        Node::If { branches } => {
            for br in branches {
                if let Some(cond) = &br.cond {
                    if !is_truthy(&eval_to_value(env, state, cond, stack)?) {
                        continue;
                    }
                }
                return render_children(env, state, &br.body, stack);
            }
            Ok(String::new())
        }
        Node::Switch {
            expr,
            cases,
            default_body,
        } => render_switch(env, state, expr, cases, default_body.as_deref(), stack),
        Node::For {
            vars,
            iter,
            body,
            else_body,
        } => render_for(env, state, vars, iter, body, else_body.as_deref(), stack),
        Node::Set {
            targets,
            value,
            body,
        } => {
            if let Some(expr) = value {
                let v = eval_to_value(env, state, expr, stack)?;
                for t in targets {
                    stack.set(t, v.clone());
                }
            } else if let Some(nodes) = body {
                let s = render_children(env, state, nodes, stack)?;
                if let Some(t) = targets.first() {
                    stack.set(t, Value::String(s));
                }
            }
            Ok(String::new())
        }
        Node::Include {
            template,
            ignore_missing,
        } => {
            let loader = state.loader.ok_or_else(|| {
                RunjucksError::new("`include` requires a template loader")
            })?;
            let name = crate::value::value_to_string(&eval_to_value(env, state, template, stack)?);
            let src = match loader.load(&name) {
                Ok(s) => s,
                Err(_) if *ignore_missing => return Ok(String::new()),
                Err(e) => return Err(e),
            };
            let tokens = lexer::tokenize(&src)?;
            let included = parser::parse(&tokens)?;
            state.push_template(&name)?;
            let out = render_entry(env, state, &included, stack)?;
            state.pop_template();
            Ok(out)
        }
        Node::Extends { .. } => Err(RunjucksError::new(
            "`extends` is only valid at the top level of a loaded template",
        )),
        Node::Block { name, body } => {
            let to_render: Vec<Node> = if let Some(ref ov) = state.block_overrides {
                ov.get(name).cloned().unwrap_or_else(|| body.clone())
            } else {
                body.clone()
            };
            render_children(env, state, &to_render, stack)
        }
        Node::MacroDef(_) => Ok(String::new()),
    }
}

fn render_switch(
    env: &Environment,
    state: &mut RenderState<'_>,
    disc_expr: &Expr,
    cases: &[SwitchCase],
    default_body: Option<&[Node]>,
    stack: &mut CtxStack,
) -> Result<String> {
    let disc = eval_to_value(env, state, disc_expr, stack)?;
    let mut start = None;
    for (i, c) in cases.iter().enumerate() {
        if eval_to_value(env, state, &c.cond, stack)? == disc {
            start = Some(i);
            break;
        }
    }
    let mut acc = String::new();
    if let Some(mut idx) = start {
        loop {
            let body = &cases[idx].body;
            acc.push_str(&render_children(env, state, body, stack)?);
            if !body.is_empty() {
                return Ok(acc);
            }
            idx += 1;
            if idx >= cases.len() {
                break;
            }
        }
    }
    if let Some(db) = default_body {
        acc.push_str(&render_children(env, state, db, stack)?);
    }
    Ok(acc)
}

enum Iterable {
    Rows(Vec<Value>),
    Pairs(Vec<(String, Value)>),
}

fn iterable_from_value(v: Value) -> Iterable {
    match v {
        Value::Null => Iterable::Rows(vec![]),
        Value::Array(a) => Iterable::Rows(a),
        Value::Object(o) => {
            let mut keys: Vec<String> = o.keys().cloned().collect();
            keys.sort();
            let pairs: Vec<(String, Value)> = keys
                .into_iter()
                .map(|k| {
                    let val = o.get(&k).cloned().unwrap_or(Value::Null);
                    (k, val)
                })
                .collect();
            Iterable::Pairs(pairs)
        }
        _ => Iterable::Rows(vec![]),
    }
}

fn iterable_empty(it: &Iterable) -> bool {
    match it {
        Iterable::Rows(a) => a.is_empty(),
        Iterable::Pairs(p) => p.is_empty(),
    }
}

fn inject_loop(stack: &mut CtxStack, i: usize, len: usize) {
    let m = json!({
        "index": i + 1,
        "index0": i,
        "first": i == 0,
        "last": len > 0 && i + 1 == len,
        "length": len,
        "revindex": len.saturating_sub(i),
        "revindex0": len.saturating_sub(1).saturating_sub(i),
    });
    stack.set_local("loop", m);
}

fn render_for(
    env: &Environment,
    state: &mut RenderState<'_>,
    vars: &ForVars,
    iter_expr: &Expr,
    body: &[Node],
    else_body: Option<&[Node]>,
    stack: &mut CtxStack,
) -> Result<String> {
    let v = eval_to_value(env, state, iter_expr, stack)?;
    let it = iterable_from_value(v);
    if iterable_empty(&it) {
        return if let Some(eb) = else_body {
            render_children(env, state, eb, stack)
        } else {
            Ok(String::new())
        };
    }

    stack.push_frame();
    let mut acc = String::new();

    match (vars, it) {
        (ForVars::Single(x), Iterable::Rows(items)) => {
            let len = items.len();
            for (i, item) in items.into_iter().enumerate() {
                inject_loop(stack, i, len);
                stack.set_local(x, item);
                acc.push_str(&render_children(env, state, body, stack)?);
            }
        }
        (ForVars::Multi(names), Iterable::Rows(rows)) if names.len() >= 2 => {
            let len = rows.len();
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
                acc.push_str(&render_children(env, state, body, stack)?);
            }
        }
        (ForVars::Multi(names), Iterable::Pairs(pairs)) if names.len() == 2 => {
            let len = pairs.len();
            for (i, (k, v)) in pairs.into_iter().enumerate() {
                inject_loop(stack, i, len);
                stack.set_local(&names[0], Value::String(k));
                stack.set_local(&names[1], v);
                acc.push_str(&render_children(env, state, body, stack)?);
            }
        }
        (ForVars::Single(_), _) | (ForVars::Multi(_), _) => {
            stack.pop_frame();
            return if let Some(eb) = else_body {
                render_children(env, state, eb, stack)
            } else {
                Ok(String::new())
            };
        }
    }

    stack.pop_frame();
    Ok(acc)
}

fn render_children(
    env: &Environment,
    state: &mut RenderState<'_>,
    nodes: &[Node],
    stack: &mut CtxStack,
) -> Result<String> {
    let mut out = String::new();
    for child in nodes {
        out.push_str(&render_node(env, state, child, stack)?);
    }
    Ok(out)
}

fn render_output(
    env: &Environment,
    state: &mut RenderState<'_>,
    exprs: &[Expr],
    stack: &CtxStack,
) -> Result<String> {
    let mut out = String::new();
    for e in exprs {
        out.push_str(&eval_for_output(env, state, e, stack)?);
    }
    Ok(out)
}

/// Template output for `{{ expr }}`: literals are not auto-escaped; other values respect [`Environment::autoescape`].
fn eval_for_output(
    env: &Environment,
    state: &mut RenderState<'_>,
    e: &Expr,
    stack: &CtxStack,
) -> Result<String> {
    match e {
        Expr::Literal(v) => Ok(crate::value::value_to_string(v)),
        _ => {
            let v = eval_to_value(env, state, e, stack)?;
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
    state: &mut RenderState<'_>,
    test_name: &str,
    value: &Value,
    arg_exprs: &[Expr],
    stack: &CtxStack,
) -> Result<bool> {
    let arg_vals: Vec<Value> = arg_exprs
        .iter()
        .map(|e| eval_to_value(env, state, e, stack))
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

fn render_macro_body(
    env: &Environment,
    state: &mut RenderState<'_>,
    m: &MacroDef,
    arg_vals: &[Value],
    outer: &CtxStack,
) -> Result<String> {
    let mut inner = outer.flatten();
    for (i, p) in m.params.iter().enumerate() {
        let v = arg_vals.get(i).cloned().unwrap_or(Value::Null);
        inner.insert(p.clone(), v);
    }
    let mut stack = CtxStack::from_root(inner);
    render_children(env, state, &m.body, &mut stack)
}

fn eval_to_value(
    env: &Environment,
    state: &mut RenderState<'_>,
    e: &Expr,
    stack: &CtxStack,
) -> Result<Value> {
    match e {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Variable(name) => Ok(stack.get(name)),
        Expr::Unary { op, expr } => {
            let v = eval_to_value(env, state, expr, stack)?;
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
                &eval_to_value(env, state, left, stack)?,
                &eval_to_value(env, state, right, stack)?,
            )),
            BinOp::Concat => Ok(Value::String(format!(
                "{}{}",
                eval_for_output(env, state, left, stack)?,
                eval_for_output(env, state, right, stack)?
            ))),
            BinOp::Sub => {
                let a = eval_to_value(env, state, left, stack)?;
                let b = eval_to_value(env, state, right, stack)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`-` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`-` expects numbers"))?;
                Ok(json_num(x - y))
            }
            BinOp::Mul => {
                let a = eval_to_value(env, state, left, stack)?;
                let b = eval_to_value(env, state, right, stack)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`*` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`*` expects numbers"))?;
                Ok(json_num(x * y))
            }
            BinOp::Div => {
                let a = eval_to_value(env, state, left, stack)?;
                let b = eval_to_value(env, state, right, stack)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`/` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`/` expects numbers"))?;
                Ok(json!(x / y))
            }
            BinOp::FloorDiv => {
                let a = eval_to_value(env, state, left, stack)?;
                let b = eval_to_value(env, state, right, stack)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`//` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`//` expects numbers"))?;
                if y == 0.0 {
                    return Err(RunjucksError::new("division by zero"));
                }
                Ok(json_num((x / y).floor()))
            }
            BinOp::Mod => {
                let a = eval_to_value(env, state, left, stack)?;
                let b = eval_to_value(env, state, right, stack)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`%` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`%` expects numbers"))?;
                Ok(json_num(x % y))
            }
            BinOp::Pow => {
                let a = eval_to_value(env, state, left, stack)?;
                let b = eval_to_value(env, state, right, stack)?;
                let x = as_number(&a).ok_or_else(|| RunjucksError::new("`**` expects numbers"))?;
                let y = as_number(&b).ok_or_else(|| RunjucksError::new("`**` expects numbers"))?;
                Ok(json!(x.powf(y)))
            }
            BinOp::And => {
                let l = eval_to_value(env, state, left, stack)?;
                if !is_truthy(&l) {
                    return Ok(l);
                }
                eval_to_value(env, state, right, stack)
            }
            BinOp::Or => {
                let l = eval_to_value(env, state, left, stack)?;
                if is_truthy(&l) {
                    return Ok(l);
                }
                eval_to_value(env, state, right, stack)
            }
            BinOp::In => {
                let key = eval_to_value(env, state, left, stack)?;
                let container = eval_to_value(env, state, right, stack)?;
                Ok(Value::Bool(eval_in(&key, &container)?))
            }
            BinOp::Is => {
                let (test_name, arg_exprs) = is_test_parts(right).ok_or_else(|| {
                    RunjucksError::new("`is` test must be an identifier, call, string, or null")
                })?;
                if test_name == "defined" {
                    if let Expr::Variable(n) = &**left {
                        return Ok(Value::Bool(stack.defined(n)));
                    }
                }
                let v = eval_to_value(env, state, left, stack)?;
                Ok(Value::Bool(eval_is_test(
                    env, state, test_name, &v, arg_exprs, stack,
                )?))
            }
        },
        Expr::Compare { head, rest } => {
            let mut acc = eval_to_value(env, state, head, stack)?;
            for (op, rhs_e) in rest.iter() {
                let r = eval_to_value(env, state, rhs_e, stack)?;
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
            let c = eval_to_value(env, state, cond, stack)?;
            if is_truthy(&c) {
                eval_to_value(env, state, then_expr, stack)
            } else if let Some(els) = else_expr {
                eval_to_value(env, state, els, stack)
            } else {
                Ok(Value::Null)
            }
        }
        Expr::GetAttr { base, attr } => {
            let b = eval_to_value(env, state, base, stack)?;
            match b {
                Value::Object(o) => Ok(o.get(attr).cloned().unwrap_or(Value::Null)),
                _ => Ok(Value::Null),
            }
        }
        Expr::GetItem { base, index } => {
            let b = eval_to_value(env, state, base, stack)?;
            let i = eval_to_value(env, state, index, stack)?;
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
        Expr::Call { callee, args } => {
            let arg_vals: Vec<Value> = args
                .iter()
                .map(|a| eval_to_value(env, state, a, stack))
                .collect::<Result<_>>()?;
            if let Expr::Variable(name) = callee.as_ref() {
                if let Some(mdef) = state.lookup_macro(name).cloned() {
                    let s = render_macro_body(env, state, &mdef, &arg_vals, stack)?;
                    return Ok(Value::String(s));
                }
            }
            Err(RunjucksError::new(
                "only template macro calls are supported for `()` expressions",
            ))
        }
        Expr::Filter { name, input, args } => {
            let input_v = eval_to_value(env, state, input, stack)?;
            let arg_vals: Vec<Value> = args
                .iter()
                .map(|a| eval_to_value(env, state, a, stack))
                .collect::<Result<_>>()?;
            crate::filters::apply_builtin(env, name, &input_v, &arg_vals)
        }
        Expr::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(eval_to_value(env, state, it, stack)?);
            }
            Ok(Value::Array(out))
        }
        Expr::Dict(pairs) => {
            use serde_json::Map;
            let mut m = Map::new();
            for (k, v) in pairs {
                let key_v = eval_to_value(env, state, k, stack)?;
                let key = match key_v {
                    Value::String(s) => s,
                    _ => crate::value::value_to_string(&key_v),
                };
                m.insert(key, eval_to_value(env, state, v, stack)?);
            }
            Ok(Value::Object(m))
        }
    }
}
