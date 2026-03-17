use leptos::prelude::*;

use crate::components::sql_block::SqlBlock;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="hero">
            <h1>"pgdmn"</h1>
            <p class="tagline">
                "Run DMN decision tables inside PostgreSQL. No network hop, no external engine — just SQL."
            </p>
        </div>
        <h2>"Quick Start"</h2>
        <SqlBlock code="-- Install the extension\nCREATE EXTENSION pgdmn;\n\n-- Evaluate a FEEL expression\nSELECT feel_eval('1 + 2');\n-- Returns: 3\n\n-- Load a DMN model and evaluate a decision\nSELECT dmn_eval(\n  dmn_load('<your DMN XML>'),\n  'Decision Name',\n  '{\"input\": \"value\"}'::jsonb\n);"/>
    }
}
