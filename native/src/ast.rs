use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Root(Vec<Node>),
    Text(String),
    Output(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    Variable(String),
}
