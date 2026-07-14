pub const WHY: &str = "why";
pub const DOCS: &str = "docs";
pub const EXAMPLES: &str = "examples";
pub const BLOG: &str = "blog";
/// Prerendered to `404.html` at the site root, which static hosts (GitHub
/// Pages among them) serve for any path that does not exist.
pub const NOT_FOUND: &str = "404";

/// Path to a post, with the trailing slash that addresses its prerendered file
/// directly rather than by way of a redirect.
pub fn post(slug: &str) -> String {
    format!("/{BLOG}/{slug}/")
}
