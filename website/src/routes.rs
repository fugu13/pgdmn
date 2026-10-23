/// The canonical host. GitHub Pages reads it from the `CNAME` file the
/// prerender writes, and redirects the apex (pgdmn.com) and plain HTTP here.
/// Every canonical, social, and sitemap URL is built on it.
pub const HOST: &str = "www.pgdmn.com";
/// The site's name, as link previews and structured data show it.
pub const SITE_NAME: &str = "pgdmn";
pub const REPOSITORY: &str = "https://github.com/fugu13/pgdmn";
pub const AUTHOR: &str = "Russell Duhon";
pub const AUTHOR_URL: &str = "https://www.russellduhon.com";
pub const MIT_LICENSE: &str = "https://opensource.org/licenses/MIT";
pub const APACHE_LICENSE: &str = "https://www.apache.org/licenses/LICENSE-2.0";

pub const WHY: &str = "why";
pub const DOCS: &str = "docs";
pub const EXAMPLES: &str = "examples";
pub const ARTICLES: &str = "articles";
/// Prerendered to `404.html` at the site root, which static hosts (GitHub
/// Pages among them) serve for any path that does not exist.
pub const NOT_FOUND: &str = "404";

/// Path to an article, with the trailing slash that addresses its prerendered
/// file directly rather than by way of a redirect.
pub fn article(slug: &str) -> String {
    format!("/{ARTICLES}/{slug}/")
}

/// Path to a top-level page, with the trailing slash that addresses its
/// prerendered `index.html` directly. The empty name is the home page.
pub fn page(name: &str) -> String {
    if name.is_empty() {
        "/".to_string()
    } else {
        format!("/{name}/")
    }
}

/// The absolute URL search engines and link previews should use for `path`.
pub fn absolute(path: &str) -> String {
    format!("https://{HOST}{path}")
}
