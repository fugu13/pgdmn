pub const WHY: &str = "why";
pub const DOCS: &str = "docs";
pub const EXAMPLES: &str = "examples";
pub const ARTICLES: &str = "articles";
/// Prerendered to `404.html` at the site root, which static hosts (GitHub
/// Pages among them) serve for any path that does not exist.
pub const NOT_FOUND: &str = "404";

/// Path to the articles index, with the trailing slash that addresses its
/// prerendered file directly rather than by way of a redirect.
pub fn articles() -> String {
    format!("/{ARTICLES}/")
}

/// Path to an article, with the trailing slash that addresses its prerendered
/// file directly rather than by way of a redirect.
pub fn article(slug: &str) -> String {
    format!("/{ARTICLES}/{slug}/")
}
