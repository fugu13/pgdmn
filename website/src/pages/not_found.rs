use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <Title text="Not found — pgdmn"/>
        <h1>"404"</h1>
        <p>"The page you requested could not be found."</p>
    }
}
