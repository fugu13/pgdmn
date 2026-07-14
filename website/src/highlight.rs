//! Syntax highlighting, done at build time.
//!
//! The site ships no JavaScript, so nothing can highlight in the browser. Output
//! is CSS classes rather than inline colours, which keeps the palette in the
//! stylesheet — one place to change, and one place to check for contrast.

use std::sync::OnceLock;

use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Prefixed so a token class can never collide with a layout class.
const CLASSES: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Highlight SQL, returning HTML.
///
/// If the grammar is missing or a line will not parse, this falls back to the
/// escaped source: unhighlighted code is a cosmetic loss, but a page that fails
/// to build over a stray backslash is not.
pub fn sql(code: &str) -> String {
    let syntaxes = syntaxes();
    let Some(syntax) = syntaxes.find_syntax_by_extension("sql") else {
        return escape(code);
    };

    let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, syntaxes, CLASSES);
    for line in LinesWithEndings::from(code) {
        if generator
            .parse_html_for_line_which_includes_newline(line)
            .is_err()
        {
            return escape(code);
        }
    }
    generator.finalize()
}

fn escape(code: &str) -> String {
    code.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::{escape, sql};

    #[test]
    fn keywords_and_strings_become_classed_spans() {
        let html = sql("SELECT 'x' FROM t; -- note\n");
        assert!(html.contains("hl-keyword"), "no keyword class in: {html}");
        assert!(html.contains("hl-string"), "no string class in: {html}");
        assert!(html.contains("hl-comment"), "no comment class in: {html}");
    }

    #[test]
    fn the_code_itself_survives() {
        let html = sql("SELECT dmn_eval(model, 'Eligibility');\n");

        // Strip the markup back off. Quotes come back as entities, because the
        // highlighter escapes what it emits — so compare against that.
        let mut stripped = String::new();
        let mut in_tag = false;
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => stripped.push(c),
                _ => {}
            }
        }
        let text = stripped.replace("&#39;", "'").replace("&amp;", "&");
        assert!(
            text.contains("SELECT dmn_eval(model, 'Eligibility');"),
            "source did not survive highlighting: {text}"
        );
    }

    #[test]
    fn markup_in_source_is_escaped() {
        assert_eq!(escape("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }
}
