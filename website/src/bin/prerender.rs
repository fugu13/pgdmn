//! Renders every route to static HTML in `dist/`, ready for a static host.
//!
//! There is no server in production: whatever this writes is exactly what gets
//! served.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use leptos::prelude::LeptosOptions;
use leptos_axum::generate_route_list_with_ssg;
use pgdmn_website::app::shell;
use pgdmn_website::articles;
use pgdmn_website::discovery;
use pgdmn_website::routes;
// The canonical host, shared with the pages: the CNAME written here and the
// absolute URLs in every page's social metadata must name the same site.
use pgdmn_website::site::DOMAIN;
use pgdmn_website::stylesheet;

type BoxError = Box<dyn std::error::Error>;

/// The generated site. Its contents are the deployable artifact.
const DIST: &str = "dist";
/// Assets copied verbatim into the site root.
const PUBLIC: &str = "public";
const STYLESHEET: &str = "style/main.scss";
/// The one page that keeps its scripts: an interactive DMN viewer, copied
/// verbatim from `public/`. See `strip_scripts`.
const DMN_VIEWER: &str = "dmn-viewer.html";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    if Path::new(DIST).exists() {
        fs::remove_dir_all(DIST)?;
    }
    fs::create_dir_all(DIST)?;

    compile_stylesheet()?;
    copy_dir(Path::new(PUBLIC), Path::new(DIST))?;

    // Parse the articles before rendering anything, so a broken one fails the
    // build with a message naming the file, rather than quietly vanishing from
    // the site.
    let articles = articles::load()?;
    println!(
        "{} article(s): {}",
        articles.len(),
        articles
            .iter()
            .map(|article| article.slug.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let options = LeptosOptions::builder()
        .output_name("pgdmn-website")
        .site_root(DIST)
        .build();

    let (_, static_routes) = generate_route_list_with_ssg(shell);
    static_routes.generate(&options).await;

    clean_urls(Path::new(DIST))?;
    strip_scripts(Path::new(DIST))?;

    // Crawler discovery, generated from the output itself so it can never
    // disagree with what is served. The site-absolute constants double as
    // filenames under dist/.
    let pages = page_paths(Path::new(DIST))?;
    for (path, contents) in [
        (discovery::SITEMAP, discovery::sitemap(&pages, &articles)),
        (discovery::ROBOTS, discovery::robots()),
        (discovery::FEED, discovery::feed(&articles)),
    ] {
        fs::write(Path::new(DIST).join(path.trim_start_matches('/')), contents)?;
    }

    // GitHub Pages runs the pages through Jekyll unless this file exists, which
    // silently drops anything whose name starts with an underscore.
    fs::write(Path::new(DIST).join(".nojekyll"), "")?;
    fs::write(Path::new(DIST).join("CNAME"), format!("{DOMAIN}\n"))?;

    println!("prerendered site written to {DIST}/");
    Ok(())
}

/// Sass is compiled in-process rather than by an external binary, so the build
/// has no host tool to keep version-matched.
///
/// The output filename is content-hashed (`style.<hash>.css`) so a browser or
/// CDN cache can never serve a stale stylesheet after a change: the URL
/// itself changes whenever the content does, rather than relying on a cache
/// lifetime the site doesn't control (GitHub Pages allows no custom cache
/// headers). Every route renders after this runs, so `stylesheet::set_href`
/// only ever needs to happen once.
fn compile_stylesheet() -> Result<(), BoxError> {
    let options = grass::Options::default().style(grass::OutputStyle::Compressed);
    let css = grass::from_path(STYLESHEET, &options)
        .map_err(|e| format!("failed to compile {STYLESHEET}: {e}"))?;

    let mut hasher = DefaultHasher::new();
    css.hash(&mut hasher);
    // Truncated to 32 bits on purpose: a short cache-busting id, not a
    // strength-critical hash—one file, never compared against another.
    #[expect(clippy::cast_possible_truncation)]
    let short_hash = hasher.finish() as u32;
    let filename = format!("style.{short_hash:08x}.css");

    fs::write(Path::new(DIST).join(&filename), css)?;
    stylesheet::set_href(format!("/{filename}"));
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), BoxError> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Leptos writes the `/why` route as `why.html`. Moving it to `why/index.html`
/// makes the extensionless URL resolve on any static host and in the local
/// preview alike, instead of depending on one host's implicit `.html` lookup.
///
/// Recurses, because nested routes land in nested directories:
/// `blog/loan-eligibility.html` has to become `blog/loan-eligibility/index.html`
/// for the same reason the top-level ones do.
///
/// `index.html` keeps its name everywhere. `404.html` keeps its name at the
/// root, where hosts look for it by name.
fn clean_urls(directory: &Path) -> Result<(), BoxError> {
    // Collected up front: the loop creates directories as it goes, and reading
    // the directory while rewriting it invites surprises.
    let entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;

    let at_root = directory == Path::new(DIST);

    for path in entries {
        if path.is_dir() {
            clean_urls(&path)?;
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "html") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let is_viewer = path.file_name().is_some_and(|name| name == DMN_VIEWER);
        // index.html and 404.html keep their names by convention; the DMN viewer
        // keeps its `.html` so it stays a single file with a stable URL.
        if stem == "index" || (at_root && (stem == routes::NOT_FOUND || is_viewer)) {
            continue;
        }

        let target = directory.join(stem);
        fs::create_dir_all(&target)?;
        fs::rename(&path, target.join("index.html"))?;
    }
    Ok(())
}

/// The marker a page uses to ask crawlers to stay away, exactly as `PageMeta`
/// (and the hand-written viewer) emit it.
const NOINDEX: &str = r#"<meta name="robots" content="noindex"#;

/// Every indexable page, as the site-absolute path of its directory.
///
/// Collected by walking `dist/` after [`clean_urls`], when every page sits at
/// `<route>/index.html`—so the sitemap describes what is actually served, and
/// a new route joins it without anyone remembering to add it. A page that
/// declares itself [`NOINDEX`] has asked crawlers to stay away and is excluded
/// on that declaration—today that is `404.html` and the DMN viewer (whose
/// non-`index.html` names already skip them), but it holds for any future
/// noindex route too.
fn page_paths(dist: &Path) -> Result<Vec<String>, BoxError> {
    fn walk(directory: &Path, dist: &Path, pages: &mut Vec<String>) -> Result<(), BoxError> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, dist, pages)?;
                continue;
            }
            if path.file_name().is_none_or(|name| name != "index.html")
                || fs::read_to_string(&path)?.contains(NOINDEX)
            {
                continue;
            }
            let relative = path
                .parent()
                .and_then(|parent| parent.strip_prefix(dist).ok())
                .and_then(Path::to_str)
                .ok_or_else(|| format!("unexpected page path: {}", path.display()))?;
            pages.push(if relative.is_empty() {
                "/".to_string()
            } else {
                format!("/{relative}/")
            });
        }
        Ok(())
    }

    let mut pages = Vec::new();
    walk(dist, dist, &mut pages)?;
    // Directory order is filesystem order; the sitemap is read by people too.
    pages.sort();
    Ok(pages)
}

/// The site ships no JavaScript. Leptos still emits inline hydration
/// bookkeeping (`__RESOLVED_RESOURCES` and friends) even for static routes,
/// which is inert with nothing to hydrate—so remove it and let "no scripts"
/// be a property of the output rather than an aspiration.
///
/// A page that genuinely needs a script would have to reverse WEB-001 first.
/// JSON-LD blocks (`<script type="application/ld+json">`) are kept: they are
/// data for crawlers, not executable code, and stripping them would undo the
/// structured data the pages deliberately state.
///
/// The one exception is [`DMN_VIEWER`]: a deliberate, isolated interactive page
/// that loads dmn-js to show a model in standard DMN tooling. It is not a content
/// page, and it keeps its scripts.
fn strip_scripts(directory: &Path) -> Result<(), BoxError> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            strip_scripts(&path)?;
            continue;
        }
        let is_html = path
            .extension()
            .is_some_and(|extension| extension == "html");
        let is_viewer = path.file_name().is_some_and(|name| name == DMN_VIEWER);
        if is_html && !is_viewer {
            let html = fs::read_to_string(&path)?;
            fs::write(&path, without_script_elements(&html))?;
        }
    }
    Ok(())
}

fn without_script_elements(html: &str) -> String {
    const OPEN: &str = "<script";
    const CLOSE: &str = "</script>";
    /// The carve-out: a block whose opening tag declares this type is data,
    /// not code. The CI no-script check permits exactly the same attribute.
    const JSON_LD: &str = r#"type="application/ld+json""#;

    let mut kept = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(start) = rest.find(OPEN) {
        let Some(end) = rest[start..].find(CLOSE) else {
            // Unterminated: keep it rather than silently truncating the page.
            break;
        };
        let after = start + end + CLOSE.len();
        // The opening tag runs to the first `>`; JSON-LD content cannot hide
        // one earlier, because serialization escapes `<` and the payload's
        // own `>`s only ever follow the real tag end.
        let keeps_data_block = rest[start..]
            .split_once('>')
            .is_some_and(|(tag, _)| tag.contains(JSON_LD));
        kept.push_str(&rest[..if keeps_data_block { after } else { start }]);
        rest = &rest[after..];
    }
    kept.push_str(rest);
    kept
}

#[cfg(test)]
mod tests {
    use super::{page_paths, without_script_elements};

    #[test]
    fn removes_inline_script() {
        let html = "<body><p>hi</p><script nonce=\"x\">__PENDING=[];</script></body>";
        assert_eq!(without_script_elements(html), "<body><p>hi</p></body>");
    }

    #[test]
    fn removes_every_script() {
        let html = "<a></a><script>one</script><b></b><script>two</script><c></c>";
        assert_eq!(without_script_elements(html), "<a></a><b></b><c></c>");
    }

    #[test]
    fn leaves_script_free_html_untouched() {
        let html = "<main><h1>pgdmn</h1></main>";
        assert_eq!(without_script_elements(html), html);
    }

    #[test]
    fn keeps_unterminated_script_rather_than_truncating() {
        let html = "<p>keep me</p><script>oops";
        assert_eq!(without_script_elements(html), html);
    }

    #[test]
    fn keeps_json_ld_blocks_while_stripping_scripts_around_them() {
        let html = "<script>code</script>\
                    <script type=\"application/ld+json\">{\"@type\":\"TechArticle\"}</script>\
                    <script nonce=\"x\">more</script>";
        assert_eq!(
            without_script_elements(html),
            "<script type=\"application/ld+json\">{\"@type\":\"TechArticle\"}</script>"
        );
    }

    #[test]
    fn the_json_ld_carve_out_reads_the_whole_opening_tag() {
        // The type attribute need not come first for the block to be data.
        let html = "<p>a</p><script id=\"schema\" type=\"application/ld+json\">{}</script>";
        assert_eq!(without_script_elements(html), html);
    }

    #[test]
    fn a_script_merely_mentioning_json_ld_in_its_body_is_still_stripped() {
        let html = "<script>let t = 'type=\"application/ld+json\"';</script><p>b</p>";
        assert_eq!(without_script_elements(html), "<p>b</p>");
    }

    /// The whole page-set contract in one fixture: nested `index.html` files
    /// become sorted site-absolute paths; anything else—other `.html` names,
    /// assets, and any page declaring itself noindex—stays out of the sitemap.
    #[test]
    fn page_paths_finds_indexable_pages_and_only_those() {
        use std::fs;

        let dist = std::env::temp_dir().join(format!("pgdmn-page-paths-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dist);
        for dir in ["why", "articles/loans"] {
            fs::create_dir_all(dist.join(dir)).unwrap();
        }
        for (file, contents) in [
            ("index.html", "<html>home</html>"),
            ("why/index.html", "<html>why</html>"),
            ("articles/loans/index.html", "<html>post</html>"),
            (
                "404.html",
                "<meta name=\"robots\" content=\"noindex, follow\">",
            ),
            ("dmn-viewer.html", "<html>viewer</html>"),
            ("style.css", "body{}"),
        ] {
            fs::write(dist.join(file), contents).unwrap();
        }
        // A future noindex route lands at <route>/index.html; its own
        // declaration keeps it out, not its filename.
        fs::create_dir_all(dist.join("draft")).unwrap();
        fs::write(
            dist.join("draft/index.html"),
            "<meta name=\"robots\" content=\"noindex, follow\">",
        )
        .unwrap();

        let pages = page_paths(&dist).unwrap();
        let _ = fs::remove_dir_all(&dist);
        assert_eq!(pages, ["/", "/articles/loans/", "/why/"]);
    }
}
