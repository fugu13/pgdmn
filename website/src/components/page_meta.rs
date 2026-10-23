use leptos::prelude::*;
use leptos_meta::{Link, Meta, Title};

use crate::routes;

/// The `<head>` metadata every indexable page carries: its title, a
/// description for search results and link previews, and the canonical URL.
///
/// The canonical URL is absolute and keeps the trailing slash, so the
/// slashless and apex variants of a page all consolidate on one address.
/// `published` (an article's `YYYY-MM-DD` date) makes the page an OpenGraph
/// `article` rather than a `website`.
// Component props are owned by construction; `path` is only read, and a
// borrowed prop would need a lifetime the `#[component]` macro can't carry.
#[expect(clippy::needless_pass_by_value)]
#[component]
pub fn PageMeta(
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    /// Site-relative path with its trailing slash, as `routes::page` builds it.
    #[prop(into)]
    path: String,
    #[prop(optional, into)] published: Option<String>,
) -> impl IntoView {
    let url = routes::absolute(&path);
    let kind = if published.is_some() {
        "article"
    } else {
        "website"
    };

    view! {
        <Title text=title.clone()/>
        <Meta name="description" content=description.clone()/>
        <Link rel="canonical" href=url.clone()/>
        <Meta property="og:site_name" content="pgdmn"/>
        <Meta property="og:type" content=kind/>
        <Meta property="og:title" content=title/>
        <Meta property="og:description" content=description/>
        <Meta property="og:url" content=url/>
        <Meta name="twitter:card" content="summary"/>
        {published.map(|date| view! { <Meta property="article:published_time" content=date/> })}
    }
}
