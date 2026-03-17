use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn ExamplesPage() -> impl IntoView {
    view! {
        <Title text="Examples — pgdmn"/>
        <h1>"Examples"</h1>
        <p>"Real-world scenarios showing pgdmn in action."</p>

        <div class="card-grid">
            <div class="card">
                <h3>"Loan Eligibility"</h3>
                <p>
                    "A multi-hit decision table evaluating applicant age, income, and credit score "
                    "to produce eligibility and rate decisions."
                </p>
            </div>
            <div class="card">
                <h3>"Pricing Tiers"</h3>
                <p>
                    "Dynamic pricing rules that change by swapping the DMN model — no code deploy needed."
                </p>
            </div>
            <div class="card">
                <h3>"Compliance Checks"</h3>
                <p>
                    "Data-handling rules as a decision table, evaluated per-row on a customers table."
                </p>
            </div>
            <div class="card">
                <h3>"Routing Rules"</h3>
                <p>
                    "Route requests, tickets, or tasks based on structured decision logic "
                    "co-located with your data."
                </p>
            </div>
        </div>

        <div class="placeholder-notice">
            <p>"Full worked examples with SQL coming soon."</p>
        </div>
    }
}
