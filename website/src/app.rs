use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::components::skip_link::SkipLink;
use crate::pages::blog::BlogPage;
use crate::pages::docs::DocsPage;
use crate::pages::examples::ExamplesPage;
use crate::pages::home::HomePage;
use crate::pages::not_found::NotFoundPage;
use crate::pages::why::WhyPage;
use crate::routes;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
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
        <Stylesheet id="leptos" href="/pkg/pgdmn-website.css"/>
        <Title text="pgdmn — DMN for PostgreSQL"/>
        <Router>
            <SkipLink/>
            <Header/>
            <main id="main-content" tabindex="-1">
                <Routes fallback=|| view! { <NotFoundPage/> }>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=StaticSegment(routes::WHY) view=WhyPage/>
                    <Route path=StaticSegment(routes::DOCS) view=DocsPage/>
                    <Route path=StaticSegment(routes::EXAMPLES) view=ExamplesPage/>
                    <Route path=StaticSegment(routes::BLOG) view=BlogPage/>
                </Routes>
            </main>
            <Footer/>
        </Router>
    }
}
