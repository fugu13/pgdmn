use leptos::prelude::*;
use leptos_meta::{Meta, Title};

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <Title text="Not found · pgdmn"/>
        // Served with a 404 status already; this also keeps the page out of an
        // index if a host ever serves it as a 200.
        <Meta name="robots" content="noindex"/>
        <h1>"404"</h1>
        <p>"The page you requested could not be found."</p>
    }
}
