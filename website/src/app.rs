use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, SsrMode, StaticSegment,
    components::{Route, Router, Routes},
    static_routes::{StaticParamsMap, StaticRoute},
};

use crate::articles;
use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::components::skip_link::SkipLink;
use crate::pages::article::ArticlePage;
use crate::pages::articles::ArticlesPage;
use crate::pages::docs::DocsPage;
use crate::pages::examples::ExamplesPage;
use crate::pages::home::HomePage;
use crate::pages::not_found::NotFoundPage;
use crate::pages::why::WhyPage;
use crate::routes;
use crate::stylesheet;

/// Every route is rendered once at build time and written to disk. Nothing is
/// rendered per request, so no route may depend on request state.
fn prerendered() -> SsrMode {
    SsrMode::Static(StaticRoute::new())
}

/// One route serves every article, and the slugs it prerenders come from the
/// markdown files themselves—so adding an article is adding a file, and
/// nothing here has to change.
fn article_routes() -> SsrMode {
    SsrMode::Static(StaticRoute::new().prerender_params(|| async {
        let mut params = StaticParamsMap::new();
        params.insert("slug", articles::slugs());
        params
    }))
}

pub fn shell() -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href=stylesheet::href()/>
        <Title text="pgdmn · DMN for PostgreSQL"/>
        <Router>
            <SkipLink/>
            <Header/>
            <main id="main-content" tabindex="-1">
                <Routes fallback=|| view! { <NotFoundPage/> }>
                    <Route path=StaticSegment("") view=HomePage ssr=prerendered()/>
                    <Route path=StaticSegment(routes::WHY) view=WhyPage ssr=prerendered()/>
                    <Route path=StaticSegment(routes::DOCS) view=DocsPage ssr=prerendered()/>
                    <Route
                        path=StaticSegment(routes::EXAMPLES)
                        view=ExamplesPage
                        ssr=prerendered()
                    />
                    <Route
                        path=StaticSegment(routes::ARTICLES)
                        view=ArticlesPage
                        ssr=prerendered()
                    />
                    <Route
                        path=(StaticSegment(routes::ARTICLES), ParamSegment("slug"))
                        view=ArticlePage
                        ssr=article_routes()
                    />
                    <Route
                        path=StaticSegment(routes::NOT_FOUND)
                        view=NotFoundPage
                        ssr=prerendered()
                    />
                </Routes>
            </main>
            <Footer/>
        </Router>
    }
}
