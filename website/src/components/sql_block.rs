use leptos::prelude::*;

#[component]
pub fn SqlBlock(#[prop(into)] code: String) -> impl IntoView {
    view! {
        <pre class="sql-block" role="region" aria-label="SQL example">
            <code>{code}</code>
        </pre>
    }
}
