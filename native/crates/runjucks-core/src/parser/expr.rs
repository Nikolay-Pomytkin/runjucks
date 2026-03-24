//! Expression parsing for `{{ … }}` bodies (Nunjucks-style precedence).
//!
//! Reference: `nunjucks/nunjucks/src/parser.js` (`parseExpression` → `parseOr` → … → `parsePrimary`).

use crate::ast::{BinOp, CompareOp, Expr, UnaryOp};
use crate::errors::{Result, RunjucksError};
use nom::branch::alt;
use nom::character::complete::{char, digit1};
use nom::combinator::{all_consuming, map_res, opt, recognize};
use nom::IResult;
use nom::Parser;
use serde_json::{json, Value};

fn trim_start(s: &str) -> &str {
    s.trim_start()
}

/// True if `s` has a full ASCII keyword of length `kw_len` at the start (not a prefix of a longer identifier).
fn keyword_boundary(s: &str, kw_len: usize) -> bool {
    s.as_bytes()
        .get(kw_len)
        .is_none_or(|&b| !b.is_ascii_alphanumeric() && b != b'_')
}

fn parse_keyword<'a>(input: &'a str, kw: &str) -> Option<&'a str> {
    let input = trim_start(input);
    if input.starts_with(kw) && keyword_boundary(input, kw.len()) {
        Some(&input[kw.len()..])
    } else {
        None
    }
}

fn parse_string(input: &str) -> IResult<&str, String> {
    let input = trim_start(input);
    let mut chars = input.chars();
    let quote = match chars.next() {
        Some('"') => '"',
        Some('\'') => '\'',
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
    };
    let rest = &input[quote.len_utf8()..];
    let mut out = String::new();
    let mut i = 0usize;
    let rest_bytes = rest.as_bytes();
    while i < rest_bytes.len() {
        let c = rest[i..].chars().next().unwrap();
        if c == quote {
            return Ok((&rest[i + c.len_utf8()..], out));
        }
        if c == '\\' {
            i += 1;
            let Some(esc) = rest.get(i..).and_then(|s| s.chars().next()) else {
                return Err(nom::Err::Failure(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Escaped,
                )));
            };
            match esc {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                q if q == quote => out.push(q),
                _ => out.push(esc),
            }
            i += esc.len_utf8();
            continue;
        }
        out.push(c);
        i += c.len_utf8();
    }
    Err(nom::Err::Failure(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

fn parse_number(input: &str) -> IResult<&str, Value> {
    let input = trim_start(input);
    map_res(
        recognize((
            opt(char('-')),
            alt((
                recognize((digit1, char('.'), digit1)),
                digit1,
            )),
        )),
        |s: &str| -> std::result::Result<Value, ()> {
            if s.contains('.') {
                s.parse::<f64>().map(|x| json!(x)).map_err(|_| ())
            } else if let Ok(n) = s.parse::<i64>() {
                Ok(json!(n))
            } else {
                s.parse::<f64>().map(|x| json!(x)).map_err(|_| ())
            }
        },
    )
    .parse(input)
}

fn parse_bool_or_none(input: &str) -> IResult<&str, Value> {
    let input = trim_start(input);
    if input.starts_with("true") && keyword_boundary(input, 4) {
        return Ok((&input[4..], json!(true)));
    }
    if input.starts_with("false") && keyword_boundary(input, 5) {
        return Ok((&input[5..], json!(false)));
    }
    if input.starts_with("none") && keyword_boundary(input, 4) {
        return Ok((&input[4..], Value::Null));
    }
    if input.starts_with("null") && keyword_boundary(input, 4) {
        return Ok((&input[4..], Value::Null));
    }
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

fn parse_identifier(input: &str) -> IResult<&str, String> {
    let input = trim_start(input);
    let mut chars = input.chars();
    let first = match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => c,
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
    };
    let len_first = first.len_utf8();
    let take = input[len_first..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .map(|c| c.len_utf8())
        .sum::<usize>();
    let end = len_first + take;
    Ok((&input[end..], input[..end].to_string()))
}

/// `ident` or `foo.bar` filter names (Nunjucks [`parseFilterName`](https://github.com/mozilla/nunjucks/blob/master/nunjucks/src/parser.js)).
fn parse_filter_name(input: &str) -> IResult<&str, String> {
    let (mut rest, mut name) = parse_identifier(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('.') {
            let (r3, part) = parse_identifier(trim_start(r2))?;
            name.push('.');
            name.push_str(&part);
            rest = r3;
        } else {
            break;
        }
    }
    Ok((rest, name))
}

fn parse_call_argument_list(input: &str) -> IResult<&str, Vec<Expr>> {
    let mut rest = trim_start(input);
    if let Some(r) = rest.strip_prefix(')') {
        return Ok((r, vec![]));
    }
    let mut args = Vec::new();
    loop {
        let (r, e) = parse_inline_if(rest)?;
        args.push(e);
        let r = trim_start(r);
        if let Some(r2) = r.strip_prefix(')') {
            return Ok((r2, args));
        }
        let r = r.strip_prefix(',').ok_or_else(|| {
            nom::Err::Failure(nom::error::Error::new(
                r,
                nom::error::ErrorKind::Tag,
            ))
        })?;
        rest = trim_start(r);
    }
}

fn parse_postfix(input: &str, mut node: Expr) -> IResult<&str, Expr> {
    let mut rest = input;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('.') {
            let (r3, attr) = parse_identifier(trim_start(r2))?;
            node = Expr::GetAttr {
                base: Box::new(node),
                attr,
            };
            rest = r3;
            continue;
        }
        if let Some(r2) = r.strip_prefix('[') {
            let (r3, idx) = parse_inline_if(trim_start(r2))?;
            let r3 = trim_start(r3);
            let r4 = r3.strip_prefix(']').ok_or_else(|| {
                nom::Err::Failure(nom::error::Error::new(
                    r3,
                    nom::error::ErrorKind::Tag,
                ))
            })?;
            node = Expr::GetItem {
                base: Box::new(node),
                index: Box::new(idx),
            };
            rest = r4;
            continue;
        }
        if let Some(r2) = r.strip_prefix('(') {
            let (r3, args) = parse_call_argument_list(r2)?;
            node = Expr::Call {
                callee: Box::new(node),
                args,
            };
            rest = r3;
            continue;
        }
        break;
    }
    Ok((rest, node))
}

fn parse_list_literal(input: &str) -> IResult<&str, Expr> {
    let input = trim_start(input);
    let after = input.strip_prefix('[').ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        ))
    })?;
    let mut rest = trim_start(after);
    if let Some(r) = rest.strip_prefix(']') {
        return Ok((r, Expr::List(vec![])));
    }
    let mut items = Vec::new();
    loop {
        let (r, e) = parse_inline_if(rest)?;
        items.push(e);
        let r = trim_start(r);
        if let Some(r2) = r.strip_prefix(']') {
            return Ok((r2, Expr::List(items)));
        }
        let r = r.strip_prefix(',').ok_or_else(|| {
            nom::Err::Failure(nom::error::Error::new(
                r,
                nom::error::ErrorKind::Tag,
            ))
        })?;
        rest = trim_start(r);
    }
}

fn parse_dict_literal(input: &str) -> IResult<&str, Expr> {
    let input = trim_start(input);
    let after = input.strip_prefix('{').ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        ))
    })?;
    let mut rest = trim_start(after);
    if let Some(r) = rest.strip_prefix('}') {
        return Ok((r, Expr::Dict(vec![])));
    }
    let mut pairs = Vec::new();
    loop {
        let (r, k) = parse_inline_if(rest)?;
        let r = trim_start(r);
        let r = r.strip_prefix(':').ok_or_else(|| {
            nom::Err::Failure(nom::error::Error::new(
                r,
                nom::error::ErrorKind::Tag,
            ))
        })?;
        let (r, v) = parse_inline_if(trim_start(r))?;
        pairs.push((k, v));
        let r = trim_start(r);
        if let Some(r2) = r.strip_prefix('}') {
            return Ok((r2, Expr::Dict(pairs)));
        }
        let r = r.strip_prefix(',').ok_or_else(|| {
            nom::Err::Failure(nom::error::Error::new(
                r,
                nom::error::ErrorKind::Tag,
            ))
        })?;
        rest = trim_start(r);
    }
}

fn parse_atom(input: &str) -> IResult<&str, Expr> {
    let input = trim_start(input);
    if let Some(after) = input.strip_prefix('(') {
        let (rest, e) = parse_inline_if(after)?;
        let rest = trim_start(rest);
        let r = rest.strip_prefix(')').ok_or_else(|| {
            nom::Err::Failure(nom::error::Error::new(
                rest,
                nom::error::ErrorKind::Tag,
            ))
        })?;
        return Ok((r, e));
    }
    if input.starts_with('[') {
        return parse_list_literal(input);
    }
    if input.starts_with('{') {
        return parse_dict_literal(input);
    }
    if let Ok((rest, v)) = parse_string(input) {
        return Ok((rest, Expr::Literal(json!(v))));
    }
    if let Ok((rest, v)) = parse_bool_or_none(input) {
        return Ok((rest, Expr::Literal(v)));
    }
    if let Ok((rest, v)) = parse_number(input) {
        return Ok((rest, Expr::Literal(v)));
    }
    let (rest, name) = parse_identifier(input)?;
    Ok((rest, Expr::Variable(name)))
}

fn parse_atom_with_postfix(input: &str) -> IResult<&str, Expr> {
    let (rest, atom) = parse_atom(input)?;
    parse_postfix(rest, atom)
}

fn parse_filter_chain(input: &str, mut node: Expr) -> IResult<&str, Expr> {
    let mut rest = input;
    loop {
        let r = trim_start(rest);
        if !r.starts_with('|') {
            break;
        }
        let after = &r[1..];
        let after = trim_start(after);
        let (r2, name) = parse_filter_name(after)?;
        let after_name = trim_start(r2);
        let (r3, extra_args) = if let Some(inner) = after_name.strip_prefix('(') {
            parse_call_argument_list(inner)?
        } else {
            (after_name, vec![])
        };
        node = Expr::Filter {
            name,
            input: Box::new(node),
            args: extra_args,
        };
        rest = r3;
    }
    Ok((rest, node))
}

fn parse_unary_no_filters(input: &str) -> IResult<&str, Expr> {
    let t = trim_start(input);
    if let Some(rest) = parse_keyword(t, "not") {
        let (rest, e) = parse_unary_no_filters(rest)?;
        return Ok((
            rest,
            Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(e),
            },
        ));
    }
    if t.starts_with('-')
        && t
            .as_bytes()
            .get(1)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
    {
        let (rest, v) = parse_number(t)?;
        return Ok((rest, Expr::Literal(v)));
    }
    if let Some(rest) = t.strip_prefix('-') {
        let (rest, e) = parse_unary_no_filters(rest)?;
        return Ok((
            rest,
            Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(e),
            },
        ));
    }
    if let Some(rest) = t.strip_prefix('+') {
        let (rest, e) = parse_unary_no_filters(rest)?;
        return Ok((
            rest,
            Expr::Unary {
                op: UnaryOp::Pos,
                expr: Box::new(e),
            },
        ));
    }
    parse_atom_with_postfix(input)
}

fn parse_unary(input: &str) -> IResult<&str, Expr> {
    let (rest, e) = parse_unary_no_filters(input)?;
    parse_filter_chain(rest, e)
}

fn parse_pow(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_unary(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix("**") {
            rest = r2;
            let (r2, rhs) = parse_unary(rest)?;
            rest = r2;
            acc = Expr::Binary {
                op: BinOp::Pow,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_mod(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_pow(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('%') {
            if r2.starts_with('%') {
                break;
            }
            let (r3, rhs) = parse_pow(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::Mod,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_floor_div(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_mod(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix("//") {
            let (r3, rhs) = parse_mod(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::FloorDiv,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_div(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_floor_div(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('/') {
            if r2.starts_with('/') {
                break;
            }
            let (r3, rhs) = parse_floor_div(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::Div,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_mul(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_div(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('*') {
            if r2.starts_with('*') {
                break;
            }
            let (r3, rhs) = parse_div(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::Mul,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_sub(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_mul(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('-') {
            let (r3, rhs) = parse_mul(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::Sub,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_add(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_sub(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('+') {
            let (r3, rhs) = parse_sub(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::Add,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_concat(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_add(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = r.strip_prefix('~') {
            let (r3, rhs) = parse_add(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::Concat,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_compare_op(rest: &str) -> Option<(CompareOp, usize)> {
    if rest.starts_with("===") {
        Some((CompareOp::StrictEq, 3))
    } else if rest.starts_with("!==") {
        Some((CompareOp::StrictNe, 3))
    } else if rest.starts_with("==") {
        Some((CompareOp::Eq, 2))
    } else if rest.starts_with("!=") {
        Some((CompareOp::Ne, 2))
    } else if rest.starts_with("<=") {
        Some((CompareOp::Le, 2))
    } else if rest.starts_with(">=") {
        Some((CompareOp::Ge, 2))
    } else if rest.starts_with('<') {
        Some((CompareOp::Lt, 1))
    } else if rest.starts_with('>') {
        Some((CompareOp::Gt, 1))
    } else {
        None
    }
}

fn parse_compare(input: &str) -> IResult<&str, Expr> {
    let (mut rest, head) = parse_concat(input)?;
    let mut rest_vec: Vec<(CompareOp, Expr)> = Vec::new();
    loop {
        let r = trim_start(rest);
        if let Some((op, len)) = parse_compare_op(r) {
            let after = &r[len..];
            let (r2, rhs) = parse_concat(after)?;
            rest = r2;
            rest_vec.push((op, rhs));
            continue;
        }
        break;
    }
    if rest_vec.is_empty() {
        Ok((rest, head))
    } else {
        Ok((
            rest,
            Expr::Compare {
                head: Box::new(head),
                rest: rest_vec,
            },
        ))
    }
}

fn parse_is(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_compare(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = parse_keyword(r, "is") {
            let mut after = trim_start(r2);
            let mut negated = false;
            if let Some(r3) = parse_keyword(after, "not") {
                negated = true;
                after = trim_start(r3);
            }
            let (r3, rhs) = parse_compare(after)?;
            rest = r3;
            let node = Expr::Binary {
                op: BinOp::Is,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            acc = if negated {
                Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(node),
                }
            } else {
                node
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_in(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_is(input)?;
    loop {
        let r = trim_start(rest);
        let (invert, after_not) = if let Some(r2) = parse_keyword(r, "not") {
            let r3 = trim_start(r2);
            if let Some(r4) = parse_keyword(r3, "in") {
                (true, r4)
            } else {
                break;
            }
        } else if let Some(r2) = parse_keyword(r, "in") {
            (false, r2)
        } else {
            break;
        };
        let (r2, rhs) = parse_is(after_not)?;
        rest = r2;
        let mut node = Expr::Binary {
            op: BinOp::In,
            left: Box::new(acc),
            right: Box::new(rhs),
        };
        if invert {
            node = Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(node),
            };
        }
        acc = node;
    }
    Ok((rest, acc))
}

fn parse_and(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_in(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = parse_keyword(r, "and") {
            let (r3, rhs) = parse_in(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::And,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

fn parse_or(input: &str) -> IResult<&str, Expr> {
    let (mut rest, mut acc) = parse_and(input)?;
    loop {
        let r = trim_start(rest);
        if let Some(r2) = parse_keyword(r, "or") {
            let (r3, rhs) = parse_and(r2)?;
            rest = r3;
            acc = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(acc),
                right: Box::new(rhs),
            };
            continue;
        }
        break;
    }
    Ok((rest, acc))
}

pub(crate) fn parse_inline_if(input: &str) -> IResult<&str, Expr> {
    let (rest, first) = parse_or(input)?;
    let r = trim_start(rest);
    if let Some(r2) = parse_keyword(r, "if") {
        let (r3, cond) = parse_or(r2)?;
        let r3 = trim_start(r3);
        let (rest, else_expr) = if let Some(r4) = parse_keyword(r3, "else") {
            let (r5, e) = parse_or(r4)?;
            (r5, Some(e))
        } else {
            (r3, None)
        };
        return Ok((
            rest,
            Expr::InlineIf {
                cond: Box::new(cond),
                then_expr: Box::new(first),
                else_expr: else_expr.map(Box::new),
            },
        ));
    }
    Ok((rest, first))
}

/// Parses a full `{{ … }}` body (must consume all non-whitespace).
pub fn parse_expression(source: &str) -> Result<Expr> {
    let s = source.trim();
    if s.is_empty() {
        return Err(RunjucksError::new(
            "empty expression inside `{{ }}` is not allowed",
        ));
    }
    match all_consuming(parse_inline_if).parse(s) {
        Ok((_, expr)) => Ok(expr),
        Err(e) => Err(RunjucksError::new(format!("expression parse error: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BinOp;

    #[test]
    fn precedence_mul_before_add() {
        let e = parse_expression("2 + 3 * 4").unwrap();
        match e {
            Expr::Binary {
                op: BinOp::Add,
                left,
                right,
            } => {
                assert!(matches!(*left, Expr::Literal(_)));
                assert!(matches!(
                    *right,
                    Expr::Binary {
                        op: BinOp::Mul,
                        ..
                    }
                ));
            }
            _ => panic!("unexpected {:?}", e),
        }
    }
}
