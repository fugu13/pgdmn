use leptos::prelude::*;

use crate::components::page_meta::PageMeta;
use crate::components::sql_block::SqlBlock;
use crate::components::structured_data::{JsonLd, software_application};

/// The one sentence the site leads with, told identically to a reader (the
/// hero), a crawler (the meta description), and a machine (the JSON-LD).
const TAGLINE: &str =
    "Run DMN decision tables inside PostgreSQL. No network hop, no external engine—just SQL.";

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <PageMeta title="pgdmn · DMN for PostgreSQL" description=TAGLINE path="/"/>
        <JsonLd json=software_application(TAGLINE)/>
        <div class="hero">
            <h1>"pgdmn"</h1>
            <p class="tagline">{TAGLINE}</p>
        </div>

        <h2>"Quick start"</h2>
        <SqlBlock
            label="Quick start SQL"
            code="CREATE EXTENSION pgdmn;

-- Load a DMN model and evaluate a decision.
SELECT dmn_eval_text(
  dmn_load('<your DMN XML>'),
  'Eligibility',  -- named output
  '{\"Age\": 34, \"Income\": 82000, \"Bankrupt\": false}'::jsonb
) AS decision;

--  decision
-- ----------
--  Approved"
        />

        <h2>"DMN in your database"</h2>
        <p>
            "DMN provides flexibility to update business rules on the fly, but right now that
            flexibility usually lives in specialized platforms or custom applications. By
            putting DMN in your database, you keep all the flexibility of DMN with the ease and
            integration of PostgreSQL."
        </p>
        <p>
            "See it run against realistic data on the "<a href="/examples/">"Examples"</a>
            " page—downloadable models, sample datasets, and complete SQL examples."
        </p>
    }
}
