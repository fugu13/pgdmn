use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::download::{Download, FileKind};
use crate::components::sql_block::SqlBlock;
use crate::routes;

/// The way out of an example and into its walkthrough.
#[component]
fn Walkthrough(#[prop(into)] slug: String, #[prop(into)] scenario: String) -> impl IntoView {
    view! {
        <p class="more">
            <a href=routes::article(&slug)>
                {format!("Read the complete walkthrough: {scenario}")}
            </a>
        </p>
    }
}

#[component]
pub fn ExamplesPage() -> impl IntoView {
    view! {
        <Title text="Examples — pgdmn"/>
        <h1>"Examples"</h1>

        <nav class="quicklinks" aria-label="The examples on this page">
            <ul>
                <li>
                    <a href="#loan">"Loan eligibility"</a>
                    <span>"Approve/deny, with a business-approved phrasing of the decision"</span>
                </li>
                <li>
                    <a href="#pricing">"Order pricing"</a>
                    <span>"Swap pricing models on the fly with the same queries"</span>
                </li>
                <li>
                    <a href="#compliance">"Compliance checks"</a>
                    <span>"Apply regulatory rules to determine data handling procedures"</span>
                </li>
                <li>
                    <a href="#routing">"Ticket routing"</a>
                    <span>"Control how tickets are assigned to support queues"</span>
                </li>
            </ul>
        </nav>

        <h2 id="loan">"Loan eligibility"</h2>
        <p>
            "Decide based on complex inputs with rules that are hard to read and verify in SQL.
            Verify the model when authoring the DMN, and run the exact same model in production."
        </p>

        <ul class="download-list">
            <Download file="loan-eligibility.dmn" kind=FileKind::Model/>
            <Download file="applicants.csv" kind=FileKind::Dataset/>
        </ul>

        <SqlBlock
            label="Loan eligibility: decide one applicant, then all of them"
            code="-- One applicant.
SELECT dmn_eval_text(model, 'Eligibility', '{
        \"Age\": 34, \"Income\": 82000, \"Bankrupt\": false
    }'::jsonb) AS decision
FROM models WHERE name = 'loan';
--  Approved

-- Now check it for everybody.
SELECT a.name, a.age, a.income, a.bankrupt,
    dmn_eval_text(m.model, 'Eligibility', jsonb_build_object(
        'Age',      a.age,
        'Income',   a.income,
        'Bankrupt', a.bankrupt
    )) AS decision
FROM applicants a
CROSS JOIN models m
WHERE m.name = 'loan'
ORDER BY a.id;
--      name      | age | income | bankrupt |        decision
-- ---------------+-----+--------+----------+--------------------------
--  Ada Okafor    |  34 |  82000 | f        | Approved
--  Bo Zhang      |  17 |      0 | f        | Denied: underage
--  Chen Ruiz     |  29 |  41000 | f        | Denied: low income
--  Dara Singh    |  45 | 120000 | t        | Denied: prior bankruptcy
--  Eli Novak     |  22 |  50000 | f        | Approved
--  Fay Mbeki     |  19 |  49999 | f        | Denied: low income
--  Gus Halvorsen |  64 |  68000 | f        | Approved
--  Hana Ito      |  17 |  95000 | f        | Denied: underage"
        />

        <Walkthrough slug="loan-eligibility" scenario="Loan eligibility"/>

        <h2 id="pricing">"Order pricing"</h2>
        <p>
            "Dynamic pricing rules can change with no code deploy. Swap the DMN model for another
            with the same inputs while the SQL stays the same."
        </p>

        <ul class="download-list">
            <Download file="order-pricing.dmn" kind=FileKind::Model/>
            <Download file="order-pricing-promo.dmn" kind=FileKind::Model/>
            <Download file="orders.csv" kind=FileKind::Dataset/>
        </ul>

        <SqlBlock
            label="Order pricing: two policies, swapped by name"
            code="-- Two policies live in `models` under different names. The promo
-- model takes 10% off before tax; the standard one does not.
-- Both answer to the same name, 'Total Price'.
-- Identical queries get the right result for the current model.
SELECT o.customer, o.base_price,
    round(dmn_eval_numeric(s.model, 'Total Price', p.input), 2) AS standard,
    round(dmn_eval_numeric(t.model, 'Total Price', p.input), 2) AS promo
FROM orders o
CROSS JOIN LATERAL (
    SELECT jsonb_build_object(
        'Base Price', o.base_price,
        'Tax Rate',   o.tax_rate
    ) AS input
) p
JOIN models s ON s.name = 'pricing-standard'
JOIN models t ON t.name = 'pricing-promo'
ORDER BY o.id;
--      customer      | base_price | standard |  promo
-- -------------------+------------+----------+---------
--  Northwind Traders |     100.00 |   110.00 |   99.00
--  Globex            |    2499.99 |  2706.24 | 2435.62
--  Initech           |      45.50 |    54.60 |   49.14
--  Umbrella Corp     |    1000.00 |  1000.00 |  900.00
--  Acme Supply       |      19.99 |    21.49 |   19.34"
        />

        <Walkthrough slug="order-pricing" scenario="Order pricing"/>

        <h2 id="compliance">"Compliance checks"</h2>
        <p>
            "Keep regulatory requirements in separately auditable business rules rather than
            burying them in application logic."
        </p>

        <ul class="download-list">
            <Download file="compliance.dmn" kind=FileKind::Model/>
            <Download file="customers.csv" kind=FileKind::Dataset/>
        </ul>

        <SqlBlock
            label="Compliance: what we are obliged to do with each row"
            code="-- With a view, always have the correct policy outcome as a column.
CREATE VIEW obligations AS
SELECT c.id, c.name, c.region, c.data_class,
    dmn_eval_text(m.model, 'Handling', jsonb_build_object(
        'Region',     c.region,
        'Data Class', c.data_class
    )) AS handling
FROM customers c
CROSS JOIN models m
WHERE m.name = 'compliance';

SELECT name, region, data_class, handling
FROM obligations ORDER BY id;
--         name        | region | data_class |           handling
-- --------------------+--------+------------+---------------------------
--  Northwind Traders  | EU     | personal   | store in EU, retain 24 ...
--  Globex             | US     | personal   | retain 24 months
--  Initech            | US     | special    | encrypt, restrict acces...
--  Umbrella Corp      | EU     | special    | encrypt, restrict acces...
--  Acme Supply        | UK     | public     | standard handling
--  Soylent Industries | EU     | public     | standard handling
--  Tyrell Corp        | US     | public     | standard handling
--  Wayne Enterprises  | UK     | personal   | retain 24 months

SELECT handling, count(*), string_agg(name, ', ' ORDER BY id) AS who
FROM obligations
WHERE handling <> 'standard handling'
GROUP BY handling
ORDER BY count(*) DESC, handling;
--            handling            | count |          who
-- -------------------------------+-------+------------------------
--  encrypt, restrict access, ... |     2 | Initech, Umbrella Corp
--  retain 24 months              |     2 | Globex, Wayne Enterp...
--  store in EU, retain 24 months |     1 | Northwind Traders"
        />

        <Walkthrough slug="compliance" scenario="Compliance checks"/>

        <h2 id="routing">"Ticket routing"</h2>
        <p>
            "Automatically visualize the ticket routing decision table with DMN then put the same
            logic directly into the database. What the documentation site says is what happens
            because they're the same rules."
        </p>

        <ul class="download-list">
            <Download file="ticket-routing.dmn" kind=FileKind::Model/>
            <Download file="tickets.csv" kind=FileKind::Dataset/>
        </ul>

        <SqlBlock
            label="Ticket routing: a view that routes every ticket"
            code="-- Change the business requirements in the database and every ticket
-- reroutes automatically.
CREATE VIEW routed_tickets AS
SELECT t.id, t.subject, t.priority, t.customer_tier,
    dmn_eval_text(m.model, 'Queue', jsonb_build_object(
        'Priority',      t.priority,
        'Customer Tier', t.customer_tier
    )) AS queue
FROM tickets t
CROSS JOIN models m
WHERE m.name = 'routing';

SELECT queue, count(*), string_agg(subject, '; ' ORDER BY id) AS work
FROM routed_tickets
GROUP BY queue
ORDER BY count(*) DESC, queue;
--  queue  | count |                  work
-- --------+-------+-----------------------------------------
--  pager  |     3 | Cannot log in after SSO change; Bill...
--  tier-1 |     3 | Feature request: dark mode; Typo in ...
--  tier-2 |     2 | API latency spike in eu-west; Passwo..."
        />

        <Walkthrough slug="ticket-routing" scenario="Ticket routing"/>
    }
}
