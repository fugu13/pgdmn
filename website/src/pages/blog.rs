use leptos::prelude::*;
use leptos_meta::Title;

use crate::posts;
use crate::routes;

#[component]
pub fn BlogPage() -> impl IntoView {
    let posts = posts::all();

    view! {
        <Title text="Blog — pgdmn"/>
        <h1>"Blog"</h1>
        <p class="lede">
            "Walkthroughs of the examples: the rules, every row they decide, and why the answer
            is what it is. The runnable SQL for each is on the "
            <a href="/examples/">"Examples"</a>" page."
        </p>

        <ul class="post-list">
            {posts
                .iter()
                .map(|post| {
                    view! {
                        <li class="post-summary">
                            <h2>
                                <a href=routes::post(&post.slug)>{post.title.clone()}</a>
                            </h2>
                            <p class="post-date">
                                <time datetime=post.date.clone()>{post.date.clone()}</time>
                            </p>
                            <p>{post.summary.clone()}</p>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}
