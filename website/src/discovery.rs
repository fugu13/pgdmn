//! What tells a crawler the site's URL set: `sitemap.xml`, `robots.txt`, and
//! the Atom feed.
//!
//! All three are generated during the prerender and written next to the pages
//! they describe, so they can never drift from what is actually served.
//!
//! The sitemap carries only `<loc>` and, for articles, `<lastmod>`: Google
//! ignores `<changefreq>` and `<priority>`, and trusts `<lastmod>` only when
//! it is accurate. Article dates come from front matter and are real; the
//! hand-written pages have no honest modification date, so they state none.
//!
//! Escaping rule throughout: values derived from content (titles, summaries,
//! and URLs built from filenames) go through [`crate::escape::element_text`];
//! in-repo constants are written as-is.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::articles::Article;
use crate::escape::element_text;
use crate::routes;
use crate::site;

// Site-absolute paths; all three sit at the site root, so trimming the
// leading slash also gives the output filename under `dist/`.
pub const SITEMAP: &str = "/sitemap.xml";
pub const ROBOTS: &str = "/robots.txt";
/// The Atom feed, linked from every page's head.
pub const FEED: &str = "/feed.xml";
/// What a feed reader shows as the subscription's name.
pub const FEED_TITLE: &str = "pgdmn articles";

/// The sitemap for the given pages, which the prerender pass collects from the
/// generated output itself—a new route appears here without anyone remembering
/// to add it.
pub fn sitemap(pages: &[String], articles: &[Article]) -> String {
    let dates: HashMap<String, &str> = articles
        .iter()
        .map(|article| (routes::article(&article.slug), article.date.as_str()))
        .collect();

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );
    for page in pages {
        let _ = write!(xml, "  <url><loc>{}</loc>", element_text(&site::url(page)));
        if let Some(date) = dates.get(page) {
            let _ = write!(xml, "<lastmod>{date}</lastmod>");
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

/// Allows everything and names the sitemap—the discovery path for crawlers
/// nobody has registered the site with.
pub fn robots() -> String {
    format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}\n",
        site::url(SITEMAP)
    )
}

/// The Atom feed of every article, newest first (the order [`Article`]s load in).
///
/// Entries carry the summary, not the body: the feed's job is telling a
/// reader—human or crawler—that something new exists and where it is.
pub fn feed(articles: &[Article]) -> String {
    // Front-matter dates are days; midnight UTC turns them into the instants
    // Atom requires without inventing a time of day that looks meaningful.
    let updated = articles
        .first()
        .map_or("1970-01-01", |article| article.date.as_str());

    let mut xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>{FEED_TITLE}</title>
  <id>{feed_url}</id>
  <link rel="self" href="{feed_url}"/>
  <link href="{index}"/>
  <updated>{updated}T00:00:00Z</updated>
  <author><name>{author}</name><uri>{author_url}</uri></author>
"#,
        feed_url = site::url(FEED),
        index = site::url(&routes::articles()),
        author = site::AUTHOR,
        author_url = site::AUTHOR_URL,
    );
    for article in articles {
        let url = element_text(&site::url(&routes::article(&article.slug)));
        let _ = write!(
            xml,
            r#"  <entry>
    <title>{title}</title>
    <id>{url}</id>
    <link href="{url}"/>
    <updated>{date}T00:00:00Z</updated>
    <summary>{summary}</summary>
  </entry>
"#,
            title = element_text(&article.title),
            date = article.date,
            summary = element_text(article.card_description()),
        );
    }
    xml.push_str("</feed>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::{FEED, SITEMAP, feed, robots, sitemap};
    use crate::articles::{self, Article};

    fn article(slug: &str, title: &str, date: &str, summary: &str) -> Article {
        Article {
            slug: slug.to_string(),
            title: title.to_string(),
            date: date.to_string(),
            summary: summary.to_string(),
            description: None,
            example: None,
            files: Vec::new(),
            body: String::new(),
        }
    }

    #[test]
    fn sitemap_lists_every_page_absolutely() {
        let pages = vec!["/".to_string(), "/why/".to_string()];
        let xml = sitemap(&pages, &[]);
        assert!(xml.contains("<loc>https://www.pgdmn.com/</loc>"));
        assert!(xml.contains("<loc>https://www.pgdmn.com/why/</loc>"));
        assert!(xml.starts_with("<?xml"));
        assert!(xml.ends_with("</urlset>\n"));
    }

    #[test]
    fn article_pages_carry_their_publication_date_as_lastmod() {
        let pages = vec!["/why/".to_string(), "/articles/loans/".to_string()];
        let posts = [article("loans", "Loans", "2026-07-01", "Lending.")];
        let xml = sitemap(&pages, &posts);
        assert!(xml.contains(
            "<url><loc>https://www.pgdmn.com/articles/loans/</loc>\
             <lastmod>2026-07-01</lastmod></url>"
        ));
        // A hand-written page has no honest modification date, so it states none:
        // Google trusts lastmod only while it never catches the site inventing one.
        assert!(xml.contains("<url><loc>https://www.pgdmn.com/why/</loc></url>"));
    }

    #[test]
    fn sitemap_never_states_changefreq_or_priority() {
        let pages = vec!["/".to_string()];
        let xml = sitemap(&pages, &[]);
        assert!(!xml.contains("changefreq"));
        assert!(!xml.contains("priority"));
    }

    #[test]
    fn robots_allows_everything_and_names_the_sitemap() {
        let txt = robots();
        assert!(txt.contains("User-agent: *\nAllow: /\n"));
        assert!(txt.contains("Sitemap: https://www.pgdmn.com/sitemap.xml\n"));
        assert_eq!(SITEMAP, "/sitemap.xml");
    }

    #[test]
    fn feed_has_one_entry_per_article_with_absolute_links() {
        let posts = [
            article("newer", "Newer", "2026-07-02", "Second."),
            article("older", "Older", "2026-07-01", "First."),
        ];
        let xml = feed(&posts);
        assert_eq!(xml.matches("<entry>").count(), 2);
        assert!(xml.contains(r#"<link href="https://www.pgdmn.com/articles/newer/"/>"#));
        assert!(xml.contains(r#"<link rel="self" href="https://www.pgdmn.com/feed.xml"/>"#));
        // The feed's own timestamp is the newest article's.
        assert!(xml.contains("<updated>2026-07-02T00:00:00Z</updated>"));
        assert_eq!(FEED, "/feed.xml");
    }

    #[test]
    fn titles_and_summaries_are_escaped_for_xml() {
        let posts = [article(
            "ops",
            "Loans & <limits>",
            "2026-07-01",
            "On a < b.",
        )];
        let xml = feed(&posts);
        assert!(xml.contains("<title>Loans &amp; &lt;limits&gt;</title>"));
        assert!(xml.contains("<summary>On a &lt; b.</summary>"));
    }

    /// The real articles, through the real generators: whatever is in
    /// `articles/` right now must produce a feed and sitemap without surprises.
    #[test]
    fn the_repo_articles_produce_wellformed_output() {
        let posts = articles::all();
        let pages: Vec<String> = posts
            .iter()
            .map(|post| crate::routes::article(&post.slug))
            .collect();
        let xml = sitemap(&pages, posts);
        assert_eq!(xml.matches("<lastmod>").count(), posts.len());
        assert_eq!(feed(posts).matches("<entry>").count(), posts.len());
    }
}
