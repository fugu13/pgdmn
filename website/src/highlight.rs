//! Syntax highlighting, done at build time.
//!
//! The site ships no JavaScript, so nothing can highlight in the browser. Output
//! is CSS classes rather than inline colours, which keeps the palette in the
//! stylesheet—one place to change, and one place to check for contrast.

use std::sync::OnceLock;

use crate::escape::element_text as escape;

use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxDefinition, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Prefixed so a token class can never collide with a layout class.
const CLASSES: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };

/// A vendored, XSLT-aware variant of the bundled XML grammar. See
/// `syntaxes/xslt.sublime-syntax` for provenance and why it's a full,
/// self-contained file rather than a small syntax on top of the default one.
const XSLT_SYNTAX: &str = include_str!("../syntaxes/xslt.sublime-syntax");

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(|| {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        if let Ok(xslt) = SyntaxDefinition::load_from_str(XSLT_SYNTAX, true, None) {
            builder.add(xslt);
        }
        builder.build()
    })
}

/// Run syntect's classed-HTML highlighter for one syntax over one source
/// string. `None` if a line will not parse: unhighlighted code is a cosmetic
/// loss, but a page that fails to build over a stray backslash is not.
fn run(syntax: &syntect::parsing::SyntaxReference, code: &str) -> Option<String> {
    let syntaxes = syntaxes();
    let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, syntaxes, CLASSES);
    for line in LinesWithEndings::from(code) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .ok()?;
    }
    Some(generator.finalize())
}

/// Highlight SQL, returning HTML. Falls back to the escaped source if the
/// grammar is missing or a line will not parse.
pub fn sql(code: &str) -> String {
    syntaxes()
        .find_syntax_by_extension("sql")
        .and_then(|syntax| run(syntax, code))
        .map_or_else(|| escape(code), |html| patch_missing_keywords(&html))
}

/// Highlight code in the language named by a markdown fence's info string.
///
/// Recognizes `html`, `xml`, `sh`/`bash`, and `xslt`/`xsl`. `None` for any
/// other language or a syntax error, so the caller can fall back to an
/// escaped plain block.
pub fn fenced(lang: &str, code: &str) -> Option<String> {
    let syntax = match lang {
        "html" | "xml" => syntaxes().find_syntax_by_extension(lang)?,
        "sh" | "bash" => syntaxes().find_syntax_by_extension("sh")?,
        // By name, not extension: the bundled default XML grammar already
        // claims the .xslt/.xsl extensions, and would win the lookup.
        "xslt" | "xsl" => syntaxes().find_syntax_by_name("XSLT")?,
        _ => return None,
    };
    run(syntax, code)
}

/// syntect's default SQL grammar does not know a few Postgres-specific keywords
/// and leaves them unhighlighted—`CREATE EXTENSION` most visibly. Each such
/// phrase survives highlighting as contiguous plain text (the phrases the
/// grammar *does* know come back split across spans), so wrapping the literal is
/// safe and targeted.
fn patch_missing_keywords(html: &str) -> String {
    html.replace(
        "CREATE EXTENSION",
        "<span class=\"hl-keyword\">CREATE EXTENSION</span>",
    )
}

/// Words a signature colours as keywords, and as types. Kept small and explicit
/// because the vocabulary of a signature is small and known.
const SIG_KEYWORDS: &[&str] = &["DEFAULT", "NULL", "RETURNS", "setof"];
const SIG_TYPES: &[&str] = &[
    "dmnmodel",
    "text",
    "jsonb",
    "record",
    "numeric",
    "boolean",
    "date",
    "timestamp",
    "interval",
];

/// Highlight a function signature: every type and keyword, nothing else.
///
/// syntect's SQL grammar only knows a subset of Postgres types—it will not
/// colour `dmnmodel`, `record`, or `setof`—so a signature gets a dedicated
/// pass over a fixed vocabulary. The output is plain word spans, which wrap
/// cleanly at the spaces between arguments on a narrow screen (a single syntect
/// wrapper element did not).
pub fn signature(sig: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    for ch in sig.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            word.push(ch);
            continue;
        }
        push_word(&word, &mut out);
        word.clear();
        match ch {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            other => out.push(other),
        }
    }
    push_word(&word, &mut out);
    out
}

fn push_word(word: &str, out: &mut String) {
    use std::fmt::Write as _;
    if word.is_empty() {
        return;
    }
    if SIG_KEYWORDS.contains(&word) {
        let _ = write!(out, "<span class=\"hl-keyword\">{word}</span>");
    } else if SIG_TYPES.contains(&word) {
        let _ = write!(out, "<span class=\"hl-type\">{word}</span>");
    } else {
        out.push_str(word);
    }
}

#[cfg(test)]
mod tests {
    use super::{escape, fenced, sql};

    #[test]
    fn xsl_tags_are_keywords_and_plain_tags_are_not() {
        let html = fenced(
            "xslt",
            "<xsl:template match=\"/\"><html><xsl:value-of select=\"foo\"/></html></xsl:template>",
        )
        .expect("vendored xslt.sublime-syntax should load and highlight");
        assert!(html.contains("hl-keyword"), "no keyword class in: {html}");
        assert!(html.contains(">html<"), "plain tag lost its name: {html}");
        assert!(
            !html.contains("hl-keyword\">html<"),
            "plain html tag was scoped as a keyword: {html}"
        );
    }

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
        // highlighter escapes what it emits—so compare against that.
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
