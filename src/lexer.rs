//! Tokenizer for Nunjucks templates. Currently passes through plain text; delimiter-aware lexing comes next.

use crate::errors::Result;

/// A single lexical unit. This will grow to cover `{{`, `{%`, symbols, strings, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Raw template text between tag regions.
    Text(String),
}

/// Tokenize `input` into a stream of tokens.
///
/// Phase 1: the entire template is emitted as one [`Token::Text`] chunk so the NAPI and render
/// pipeline can be exercised end-to-end. The next step is porting delimiter handling from
/// `nunjucks/src/lexer.js`.
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    Ok(vec![Token::Text(input.to_owned())])
}
