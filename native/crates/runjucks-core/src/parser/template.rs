//! `{% … %}` statement parsing: `if` / `elif` / `else`, `for`, `set`.

#![allow(clippy::manual_contains)]

use crate::ast::{Expr, IfBranch, Node};
use crate::errors::{Result, RunjucksError};
use crate::lexer::Token;
use crate::parser::parse_expr;

/// Longer keywords first so `elseif` does not match as `else`.
const TAG_KEYWORDS: &[&str] = &[
    "elseif", "elif", "endif", "endfor", "for", "set", "if", "else",
];

fn first_tag_keyword(body: &str) -> String {
    let s = body.trim();
    for kw in TAG_KEYWORDS {
        if s == *kw || s.starts_with(&format!("{kw} ")) {
            return (*kw).to_string();
        }
    }
    s.split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

fn strip_keyword_prefix<'a>(body: &'a str, kws: &[&str]) -> Result<&'a str> {
    let s = body.trim();
    for kw in kws {
        if s == *kw {
            return Ok("");
        }
        let prefix = format!("{kw} ");
        if s.starts_with(&prefix) {
            return Ok(s[prefix.len()..].trim_start());
        }
    }
    Err(RunjucksError::new(format!(
        "expected tag starting with one of {kws:?}, got {body:?}"
    )))
}

fn peek_tag_keyword(tokens: &[Token], i: usize) -> Option<String> {
    match tokens.get(i)? {
        Token::Tag(b) => Some(first_tag_keyword(b)),
        _ => None,
    }
}

fn expect_tag(tokens: &[Token], i: &mut usize, kws: &[&str]) -> Result<()> {
    let Some(Token::Tag(b)) = tokens.get(*i) else {
        return Err(RunjucksError::new("expected `{% %}` tag"));
    };
    let kw = first_tag_keyword(b);
    if !kws.iter().any(|k| *k == kw.as_str()) {
        return Err(RunjucksError::new(format!(
            "expected one of {kws:?}, found `{kw}`"
        )));
    }
    *i += 1;
    Ok(())
}

fn parse_for_header(rest: &str) -> Result<(String, Expr)> {
    let s = rest.trim();
    let pos = s
        .find(" in ")
        .ok_or_else(|| RunjucksError::new("expected `for <name> in <expr>`"))?;
    let left = s[..pos].trim();
    let right = s[pos + 4..].trim();
    if left.is_empty()
        || !left
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(RunjucksError::new(
            "`for` loop variable must be a single identifier",
        ));
    }
    Ok((left.to_string(), parse_expr(right)?))
}

fn parse_set_rhs(after_set: &str) -> Result<(String, Expr)> {
    let s = after_set.trim();
    let eq = s
        .find('=')
        .ok_or_else(|| RunjucksError::new("expected `set <name> = <expr>`"))?;
    let name = s[..eq].trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(RunjucksError::new("`set` target must be an identifier"));
    }
    let expr_s = s[eq + 1..].trim();
    Ok((name.to_string(), parse_expr(expr_s)?))
}

/// Parses a sequence of nodes until a tag whose first keyword is in `stop` (tag not consumed).
fn parse_until_tags(tokens: &[Token], i: &mut usize, stop: &[&str]) -> Result<Vec<Node>> {
    let mut out = Vec::new();
    while *i < tokens.len() {
        if let Some(kw) = peek_tag_keyword(tokens, *i) {
            if stop.iter().any(|s| *s == kw.as_str()) {
                break;
            }
        }
        out.push(parse_node(tokens, i)?);
    }
    Ok(out)
}

fn parse_if_chain(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `if` tag"));
    };
    let cond_s = strip_keyword_prefix(body, &["if"])?;
    let cond = parse_expr(cond_s)?;
    *i += 1;
    let mut branches = vec![IfBranch {
        cond: Some(cond),
        body: parse_until_tags(tokens, i, &["elif", "elseif", "else", "endif"])?,
    }];
    loop {
        if *i >= tokens.len() {
            return Err(RunjucksError::new(
                "unclosed `{% if %}`: missing `{% endif %}`",
            ));
        }
        let Token::Tag(b) = &tokens[*i] else {
            return Err(RunjucksError::new("internal parse state"));
        };
        let kw = first_tag_keyword(b);
        match kw.as_str() {
            "elif" | "elseif" => {
                let rest = strip_keyword_prefix(b, &["elif", "elseif"])?;
                let c = parse_expr(rest)?;
                *i += 1;
                branches.push(IfBranch {
                    cond: Some(c),
                    body: parse_until_tags(tokens, i, &["elif", "elseif", "else", "endif"])?,
                });
            }
            "else" => {
                *i += 1;
                let body = parse_until_tags(tokens, i, &["endif"])?;
                branches.push(IfBranch { cond: None, body });
                expect_tag(tokens, i, &["endif"])?;
                return Ok(Node::If { branches });
            }
            "endif" => {
                *i += 1;
                return Ok(Node::If { branches });
            }
            _ => {
                return Err(RunjucksError::new(format!(
                    "unexpected tag `{kw}` inside `if` block"
                )));
            }
        }
    }
}

fn parse_for_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `for` tag"));
    };
    let rest = strip_keyword_prefix(body, &["for"])?;
    let (var, iter) = parse_for_header(rest)?;
    *i += 1;
    let body = parse_until_tags(tokens, i, &["else", "endfor"])?;
    let else_body = if peek_tag_keyword(tokens, *i).as_deref() == Some("else") {
        *i += 1;
        Some(parse_until_tags(tokens, i, &["endfor"])?)
    } else {
        None
    };
    expect_tag(tokens, i, &["endfor"])?;
    Ok(Node::For {
        var,
        iter,
        body,
        else_body,
    })
}

fn parse_set_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `set` tag"));
    };
    let after = strip_keyword_prefix(body, &["set"])?;
    let (name, value) = parse_set_rhs(after)?;
    *i += 1;
    Ok(Node::Set { name, value })
}

fn parse_node(tokens: &[Token], i: &mut usize) -> Result<Node> {
    match &tokens[*i] {
        Token::Text(s) => {
            *i += 1;
            Ok(Node::Text(s.clone()))
        }
        Token::Expression(inner) => {
            let e = parse_expr(inner)?;
            *i += 1;
            Ok(Node::Output(vec![e]))
        }
        Token::Tag(body) => {
            let kw = first_tag_keyword(body);
            match kw.as_str() {
                "if" => parse_if_chain(tokens, i),
                "for" => parse_for_stmt(tokens, i),
                "set" => parse_set_stmt(tokens, i),
                "elif" | "elseif" | "else" | "endif" | "endfor" => Err(RunjucksError::new(
                    format!("unexpected `{{%{body}%}}` (no matching opening tag)"),
                )),
                _ => Err(RunjucksError::new(format!(
                    "unsupported tag keyword `{kw}`"
                ))),
            }
        }
    }
}

pub fn parse_template_tokens(tokens: &[Token]) -> Result<Node> {
    let mut i = 0usize;
    let mut nodes = Vec::new();
    while i < tokens.len() {
        nodes.push(parse_node(tokens, &mut i)?);
    }
    Ok(Node::Root(nodes))
}
