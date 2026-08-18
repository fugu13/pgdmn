//! The metadata a page shows when it is not being read directly: a search
//! result, or a link pasted into a chat client.
//!
//! Every page states its own title, description and path here, and gets the
//! sitewide values (card image, site name, locale) for free. The values are
//! literals known at build time, so nothing here depends on request state—the
//! constraint every route on this site works under (WEB-001).

use leptos::prelude::*;
use leptos_meta::{Link, Meta, Title};

use crate::site;

#[component]
// Every prop is taken by value because that is how leptos passes props, and
// `#[prop(into)]` is what lets a page write either a literal or a `format!`.
// `path` is the one that is only read—borrowing it would make each caller keep
// a `String` alive for the whole render instead.
#[expect(clippy::needless_pass_by_value, reason = "leptos prop ergonomics")]
pub fn PageMeta(
    /// The browser tab title, in full: "Examples · pgdmn".
    #[prop(into)]
    title: String,
    /// The headline on the share card, where the site name is already shown
    /// separately and the suffix would only repeat it. Defaults to `title`.
    #[prop(optional, into)]
    card_title: Option<String>,
    /// One sentence, at most [`site::DESCRIPTION_LIMIT`] characters.
    /// (`site::DESCRIPTION`, the home page's, deliberately runs longer—the
    /// one documented exception.)
    #[prop(into)]
    description: String,
    /// This page's site-absolute path, with the trailing slash that addresses
    /// its prerendered file directly: "/why/".
    #[prop(into)]
    path: String,
    // (kept by value: leptos props are moved in, and the URL is built once)
    /// Publication date (`YYYY-MM-DD`) for articles, which are `og:type=article`
    /// rather than `website`. Absent on every other page.
    #[prop(optional, into)]
    published: Option<String>,
    /// Set on pages that exist for a browser but have no business in an index.
    #[prop(optional)]
    noindex: bool,
) -> impl IntoView {
    let card_title = card_title.unwrap_or_else(|| title.clone());
    let url = site::url(&path);
    let card = site::url(site::CARD);
    let kind = if published.is_some() {
        "article"
    } else {
        "website"
    };

    view! {
        <Title text=title/>
        <Meta name="description" content=description.clone()/>
        <Link rel="canonical" href=url.clone()/>
        {noindex.then(|| view! { <Meta name="robots" content="noindex, follow"/> })}

        <Meta property="og:site_name" content=site::NAME/>
        <Meta property="og:locale" content="en_US"/>
        <Meta property="og:type" content=kind/>
        <Meta property="og:title" content=card_title.clone()/>
        <Meta property="og:description" content=description.clone()/>
        <Meta property="og:url" content=url/>
        <Meta property="og:image" content=card.clone()/>
        <Meta property="og:image:alt" content=site::CARD_ALT/>
        <Meta property="og:image:width" content=site::CARD_WIDTH/>
        <Meta property="og:image:height" content=site::CARD_HEIGHT/>
        {published.map(|date| view! { <Meta property="article:published_time" content=date/> })}

        <Meta name="twitter:card" content="summary_large_image"/>
        <Meta name="twitter:title" content=card_title/>
        <Meta name="twitter:description" content=description/>
        <Meta name="twitter:image" content=card/>
        <Meta name="twitter:image:alt" content=site::CARD_ALT/>
    }
}
