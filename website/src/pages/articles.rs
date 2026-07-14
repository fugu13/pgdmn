use leptos::prelude::*;
use leptos_meta::Title;

use crate::articles;
use crate::routes;

#[component]
pub fn ArticlesPage() -> impl IntoView {
    let articles = articles::all();

    view! {
        <Title text="Articles — pgdmn"/>
        <h1>"Articles"</h1>

        <ul class="post-list">
            {articles
                .iter()
                .map(|article| {
                    view! {
                        <li class="post-summary">
                            <h2>
                                <a href=routes::article(&article.slug)>{article.title.clone()}</a>
                            </h2>
                            <p class="post-date">
                                <time datetime=article.date.clone()>{article.date.clone()}</time>
                            </p>
                            <p>{article.summary.clone()}</p>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}
