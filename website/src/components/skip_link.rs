use leptos::prelude::*;

#[component]
pub fn SkipLink() -> impl IntoView {
    view! {
        <a href="#main-content" class="skip-link">
            "Skip to main content"
        </a>
    }
}
