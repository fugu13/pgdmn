//! JSON-LD structured data: the schema.org description of a page that search
//! engines read to label results.
//!
//! A JSON-LD block is a `<script type="application/ld+json">` element, but it
//! is data, not code. Browsers never execute it, so a page carrying one works
//! and looks the same with scripting on or off. The prerender keeps these
//! blocks and strips every other script.

use leptos::prelude::*;
use serde_json::{Value, json};

use crate::articles::Article;
use crate::routes;

/// The attribute that marks a script element as inert data. The prerender
/// keeps exactly the script elements whose opening tag carries it.
pub const TYPE_ATTRIBUTE: &str = "type=\"application/ld+json\"";

/// A JSON-LD block, placed where the component is used.
///
/// A plain element rather than `leptos_meta`'s `Script`: that component adds
/// a random per-build `nonce`, which a data block has no use for. Google reads
/// JSON-LD anywhere in the document, so the block need not be in `<head>`.
// Component props are owned by construction; `data` is only read.
#[expect(clippy::needless_pass_by_value)]
#[component]
pub fn StructuredData(data: Value) -> impl IntoView {
    view! { <script type="application/ld+json" inner_html=to_script_text(&data)></script> }
}

/// The site itself and the project it describes, for the home page.
pub fn home(description: &str) -> Value {
    let home = routes::absolute(&routes::page(""));
    json!({
        "@context": "https://schema.org",
        "@graph": [
            {
                "@type": "WebSite",
                "@id": website_id(),
                "name": routes::SITE_NAME,
                "url": home,
                "inLanguage": "en",
            },
            {
                "@type": "SoftwareSourceCode",
                "@id": format!("{home}#software"),
                "name": routes::SITE_NAME,
                "description": description,
                "url": home,
                "codeRepository": routes::REPOSITORY,
                "programmingLanguage": "Rust",
                "runtimePlatform": "PostgreSQL 17",
                "license": [routes::MIT_LICENSE, routes::APACHE_LICENSE],
                "author": author(),
            },
        ],
    })
}

/// One article: its headline, summary, publication date, and author.
pub fn article(article: &Article) -> Value {
    let url = routes::absolute(&routes::article(&article.slug));
    json!({
        "@context": "https://schema.org",
        "@type": "TechArticle",
        "headline": article.title,
        "description": article.summary,
        "datePublished": article.date,
        "url": url,
        "mainEntityOfPage": url,
        "inLanguage": "en",
        "author": author(),
        "isPartOf": { "@id": website_id() },
    })
}

/// The home page's `WebSite` node, which each article declares itself part of.
fn website_id() -> String {
    format!("{}#website", routes::absolute(&routes::page("")))
}

fn author() -> Value {
    json!({ "@type": "Person", "name": routes::AUTHOR, "url": routes::AUTHOR_URL })
}

/// The JSON text to place inside the script element.
///
/// A script element's content ends at the first `</script`, whatever the JSON
/// around it says, so `<`, `>`, and `&` are written as JSON unicode escapes.
/// The value a parser reads back is unchanged.
fn to_script_text(data: &Value) -> String {
    data.to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{home, to_script_text};

    #[test]
    fn script_text_cannot_close_its_element() {
        let data = json!({ "headline": "a </script><b>&" });
        let text = to_script_text(&data);
        assert!(!text.contains('<'));
        assert!(!text.contains('>'));
        assert!(!text.contains('&'));
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn home_describes_the_site_and_the_project() {
        let data = home("about");
        let graph = data["@graph"].as_array().unwrap();
        assert_eq!(graph[0]["@type"], "WebSite");
        assert_eq!(graph[0]["url"], "https://www.pgdmn.com/");
        assert_eq!(graph[1]["@type"], "SoftwareSourceCode");
        assert_eq!(graph[1]["description"], "about");
    }
}
