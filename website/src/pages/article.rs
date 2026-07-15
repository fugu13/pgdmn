use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::articles;
use crate::pages::not_found::NotFoundPage;

/// One article, rendered from its markdown file.
///
/// The slug comes from the route. Every slug that exists is prerendered, so the
/// not-found branch is for a hand-typed URL, not for a missing file.
#[component]
pub fn ArticlePage() -> impl IntoView {
    let params = use_params_map();
    let slug = params.read_untracked().get("slug").unwrap_or_default();

    let Some(article) = articles::by_slug(&slug) else {
        return view! { <NotFoundPage/> }.into_any();
    };

    let example = article
        .example
        .as_ref()
        .map(|anchor| format!("/examples/#{anchor}"));

    view! {
        <Title text=format!("{} — pgdmn", article.title)/>
        <p class="post-kind">"Walkthrough"</p>
        <h1>{article.title.clone()}</h1>
        <p class="post-date">
            <time datetime=article.date.clone()>{article.date.clone()}</time>
        </p>

        <p>
            "Everything to run this is below. You need pgdmn installed (see "
            <a href="/docs/#install">"Install"</a>
            "); the files it uses are here."
        </p>

        <ul class="file-links">
            {article
                .files
                .iter()
                .map(|file| {
                    let href = format!("/examples/{file}");
                    view! {
                        <li>
                            <a href=href download><code>{file.clone()}</code></a>
                        </li>
                    }
                })
                .collect_view()}
        </ul>

        // The article body is markdown we authored and rendered at build time —
        // there is no user input anywhere near this.
        <div class="post-content" inner_html=article.body.clone()></div>

        <p class="post-nav">
            {example.map(|href| view! { <a href=href>"← The short version"</a> })}
            " · "
            <a href="/articles/">"All articles"</a>
        </p>
    }
    .into_any()
}
