//! Files that exist for crawlers rather than readers: `sitemap.xml`,
//! `robots.txt`, and `llms.txt`.
//!
//! The sitemap and `llms.txt` are built from the prerendered output, not from
//! a hand-kept list of routes, so a page that exists is a page that is listed.

use std::collections::HashSet;
use std::fmt::Write as _;

use crate::articles::escape_html;
use crate::routes;

/// One sitemap entry.
pub struct SitemapEntry {
    /// Site-relative path with its trailing slash, such as `/docs/`.
    pub path: String,
    /// `YYYY-MM-DD`, when the page has a meaningful date (an article's).
    /// Left out otherwise: a guessed date is worse than none to a crawler.
    pub lastmod: Option<String>,
}

/// The sitemap protocol document for `entries`, in the order given.
pub fn sitemap(entries: &[SitemapEntry]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for entry in entries {
        let _ = write!(
            xml,
            "  <url><loc>{}</loc>",
            escape_html(&routes::absolute(&entry.path))
        );
        if let Some(date) = &entry.lastmod {
            let _ = write!(xml, "<lastmod>{}</lastmod>", escape_html(date));
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

/// Allows everything and points crawlers at the sitemap.
pub fn robots() -> String {
    format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}\n",
        routes::absolute("/sitemap.xml")
    )
}

/// A prerendered page as `llms.txt` lists it.
#[derive(Debug, PartialEq, Eq)]
pub struct PageSummary {
    /// Site-relative path with its trailing slash, such as `/docs/`.
    pub path: String,
    /// The page's `<title>`, without the ` · pgdmn` suffix.
    pub title: String,
    /// The page's meta description.
    pub description: String,
}

/// Reads the title and meta description back out of a page `PageMeta`
/// rendered. `None` when either is missing, which means the page skipped
/// `PageMeta`.
pub fn page_summary(path: String, html: &str) -> Option<PageSummary> {
    let title = between(html, "<title>", "</title>")?;
    let description = between(html, "<meta name=\"description\" content=\"", "\"")?;
    let title = unescape_html(title);
    let suffix = format!(" · {}", routes::SITE_NAME);
    Some(PageSummary {
        path,
        title: title.strip_suffix(&suffix).unwrap_or(&title).to_string(),
        description: unescape_html(description),
    })
}

/// The llmstxt.org file for the site, or `None` without a home page.
///
/// It has the site's name, the home page's description as the summary, then a
/// link with its description for every other page. Pages whose path is in
/// `article_paths` get their own section. Pages are listed in the order given.
pub fn llms_txt(pages: &[PageSummary], article_paths: &HashSet<String>) -> Option<String> {
    let home_path = routes::page("");
    let home = pages.iter().find(|page| page.path == home_path)?;
    let (article_pages, other_pages): (Vec<_>, Vec<_>) = pages
        .iter()
        .filter(|page| page.path != home_path)
        .partition(|page| article_paths.contains(&page.path));

    let mut text = format!(
        "# {}\n\n> {}\n\nSource code: {} (MIT or Apache-2.0)\n",
        routes::SITE_NAME,
        home.description,
        routes::REPOSITORY
    );
    for (heading, section) in [("Pages", other_pages), ("Articles", article_pages)] {
        if section.is_empty() {
            continue;
        }
        let _ = write!(text, "\n## {heading}\n\n");
        for page in section {
            let _ = writeln!(
                text,
                "- [{}]({}): {}",
                page.title,
                routes::absolute(&page.path),
                page.description
            );
        }
    }
    Some(text)
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = &text[text.find(start)? + start.len()..];
    Some(&rest[..rest.find(end)?])
}

/// Undoes the entity escaping Leptos applies to text and attribute values.
/// `&amp;` goes last, so an escaped entity such as `&amp;lt;` comes back as
/// the literal text `&lt;`.
fn unescape_html(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{PageSummary, SitemapEntry, llms_txt, page_summary, robots, sitemap};

    fn summary(path: &str, title: &str, description: &str) -> PageSummary {
        PageSummary {
            path: path.into(),
            title: title.into(),
            description: description.into(),
        }
    }

    #[test]
    fn page_summary_reads_title_and_description() {
        let html = "<head><meta name=\"description\" content=\"Rules &amp; tables &lt;here&gt;\">\
                    <title>Docs &#39;n&#39; such · pgdmn</title></head>";
        assert_eq!(
            page_summary("/docs/".into(), html),
            Some(summary("/docs/", "Docs 'n' such", "Rules & tables <here>"))
        );
    }

    #[test]
    fn page_summary_keeps_a_title_without_the_suffix() {
        let html = "<title>pgdmn · DMN for PostgreSQL</title>\
                    <meta name=\"description\" content=\"d\">";
        let page = page_summary("/".into(), html).unwrap();
        assert_eq!(page.title, "pgdmn · DMN for PostgreSQL");
    }

    #[test]
    fn page_summary_requires_a_description() {
        assert_eq!(page_summary("/".into(), "<title>t</title>"), None);
    }

    #[test]
    fn unescaping_handles_an_escaped_ampersand_last() {
        assert_eq!(super::unescape_html("&amp;lt;"), "&lt;");
    }

    #[test]
    fn llms_txt_needs_a_home_page() {
        let pages = [summary("/docs/", "Documentation", "Every function.")];
        assert_eq!(llms_txt(&pages, &HashSet::new()), None);
    }

    #[test]
    fn llms_txt_lists_pages_then_articles() {
        let pages = [
            summary("/", "pgdmn · DMN for PostgreSQL", "DMN in SQL."),
            summary("/articles/", "Articles", "All articles."),
            summary("/articles/a/", "First", "About a."),
            summary("/docs/", "Documentation", "Every function."),
        ];
        assert_eq!(
            llms_txt(&pages, &HashSet::from(["/articles/a/".to_string()])).unwrap(),
            "# pgdmn\n\n> DMN in SQL.\n\n\
             Source code: https://github.com/fugu13/pgdmn (MIT or Apache-2.0)\n\n\
             ## Pages\n\n\
             - [Articles](https://www.pgdmn.com/articles/): All articles.\n\
             - [Documentation](https://www.pgdmn.com/docs/): Every function.\n\n\
             ## Articles\n\n\
             - [First](https://www.pgdmn.com/articles/a/): About a.\n"
        );
    }

    #[test]
    fn sitemap_lists_absolute_urls_with_optional_lastmod() {
        let xml = sitemap(&[
            SitemapEntry {
                path: "/".into(),
                lastmod: None,
            },
            SitemapEntry {
                path: "/articles/a/".into(),
                lastmod: Some("2026-07-10".into()),
            },
        ]);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
        assert!(xml.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
        assert!(xml.contains("<url><loc>https://www.pgdmn.com/</loc></url>"));
        assert!(xml.contains(
            "<url><loc>https://www.pgdmn.com/articles/a/</loc>\
             <lastmod>2026-07-10</lastmod></url>"
        ));
        assert!(xml.trim_end().ends_with("</urlset>"));
    }

    #[test]
    fn sitemap_escapes_special_characters() {
        let xml = sitemap(&[SitemapEntry {
            path: "/a&b/".into(),
            lastmod: None,
        }]);
        assert!(xml.contains("<loc>https://www.pgdmn.com/a&amp;b/</loc>"));
    }

    #[test]
    fn robots_allows_all_and_names_the_sitemap() {
        let txt = robots();
        assert!(txt.contains("User-agent: *\nAllow: /\n"));
        assert!(txt.contains("Sitemap: https://www.pgdmn.com/sitemap.xml\n"));
    }
}
