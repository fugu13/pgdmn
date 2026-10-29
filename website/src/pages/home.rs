use leptos::prelude::*;

use crate::components::page_meta::PageMeta;
use crate::components::sql_block::SqlBlock;
use crate::routes;
use crate::structured_data::{self, StructuredData};

const DESCRIPTION: &str = "pgdmn is a PostgreSQL extension that evaluates DMN decision tables and \
                           FEEL expressions in SQL, with no network hop and no external rules \
                           engine.";

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <PageMeta
            title="pgdmn · DMN for PostgreSQL"
            description=DESCRIPTION
            path=routes::page("")
        />
        <StructuredData data=structured_data::home(DESCRIPTION)/>
        <div class="hero">
            <h1>"pgdmn"</h1>
            <p class="tagline">
                "Run DMN decision tables inside PostgreSQL. No network hop, no external engine—just SQL."
            </p>
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
