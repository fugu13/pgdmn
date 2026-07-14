use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::pages::not_found::NotFoundPage;
use crate::posts;

/// One blog post, rendered from its markdown file.
///
/// The slug comes from the route. Every slug that exists is prerendered, so the
/// not-found branch is for a hand-typed URL, not for a missing file.
#[component]
pub fn PostPage() -> impl IntoView {
    let params = use_params_map();
    let slug = params.read_untracked().get("slug").unwrap_or_default();

    let Some(post) = posts::by_slug(&slug) else {
        return view! { <NotFoundPage/> }.into_any();
    };

    let example = post
        .example
        .as_ref()
        .map(|anchor| format!("/examples/#{anchor}"));

    view! {
        <Title text=format!("{} — pgdmn", post.title)/>
        <p class="post-kind">"Walkthrough"</p>
        <h1>{post.title.clone()}</h1>
        <p class="post-date">
            <time datetime=post.date.clone()>{post.date.clone()}</time>
        </p>

        {example.clone().map(|href| view! {
            <p>
                "The SQL for this example, and the files to run it, are on the "
                <a href=href>"Examples page"</a>". If you have not got pgdmn yet, start with "
                <a href="/docs/#install">"Install"</a>"."
            </p>
        })}

        // The post body is markdown we authored and rendered at build time —
        // there is no user input anywhere near this.
        <div class="post-content" inner_html=post.body.clone()></div>

        <p class="post-nav">
            {example.map(|href| view! { <a href=href>"← Run this example"</a> })}
            " · "
            <a href="/blog/">"All posts"</a>
        </p>
    }
    .into_any()
}
