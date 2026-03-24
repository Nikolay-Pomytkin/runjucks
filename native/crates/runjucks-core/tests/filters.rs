use runjucks_core::filters::escape_html;

#[test]
fn escape_html_escapes_special_chars() {
    assert_eq!(escape_html(r#"&<>"'"#), "&amp;&lt;&gt;&quot;&#39;");
}

#[test]
fn escape_html_preserves_safe_text() {
    assert_eq!(escape_html("hello world 123"), "hello world 123");
}

#[test]
fn escape_html_empty() {
    assert_eq!(escape_html(""), "");
}
