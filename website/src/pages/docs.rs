use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::sql_block::SqlBlock;
use crate::highlight;

/// One function: its full signature, then what it does.
///
/// A description list is the honest markup here — each signature is a term and
/// each explanation defines it — and it gives screen-reader users a structure
/// they can jump through, which a pile of paragraphs would not.
///
/// The signature is highlighted with the SQL grammar (which is why it reads
/// `RETURNS` rather than an arrow) and is allowed to wrap, so it stays legible
/// on a narrow screen instead of scrolling off the edge.
#[component]
fn Fun(#[prop(into)] signature: String, children: Children) -> impl IntoView {
    view! {
        <dt><code inner_html=highlight::signature(&signature)></code></dt>
        <dd>{children()}</dd>
    }
}

/// A section heading that is also a link to itself, so a reader can click it to
/// get a shareable deep link to that section. The `id` is the anchor.
#[component]
fn H2(#[prop(into)] id: String, #[prop(into)] title: String) -> impl IntoView {
    let href = format!("#{id}");
    view! { <h2 id=id class="anchored"><a href=href>{title}</a></h2> }
}

/// The same, one level down.
#[component]
fn H3(#[prop(into)] id: String, #[prop(into)] title: String) -> impl IntoView {
    let href = format!("#{id}");
    view! { <h3 id=id class="anchored"><a href=href>{title}</a></h3> }
}

#[component]
pub fn DocsPage() -> impl IntoView {
    view! {
        <Title text="Documentation — pgdmn"/>
        <h1 id="documentation">"Documentation"</h1>
        <p class="lede">"Every function pgdmn installs, with its arguments."</p>
        <p>
            "Worked examples, with models and datasets you can download and run, are on the "
            <a href="/examples/">"Examples"</a>" page."
        </p>

        <H2 id="install" title="Install"/>
        <p>
            "pgdmn is a PostgreSQL extension. Once the files are in place on the server, one
            statement per database installs it:"
        </p>
        <SqlBlock
            label="Install the extension"
            code="CREATE EXTENSION pgdmn;

-- Confirm it is there.
SELECT feel_eval('1 + 2');
--  3"
        />
        <p>
            "Everything below is then available in that database. Building the extension from
            source is covered in the "
            <a href="https://github.com/fugu13/pgdmn#readme" rel="noopener noreferrer" target="_blank">
                "project README"
            </a>"."
        </p>

        <H2 id="dmn-functions" title="DMN functions"/>
        <dl class="fn-list">
            <Fun signature="dmn_load(xml text) RETURNS dmnmodel">
                "Parse DMN XML into a "<code>"dmnmodel"</code>
                " value. This is the entry point for everything else. The result is an
                ordinary PostgreSQL value: store it in a column, pass it around, join
                against it. It displays as "<code>"namespace::name"</code>"."
            </Fun>
            <Fun signature="dmn_eval(model dmnmodel, invocable text, input jsonb DEFAULT NULL) RETURNS jsonb">
                "Evaluate a named decision, business knowledge model, or decision service.
                Decisions that depend on other decisions are resolved for you — ask for the
                one you want, and pgdmn works out what it needs first."
            </Fun>
            <Fun signature="dmn_record_eval(model dmnmodel, invocable text, input record DEFAULT NULL) RETURNS jsonb">
                "The same, but the input is a composite-type record rather than JSONB, so
                table columns map straight onto model inputs with no JSON in between."
            </Fun>
        </dl>

        <H3 id="dmn-typed-variants" title="Typed variants"/>
        <p>
            <code>"dmn_eval"</code>" returns JSONB, so a decision that produces the string "
            <em>"Approved"</em>" comes back as "<code>"\"Approved\""</code>" — quoted. Unwrapping
            that by hand means "<code>"dmn_eval(…) #>> '{}'"</code>", and a numeric decision means
            "<code>"(dmn_eval(…) #>> '{}')::numeric"</code>"."
        </p>
        <p>
            "These take the same arguments and hand back a native PostgreSQL type instead. Each
            errors if the decision returns something else — asking for a number and getting a
            string is a mistake worth hearing about."
        </p>
        <dl class="fn-list">
            <Fun signature="dmn_eval_text(model dmnmodel, invocable text, input jsonb DEFAULT NULL) RETURNS text">
                "A decision that returns a string, unquoted."
            </Fun>
            <Fun signature="dmn_eval_numeric(model dmnmodel, invocable text, input jsonb DEFAULT NULL) RETURNS numeric">
                "A decision that returns a number, ready for arithmetic without a cast: "
                <code>"round(dmn_eval_numeric(…), 2)"</code>"."
            </Fun>
            <Fun signature="dmn_eval_bool(model dmnmodel, invocable text, input jsonb DEFAULT NULL) RETURNS boolean">
                "A decision that returns a boolean — usable directly in a "<code>"WHERE"</code>
                " clause or a "<code>"CHECK"</code>" constraint."
            </Fun>
            <Fun signature="dmn_eval_date(model dmnmodel, invocable text, input jsonb DEFAULT NULL) RETURNS date">
                "A decision that returns a date."
            </Fun>
            <Fun signature="dmn_eval_timestamp(model dmnmodel, invocable text, input jsonb DEFAULT NULL) RETURNS timestamp">
                "A decision that returns a date and time."
            </Fun>
            <Fun signature="dmn_eval_interval(model dmnmodel, invocable text, input jsonb DEFAULT NULL) RETURNS interval">
                "A decision that returns a duration. Both flavours convert: years and months, and
                days and time."
            </Fun>
        </dl>

        <H2 id="introspection-functions" title="Introspection functions"/>
        <dl class="fn-list">
            <Fun signature="dmn_invocables(model dmnmodel) RETURNS setof (name text, kind text)">
                "Every invocable element in the model, as rows. "<code>"kind"</code>" is "
                <code>"decision"</code>", "<code>"business_knowledge_model"</code>", or "
                <code>"decision_service"</code>". Useful for discovering what a model you were
                handed can actually answer."
            </Fun>
            <Fun signature="dmn_info(model dmnmodel) RETURNS jsonb">
                "Model metadata: name, namespace, a count of each element type, and the list
                of invocable names."
            </Fun>
            <Fun signature="dmn_xml(model dmnmodel) RETURNS text">
                "The original XML source, byte for byte. What you loaded is what you get back
                — which is what makes a stored model auditable."
            </Fun>
            <Fun signature="dmn_name(model dmnmodel) RETURNS text">"The model's name."</Fun>
            <Fun signature="dmn_namespace(model dmnmodel) RETURNS text">"The model's namespace."</Fun>
        </dl>

        <H2 id="feel-functions" title="FEEL functions"/>
        <p>
            "FEEL is the expression language DMN is built on. These evaluate it directly,
            with no model involved."
        </p>
        <dl class="fn-list">
            <Fun signature="feel_eval(expression text, context jsonb DEFAULT NULL) RETURNS jsonb">
                "Evaluate a FEEL expression. Keys of the context become variables in scope."
            </Fun>
            <Fun signature="feel_record_eval(expression text, context record DEFAULT NULL) RETURNS jsonb">
                "The same, with a composite-type record as the context. Columns become
                variables."
            </Fun>
        </dl>

        <H3 id="feel-typed-variants" title="Typed variants"/>
        <p>
            "Each returns a native PostgreSQL type instead of JSONB, so the result drops into
            a typed column or a comparison without a cast. Each raises an error if the
            expression returns something else — asking for a number and getting a string is a
            mistake worth hearing about."
        </p>
        <dl class="fn-list">
            <Fun signature="feel_eval_numeric(expression text, context jsonb DEFAULT NULL) RETURNS numeric">
                "A FEEL number, as "<code>"numeric"</code>"."
            </Fun>
            <Fun signature="feel_eval_bool(expression text, context jsonb DEFAULT NULL) RETURNS boolean">
                "A FEEL boolean — usable directly in a "<code>"WHERE"</code>" clause or a "
                <code>"CHECK"</code>" constraint."
            </Fun>
            <Fun signature="feel_eval_text(expression text, context jsonb DEFAULT NULL) RETURNS text">
                "A FEEL string, unquoted."
            </Fun>
            <Fun signature="feel_eval_date(expression text, context jsonb DEFAULT NULL) RETURNS date">
                "A FEEL date, as "<code>"date"</code>"."
            </Fun>
            <Fun signature="feel_eval_timestamp(expression text, context jsonb DEFAULT NULL) RETURNS timestamp">
                "A FEEL date and time, as "<code>"timestamp"</code>"."
            </Fun>
            <Fun signature="feel_eval_interval(expression text, context jsonb DEFAULT NULL) RETURNS interval">
                "A FEEL duration, as "<code>"interval"</code>". Both flavours convert: years
                and months, and days and time."
            </Fun>
        </dl>

        <H2 id="performance" title="Performance"/>
        <p>
            "Every function above is "<code>"IMMUTABLE"</code>" and "<code>"PARALLEL SAFE"</code>
            ". PostgreSQL is free to spread a decision table across as many parallel workers as
            it likes, and to skip repeated calls with identical arguments."
        </p>
        <p>
            "A model is not re-parsed per row. Parsed models are cached, keyed by the XML
            itself, so evaluating one model against a whole table parses it once."
        </p>

        <H3 id="writing-for-parallelism" title="Writing for parallelism"/>
        <p>
            "Evaluating a decision is pure, per-row work — the ideal parallel workload, and where
            nearly all the speed is. Write the query as a plain scan so the planner can split it
            across workers: one call per row, no wrapping subquery that forces the evaluation to
            run in a single serial step."
        </p>
        <SqlBlock
            label="A query the planner can parallelize"
            code="-- One call per row over a plain scan. On a large table PostgreSQL
-- runs this across parallel workers on its own.
SELECT t.id,
    dmn_eval_text(m.model, 'Decision', jsonb_build_object('x', t.x))
FROM big_table t
CROSS JOIN models m
WHERE m.name = 'my-model';

-- Check that it actually parallelized: the plan should contain a
-- Gather node. If it does not and the table is large, nudge the planner.
EXPLAIN (ANALYZE)
SELECT dmn_eval_text(m.model, 'Decision', jsonb_build_object('x', t.x))
FROM big_table t CROSS JOIN models m WHERE m.name = 'my-model';

SET max_parallel_workers_per_gather = 4;"
        />
        <p>
            "Two things to avoid. Postgres will not parallelize a small table by default, so a
            quick test on a few rows can look slower than it is — measure on a realistic size.
            And if the same inputs recur many times, it is tempting to evaluate the distinct set
            once and join the answers back; that only helps when the intermediate is "
            <code>"MATERIALIZED"</code>", because otherwise the planner folds the call back above
            the join and evaluates it per row anyway — and materializing gives up the parallelism,
            which is usually the worse trade."
        </p>
    }
}
