//! Tokenization: splits template source into [`Token`]s for [`crate::parser::parse`].
//!
//! Recognized regions:
//! - `{#` … `#}` — comments (omitted from output).
//! - `{%` / `{%-` … `%}` / `-%}` — statement tags as [`Token::Tag`] (inner body is whitespace-trimmed).
//! - `{{` / `{{-` … `}}` / `-}}` — expressions as [`Token::Expression`] (inner spaces preserved unless trim markers strip them).
//!
//! `{% raw %}…{% endraw %}` and `{% verbatim %}…{% endverbatim %}` treat the middle as literal [`Token::Text`].

use crate::errors::{Result, RunjucksError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenKind {
    Comment,
    Tag { trim_open: bool },
    Var { trim_open: bool },
}

fn next_opener(rest: &str) -> Option<(usize, OpenKind)> {
    let mut best: Option<(usize, OpenKind)> = None;
    for (i, _) in rest.char_indices() {
        let s = &rest[i..];
        let candidate = if s.starts_with("{#") {
            Some((i, OpenKind::Comment))
        } else if s.starts_with("{%-") {
            Some((i, OpenKind::Tag { trim_open: true }))
        } else if s.starts_with("{%") {
            Some((i, OpenKind::Tag { trim_open: false }))
        } else if s.starts_with("{{-") {
            Some((i, OpenKind::Var { trim_open: true }))
        } else if s.starts_with("{{") {
            Some((i, OpenKind::Var { trim_open: false }))
        } else {
            None
        };
        if let Some((idx, kind)) = candidate {
            best = match best {
                None => Some((idx, kind)),
                Some((bi, _)) if idx < bi => Some((idx, kind)),
                Some(b) => Some(b),
            };
        }
    }
    best
}

fn parse_tag_prefix(rest: &str) -> Result<(String, usize)> {
    let open_len = if rest.starts_with("{%-") {
        3
    } else if rest.starts_with("{%") {
        2
    } else {
        return Err(RunjucksError::new("internal lexer error: expected `{%`"));
    };
    let after_open = &rest[open_len..];
    let (body_end, close_len) = find_tag_close(after_open)?;
    let body = after_open[..body_end].trim().to_string();
    let total = open_len + body_end + close_len;
    Ok((body, total))
}

/// Byte-scan so `{%` inside verbatim/raw is not mistaken for a tag unless it completes `{% endraw %}` / etc.
fn find_closing_tag_open(rest: &str, end_name: &str) -> Option<usize> {
    let prefix = format!("{end_name} ");
    let mut i = 0;
    while i < rest.len() {
        if rest[i..].starts_with("{%") || rest[i..].starts_with("{%-") {
            if let Ok((body, _)) = parse_tag_prefix(&rest[i..]) {
                if body == end_name || body.starts_with(&prefix) {
                    return Some(i);
                }
            }
        }
        i += 1;
    }
    None
}

fn find_var_close(after_open: &str) -> Result<(usize, usize)> {
    let mut i = 0;
    while i < after_open.len() {
        if after_open[i..].starts_with("{{") {
            return Err(RunjucksError::new(
                "nested `{{` inside a variable expression is not allowed",
            ));
        }
        let trim_close = after_open[i..].starts_with("-}}");
        let plain_close = after_open[i..].starts_with("}}");
        if trim_close || plain_close {
            if after_open[..i].contains("{{") {
                return Err(RunjucksError::new(
                    "nested `{{` inside a variable expression is not allowed",
                ));
            }
            let close_len = if trim_close { 3 } else { 2 };
            return Ok((i, close_len));
        }
        i += 1;
    }
    Err(RunjucksError::new(
        "unclosed variable tag: expected `}}` or `-}}` after `{{`",
    ))
}

fn find_tag_close(after_open: &str) -> Result<(usize, usize)> {
    let mut i = 0;
    while i < after_open.len() {
        let trim_close = after_open[i..].starts_with("-%}");
        let plain_close = after_open[i..].starts_with("%}");
        if trim_close || plain_close {
            let close_len = if trim_close { 3 } else { 2 };
            return Ok((i, close_len));
        }
        i += 1;
    }
    Err(RunjucksError::new(
        "unclosed template tag: expected `%}` or `-%}` after `{%`",
    ))
}

fn apply_var_trim(body: &str, trim_open: bool, trim_close: bool) -> String {
    let mut s = body;
    if trim_open {
        s = s.trim_start();
    }
    if trim_close {
        s = s.trim_end();
    }
    s.to_string()
}

/// One lexical unit from a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Text(String),
    Expression(String),
    Tag(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LexerMode {
    Normal,
    Raw,
    Verbatim,
}

/// Incremental lexer over a template string.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    mode: LexerMode,
    pending: Option<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            mode: LexerMode::Normal,
            pending: None,
        }
    }

    #[inline]
    pub fn rest(&self) -> &'a str {
        self.input.get(self.position..).unwrap_or("")
    }

    #[inline]
    pub fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    fn skip_comment(&mut self) -> Result<()> {
        let rest = self.rest();
        if !rest.starts_with("{#") {
            return Err(RunjucksError::new("internal lexer error: expected `{#`"));
        }
        let Some(end_rel) = rest.find("#}") else {
            return Err(RunjucksError::new(
                "unclosed comment: expected `#}` after `{#`",
            ));
        };
        self.position += end_rel + "#}".len();
        Ok(())
    }

    fn consume_variable(&mut self, trim_open: bool) -> Result<Token> {
        let rest = self.rest();
        let open_len = if rest.starts_with("{{-") { 3 } else { 2 };
        self.position += open_len;
        let after_open = self.rest();
        let (body_end, close_len) = find_var_close(after_open)?;
        let trim_close = after_open[body_end..].starts_with("-}}");
        let body = &after_open[..body_end];
        let expr = apply_var_trim(body, trim_open, trim_close);
        self.position += body_end + close_len;
        Ok(Token::Expression(expr))
    }

    fn consume_tag_at_position(&mut self) -> Result<String> {
        let rest = self.rest();
        let (body, total) = parse_tag_prefix(rest)?;
        self.position += total;
        Ok(body)
    }

    fn end_tag_name(mode: LexerMode) -> &'static str {
        match mode {
            LexerMode::Raw => "endraw",
            LexerMode::Verbatim => "endverbatim",
            LexerMode::Normal => "",
        }
    }

    fn next_token_block_mode(&mut self) -> Result<Option<Token>> {
        let mode = self.mode;
        let end_name = Self::end_tag_name(mode);
        let rest = self.rest();
        let idx = find_closing_tag_open(rest, end_name).ok_or_else(|| {
            RunjucksError::new(format!(
                "unclosed {end_name} block: expected matching `%}}` tag"
            ))
        })?;
        let literal = rest[..idx].to_string();
        self.position += idx;
        let rest2 = self.rest();
        let (body, total) = parse_tag_prefix(rest2)?;
        self.position += total;
        self.mode = LexerMode::Normal;
        if !literal.is_empty() {
            self.pending = Some(Token::Tag(body));
            return Ok(Some(Token::Text(literal)));
        }
        Ok(Some(Token::Tag(body)))
    }

    fn next_token_normal(&mut self) -> Result<Option<Token>> {
        loop {
            if self.is_eof() {
                return Ok(None);
            }

            let rest = self.rest();

            match next_opener(rest) {
                None => {
                    let text = rest.to_owned();
                    self.position = self.input.len();
                    return Ok(Some(Token::Text(text)));
                }
                Some((0, OpenKind::Comment)) => {
                    self.skip_comment()?;
                    continue;
                }
                Some((0, OpenKind::Tag { .. })) => {
                    let body = self.consume_tag_at_position()?;
                    if body == "raw" {
                        self.mode = LexerMode::Raw;
                    } else if body == "verbatim" {
                        self.mode = LexerMode::Verbatim;
                    }
                    return Ok(Some(Token::Tag(body)));
                }
                Some((0, OpenKind::Var { trim_open })) => {
                    return self.consume_variable(trim_open).map(Some);
                }
                Some((idx, _)) => {
                    let text = rest[..idx].to_owned();
                    self.position += idx;
                    return Ok(Some(Token::Text(text)));
                }
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token>> {
        if let Some(t) = self.pending.take() {
            return Ok(Some(t));
        }
        match self.mode {
            LexerMode::Normal => self.next_token_normal(),
            LexerMode::Raw | LexerMode::Verbatim => self.next_token_block_mode(),
        }
    }
}

/// Tokenizes the full `input` into a [`Vec`] of [`Token`]s.
///
/// An empty string yields a single [`Token::Text`] with empty content.
///
/// # Examples
///
/// ```
/// use runjucks_core::lexer::{tokenize, Token};
///
/// let tokens = tokenize("Hi {{ name }}").unwrap();
/// assert!(matches!(tokens[0], Token::Text(_)));
/// assert!(matches!(tokens[1], Token::Expression(_)));
/// ```
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    if input.is_empty() {
        return Ok(vec![Token::Text(String::new())]);
    }
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    while let Some(t) = lexer.next_token()? {
        tokens.push(t);
    }
    Ok(tokens)
}
