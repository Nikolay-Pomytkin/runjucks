use crate::errors::{Result, RunjucksError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Comment,
    Variable,
}

impl Region {
    pub const ALL: &'static [Region] = &[Region::Comment, Region::Variable];

    pub fn open(self) -> &'static str {
        match self {
            Region::Comment => "{#",
            Region::Variable => "{{",
        }
    }

    pub fn close(self) -> &'static str {
        match self {
            Region::Comment => "#}",
            Region::Variable => "}}",
        }
    }
}

fn earliest_region(rest: &str) -> Option<(Region, usize)> {
    Region::ALL
        .iter()
        .copied()
        .filter_map(|r| rest.find(r.open()).map(|idx| (r, idx)))
        .min_by_key(|(_, idx)| *idx)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Text(String),
    Expression(String),
}

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
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
        let open = Region::Comment.open();
        if !rest.starts_with(open) {
            return Err(RunjucksError::new("internal lexer error: expected `{#`"));
        }
        let close = Region::Comment.close();
        let Some(end_rel) = rest.find(close) else {
            return Err(RunjucksError::new(
                "unclosed comment: expected `#}` after `{#`",
            ));
        };
        self.position += end_rel + close.len();
        Ok(())
    }

    fn consume_variable(&mut self) -> Result<Token> {
        let var = Region::Variable;
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
                Some((Region::Comment, 0)) => {
                    self.skip_comment()?;
                    continue;
                }
                Some((Region::Variable, 0)) => {
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
