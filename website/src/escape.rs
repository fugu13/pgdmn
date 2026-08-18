//! The one escaping rule for markup element content, shared by every
//! generator that writes text into HTML or XML.

/// Escape text for element content. Only `&`, `<`, and `>` can change meaning
/// there; a literal `&quot;` in the source becomes `&amp;quot;` and renders
/// back as `&quot;`.
pub fn element_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::element_text;

    #[test]
    fn escapes_the_three_meaningful_characters_and_nothing_else() {
        assert_eq!(element_text("a < b & b > c"), "a &lt; b &amp; b &gt; c");
        assert_eq!(element_text("&quot;"), "&amp;quot;");
        assert_eq!(element_text("plain \"text\""), "plain \"text\"");
    }
}
