use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn WhyPage() -> impl IntoView {
    view! {
        <Title text="Why pgdmn · pgdmn"/>
        <h1>"Why run decisions in your database?"</h1>

        <h2>"No network hop"</h2>
        <p>
            "Your data is already in PostgreSQL. Sending it to an external rules engine "
            "and back adds latency, failure modes, and operational complexity. "
            "pgdmn evaluates decisions right where the data lives."
        </p>

        <h2>"Transactional consistency"</h2>
        <p>
            "Decision results are computed inside the same transaction as the data they depend on. "
            "No stale reads, no eventual consistency, no distributed coordination."
        </p>

        <h2>"Auditable by default"</h2>
        <p>
            "The DMN model XML is a first-class PostgreSQL value. Store it in a table, "
            "version it with timestamps, and trace every evaluation back to the exact "
            "model version that produced the result."
        </p>

        <h2>"Co-located with your data"</h2>
        <p>
            "Use lateral joins to evaluate a decision table against every row in a table "
            "in a single query. Batch thousands of decisions with no application code."
        </p>

        <h2>"Standards-based"</h2>
        <p>
            "DMN is an "
            <a href="https://www.omg.org/dmn/" rel="noopener noreferrer" target="_blank">
                "OMG standard"
            </a>
            " supported by tools from Camunda, Trisotech, and others. "
            "Author models visually, test them independently, and deploy to PostgreSQL."
        </p>
    }
}
