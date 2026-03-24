//! Tokenization: splits template source into [`Token`]s for [`crate::parser::parse`].
//!
//! Recognized regions: `{#` … `#}` comments (discarded), `{{` … `}}` variable openings.
//! `{%` … `%}` tag bodies exist as [`Token::Tag`] in the enum but are not yet emitted by the lexer.

use crate::errors::{Result, RunjucksError};

/// Which special delimiter region appears next in the input.
///
/// Used internally to find the earliest of comment vs variable openers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagRegion {
    /// `{#` … `#}` comment (content omitted from token stream).
    Comment,
    /// `{{` … `}}` variable / expression.
    Variable,
}

impl TagRegion {
    /// Regions scanned when searching for the next delimiter.
    pub const ALL: &'static [TagRegion] = &[TagRegion::Comment, TagRegion::Variable];

    /// Opening delimiter for this region.
    pub fn open(self) -> &'static str {
        match self {
            TagRegion::Comment => "{#",
            TagRegion::Variable => "{{",
        }
    }

    /// Closing delimiter for this region.
    pub fn close(self) -> &'static str {
        match self {
            TagRegion::Comment => "#}",
            TagRegion::Variable => "}}",
        }
    }
}

fn earliest_region(rest: &str) -> Option<(TagRegion, usize)> {
    TagRegion::ALL
        .iter()
        .copied()
        .filter_map(|r| rest.find(r.open()).map(|idx| (r, idx)))
        .min_by_key(|(_, idx)| *idx)
}

/// One lexical unit from a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Plain text until the next special delimiter.
    Text(String),
    /// Body inside `{{` … `}}` (inner whitespace is preserved today).
    Expression(String),
    /// Body inside `{%` … `%}`; parsing of tags is not implemented yet.
    Tag(String),
}

/// Incremental lexer over a template string.
///
/// Prefer [`tokenize`] unless you need streaming consumption.
///
/// # Examples
///
/// ```
/// use runjucks_core::lexer::{Lexer, Token};
///
/// let mut lex = Lexer::new("a{{b}}");
/// assert_eq!(lex.next_token().unwrap(), Some(Token::Text("a".into())));
/// assert_eq!(lex.next_token().unwrap(), Some(Token::Expression("b".into())));
/// assert_eq!(lex.next_token().unwrap(), None);
/// ```
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    /// Starts at the beginning of `input`.
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// Remaining unconsumed input.
    #[inline]
    pub fn rest(&self) -> &'a str {
        self.input.get(self.position..).unwrap_or("")
    }

    /// `true` when all input has been consumed.
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    fn skip_comment(&mut self) -> Result<()> {
        let rest = self.rest();
        let open = TagRegion::Comment.open();
        if !rest.starts_with(open) {
            return Err(RunjucksError::new("internal lexer error: expected `{#`"));
        }
        let close = TagRegion::Comment.close();
        let Some(end_rel) = rest.find(close) else {
            return Err(RunjucksError::new(
                "unclosed comment: expected `#}` after `{#`",
            ));
        };
        self.position += end_rel + close.len();
        Ok(())
    }

    fn consume_variable(&mut self) -> Result<Token> {
        let var = TagRegion::Variable;
        self.position += var.open().len();
        let after_open = self.rest();

        let Some(end_rel) = after_open.find(var.close()) else {
            return Err(RunjucksError::new(
                "unclosed variable tag: expected `}}` after `{{`",
            ));
        };

        if let Some(nested_open) = after_open.find(var.open()) {
            if nested_open < end_rel {
                return Err(RunjucksError::new(
                    "nested `{{` inside a variable expression is not allowed",
                ));
            }
        }

        let expression = after_open[..end_rel].to_owned();
        self.position += end_rel + var.close().len();
        Ok(Token::Expression(expression))
    }

    /// Returns the next token, or `None` at end of input.
    ///
    /// # Errors
    ///
    /// Malformed comments, unclosed `{{`, or nested `{{` inside an expression.
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        loop {
            if self.is_eof() {
                return Ok(None);
            }

            let rest = self.rest();

            match earliest_region(rest) {
                None => {
                    let text = rest.to_owned();
                    self.position = self.input.len();
                    return Ok(Some(Token::Text(text)));
                }
                Some((TagRegion::Comment, 0)) => {
                    self.skip_comment()?;
                    continue;
                }
                Some((TagRegion::Variable, 0)) => {
                    return self.consume_variable().map(Some);
                }
                Some((_, idx)) => {
                    let text = rest[..idx].to_owned();
                    self.position += idx;
                    return Ok(Some(Token::Text(text)));
                }
            }
        }
    }
}

/// Tokenizes the full `input` into a [`Vec`] of [`Token`]s.
///
/// An empty string yields a single [`Token::Text`] with empty content.
///
/// # Errors
///
/// Same as [`Lexer::next_token`].
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
