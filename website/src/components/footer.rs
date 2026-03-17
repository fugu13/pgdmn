use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <p>
                "pgdmn is dual-licensed under "
                <a
                    href="https://opensource.org/licenses/MIT"
                    rel="noopener noreferrer"
                    target="_blank"
                >
                    "MIT"
                </a>
                " and "
                <a
                    href="https://www.apache.org/licenses/LICENSE-2.0"
                    rel="noopener noreferrer"
                    target="_blank"
                >
                    "Apache 2.0"
                </a>
                "."
            </p>
        </footer>
    }
}
