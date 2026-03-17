use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn DocsPage() -> impl IntoView {
    view! {
        <Title text="Documentation — pgdmn"/>
        <h1>"Documentation"</h1>
        <p>
            "The full function reference with examples for every SQL function is in the "
            <a
                href="https://github.com/fugu13/pgdmn#readme"
                rel="noopener noreferrer"
                target="_blank"
            >
                "project README"
            </a>
            "."
        </p>

        <h2>"Function categories"</h2>

        <h3>"DMN functions"</h3>
        <ul>
            <li><code>"dmn_load"</code>" — parse DMN XML into a model value"</li>
            <li><code>"dmn_eval"</code>" — evaluate a decision with JSONB input"</li>
            <li><code>"dmn_record_eval"</code>" — evaluate with a composite-type record"</li>
        </ul>

        <h3>"Introspection functions"</h3>
        <ul>
            <li><code>"dmn_invocables"</code>" — list all invocable elements"</li>
            <li><code>"dmn_info"</code>" — model metadata as JSONB"</li>
            <li><code>"dmn_xml"</code>" — extract raw XML"</li>
            <li><code>"dmn_name"</code>", " <code>"dmn_namespace"</code>" — model identity"</li>
        </ul>

        <h3>"FEEL functions"</h3>
        <ul>
            <li><code>"feel_eval"</code>" — evaluate a FEEL expression (returns JSONB)"</li>
            <li><code>"feel_record_eval"</code>" — FEEL eval with composite-type context"</li>
            <li>
                "Typed variants: "
                <code>"feel_eval_numeric"</code>", "
                <code>"feel_eval_bool"</code>", "
                <code>"feel_eval_text"</code>", "
                <code>"feel_eval_date"</code>", "
                <code>"feel_eval_timestamp"</code>", "
                <code>"feel_eval_interval"</code>
            </li>
        </ul>
    }
}
