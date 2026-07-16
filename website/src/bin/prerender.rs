//! Renders every route to static HTML in `dist/`, ready for a static host.
//!
//! There is no server in production: whatever this writes is exactly what gets
//! served.

use std::fs;
use std::path::Path;

use leptos::prelude::LeptosOptions;
use leptos_axum::generate_route_list_with_ssg;
use pgdmn_website::app::shell;
use pgdmn_website::articles;
use pgdmn_website::routes;

type BoxError = Box<dyn std::error::Error>;

/// The generated site. Its contents are the deployable artifact.
const DIST: &str = "dist";
/// Assets copied verbatim into the site root.
const PUBLIC: &str = "public";
const STYLESHEET: &str = "style/main.scss";
/// The canonical host. GitHub Pages reads this from the published output and
/// redirects the apex (pgdmn.com) here. Pages are linked with absolute paths,
/// so the site must be served from a domain root—not a `github.io` subpath.
const DOMAIN: &str = "www.pgdmn.com";
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

    // GitHub Pages runs the pages through Jekyll unless this file exists, which
    // silently drops anything whose name starts with an underscore.
    fs::write(Path::new(DIST).join(".nojekyll"), "")?;
    fs::write(Path::new(DIST).join("CNAME"), format!("{DOMAIN}\n"))?;

    println!("prerendered site written to {DIST}/");
    Ok(())
}

/// Sass is compiled in-process rather than by an external binary, so the build
/// has no host tool to keep version-matched.
fn compile_stylesheet() -> Result<(), BoxError> {
    let options = grass::Options::default().style(grass::OutputStyle::Compressed);
    let css = grass::from_path(STYLESHEET, &options)
        .map_err(|e| format!("failed to compile {STYLESHEET}: {e}"))?;
    fs::write(Path::new(DIST).join("style.css"), css)?;
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

/// The site ships no JavaScript. Leptos still emits inline hydration
/// bookkeeping (`__RESOLVED_RESOURCES` and friends) even for static routes,
/// which is inert with nothing to hydrate—so remove it and let "no scripts"
/// be a property of the output rather than an aspiration.
///
/// A page that genuinely needs a script would have to reverse WEB-001 first.
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

    let mut kept = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(start) = rest.find(OPEN) {
        let Some(end) = rest[start..].find(CLOSE) else {
            // Unterminated: keep it rather than silently truncating the page.
            break;
        };
        kept.push_str(&rest[..start]);
        rest = &rest[start + end + CLOSE.len()..];
    }
    kept.push_str(rest);
    kept
}

#[cfg(test)]
mod tests {
    use super::without_script_elements;

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
}
