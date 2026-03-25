//! `{% … %}` statement parsing: `if` / `elif` / `else`, `for`, `set`, `include`, `import`, `from`, `switch`, `extends` (expression), `block`, `macro`.

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
    "endcall",
    "endfilter",
    "endblock",
    "endmacro",
    "endif",
    "endfor",
    "extends",
    "include",
    "import",
    "from",
    "macro",
    "call",
    "filter",
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

/// Byte index of `needle` in `rest` only when outside double-quoted regions (`\"` escapes respected).
fn find_keyword_outside_quotes(rest: &str, needle: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let nlen = needle.len();
    if nlen == 0 || rest.len() < nlen {
        return None;
    }
    let mut i = 0usize;
    let mut in_dq = false;
    let mut escaped = false;
    while i + nlen <= bytes.len() {
        if in_dq {
            if escaped {
                escaped = false;
                i += 1;
                continue;
            }
            if bytes[i] == b'\\' {
                escaped = true;
                i += 1;
                continue;
            }
            if bytes[i] == b'"' {
                in_dq = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_dq = true;
            i += 1;
            continue;
        }
        if rest[i..].starts_with(needle) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_simple_ident(rest: &str) -> Result<(String, &str)> {
    let s = rest.trim();
    let end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_alphanumeric() && *c != '_')
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 {
        return Err(RunjucksError::new("expected identifier"));
    }
    let name = s[..end].to_string();
    Ok((name, s[end..].trim_start()))
}

fn parse_import_as_suffix(rest_after_alias: &str) -> Result<Option<bool>> {
    let t = rest_after_alias.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if t == "without context" {
        return Ok(Some(false));
    }
    if t == "with context" {
        return Ok(Some(true));
    }
    Err(RunjucksError::new(format!(
        "expected end of tag, `with context`, or `without context`, found {t:?}"
    )))
}

fn split_comma_outside_quotes(s: &str) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let mut in_dq = false;
    let mut escaped = false;
    while i < bytes.len() {
        if in_dq {
            if escaped {
                escaped = false;
                i += 1;
                continue;
            }
            if bytes[i] == b'\\' {
                escaped = true;
                i += 1;
                continue;
            }
            if bytes[i] == b'"' {
                in_dq = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_dq = true;
            i += 1;
            continue;
        }
        if bytes[i] == b',' {
            out.push(s[start..i].trim());
            start = i + 1;
        }
        i += 1;
    }
    out.push(s[start..].trim());
    out
}

fn strip_trailing_with_context_segment(seg: &str) -> (&str, Option<bool>) {
    let t = seg.trim();
    if let Some(p) = t.strip_suffix("without context") {
        return (p.trim_end(), Some(false));
    }
    if let Some(p) = t.strip_suffix("with context") {
        return (p.trim_end(), Some(true));
    }
    (t, None)
}

fn parse_from_import_name_segment(seg: &str) -> Result<(String, Option<String>, Option<bool>)> {
    let (core, wc_seg) = strip_trailing_with_context_segment(seg);
    let core = core.trim();
    if let Some(idx) = find_keyword_outside_quotes(core, " as ") {
        let left = core[..idx].trim();
        let right = core[idx + 4..].trim();
        if left.starts_with('_') {
            return Err(RunjucksError::new(
                "names starting with an underscore cannot be imported",
            ));
        }
        let (export_name, tail_left) = parse_simple_ident(left)?;
        if !tail_left.trim().is_empty() {
            return Err(RunjucksError::new("invalid `from` import name"));
        }
        let (alias, tail) = parse_simple_ident(right)?;
        if !tail.trim().is_empty() {
            return Err(RunjucksError::new("unexpected tokens after `as` alias"));
        }
        return Ok((export_name, Some(alias), wc_seg));
    }
    let (name, tail) = parse_simple_ident(core)?;
    if !tail.trim().is_empty() {
        return Err(RunjucksError::new("unexpected tokens in `from` import list"));
    }
    if name.starts_with('_') {
        return Err(RunjucksError::new(
            "names starting with an underscore cannot be imported",
        ));
    }
    Ok((name, None, wc_seg))
}

fn parse_import_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `import` tag"));
    };
    let rest = strip_keyword_prefix(body, &["import"])?;
    let idx = find_keyword_outside_quotes(rest, " as ").ok_or_else(|| {
        RunjucksError::new("expected `{% import <expr> as <name> %}`")
    })?;
    let template_part = rest[..idx].trim();
    let after_as = rest[idx + 4..].trim();
    let template = parse_expr(template_part)?;
    let (alias, after_alias) = parse_simple_ident(after_as)?;
    let with_context = parse_import_as_suffix(after_alias)?;
    *i += 1;
    Ok(Node::Import {
        template,
        alias,
        with_context,
    })
}

fn parse_from_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `from` tag"));
    };
    let rest = strip_keyword_prefix(body, &["from"])?;
    let idx = find_keyword_outside_quotes(rest, " import ").ok_or_else(|| {
        RunjucksError::new("expected `{% from <expr> import <names> %}`")
    })?;
    let template_part = rest[..idx].trim();
    let list_part = rest[idx + " import ".len()..].trim();
    if list_part.is_empty() {
        return Err(RunjucksError::new(
            "expected at least one name in `from` import list",
        ));
    }
    let template = parse_expr(template_part)?;
    let segments = split_comma_outside_quotes(list_part);
    let mut names: Vec<(String, Option<String>)> = Vec::new();
    let mut with_context: Option<bool> = None;
    for seg in segments {
        if seg.is_empty() {
            continue;
        }
        let (export_name, alias, wc) = parse_from_import_name_segment(seg)?;
        names.push((export_name, alias));
        if let Some(w) = wc {
            with_context = Some(w);
        }
    }
    if names.is_empty() {
        return Err(RunjucksError::new(
            "expected at least one name in `from` import list",
        ));
    }
    *i += 1;
    Ok(Node::FromImport {
        template,
        names,
        with_context,
    })
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

fn split_top_level_commas(s: &str) -> Vec<&str> {
    if s.is_empty() {
        return Vec::new();
    }
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_string: Option<u8> = None;
    let mut escaped = false;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_string {
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
            if c == q {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            in_string = Some(c);
            i += 1;
            continue;
        }
        match c {
            b'(' => depth += 1,
            b')' => depth = (depth - 1).max(0),
            b',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(s[start..].trim());
    out
}

fn parse_filter_tag_header(rest: &str) -> Result<(String, Vec<Expr>)> {
    let s = rest.trim();
    if s.is_empty() {
        return Err(RunjucksError::new("`filter` requires a filter name"));
    }
    if let Some(open_paren) = s.find('(') {
        let name = s[..open_paren].trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(RunjucksError::new("invalid filter name"));
        }
        let after = &s[open_paren + 1..];
        let close = after
            .rfind(')')
            .ok_or_else(|| RunjucksError::new("unclosed `)` in `filter` tag"))?;
        let inner = after[..close].trim();
        let tail = after[close + 1..].trim();
        if !tail.is_empty() {
            return Err(RunjucksError::new("unexpected tokens after `filter` call"));
        }
        let segs = split_top_level_commas(inner);
        let args: Vec<Expr> = if segs.len() == 1 && segs[0].is_empty() {
            vec![]
        } else {
            segs
                .into_iter()
                .map(|t| parse_expr(t))
                .collect::<Result<_>>()?
        };
        return Ok((name.to_string(), args));
    }
    let (name, tail) = parse_simple_ident(s)?;
    if !tail.is_empty() {
        return Err(RunjucksError::new("unexpected tokens after filter name"));
    }
    Ok((name, vec![]))
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
    let parent = parse_expr(rest.trim())?;
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

fn parse_filter_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `filter` tag"));
    };
    let rest = strip_keyword_prefix(body, &["filter"])?;
    let (name, args) = parse_filter_tag_header(rest)?;
    *i += 1;
    let body = parse_until_tags(tokens, i, &["endfilter"])?;
    expect_tag(tokens, i, &["endfilter"])?;
    Ok(Node::FilterBlock { name, args, body })
}

fn parse_call_stmt(tokens: &[Token], i: &mut usize) -> Result<Node> {
    let Token::Tag(body) = &tokens[*i] else {
        return Err(RunjucksError::new("internal: expected `call` tag"));
    };
    let rest = strip_keyword_prefix(body, &["call"])?;
    let callee = parse_expr(rest)?;
    *i += 1;
    let body = parse_until_tags(tokens, i, &["endcall"])?;
    expect_tag(tokens, i, &["endcall"])?;
    Ok(Node::CallBlock { callee, body })
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

/// `{% raw %}…{% endraw %}` / `{% verbatim %}…{% endverbatim %}` — lexer emits literal [`Token::Text`]
/// between opening and closing tags (including nested inner raw/verbatim pairs).
fn parse_raw_block(tokens: &[Token], i: &mut usize, close_tags: &[&str]) -> Result<Node> {
    let Some(Token::Tag(_)) = tokens.get(*i) else {
        return Err(RunjucksError::new("internal: expected raw/verbatim tag"));
    };
    *i += 1;
    let mut content = String::new();
    if let Some(Token::Text(s)) = tokens.get(*i) {
        content = s.clone();
        *i += 1;
    }
    expect_tag(tokens, i, close_tags)?;
    Ok(Node::Text(content))
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
                "import" => parse_import_stmt(tokens, i),
                "from" => parse_from_stmt(tokens, i),
                "extends" => parse_extends_stmt(tokens, i),
                "block" => parse_block_stmt(tokens, i),
                "filter" => parse_filter_stmt(tokens, i),
                "call" => parse_call_stmt(tokens, i),
                "macro" => parse_macro_stmt(tokens, i),
                "raw" => parse_raw_block(tokens, i, &["endraw"]),
                "verbatim" => parse_raw_block(tokens, i, &["endverbatim"]),
                "elif" | "elseif" | "else" | "endif" | "endfor" | "endblock" | "endmacro"
                | "endfilter" | "endcall" | "case" | "default" | "endswitch" | "endset"
                | "endraw" | "endverbatim" => {
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
