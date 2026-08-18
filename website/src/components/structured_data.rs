//! schema.org JSON-LD: the machine-readable statement of what a page is.
//!
//! A JSON-LD block travels in a `<script type="application/ld+json">` element,
//! but it is data for crawlers, not executable code—WEB-001's no-JavaScript
//! rule carves it out by that `type`, in the prerenderer's script stripping
//! and in the CI check alike. Nothing here runs in a browser.

use leptos::prelude::*;
use serde_json::json;

use crate::articles::Article;
use crate::routes;
use crate::site;

/// One JSON-LD block. Google reads these from the body as readily as from the
/// head, and rendering in place keeps the component an ordinary child of its
/// page.
#[component]
pub fn JsonLd(json: String) -> impl IntoView {
    view! { <script type="application/ld+json" inner_html=json></script> }
}

/// What the home page says the site and the software are.
pub fn software_application(description: &str) -> String {
    serialize(&json!({
        "@context": "https://schema.org",
        "@graph": [
            {
                "@type": "WebSite",
                "name": site::NAME,
                "url": site::url("/"),
            },
            {
                "@type": "SoftwareApplication",
                "name": site::NAME,
                "description": description,
                "url": site::url("/"),
                "applicationCategory": "DeveloperApplication",
                "softwareRequirements": "PostgreSQL 17",
                "license": [
                    "https://opensource.org/licenses/MIT",
                    "https://www.apache.org/licenses/LICENSE-2.0",
                ],
                "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD" },
                "sameAs": site::REPOSITORY,
                "author": author(),
            },
        ],
    }))
}

/// What an article page says the article is.
pub fn tech_article(article: &Article) -> String {
    let url = site::url(&routes::article(&article.slug));
    serialize(&json!({
        "@context": "https://schema.org",
        "@type": "TechArticle",
        "headline": article.title,
        "description": article.card_description(),
        "datePublished": article.date,
        "url": url,
        "mainEntityOfPage": url,
        "image": site::url(site::CARD),
        "author": author(),
    }))
}

fn author() -> serde_json::Value {
    json!({ "@type": "Person", "name": site::AUTHOR, "url": site::AUTHOR_URL })
}

/// Serialize for embedding in a script element. `<` only ever appears inside
/// JSON strings, so escaping it everywhere is safe—and keeps any `</script>`
/// in a title or description from ending the element early.
fn serialize(value: &serde_json::Value) -> String {
    value.to_string().replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
    use super::{software_application, tech_article};
    use crate::articles;

    #[test]
    fn the_home_page_data_parses_back_as_json() {
        let block = software_application("Decisions in the database.");
        let value: serde_json::Value = serde_json::from_str(&block).unwrap();
        assert_eq!(value["@graph"][1]["@type"], "SoftwareApplication");
        assert_eq!(value["@graph"][0]["url"], "https://www.pgdmn.com/");
    }

    /// Every article in the repo must produce a block a crawler can parse, and
    /// none may contain a raw `<`—the one character that could end the
    /// surrounding script element early.
    #[test]
    fn every_article_block_parses_and_cannot_escape_its_element() {
        for post in articles::all() {
            let block = tech_article(post);
            assert!(!block.contains('<'), "{}: raw `<` in JSON-LD", post.slug);
            let value: serde_json::Value = serde_json::from_str(&block).unwrap();
            assert_eq!(value["headline"], post.title.as_str());
            assert_eq!(value["datePublished"], post.date.as_str());
        }
    }

    #[test]
    fn angle_brackets_in_content_are_unicode_escaped() {
        let block = software_application("a <b> c");
        assert!(!block.contains('<'));
        let value: serde_json::Value = serde_json::from_str(&block).unwrap();
        assert_eq!(value["@graph"][1]["description"], "a <b> c");
    }
}
