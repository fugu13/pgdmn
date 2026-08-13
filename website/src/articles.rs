//! Blog articles: markdown files in `articles/`, embedded at compile time and
//! rendered to HTML during the prerender.
//!
//! Nothing here runs at request time, because there are no requests—the site
//! is static. Adding a post means adding a `.md` file; no Rust changes, and no
//! route to register.

use std::fmt::Write as _;
use std::sync::OnceLock;

use include_dir::{Dir, include_dir};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};

static POSTS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/articles");

/// An article, ready to render.
pub struct Article {
    /// From the filename: `loan-eligibility.md` is served at `/articles/loan-eligibility/`.
    pub slug: String,
    pub title: String,
    /// `YYYY-MM-DD`, as written in the front matter.
    pub date: String,
    /// Shown on the articles index.
    pub summary: String,
    /// The anchor on the Examples page this article walks through, if any.
    pub example: Option<String>,
    /// The downloadable files this article uses (model and dataset filenames),
    /// in front-matter order. Shown as download links at the top of the article.
    pub files: Vec<String>,
    pub body: String,
}

/// Every post, newest first.
///
/// Parsed once. `prerender` calls [`load`] first and stops the build on a bad
/// post, so by the time a page renders, this has already been proven to parse.
pub fn all() -> &'static [Article] {
    static CACHE: OnceLock<Vec<Article>> = OnceLock::new();
    CACHE.get_or_init(|| load().unwrap_or_default())
}

pub fn by_slug(slug: &str) -> Option<&'static Article> {
    all().iter().find(|post| post.slug == slug)
}

pub fn slugs() -> Vec<String> {
    all().iter().map(|post| post.slug.clone()).collect()
}

/// Parse every embedded markdown file. The error names the file, because the
/// person who sees it is the person who just wrote the post.
pub fn load() -> Result<Vec<Article>, String> {
    let mut posts = POSTS
        .files()
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "md"))
        .map(|file| {
            let name = file.path().display().to_string();
            let slug = file
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| format!("post has no usable filename: {name}"))?;
            let source = file
                .contents_utf8()
                .ok_or_else(|| format!("post is not valid UTF-8: {name}"))?;
            parse(slug, source).map_err(|e| format!("{name}: {e}"))
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Newest first. Dates are `YYYY-MM-DD`, so sorting the strings sorts the days.
    posts.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(posts)
}

fn parse(slug: &str, source: &str) -> Result<Article, String> {
    let (front_matter, body) = split_front_matter(source)?;

    let field = |key: &str| -> Result<String, String> {
        front_matter
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| format!("front matter is missing `{key}`"))
    };

    let files = field("files").map_or_else(
        |_| Vec::new(),
        |list| {
            list.split(',')
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect()
        },
    );

    Ok(Article {
        slug: slug.to_string(),
        title: field("title")?,
        date: field("date")?,
        summary: field("summary")?,
        example: field("example").ok(),
        files,
        body: to_html(body),
    })
}

/// The `key: value` pairs at the top of a post.
type FrontMatter = Vec<(String, String)>;

/// Split `---` delimited front matter from the body, returning its `key: value`
/// pairs. Values are taken whole, so a summary may contain colons.
fn split_front_matter(source: &str) -> Result<(FrontMatter, &str), String> {
    let source = source.trim_start();
    let rest = source
        .strip_prefix("---")
        .and_then(|rest| rest.strip_prefix('\n'))
        .ok_or("post does not start with `---` front matter")?;
    let end = rest
        .find("\n---")
        .ok_or("front matter is never closed with `---`")?;

    let (front_matter, body) = rest.split_at(end);
    let body = body
        .trim_start_matches('\n')
        .trim_start_matches("---")
        .trim_start_matches('\n');

    let fields = front_matter
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split_once(':')
                .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
                .ok_or_else(|| format!("front matter line is not `key: value`: {line}"))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok((fields, body))
}

fn to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let mut rendered = String::new();
    html::push_html(
        &mut rendered,
        highlight_code_blocks(Parser::new_ext(markdown, options)).into_iter(),
    );
    // A link to a sample file should download it, not render it in the tab. The
    // inline file references in an article are written as ordinary markdown
    // links; this gives them the `download` attribute the markup can't.
    let rendered = rendered.replace("<a href=\"/examples/", "<a download href=\"/examples/");
    tables_accessible(&rendered)
}

/// Replace the contents of every fenced code block with an accessible, styled
/// block. ```` ```sql ````, ```` ```html ````, ```` ```xml ````, and
/// ```` ```sh ````/```` ```bash ```` blocks get build-time highlighting, so
/// code in a post looks like code everywhere else on the site; any other
/// language is HTML-escaped and shown as plain text. All are wrapped in a
/// focusable, horizontally scrollable region, because a block that scrolls
/// sideways has to be reachable by a keyboard.
fn highlight_code_blocks<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut out = Vec::new();
    let mut block: Option<(String, String)> = None;

    for event in events {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) => {
                block = Some((lang.as_ref().to_string(), String::new()));
            }
            Event::Text(ref text) if block.is_some() => {
                if let Some((_, buffer)) = block.as_mut() {
                    buffer.push_str(text);
                }
            }
            Event::End(TagEnd::CodeBlock) if block.is_some() => {
                if let Some((lang, code)) = block.take() {
                    let html = if lang == "sql" {
                        format!(
                            "<pre class=\"sql-block\" role=\"region\" aria-label=\"SQL\" \
                             tabindex=\"0\"><code>{}</code></pre>",
                            crate::highlight::sql(&code)
                        )
                    } else {
                        let body = crate::highlight::fenced(&lang, &code)
                            .unwrap_or_else(|| escape_html(&code));
                        format!(
                            "<pre class=\"code-block\" role=\"region\" aria-label=\"{}\" \
                             tabindex=\"0\"><code>{}</code></pre>",
                            code_block_label(&lang),
                            body
                        )
                    };
                    out.push(Event::Html(html.into()));
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// A short accessible name for a non-SQL code block, from its fence language.
fn code_block_label(lang: &str) -> &'static str {
    match lang {
        "html" => "HTML",
        "xml" => "XML",
        "xslt" | "xsl" => "XSLT",
        "sh" | "bash" | "shell" => "Shell",
        _ => "Code",
    }
}

/// Escape text for display inside a `<code>` element. Only `&`, `<`, and `>`
/// can change meaning in element content; a literal `&quot;` in the source (as
/// in the XSLT stylesheet) becomes `&amp;quot;` and renders back as `&quot;`.
fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
    out
}

/// Markdown tables come out bare. Give them back what the hand-written pages
/// had: a caption, column headers that announce as headers, and a wrapper that
/// scrolls sideways on a narrow screen—focusable, because a region you can
/// scroll is a region a keyboard user has to be able to reach.
///
/// The caption comes from a paragraph reading `Table: …` immediately above the
/// table, which is the only markdown convention this blog adds.
fn tables_accessible(html: &str) -> String {
    const OPEN: &str = "<table>";
    const CLOSE: &str = "</table>";

    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut index = 0;

    while let Some(start) = rest.find(OPEN) {
        let (before, from_table) = rest.split_at(start);
        let (before, caption) = take_caption(before);
        let Some(end) = from_table.find(CLOSE) else {
            break;
        };

        index += 1;
        let id = format!("table-{index}");
        let table = &from_table[OPEN.len()..end];

        // A decision table (its caption names a hit policy) gets a class so the
        // output column—the last one—can be tinted like a DMN tool does.
        let is_decision = caption.is_some_and(|c| c.contains("hit policy"));
        let table_open = if is_decision {
            "<table class=\"decision-table\">"
        } else {
            "<table>"
        };

        out.push_str(before);
        match caption {
            Some(caption) => {
                let _ = write!(
                    out,
                    "<div class=\"table-scroll\" role=\"region\" aria-labelledby=\"{id}\" \
                     tabindex=\"0\">{table_open}<caption id=\"{id}\">{caption}</caption>"
                );
            }
            // No caption, so no accessible name—and an unnamed `region` is
            // worse than none. Still focusable, so it can still be scrolled.
            None => {
                let _ = write!(
                    out,
                    "<div class=\"table-scroll\" tabindex=\"0\">{table_open}"
                );
            }
        }
        out.push_str(&table.replace("<th>", "<th scope=\"col\">"));
        out.push_str("</table></div>");

        rest = &from_table[end + CLOSE.len()..];
    }
    out.push_str(rest);
    out
}

/// Peel a trailing `<p>Table: …</p>` off the markup before a table.
fn take_caption(before: &str) -> (&str, Option<&str>) {
    const MARKER: &str = "<p>Table: ";
    const END: &str = "</p>";

    let trimmed = before.trim_end();
    let Some(start) = trimmed.rfind(MARKER) else {
        return (before, None);
    };
    if !trimmed.ends_with(END) {
        return (before, None);
    }
    let caption = &trimmed[start + MARKER.len()..trimmed.len() - END.len()];
    (&before[..start], Some(caption))
}

#[cfg(test)]
mod tests {
    use super::{load, split_front_matter, tables_accessible, to_html};

    #[test]
    fn every_post_in_the_repo_parses() {
        let posts = load().expect("posts should parse");
        assert!(!posts.is_empty(), "expected at least one post");
        for post in &posts {
            assert!(!post.title.is_empty(), "{} has no title", post.slug);
            assert!(!post.summary.is_empty(), "{} has no summary", post.slug);
        }
    }

    #[test]
    fn posts_are_newest_first() {
        let posts = load().unwrap();
        let mut dates: Vec<_> = posts.iter().map(|post| post.date.clone()).collect();
        let sorted = {
            let mut d = dates.clone();
            d.sort_by(|a, b| b.cmp(a));
            d
        };
        assert_eq!(dates, sorted);
        dates.dedup();
    }

    #[test]
    fn front_matter_values_may_contain_colons() {
        let (fields, body) =
            split_front_matter("---\ntitle: A thing: and another\n---\nbody\n").unwrap();
        assert_eq!(fields[0].1, "A thing: and another");
        assert_eq!(body, "body\n");
    }

    #[test]
    fn missing_front_matter_is_an_error() {
        assert!(split_front_matter("no front matter here").is_err());
    }

    #[test]
    fn a_table_gets_a_caption_a_name_and_a_scroll_wrapper() {
        let html = to_html("Table: The rules\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n");
        assert!(html.contains(
            r#"<div class="table-scroll" role="region" aria-labelledby="table-1" tabindex="0">"#
        ));
        assert!(html.contains(r#"<caption id="table-1">The rules</caption>"#));
        assert!(html.contains(r#"<th scope="col">A</th>"#));
        assert!(html.contains("</table></div>"));
        // The caption paragraph is consumed, not left above the table.
        assert!(!html.contains("<p>Table: The rules</p>"));
    }

    #[test]
    fn an_uncaptioned_table_is_scrollable_but_not_a_named_region() {
        let html = tables_accessible("<table><thead><tr><th>A</th></tr></thead></table>");
        assert!(html.contains(r#"<div class="table-scroll" tabindex="0">"#));
        assert!(!html.contains("role=\"region\""));
    }

    #[test]
    fn prose_without_tables_is_untouched() {
        let html = "<p>Just prose.</p>";
        assert_eq!(tables_accessible(html), html);
    }
}
