use leptos::prelude::*;

use crate::components::page_meta::PageMeta;
use crate::routes;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        // A page that exists only to answer a wrong URL has nothing to offer an
        // index, and no share card is wanted for one either.
        <PageMeta
            title="Not found · pgdmn"
            description="That page is not here. The docs, the examples, and the articles are all one click away."
            path=format!("/{}.html", routes::NOT_FOUND)
            noindex=true
        />
        <h1>"404"</h1>
        <p>"The page you requested could not be found."</p>
    }
}
