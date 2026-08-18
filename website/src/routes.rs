pub const WHY: &str = "why";
pub const DOCS: &str = "docs";
pub const EXAMPLES: &str = "examples";
pub const ARTICLES: &str = "articles";
/// Prerendered to `404.html` at the site root, which static hosts (GitHub
/// Pages among them) serve for any path that does not exist.
pub const NOT_FOUND: &str = "404";

/// Site-absolute path of a top-level route, with the trailing slash that
/// addresses its prerendered file directly rather than by way of a redirect.
pub fn page(segment: &str) -> String {
    format!("/{segment}/")
}

/// Path to the articles index.
pub fn articles() -> String {
    page(ARTICLES)
}

/// Path to an article, with the same trailing-slash rule as [`page`].
pub fn article(slug: &str) -> String {
    format!("/{ARTICLES}/{slug}/")
}
