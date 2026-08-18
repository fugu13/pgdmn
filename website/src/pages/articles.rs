use leptos::prelude::*;

use crate::articles;
use crate::components::page_meta::PageMeta;
use crate::routes;

#[component]
pub fn ArticlesPage() -> impl IntoView {
    let articles = articles::all();

    view! {
        <PageMeta
            title="Articles · pgdmn"
            card_title="pgdmn articles"
            description="Walkthroughs of DMN decisions running inside PostgreSQL, each with a model, a dataset, and the queries."
            path=routes::articles()
        />
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
