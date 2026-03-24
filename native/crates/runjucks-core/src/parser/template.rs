//! `{% … %}` statement parsing: `if` / `elif` / `else`, `for`, `set`, `include`, `switch`, `extends`, `block`, `macro`.

#![allow(clippy::manual_contains)]

use crate::ast::{Expr, ForVars, IfBranch, MacroDef, Node, SwitchCase};
use crate::errors::{Result, RunjucksError};
use crate::lexer::Token;
use crate::parser::parse_expr;

/// Longer keywords first so `elseif` does not match as `else`, `endblock` before `block`, etc.
const TAG_KEYWORDS: &[&str] = &[
    "endswitch",
    "endset",
    "elseif",
    "elif",
    "endblock",
    "endmacro",
    "endif",
    "endfor",
    "extends",
    "include",
    "macro",
    "block",
    "switch",
    "case",
    "default",
    "for",
    "set",
    "if",
    "else",
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

fn parse_for_header(rest: &str) -> Result<(ForVars, Expr)> {
    let s = rest.trim();
    let pos = s
        .find(" in ")
        .ok_or_else(|| RunjucksError::new("expected `for <names> in <expr>`"))?;
    let left = s[..pos].trim();
    let right = s[pos + 4..].trim();
    let names: Vec<String> = left
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if names.is_empty() {
        return Err(RunjucksError::new("`for` requires at least one variable"));
    }
    for n in &names {
        if !n
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(RunjucksError::new("invalid `for` variable name"));
        }
    }
    let vars = if names.len() == 1 {
        ForVars::Single(names[0].clone())
    } else {
        ForVars::Multi(names)
    };
    Ok((vars, parse_expr(right)?))
}

fn find_set_equals(after_set: &str) -> Option<usize> {
    let bytes = after_set.as_bytes();
    let mut i = 0usize;
    let mut in_dq = false;
    let mut escaped = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_dq {
            if escaped {
                escaped = false;
                i += 1;
                continue;
            }
            if c == b'\\' {
                escaped = true;
                i += 1;
                continue;
            }
            if c == b'"' {
                in_dq = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_dq = true;
            i += 1;
            continue;
        }
        if c == b'=' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_set_targets(lhs: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for part in lhs.split(',') {
        let name = part.trim();
        if name.is_empty() {
            continue;
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(RunjucksError::new("`set` target must be an identifier"));
        }
        out.push(name.to_string());
    }
    if out.is_empty() {
        return Err(RunjucksError::new("expected at least one `set` target"));
    }
    Ok(out)
}

fn strip_ignore_missing(rest: &str) -> (&str, bool) {
    let t = rest.trim_end();
    const SUF: &[u8] = b"ignore missing";
    let b = t.as_bytes();
    if b.len() >= SUF.len() && b[b.len() - SUF.len()..].eq_ignore_ascii_case(SUF) {
        let main = t[..t.len() - SUF.len()].trim_end();
        (main, true)
    } else {
        (t, false)
    }
}

/// Parses a quoted template path (`"a.html"` or `'a.html'`).
fn parse_quoted_path(rest: &str) -> Result<String> {
    let s = rest.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return Ok(s[1..s.len() - 1].to_string());
    }
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        return Ok(s[1..s.len() - 1].to_string());
    }
    Err(RunjucksError::new("expected quoted template path"))
}

fn parse_block_name(rest: &str) -> Result<String> {
    let mut it = rest.trim().split_whitespace();
    let name = it
        .next()
        .ok_or_else(|| RunjucksError::new("`block` requires a name"))?;
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(RunjucksError::new("invalid `block` name"));
    }
    if it.next().is_some() {
        return Err(RunjucksError::new("unexpected tokens after `block` name"));
    }
    Ok(name.to_string())
}

fn parse_macro_header(rest: &str) -> Result<(String, Vec<String>)> {
    let rest = rest.trim();
    let open = rest
        .find('(')
        .ok_or_else(|| RunjucksError::new("expected `macro name(...)`"))?;
    let name_part = rest[..open].trim();
    if name_part.is_empty()
        || !name_part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(RunjucksError::new("invalid macro name"));
    }
    let close = rest
        .rfind(')')
        .ok_or_else(|| RunjucksError::new("unclosed `)` in macro"))?;
    let inner = rest[open + 1..close].trim();
    let params: Vec<String> = if inner.is_empty() {
        vec![]
    } else {
        inner
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let tail = rest[close + 1..].trim();
    if !tail.is_empty() {
        return Err(RunjucksError::new("unexpected tokens after macro header"));
    }
    Ok((name_part.to_string(), params))
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
    let (vars, iter) = parse_for_header(rest)?;
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
        vars,
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
    if let Some(eq_pos) = find_set_equals(after) {
        let lhs = after[..eq_pos].trim();
        let rhs = after[eq_pos + 1..].trim();
        let targets = parse_set_targets(lhs)?;
        let value = parse_expr(rhs)?;
        *i += 1;
        return Ok(Node::Set {
            targets,
            value: Some(value),
            body: None,
        });
    }
    let targets = parse_set_targets(after.trim())?;
    if targets.len() != 1 {
        return Err(RunjucksError::new(
            "block `{% set %}` form allows only one target",
        ));
    }
    *i += 1;
    let body = parse_until_tags(tokens, i, &["endset"])?;
    expect_tag(tokens, i, &["endset"])?;
    Ok(Node::Set {
        targets,
        value: None,
        body: Some(body),
    })
}

fn parse_include_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `include` tag"));
    };
    let rest0 = strip_keyword_prefix(body, &["include"])?;
    let (rest, ignore_missing) = strip_ignore_missing(rest0);
    let template = parse_expr(rest.trim())?;
    *i += 1;
    Ok(Node::Include {
        template,
        ignore_missing,
    })
}

fn parse_switch_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `switch` tag"));
    };
    let rest = strip_keyword_prefix(body, &["switch"])?;
    let expr = parse_expr(rest)?;
    *i += 1;
    let mut cases: Vec<SwitchCase> = Vec::new();
    let mut default_body: Option<Vec<Node>> = None;
    loop {
        if *i >= tokens.len() {
            return Err(RunjucksError::new(
                "unclosed `{% switch %}`: missing `{% endswitch %}`",
            ));
        }
        let kw = peek_tag_keyword(tokens, *i)
            .ok_or_else(|| RunjucksError::new("expected tag inside `switch`"))?;
        match kw.as_str() {
            "case" => {
                let Token::Tag(b) = &tokens[*i] else {
                    return Err(RunjucksError::new("internal parse state"));
                };
                let r = strip_keyword_prefix(b, &["case"])?;
                let cond = parse_expr(r)?;
                *i += 1;
                let body = parse_until_tags(tokens, i, &["case", "default", "endswitch"])?;
                cases.push(SwitchCase { cond, body });
            }
            "default" => {
                *i += 1;
                let body = parse_until_tags(tokens, i, &["endswitch"])?;
                default_body = Some(body);
                expect_tag(tokens, i, &["endswitch"])?;
                return Ok(Node::Switch {
                    expr,
                    cases,
                    default_body,
                });
            }
            "endswitch" => {
                *i += 1;
                return Ok(Node::Switch {
                    expr,
                    cases,
                    default_body,
                });
            }
            _ => {
                return Err(RunjucksError::new(format!(
                    "unexpected tag `{kw}` inside `switch`"
                )));
            }
        }
    }
}

fn parse_extends_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `extends` tag"));
    };
    let rest = strip_keyword_prefix(body, &["extends"])?;
    let parent = parse_quoted_path(rest)?;
    *i += 1;
    Ok(Node::Extends { parent })
}

fn parse_block_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `block` tag"));
    };
    let rest = strip_keyword_prefix(body, &["block"])?;
    let block_name = parse_block_name(rest)?;
    *i += 1;
    let body = parse_until_tags(tokens, i, &["endblock"])?;
    let Token::Tag(eb) = &tokens[*i] else {
        return Err(RunjucksError::new("expected `{% endblock %}`"));
    };
    let after = strip_keyword_prefix(eb, &["endblock"])?;
    let end_name = after.trim();
    if !end_name.is_empty() && end_name != block_name {
        return Err(RunjucksError::new(format!(
            "`endblock` name `{end_name}` does not match `block {block_name}`"
        )));
    }
    *i += 1;
    Ok(Node::Block {
        name: block_name,
        body,
    })
}

fn parse_macro_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `macro` tag"));
    };
    let rest = strip_keyword_prefix(body, &["macro"])?;
    let (name, params) = parse_macro_header(rest)?;
    *i += 1;
    let body = parse_until_tags(tokens, i, &["endmacro"])?;
    expect_tag(tokens, i, &["endmacro"])?;
    Ok(Node::MacroDef(MacroDef { name, params, body }))
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
                "switch" => parse_switch_stmt(tokens, i),
                "set" => parse_set_stmt(tokens, i),
                "include" => parse_include_stmt(tokens, i),
                "extends" => parse_extends_stmt(tokens, i),
                "block" => parse_block_stmt(tokens, i),
                "macro" => parse_macro_stmt(tokens, i),
                "elif" | "elseif" | "else" | "endif" | "endfor" | "endblock" | "endmacro"
                | "case" | "default" | "endswitch" | "endset" => {
                    Err(RunjucksError::new(format!(
                        "unexpected `{{%{body}%}}` (no matching opening tag)"
                    )))
                }
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
