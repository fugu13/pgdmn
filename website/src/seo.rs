//! Files that exist for crawlers rather than readers: `sitemap.xml` and
//! `robots.txt`.
//!
//! The sitemap is built from the prerendered output, not from a hand-kept list
//! of routes, so a page that exists is a page that is listed.

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

#[cfg(test)]
mod tests {
    use super::{SitemapEntry, robots, sitemap};

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
