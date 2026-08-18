use leptos::prelude::*;
use leptos_router::components::A;

use crate::pages;
use crate::routes;
use crate::site;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="site-header">
            <div class="site-header-inner">
                <A href="/" attr:class="site-logo" attr:aria-label="pgdmn home">
                    "pgdmn"
                </A>
                // The content pages come from the one registry `llms.txt`
                // also walks (`pages::PAGES`), so the nav and the overview
                // can never disagree. `routes::page` carries the trailing
                // slash that addresses each prerendered file directly.
                <nav aria-label="Main navigation">
                    <ul class="site-nav">
                        <li><A href="/">"Home"</A></li>
                        {pages::PAGES
                            .iter()
                            .map(|page| {
                                view! {
                                    <li>
                                        <A href=routes::page(page.segment)>{page.label}</A>
                                    </li>
                                }
                            })
                            .collect_view()}
                        <li>
                            <a
                                href=site::REPOSITORY
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
