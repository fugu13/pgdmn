use leptos::prelude::*;
use leptos_router::components::A;

use crate::routes;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="site-header">
            <div class="site-header-inner">
                <A href="/" attr:class="site-logo" attr:aria-label="pgdmn home">
                    "pgdmn"
                </A>
                // Each page is prerendered to `<route>/index.html`, so the
                // trailing slash addresses the file directly. Without it a
                // static host answers every nav click with a redirect first.
                <nav aria-label="Main navigation">
                    <ul class="site-nav">
                        <li><A href=format!("/{}/", routes::WHY)>"Why pgdmn"</A></li>
                        <li><A href=format!("/{}/", routes::DOCS)>"Docs"</A></li>
                        <li><A href=format!("/{}/", routes::EXAMPLES)>"Examples"</A></li>
                        <li><A href=format!("/{}/", routes::BLOG)>"Blog"</A></li>
                        <li>
                            <a
                                href="https://github.com/fugu13/pgdmn"
                                rel="noopener noreferrer"
                                target="_blank"
                            >
                                "GitHub"
                            </a>
                        </li>
                    </ul>
                </nav>
            </div>
        </header>
    }
}
