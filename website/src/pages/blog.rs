use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn BlogPage() -> impl IntoView {
    view! {
        <Title text="Blog — pgdmn"/>
        <h1>"Blog"</h1>
        <div class="placeholder-notice">
            <p>"Posts coming soon."</p>
        </div>
    }
}
