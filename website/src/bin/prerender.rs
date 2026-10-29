//! Renders every route to static HTML in `dist/`, ready for a static host.
//!
//! There is no server in production: whatever this writes is exactly what gets
//! served.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use leptos::prelude::LeptosOptions;
use leptos_axum::generate_route_list_with_ssg;
use pgdmn_website::app::shell;
use pgdmn_website::articles;
use pgdmn_website::routes;
use pgdmn_website::seo::{self, PageSummary, SitemapEntry};
use pgdmn_website::structured_data;
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

    // GitHub Pages runs the pages through Jekyll unless this file exists, which
    // silently drops anything whose name starts with an underscore.
    fs::write(Path::new(DIST).join(".nojekyll"), "")?;
    // Pages are linked with absolute paths, so the site must be served from a
    // domain root—not a `github.io` subpath.
    fs::write(Path::new(DIST).join("CNAME"), format!("{}\n", routes::HOST))?;

    // Built from what was just written, so every prerendered page is listed
    // and nothing else is. Runs after `clean_urls`, which is what gives each
    // page its `index.html`.
    let pages = page_summaries(Path::new(DIST), page_paths(Path::new(DIST))?)?;
    let entries = sitemap_entries(&pages, &articles);
    fs::write(Path::new(DIST).join("sitemap.xml"), seo::sitemap(&entries))?;
    fs::write(Path::new(DIST).join("robots.txt"), seo::robots())?;
    let article_paths: HashSet<String> = articles
        .iter()
        .map(|article| routes::article(&article.slug))
        .collect();
    let llms = seo::llms_txt(&pages, &article_paths).ok_or("the home page was not prerendered")?;
    fs::write(Path::new(DIST).join("llms.txt"), llms)?;

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

/// One entry per page. Articles carry their front-matter date as `lastmod`.
fn sitemap_entries(pages: &[PageSummary], articles: &[articles::Article]) -> Vec<SitemapEntry> {
    let dates: HashMap<String, &str> = articles
        .iter()
        .map(|article| (routes::article(&article.slug), article.date.as_str()))
        .collect();
    pages
        .iter()
        .map(|page| SitemapEntry {
            lastmod: dates.get(&page.path).map(|date| (*date).to_string()),
            path: page.path.clone(),
        })
        .collect()
}

/// Each page's title and description, read back from its prerendered HTML.
/// A page without them fails the build: every indexable page renders
/// `PageMeta`.
fn page_summaries(dist: &Path, paths: Vec<String>) -> Result<Vec<PageSummary>, BoxError> {
    paths
        .into_iter()
        .map(|path| {
            let file = dist.join(path.trim_start_matches('/')).join("index.html");
            let html = fs::read_to_string(&file)?;
            seo::page_summary(path, &html).ok_or_else(|| {
                format!("{} has no title or meta description", file.display()).into()
            })
        })
        .collect()
}

/// The site-relative path of every `index.html` under `root`, sorted: the
/// pages a reader can reach. `404.html` and the DMN viewer keep other names,
/// so neither is listed.
fn page_paths(root: &Path) -> Result<Vec<String>, BoxError> {
    fn walk(root: &Path, directory: &Path, paths: &mut Vec<String>) -> Result<(), BoxError> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                walk(root, &path, paths)?;
            } else if entry.file_name() == "index.html" {
                let relative = directory.strip_prefix(root)?;
                let name = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                paths.push(routes::page(&name));
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    walk(root, root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

/// Content pages must work and look right with scripting off, so they ship no
/// executable script. Leptos still emits inline hydration bookkeeping
/// (`__RESOLVED_RESOURCES` and friends) even for static routes, which is inert
/// with nothing to hydrate—so remove it and let "no executable scripts" be a
/// property of the output rather than an aspiration.
///
/// JSON-LD blocks (`<script type="application/ld+json">`) stay: browsers never
/// execute them, so they change nothing about how a page works or looks.
///
/// The one page that keeps executable scripts is [`DMN_VIEWER`]: a deliberate,
/// isolated interactive page that loads dmn-js to show a model in standard DMN
/// tooling. It is not a content page.
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
        let element_end = start + end + CLOSE.len();
        let opening_tag = rest[start..].split('>').next().unwrap_or_default();
        let keep_until = if opening_tag.contains(structured_data::TYPE_ATTRIBUTE) {
            element_end
        } else {
            start
        };
        kept.push_str(&rest[..keep_until]);
        rest = &rest[element_end..];
    }
    kept.push_str(rest);
    kept
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{page_paths, without_script_elements};

    #[test]
    fn lists_index_pages_but_not_404_or_the_viewer() {
        let root = std::env::temp_dir().join(format!("pgdmn-sitemap-{}", std::process::id()));
        for dir in ["", "docs", "articles/a"] {
            fs::create_dir_all(root.join(dir)).unwrap();
            fs::write(root.join(dir).join("index.html"), "").unwrap();
        }
        fs::write(root.join("404.html"), "").unwrap();
        fs::write(root.join("dmn-viewer.html"), "").unwrap();

        let paths = page_paths(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(paths, ["/", "/articles/a/", "/docs/"]);
    }

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
    fn keeps_json_ld_and_removes_the_rest() {
        let html = "<head><script type=\"application/ld+json\">{\"a\":1}</script></head>\
                    <body><script>run()</script></body>";
        assert_eq!(
            without_script_elements(html),
            "<head><script type=\"application/ld+json\">{\"a\":1}</script></head><body></body>"
        );
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
