//! Abstract syntax tree for templates after [`crate::parser::parse`].
//!
//! [`Expr`] covers outputs (`{{ … }}`) including literals, variables, and operators
//! (aligned with Nunjucks [`parseExpression`](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/parser.js)).

use serde_json::Value;

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
