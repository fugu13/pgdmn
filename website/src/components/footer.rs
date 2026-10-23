use leptos::prelude::*;

use crate::routes;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <p>
                "pgdmn is dual-licensed under "
                <a
                    href=routes::MIT_LICENSE
                    rel="noopener noreferrer"
                    target="_blank"
                >
                    "MIT"
                </a>
                " and "
                <a
                    href=routes::APACHE_LICENSE
                    rel="noopener noreferrer"
                    target="_blank"
                >
                    "Apache 2.0"
                </a>
                "."
            </p>
            <p>
                "my personal website is "
                <a href=routes::AUTHOR_URL rel="noopener noreferrer" target="_blank">
                    "www.russellduhon.com"
                </a>
                "."
            </p>
        </footer>
    }
}
