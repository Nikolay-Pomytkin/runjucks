//! Output filters applied during render (e.g. HTML escaping when [`crate::Environment::autoescape`] is on).

/// Escapes a string for safe insertion into HTML text.
///
/// Escapes `&`, `<`, `>`, `"`, and `'` as entities.
///
/// # Examples
///
/// ```
/// use runjucks_core::filters::escape_html;
///
/// assert_eq!(escape_html("<a>"), "&lt;a&gt;");
/// assert_eq!(escape_html("ok"), "ok");
/// ```
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
