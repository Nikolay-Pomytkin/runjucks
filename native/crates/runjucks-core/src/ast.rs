//! Abstract syntax tree for templates after [`crate::parser::parse`].
//!
//! [`Expr`] covers outputs (`{{ … }}`) including literals, variables, and operators
//! (aligned with Nunjucks [`parseExpression`](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/parser.js)).

use serde_json::Value;

/// One branch of an `{% if %}` / `{% elif %}` / `{% else %}` chain.
#[derive(Debug, Clone, PartialEq)]
pub struct IfBranch {
    /// Condition; [`None`] for `{% else %}`.
    pub cond: Option<Expr>,
    pub body: Vec<Node>,
}

/// A template macro definition (`{% macro name(args) %}…{% endmacro %}`).
#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Node>,
}

/// Loop binding list after `for` (`x` or `k, v` or `a, b, c`).
#[derive(Debug, Clone, PartialEq)]
pub enum ForVars {
    Single(String),
    Multi(Vec<String>),
}

/// One `{% case expr %}…` branch inside `{% switch %}`.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub cond: Expr,
    pub body: Vec<Node>,
}

/// Template structure produced by the parser.
///
/// - [`Node::Root`]: sequence of top-level fragments (text + outputs).
/// - [`Node::Text`]: literal characters from the template.
/// - [`Node::Output`]: one or more [`Expr`] values to evaluate and concatenate (filters etc. are future work).
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Container for sibling template fragments.
    Root(Vec<Node>),
    /// Raw text between (or outside) template tags.
    Text(String),
    /// Content inside `{{` … `}}` after expression parsing.
    Output(Vec<Expr>),
    /// `{% if %}`, optional `{% elif %}`, optional `{% else %}`, `{% endif %}`.
    If { branches: Vec<IfBranch> },
    /// `{% for var in iter %} … {% endfor %}` (optional `{% else %}` before `endfor`).
    For {
        vars: ForVars,
        iter: Expr,
        body: Vec<Node>,
        else_body: Option<Vec<Node>>,
    },
    /// `{% set a = expr %}`, `{% set a, b = expr %}`, or `{% set a %}…{% endset %}`.
    Set {
        targets: Vec<String>,
        value: Option<Expr>,
        body: Option<Vec<Node>>,
    },
    /// `{% include expr %}` with optional `ignore missing` — template name from expression.
    Include {
        template: Expr,
        ignore_missing: bool,
    },
    /// `{% switch expr %}{% case c %}…{% default %}…{% endswitch %}`.
    Switch {
        expr: Expr,
        cases: Vec<SwitchCase>,
        default_body: Option<Vec<Node>>,
    },
    /// `{% extends "parent" %}` — must appear before meaningful content in a child template.
    Extends { parent: String },
    /// `{% block name %}…{% endblock %}` — default body when used in a base layout.
    Block { name: String, body: Vec<Node> },
    /// `{% macro name(a, b) %}…{% endmacro %}` — emits no output; registers a macro for the current template.
    MacroDef(MacroDef),
}

/// Comparison operators in a Nunjucks-style chain (`a == b < c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    StrictEq,
    Ne,
    StrictNe,
    Lt,
    Gt,
    Le,
    Ge,
}

/// Binary operators (arithmetic, logical short-circuit forms use these in the AST).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    /// String concatenation (`~`), compiled as `+ "" +` in Nunjucks JS output.
    Concat,
    And,
    Or,
    /// Membership ([`runtime.inOperator` in Nunjucks](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/lib.js)).
    In,
    /// `is` test (right-hand side is typically a variable naming the test).
    Is,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Pos,
}

/// Expression inside a variable tag or tag body (when wired up).
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// JSON literal (numbers, strings, booleans, null).
    Literal(Value),
    /// Context key; non-existent keys render as empty (Nunjucks-style).
    Variable(String),
    /// Attribute access: `base.attr` (Nunjucks `LookupVal`).
    GetAttr {
        base: Box<Expr>,
        attr: String,
    },
    /// Subscript: `base[index]`.
    GetItem {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    /// Function-style call `callee(args…)` (runtime may be limited until globals exist).
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// Filter pipeline step: `input | name` or `input | name(arg, …)`.
    Filter {
        name: String,
        input: Box<Expr>,
        args: Vec<Expr>,
    },
    /// Array literal `[a, b, …]` with arbitrary expressions.
    List(Vec<Expr>),
    /// Object literal `{ key: val, …}` (keys are expressions, usually strings or identifiers).
    Dict(Vec<(Expr, Expr)>),
    /// Python-style conditional: `body if cond else else_`.
    InlineIf {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Option<Box<Expr>>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Chained comparisons (`a == b < c` → left-associative JS-style emit in Nunjucks).
    Compare {
        head: Box<Expr>,
        rest: Vec<(CompareOp, Expr)>,
    },
}
