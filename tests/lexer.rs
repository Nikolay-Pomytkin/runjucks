use runjucks::lexer::{tokenize, Token};

#[test]
fn tokenize_empty_is_single_empty_text_token() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens, vec![Token::Text(String::new())]);
}

#[test]
fn tokenize_preserves_whitespace_and_newlines() {
    let s = "  line1\n\tline2  ";
    let tokens = tokenize(s).unwrap();
    assert_eq!(tokens, vec![Token::Text(s.to_owned())]);
}

#[test]
fn tokenize_unicode() {
    let s = "你好 🦀 «ταБЬℓσ»";
    let tokens = tokenize(s).unwrap();
    assert_eq!(tokens, vec![Token::Text(s.to_owned())]);
}
