//! Walks [`crate::ast::Node`] trees and produces output strings using an [`crate::Environment`] and JSON context.

use crate::ast::{
    BinOp, CompareOp, Expr, ForVars, MacroDef, MacroParam, Node, SwitchCase, UnaryOp,
};
use crate::environment::Environment;
use crate::errors::{Result, RunjucksError};
use crate::globals::{
    builtin_range, cycler_handle_value, is_builtin_marker_value, joiner_handle_value,
    parse_cycler_id, parse_joiner_id, CyclerState, JoinerState, RJ_CALLABLE,
};
use crate::loader::TemplateLoader;
use crate::value::{is_undefined_value, undefined_value};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

/// `{% extends %}` parent expression plus block name → AST bodies.
type ExtendsLayout = (Expr, HashMap<String, Vec<Node>>);

/// Nunjucks-style frame stack: inner frames shadow outer; `set` updates the innermost existing binding.
#[derive(Debug, Clone)]
pub struct CtxStack {
    frames: Vec<Map<String, Value>>,
}

impl CtxStack {
    pub fn from_root(root: Map<String, Value>) -> Self {
        Self { frames: vec![root] }
    }

    pub fn push_frame(&mut self) {
        self.frames.push(Map::new());
    }

    pub fn pop_frame(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    /// Borrows the innermost binding for `name` across frames (template context shadows outer).
    pub fn get_ref(&self, name: &str) -> Option<&Value> {
        for f in self.frames.iter().rev() {
            if let Some(v) = f.get(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn get(&self, name: &str) -> Value {
        self.get_ref(name).cloned().unwrap_or(Value::Null)
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

/// One active `{% call %}`: body to render for `caller()` / `caller(args…)`, plus optional formal parameters.
#[derive(Clone)]
pub struct CallerFrame {
    pub body: Vec<Node>,
    pub params: Vec<MacroParam>,
}

/// Per-render state: optional loader, include cycle stack, macro scopes, and block inheritance for `extends`.
pub struct RenderState<'a> {
    pub loader: Option<&'a (dyn TemplateLoader + Send + Sync)>,
    pub stack: Vec<String>,
    pub macro_scopes: Vec<HashMap<String, MacroDef>>,
    /// `{% import "x" as ns %}` — macros callable as `ns.macro_name()`.
    pub macro_namespaces: HashMap<String, HashMap<String, MacroDef>>,
    /// Top-level `{% set %}` exports from each `import … as ns` namespace (`ns.name`), also the
    /// lexical scope for macros defined in that namespace.
    pub macro_namespace_values: HashMap<String, HashMap<String, Value>>,
    /// Per-block inheritance: innermost template first (child → parent → …) for `{{ super() }}`.
    pub block_chains: Option<HashMap<String, Vec<Vec<Node>>>>,
    /// When rendering a block layer, `Some((block_name, layer_index))` for `super()` resolution.
    pub super_context: Option<(String, usize)>,
    /// Innermost `{% call %}` frame for `caller()` / `caller(args…)` inside macro execution.
    pub caller_stack: Vec<CallerFrame>,
    /// Stateful `cycler(...)` instances (index matches handle object).
    pub cyclers: Vec<CyclerState>,
    /// Stateful `joiner(...)` instances.
    pub joiners: Vec<JoinerState>,
    /// PRNG for `| random` (seed from [`Environment::random_seed`] when set).
    pub rng: SmallRng,
}

impl<'a> RenderState<'a> {
    pub fn new(
        loader: Option<&'a (dyn TemplateLoader + Send + Sync)>,
        rng_seed: Option<u64>,
    ) -> Self {
        let rng = match rng_seed {
            Some(s) => SmallRng::seed_from_u64(s),
            None => SmallRng::from_entropy(),
        };
        Self {
            loader,
            stack: Vec::new(),
            macro_scopes: Vec::new(),
            macro_namespaces: HashMap::new(),
            macro_namespace_values: HashMap::new(),
            block_chains: None,
            super_context: None,
            caller_stack: Vec::new(),
            cyclers: Vec::new(),
            joiners: Vec::new(),
            rng,
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

    pub fn lookup_namespaced_macro(&self, ns: &str, macro_name: &str) -> Option<&MacroDef> {
        self.macro_namespaces
            .get(ns)
            .and_then(|m| m.get(macro_name))
    }

    pub fn lookup_namespaced_value(&self, ns: &str, name: &str) -> Option<&Value> {
        self.macro_namespace_values
            .get(ns)
            .and_then(|m| m.get(name))
    }
}

/// Renders `root` to a string using `env` and `ctx_stack`.
pub fn render(
    env: &Environment,
    loader: Option<&(dyn TemplateLoader + Send + Sync)>,
    root: &Node,
    ctx_stack: &mut CtxStack,
) -> Result<String> {
    let mut state = RenderState::new(loader, env.random_seed);
    render_entry(env, &mut state, root, ctx_stack)
}

/// Entry: handle `{% extends %}` child templates, otherwise normal render.
pub fn render_entry(
    env: &Environment,
    state: &mut RenderState<'_>,
    root: &Node,
    ctx_stack: &mut CtxStack,
) -> Result<String> {
    if let Some((parent_expr, blocks)) = extract_layout_if_any(root)? {
        let parent_name =
            crate::value::value_to_string(&eval_to_value(env, state, &parent_expr, ctx_stack)?);
        render_extends(env, state, &parent_name, blocks, ctx_stack)
    } else {
        render_with_state(env, state, root, ctx_stack)
    }
}

fn extract_layout_if_any(root: &Node) -> Result<Option<ExtendsLayout>> {
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

fn collect_blocks_in_root(root: &Node) -> HashMap<String, Vec<Node>> {
    let Node::Root(children) = root else {
        return HashMap::new();
    };
    let mut m = HashMap::new();
    for n in children {
        if let Node::Block { name, body } = n {
            m.insert(name.clone(), body.clone());
        }
    }
    m
}

fn extends_parent_expr(root: &Node) -> Option<&Expr> {
    let Node::Root(children) = root else {
        return None;
    };
    for n in children {
        if let Node::Extends { parent } = n {
            return Some(parent);
        }
    }
    None
}

/// Block bodies from innermost (overriding child) to outermost for each block name.
#[allow(clippy::too_many_arguments)]
fn build_block_chains(
    parent_name: &str,
    parent_ast: &Node,
    immediate_child_overrides: &HashMap<String, Vec<Node>>,
    loader: &(dyn TemplateLoader + Send + Sync),
    visited: &mut HashSet<String>,
    env: &Environment,
    state: &mut RenderState<'_>,
    ctx_stack: &mut CtxStack,
) -> Result<HashMap<String, Vec<Vec<Node>>>> {
    if !visited.insert(parent_name.to_string()) {
        return Err(RunjucksError::new(format!(
            "circular `{{% extends %}}` involving `{parent_name}`"
        )));
    }

    let result = (|| {
        let local_blocks = collect_blocks_in_root(parent_ast);
        let inherited: HashMap<String, Vec<Vec<Node>>> =
            if let Some(gp_expr) = extends_parent_expr(parent_ast) {
                let gp_name =
                    crate::value::value_to_string(&eval_to_value(env, state, gp_expr, ctx_stack)?);
                let gp_ast = env.load_and_parse_named(&gp_name, loader)?;
                build_block_chains(
                    &gp_name,
                    gp_ast.as_ref(),
                    &local_blocks,
                    loader,
                    visited,
                    env,
                    state,
                    ctx_stack,
                )?
            } else {
                HashMap::new()
            };

        let mut all_names: HashSet<String> = immediate_child_overrides.keys().cloned().collect();
        all_names.extend(local_blocks.keys().cloned());
        all_names.extend(inherited.keys().cloned());

        let mut out = HashMap::new();
        for name in all_names {
            let mut chain: Vec<Vec<Node>> = Vec::new();
            if let Some(c) = immediate_child_overrides.get(&name) {
                chain.push(c.clone());
            }
            if let Some(rest) = inherited.get(&name) {
                chain.extend(rest.iter().cloned());
            } else if let Some(l) = local_blocks.get(&name) {
                chain.push(l.clone());
            }
            if !chain.is_empty() {
                out.insert(name, chain);
            }
        }
        Ok(out)
    })();

    visited.remove(parent_name);
    result
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
    let parent_ast = env.load_and_parse_named(parent_name, loader)?;
    state.push_template(parent_name)?;
    let mut visited = HashSet::new();
    let chains = build_block_chains(
        parent_name,
        parent_ast.as_ref(),
        &blocks,
        loader,
        &mut visited,
        env,
        state,
        ctx_stack,
    )?;
    let prev_chains = state.block_chains.take();
    state.block_chains = Some(chains);
    let out = render_with_state(env, state, parent_ast.as_ref(), ctx_stack)?;
    state.block_chains = prev_chains;
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

/// Top-level `{% macro %}` definitions only (Nunjucks `getExported` surface for macro libraries).
fn collect_top_level_macros(root: &Node) -> HashMap<String, MacroDef> {
    let mut m = HashMap::new();
    let Node::Root(children) = root else {
        return m;
    };
    for n in children {
        if let Node::MacroDef(def) = n {
            m.insert(def.name.clone(), def.clone());
        }
    }
    m
}

/// Top-level `{% set name = expr %}` (single target, no `{% set %}…{% endset %}` block).
fn collect_top_level_sets(root: &Node) -> Vec<(String, Expr)> {
    let mut out = Vec::new();
    let Node::Root(children) = root else {
        return out;
    };
    for n in children {
        if let Node::Set {
            targets,
            value: Some(expr),
            body: None,
        } = n
        {
            if targets.len() == 1 {
                out.push((targets[0].clone(), expr.clone()));
            }
        }
    }
    out
}

/// Evaluates exported top-level assignments (`getExported`) with Nunjucks-style context:
/// `with context` → parent context; omitted or `without context` → isolated root (globals still resolve via [`Environment`].
fn eval_exported_top_level_sets(
    env: &Environment,
    state: &mut RenderState<'_>,
    root: &Node,
    ctx_stack: &mut CtxStack,
    with_context: Option<bool>,
) -> Result<HashMap<String, Value>> {
    let mut out = HashMap::new();
    let sets = collect_top_level_sets(root);
    let mut import_stack = if matches!(with_context, Some(true)) {
        CtxStack::from_root(ctx_stack.flatten())
    } else {
        CtxStack::from_root(Map::new())
    };
    for (name, expr) in sets {
        let v = eval_to_value(env, state, &expr, &mut import_stack)?;
        import_stack.set(&name, v.clone());
        out.insert(name, v);
    }
    Ok(out)
}

/// Detects `{% import "x" %}` / `{% from "x" %}` cycles using **string-literal** paths only (matches
/// typical macro libraries; dynamic names are not traced here).
fn scan_literal_import_graph(
    env: &Environment,
    state: &mut RenderState<'_>,
    root: &Node,
    loader: &(dyn TemplateLoader + Send + Sync),
) -> Result<()> {
    let Node::Root(children) = root else {
        return Ok(());
    };
    for n in children {
        let template_expr = match n {
            Node::Import { template, .. } | Node::FromImport { template, .. } => template,
            _ => continue,
        };
        let Expr::Literal(Value::String(path)) = template_expr else {
            continue;
        };
        state.push_template(path)?;
        let nested = env.load_and_parse_named(path, loader)?;
        scan_literal_import_graph(env, state, nested.as_ref(), loader)?;
        state.pop_template();
    }
    Ok(())
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
            let scope_base = state.macro_scopes.len();
            if had_macros {
                state.push_macros(defs);
            }
            let mut out = String::new();
            out.reserve(nodes.len().saturating_mul(32));
            for child in nodes.iter() {
                if matches!(child, Node::MacroDef(_) | Node::Extends { .. }) {
                    continue;
                }
                out.push_str(&render_node(env, state, child, stack)?);
            }
            while state.macro_scopes.len() > scope_base {
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
            with_context,
        } => {
            let loader = state
                .loader
                .ok_or_else(|| RunjucksError::new("`include` requires a template loader"))?;
            let name = crate::value::value_to_string(&eval_to_value(env, state, template, stack)?);
            let included = match env.load_and_parse_named(&name, loader) {
                Ok(ast) => ast,
                Err(_) if *ignore_missing => return Ok(String::new()),
                Err(e) => return Err(e),
            };
            state.push_template(&name)?;
            let out = if matches!(with_context, Some(false)) {
                let mut isolated = CtxStack::from_root(Map::new());
                render_entry(env, state, included.as_ref(), &mut isolated)?
            } else {
                render_entry(env, state, included.as_ref(), stack)?
            };
            state.pop_template();
            Ok(out)
        }
        Node::Import {
            template,
            alias,
            with_context,
        } => {
            let loader = state
                .loader
                .ok_or_else(|| RunjucksError::new("`import` requires a template loader"))?;
            let name = crate::value::value_to_string(&eval_to_value(env, state, template, stack)?);
            let imported = env.load_and_parse_named(&name, loader)?;
            state.push_template(&name)?;
            scan_literal_import_graph(env, state, imported.as_ref(), loader)?;
            let defs = collect_top_level_macros(imported.as_ref());
            let exported_sets =
                eval_exported_top_level_sets(env, state, imported.as_ref(), stack, *with_context)?;
            state.pop_template();
            state.macro_namespaces.insert(alias.clone(), defs);
            state
                .macro_namespace_values
                .insert(alias.clone(), exported_sets);
            Ok(String::new())
        }
        Node::FromImport {
            template,
            names,
            with_context,
        } => {
            let loader = state
                .loader
                .ok_or_else(|| RunjucksError::new("`from` requires a template loader"))?;
            let name = crate::value::value_to_string(&eval_to_value(env, state, template, stack)?);
            let imported = env.load_and_parse_named(&name, loader)?;
            state.push_template(&name)?;
            scan_literal_import_graph(env, state, imported.as_ref(), loader)?;
            let defs = collect_top_level_macros(imported.as_ref());
            let exported_sets =
                eval_exported_top_level_sets(env, state, imported.as_ref(), stack, *with_context)?;
            state.pop_template();
            let mut scope = HashMap::new();
            for (export_name, alias_opt) in names {
                let local = alias_opt.as_ref().unwrap_or(export_name);
                if let Some(mdef) = defs.get(export_name) {
                    scope.insert(local.clone(), mdef.clone());
                } else if let Some(v) = exported_sets.get(export_name) {
                    stack.set(local, v.clone());
                } else {
                    return Err(RunjucksError::new(format!("cannot import '{export_name}'")));
                }
            }
            state.push_macros(scope);
            Ok(String::new())
        }
        Node::Extends { .. } => Err(RunjucksError::new(
            "`extends` is only valid at the top level of a loaded template",
        )),
        Node::Block { name, body } => {
            let to_render: Vec<Node> = if let Some(ref chains) = state.block_chains {
                chains
                    .get(name)
                    .and_then(|ch| ch.first().cloned())
                    .unwrap_or_else(|| body.clone())
            } else {
                body.clone()
            };
            let prev_super = state.super_context.take();
            state.super_context = Some((name.clone(), 0));
            let out = render_children(env, state, &to_render, stack);
            state.super_context = prev_super;
            out
        }
        Node::FilterBlock { name, args, body } => {
            let s = render_children(env, state, body, stack)?;
            let arg_vals: Vec<Value> = args
                .iter()
                .map(|a| eval_to_value(env, state, a, stack))
                .collect::<Result<_>>()?;
            let v = crate::filters::apply_builtin(
                env,
                &mut state.rng,
                name,
                &Value::String(s),
                &arg_vals,
            )?;
            let out = crate::value::value_to_string(&v);
            if env.autoescape && !crate::value::is_marked_safe(&v) {
                Ok(crate::filters::escape_html(&out))
            } else {
                Ok(out)
            }
        }
        Node::CallBlock {
            caller_params,
            callee,
            body,
        } => {
            let Expr::Call {
                callee: macro_target,
                args,
                kwargs,
            } = callee
            else {
                return Err(RunjucksError::new(
                    "`{% call %}` expects a macro call expression such as `wrap()` or `ns.wrap()`",
                ));
            };
            let arg_vals: Vec<Value> = args
                .iter()
                .map(|a| eval_to_value(env, state, a, stack))
                .collect::<Result<_>>()?;
            let kw_vals: Vec<(String, Value)> = kwargs
                .iter()
                .map(|(k, e)| Ok((k.clone(), eval_to_value(env, state, e, stack)?)))
                .collect::<Result<_>>()?;
            let mdef = if let Expr::Variable(name) = macro_target.as_ref() {
                state
                    .lookup_macro(name)
                    .cloned()
                    .ok_or_else(|| RunjucksError::new(format!("unknown macro `{name}`")))?
            } else if let Expr::GetAttr { base, attr } = macro_target.as_ref() {
                if let Expr::Variable(ns) = base.as_ref() {
                    state
                        .lookup_namespaced_macro(ns, attr)
                        .cloned()
                        .ok_or_else(|| RunjucksError::new(format!("unknown macro `{ns}.{attr}`")))?
                } else {
                    return Err(RunjucksError::new(
                        "`{% call %}` only supports simple macro or `namespace.macro()` calls",
                    ));
                }
            } else {
                return Err(RunjucksError::new(
                    "`{% call %}` only supports simple macro or `namespace.macro()` calls",
                ));
            };
            let frame = CallerFrame {
                body: body.clone(),
                params: caller_params.clone(),
            };
            state.caller_stack.push(frame);
            let module_closure_owned =
                if let Expr::GetAttr { base, attr: _ } = macro_target.as_ref() {
                    if let Expr::Variable(ns) = base.as_ref() {
                        state.macro_namespace_values.get(ns).cloned()
                    } else {
                        None
                    }
                } else {
                    None
                };
            let res = render_macro_body(
                env,
                state,
                &mdef,
                &arg_vals,
                &kw_vals,
                stack,
                module_closure_owned.as_ref(),
            );
            state.caller_stack.pop();
            res
        }
        Node::ExtensionTag {
            extension_name,
            args,
            body,
            ..
        } => {
            let handler = env.custom_extensions.get(extension_name).ok_or_else(|| {
                RunjucksError::new(format!("unknown extension `{extension_name}`"))
            })?;
            let ctx_val = Value::Object(stack.flatten());
            let body_s = if let Some(nodes) = body {
                Some(render_children(env, state, nodes, stack)?)
            } else {
                None
            };
            let out = handler(&ctx_val, args.as_str(), body_s)?;
            Ok(if env.autoescape {
                crate::filters::escape_html(&out)
            } else {
                out
            })
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

fn fill_loop_object(m: &mut serde_json::Map<String, Value>, i: usize, len: usize) {
    m.insert(
        "index".to_string(),
        Value::Number(((i + 1) as u64).into()),
    );
    m.insert("index0".to_string(), Value::Number((i as u64).into()));
    m.insert("first".to_string(), Value::Bool(i == 0));
    m.insert(
        "last".to_string(),
        Value::Bool(len > 0 && i + 1 == len),
    );
    m.insert("length".to_string(), Value::Number((len as u64).into()));
    m.insert(
        "revindex".to_string(),
        Value::Number((len.saturating_sub(i) as u64).into()),
    );
    m.insert(
        "revindex0".to_string(),
        Value::Number((len.saturating_sub(1).saturating_sub(i) as u64).into()),
    );
}

/// Reuses the same `loop` object map in the innermost frame when possible (avoids reallocating keys each iteration).
fn inject_loop(stack: &mut CtxStack, i: usize, len: usize) {
    let inner = stack
        .frames
        .last_mut()
        .expect("inject_loop requires an active frame");
    if let Some(Value::Object(m)) = inner.get_mut("loop") {
        fill_loop_object(m, i, len);
    } else {
        let mut m = Map::with_capacity(7);
        fill_loop_object(&mut m, i, len);
        inner.insert("loop".to_string(), Value::Object(m));
    }
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
            acc.reserve(len.saturating_mul(16));
            for (i, item) in items.into_iter().enumerate() {
                inject_loop(stack, i, len);
                stack.set_local(x, item);
                acc.push_str(&render_children(env, state, body, stack)?);
            }
        }
        (ForVars::Multi(names), Iterable::Rows(rows)) if names.len() >= 2 => {
            let len = rows.len();
            acc.reserve(len.saturating_mul(16));
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
            acc.reserve(len.saturating_mul(16));
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
    out.reserve(nodes.len().saturating_mul(32));
    for child in nodes {
        out.push_str(&render_node(env, state, child, stack)?);
    }
    Ok(out)
}

fn render_output(
    env: &Environment,
    state: &mut RenderState<'_>,
    exprs: &[Expr],
    stack: &mut CtxStack,
) -> Result<String> {
    let mut out = String::new();
    out.reserve(exprs.len().saturating_mul(24));
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
    stack: &mut CtxStack,
) -> Result<String> {
    match e {
        Expr::Literal(v) => Ok(crate::value::value_to_string(v)),
        Expr::Variable(name) => {
            let v = env.resolve_variable_ref(stack, name)?;
            let s = crate::value::value_to_string(v.as_ref());
            if env.autoescape && !crate::value::is_marked_safe(v.as_ref()) {
                Ok(crate::filters::escape_html(&s))
            } else {
                Ok(s)
            }
        }
        _ => {
            let v = eval_to_value(env, state, e, stack)?;
            let s = crate::value::value_to_string(&v);
            if env.autoescape && !crate::value::is_marked_safe(&v) {
                Ok(crate::filters::escape_html(&s))
            } else {
                Ok(s)
            }
        }
    }
}

fn is_truthy(v: &Value) -> bool {
    if crate::value::is_undefined_value(v) {
        return false;
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(true),
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

fn eval_slice_bound(
    env: &Environment,
    state: &mut RenderState<'_>,
    e: Option<&Expr>,
    stack: &mut CtxStack,
) -> Result<Option<i64>> {
    let Some(e) = e else {
        return Ok(None);
    };
    let v = eval_to_value(env, state, e, stack)?;
    if v.is_null() || crate::value::is_undefined_value(&v) {
        return Ok(None);
    }
    let n = v
        .as_i64()
        .or_else(|| v.as_f64().map(|x| x as i64))
        .or_else(|| crate::value::value_to_string(&v).parse().ok());
    match n {
        Some(x) => Ok(Some(x)),
        None => Err(RunjucksError::new("slice bound must be a number")),
    }
}

/// Jinja-compat slice (`nunjucks` `sliceLookup`).
fn jinja_slice_array(
    obj: &[Value],
    start: Option<i64>,
    stop: Option<i64>,
    step: Option<i64>,
) -> Vec<Value> {
    let len = obj.len() as i64;
    let step = step.unwrap_or(1);
    if step == 0 {
        return vec![];
    }
    let mut start = start;
    let mut stop = stop;
    if start.is_none() {
        start = Some(if step < 0 { (len - 1).max(0) } else { 0 });
    }
    if stop.is_none() {
        stop = Some(if step < 0 { -1 } else { len });
    } else if let Some(s) = stop {
        if s < 0 {
            stop = Some(s + len);
        }
    }
    if let Some(s) = start {
        if s < 0 {
            start = Some(s + len);
        }
    }
    let start = start.unwrap_or(0);
    let stop = stop.unwrap_or(len);
    let mut results = Vec::new();
    let mut i = start;
    loop {
        if i < 0 || i > len {
            break;
        }
        if step > 0 && i >= stop {
            break;
        }
        if step < 0 && i <= stop {
            break;
        }
        if let Some(item) = obj.get(i as usize) {
            results.push(item.clone());
        }
        i += step;
    }
    results
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
        Expr::Call {
            callee,
            args,
            kwargs,
        } => {
            if !kwargs.is_empty() {
                return None;
            }
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
    stack: &mut CtxStack,
) -> Result<bool> {
    let arg_vals: Vec<Value> = arg_exprs
        .iter()
        .map(|e| eval_to_value(env, state, e, stack))
        .collect::<Result<_>>()?;
    env.apply_is_test(test_name, value, &arg_vals)
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

fn can_dispatch_builtin(stack: &CtxStack, name: &str) -> bool {
    matches!(name, "range" | "cycler" | "joiner")
        && (!stack.defined(name)
            || stack
                .get_ref(name)
                .map(|v| is_builtin_marker_value(v, name))
                .unwrap_or(false))
}

fn try_dispatch_builtin(
    state: &mut RenderState<'_>,
    stack: &CtxStack,
    name: &str,
    arg_vals: &[Value],
) -> Option<Result<Value>> {
    if !can_dispatch_builtin(stack, name) {
        return None;
    }
    match name {
        "range" => Some(builtin_range(arg_vals)),
        "cycler" => {
            let id = state.cyclers.len();
            state.cyclers.push(CyclerState::new(arg_vals.to_vec()));
            Some(Ok(cycler_handle_value(id)))
        }
        "joiner" => {
            let sep = match arg_vals.len() {
                0 => ",".to_string(),
                1 => {
                    let s = crate::value::value_to_string(&arg_vals[0]);
                    if s.is_empty() {
                        ",".to_string()
                    } else {
                        s
                    }
                }
                _ => {
                    return Some(Err(RunjucksError::new(
                        "`joiner` expects at most one argument",
                    )));
                }
            };
            let id = state.joiners.len();
            state.joiners.push(JoinerState::new(sep));
            Some(Ok(joiner_handle_value(id)))
        }
        _ => None,
    }
}

fn render_macro_body(
    env: &Environment,
    state: &mut RenderState<'_>,
    m: &MacroDef,
    positional: &[Value],
    kwargs: &[(String, Value)],
    outer: &mut CtxStack,
    module_closure: Option<&HashMap<String, Value>>,
) -> Result<String> {
    let mut inner = outer.flatten();
    if let Some(mc) = module_closure {
        for (k, v) in mc {
            inner.insert(k.clone(), v.clone());
        }
    }
    for p in &m.params {
        let val = if let Some(ref d) = p.default {
            eval_to_value(env, state, d, outer)?
        } else {
            Value::Null
        };
        inner.insert(p.name.clone(), val);
    }
    for (i, p) in m.params.iter().enumerate() {
        if let Some(v) = positional.get(i) {
            inner.insert(p.name.clone(), v.clone());
        }
    }
    for (k, v) in kwargs {
        if m.params.iter().any(|p| p.name == *k) {
            inner.insert(k.clone(), v.clone());
        }
    }
    let mut stack = CtxStack::from_root(inner);
    render_children(env, state, &m.body, &mut stack)
}

/// Renders the `{% call %}` body for `caller()` / `caller(args…)` (Nunjucks `Caller` node).
fn render_caller_invocation(
    env: &Environment,
    state: &mut RenderState<'_>,
    frame: &CallerFrame,
    positional: &[Value],
    kwargs: &[(String, Value)],
    stack: &mut CtxStack,
) -> Result<String> {
    if frame.params.is_empty() {
        if !positional.is_empty() || !kwargs.is_empty() {
            return Err(RunjucksError::new("`caller()` takes no arguments"));
        }
        return render_children(env, state, &frame.body, stack);
    }
    stack.push_frame();
    for p in &frame.params {
        let val = if let Some(ref d) = p.default {
            eval_to_value(env, state, d, stack)?
        } else {
            Value::Null
        };
        stack.set_local(&p.name, val);
    }
    for (i, p) in frame.params.iter().enumerate() {
        if let Some(v) = positional.get(i) {
            stack.set_local(&p.name, v.clone());
        }
    }
    for (k, v) in kwargs {
        if frame.params.iter().any(|p| p.name == *k) {
            stack.set_local(k, v.clone());
        }
    }
    let out = render_children(env, state, &frame.body, stack)?;
    stack.pop_frame();
    Ok(out)
}

fn eval_to_value(
    env: &Environment,
    state: &mut RenderState<'_>,
    e: &Expr,
    stack: &mut CtxStack,
) -> Result<Value> {
    match e {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Variable(name) => env.resolve_variable(stack, name),
        Expr::Unary { op, expr } => {
            let v = eval_to_value(env, state, expr, stack)?;
            Ok(match op {
                UnaryOp::Not => Value::Bool(!is_truthy(&v)),
                UnaryOp::Neg => {
                    let n = as_number(&v)
                        .ok_or_else(|| RunjucksError::new("unary '-' expects a numeric value"))?;
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
                if test_name == "callable" {
                    if let Expr::Variable(n) = &**left {
                        if state.lookup_macro(n).is_some() {
                            return Ok(Value::Bool(true));
                        }
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
            if let Expr::Variable(ns) = base.as_ref() {
                if state.macro_namespaces.contains_key(ns)
                    || state.macro_namespace_values.contains_key(ns)
                {
                    if let Some(v) = state.lookup_namespaced_value(ns, attr) {
                        return Ok(v.clone());
                    }
                    if state.lookup_namespaced_macro(ns, attr).is_some() {
                        let mut m = Map::new();
                        m.insert(RJ_CALLABLE.to_string(), Value::Bool(true));
                        return Ok(Value::Object(m));
                    }
                    return Ok(undefined_value());
                }
            }
            let b = eval_to_value(env, state, base, stack)?;
            if is_undefined_value(&b) || b.is_null() {
                return Ok(undefined_value());
            }
            match b {
                Value::Object(o) => Ok(o.get(attr).cloned().unwrap_or_else(undefined_value)),
                _ => Ok(undefined_value()),
            }
        }
        Expr::GetItem { base, index } => {
            let b = eval_to_value(env, state, base, stack)?;
            if is_undefined_value(&b) || b.is_null() {
                return Ok(undefined_value());
            }
            match index.as_ref() {
                Expr::Slice {
                    start: s,
                    stop: st,
                    step: step_e,
                } => {
                    let Value::Array(a) = &b else {
                        return Ok(Value::Null);
                    };
                    let start_v = eval_slice_bound(env, state, s.as_deref(), stack)?;
                    let stop_v = eval_slice_bound(env, state, st.as_deref(), stack)?;
                    let step_v = eval_slice_bound(env, state, step_e.as_deref(), stack)?;
                    Ok(Value::Array(jinja_slice_array(a, start_v, stop_v, step_v)))
                }
                idx_e => {
                    let i = eval_to_value(env, state, idx_e, stack)?;
                    match (&b, &i) {
                        (Value::Array(a), Value::Number(n)) => {
                            let idx = n
                                .as_u64()
                                .or_else(|| n.as_f64().map(|x| x as u64))
                                .unwrap_or(0) as usize;
                            Ok(a.get(idx).cloned().unwrap_or_else(undefined_value))
                        }
                        (Value::Object(o), Value::String(k)) => {
                            Ok(o.get(k).cloned().unwrap_or_else(undefined_value))
                        }
                        _ => Ok(undefined_value()),
                    }
                }
            }
        }
        Expr::Slice { .. } => Err(RunjucksError::new(
            "slice expression is only valid inside `[ ]`",
        )),
        Expr::Call {
            callee,
            args,
            kwargs,
        } => {
            let arg_vals: Vec<Value> = args
                .iter()
                .map(|a| eval_to_value(env, state, a, stack))
                .collect::<Result<_>>()?;
            let kw_vals: Vec<(String, Value)> = kwargs
                .iter()
                .map(|(k, e)| Ok((k.clone(), eval_to_value(env, state, e, stack)?)))
                .collect::<Result<_>>()?;
            if let Expr::GetAttr { base, attr } = callee.as_ref() {
                if attr == "test" {
                    let base_v = eval_to_value(env, state, base, stack)?;
                    if crate::value::is_regexp_value(&base_v) {
                        if !kw_vals.is_empty() {
                            return Err(RunjucksError::new(
                                "regex `.test` does not accept keyword arguments",
                            ));
                        }
                        if arg_vals.len() != 1 {
                            return Err(RunjucksError::new(
                                "regex `.test` expects exactly one argument",
                            ));
                        }
                        let Some((pat, fl)) = crate::value::regexp_pattern_flags(&base_v) else {
                            return Err(RunjucksError::new("invalid regex value"));
                        };
                        let s = crate::value::value_to_string(&arg_vals[0]);
                        return Ok(Value::Bool(crate::js_regex::regexp_test(&pat, &fl, &s)?));
                    }
                }
            }
            if let Expr::GetAttr { base, attr } = callee.as_ref() {
                if attr == "next" && arg_vals.is_empty() && kw_vals.is_empty() {
                    let b = eval_to_value(env, state, base, stack)?;
                    if let Some(id) = parse_cycler_id(&b) {
                        if let Some(c) = state.cyclers.get_mut(id) {
                            return Ok(c.next());
                        }
                        return Ok(Value::Null);
                    }
                }
            }
            if let Expr::Variable(name) = callee.as_ref() {
                if name == "super" {
                    if !args.is_empty() || !kw_vals.is_empty() {
                        return Err(RunjucksError::new("`super()` takes no arguments"));
                    }
                    let (block_name, layer) = state.super_context.clone().ok_or_else(|| {
                        RunjucksError::new("`super()` is only valid inside a `{% block %}`")
                    })?;
                    let (body_to_render, next) = {
                        let chains = state.block_chains.as_ref().ok_or_else(|| {
                            RunjucksError::new(
                                "`super()` requires template inheritance (`{% extends %}`)",
                            )
                        })?;
                        let chain = chains.get(&block_name).ok_or_else(|| {
                            RunjucksError::new(format!(
                                "no super block available for `{block_name}`"
                            ))
                        })?;
                        let next = layer + 1;
                        if next >= chain.len() {
                            return Err(RunjucksError::new(
                                "no parent block available for `super()`",
                            ));
                        }
                        (chain[next].clone(), next)
                    };
                    let prev = state.super_context.replace((block_name.clone(), next));
                    let s = render_children(env, state, &body_to_render, stack)?;
                    state.super_context = prev;
                    return Ok(Value::String(s));
                }
                if name == "caller" {
                    let frame = state.caller_stack.last().cloned().ok_or_else(|| {
                        RunjucksError::new(
                            "`caller()` is only valid inside a macro invoked from `{% call %}`",
                        )
                    })?;
                    let s =
                        render_caller_invocation(env, state, &frame, &arg_vals, &kw_vals, stack)?;
                    return Ok(Value::String(s));
                }
                if let Some(mdef) = state.lookup_macro(name).cloned() {
                    let s = render_macro_body(env, state, &mdef, &arg_vals, &kw_vals, stack, None)?;
                    return Ok(Value::String(s));
                }
                if arg_vals.is_empty() {
                    let v = env.resolve_variable(stack, name)?;
                    if let Some(id) = parse_joiner_id(&v) {
                        if let Some(j) = state.joiners.get_mut(id) {
                            return Ok(Value::String(j.invoke()));
                        }
                    }
                }
                if let Some(r) = try_dispatch_builtin(state, stack, name, &arg_vals) {
                    return r;
                }
                if let Some(f) = env.custom_globals.get(name) {
                    return f(&arg_vals, &kw_vals);
                }
            }
            if let Expr::GetAttr { base, attr } = callee.as_ref() {
                if let Expr::Variable(ns) = base.as_ref() {
                    if let Some(mdef) = state.lookup_namespaced_macro(ns, attr).cloned() {
                        let mc = state.macro_namespace_values.get(ns).cloned();
                        let s = render_macro_body(
                            env,
                            state,
                            &mdef,
                            &arg_vals,
                            &kw_vals,
                            stack,
                            mc.as_ref(),
                        )?;
                        return Ok(Value::String(s));
                    }
                }
            }
            Err(RunjucksError::new(
                "only template macros, built-in globals (`range`, `cycler`, `joiner`), registered global callables, or `super`/`caller` are supported for `()` expressions",
            ))
        }
        Expr::Filter { name, input, args } => {
            let input_v = eval_to_value(env, state, input, stack)?;
            let arg_vals: Vec<Value> = args
                .iter()
                .map(|a| eval_to_value(env, state, a, stack))
                .collect::<Result<_>>()?;
            crate::filters::apply_builtin(env, &mut state.rng, name, &input_v, &arg_vals)
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
        Expr::RegexLiteral { pattern, flags } => {
            let mut m = Map::new();
            m.insert(crate::value::RJ_REGEXP.to_string(), Value::Bool(true));
            m.insert("pattern".to_string(), Value::String(pattern.clone()));
            m.insert("flags".to_string(), Value::String(flags.clone()));
            Ok(Value::Object(m))
        }
    }
}
