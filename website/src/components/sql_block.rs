use leptos::prelude::*;

use crate::highlight;

/// A block of SQL, highlighted at build time.
///
/// `label` is required rather than optional: a page carries several of these,
/// and "SQL example" repeated six times tells a screen-reader user nothing.
/// `tabindex` makes the block focusable, because it scrolls horizontally and a
/// region that scrolls has to be reachable by keyboard.
#[component]
pub fn SqlBlock(code: &'static str, #[prop(into)] label: String) -> impl IntoView {
    // The source is ours, written into these pages; the highlighter escapes it
    // on the way through either way.
    let highlighted = highlight::sql(code);
    view! {
        <pre class="sql-block" role="region" aria-label=label tabindex="0">
            <code inner_html=highlighted></code>
        </pre>
    }
}
