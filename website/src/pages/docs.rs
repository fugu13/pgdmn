use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::sql_block::SqlBlock;

/// One function: its full signature, then what it does.
///
/// A description list is the honest markup here — each signature is a term and
/// each explanation defines it — and it gives screen-reader users a structure
/// they can jump through, which a pile of paragraphs would not.
#[component]
fn Fun(#[prop(into)] signature: String, children: Children) -> impl IntoView {
    view! {
        <dt><code>{signature}</code></dt>
        <dd>{children()}</dd>
    }
}

#[component]
pub fn DocsPage() -> impl IntoView {
    view! {
        <Title text="Documentation — pgdmn"/>
        <h1>"Documentation"</h1>
        <p class="lede">
            "Every function pgdmn installs, with its arguments. An argument marked "
            <code>"DEFAULT NULL"</code>
            " may be omitted — a decision or expression that needs no input takes none."
        </p>
        <p>
            "Worked examples, with models and datasets you can download and run, are on the "
            <a href="/examples/">"Examples"</a>" page."
        </p>

        <h2 id="install">"Install"</h2>
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

        <h2>"DMN functions"</h2>
        <dl class="fn-list">
            <Fun signature="dmn_load(xml text) → dmnmodel">
                "Parse DMN XML into a "<code>"dmnmodel"</code>
                " value. This is the entry point for everything else. The result is an
                ordinary PostgreSQL value: store it in a column, pass it around, join
                against it. It displays as "<code>"namespace::name"</code>"."
            </Fun>
            <Fun signature="dmn_eval(model dmnmodel, invocable text, input jsonb DEFAULT NULL) → jsonb">
                "Evaluate a named decision, business knowledge model, or decision service.
                Decisions that depend on other decisions are resolved for you — ask for the
                one you want, and pgdmn works out what it needs first."
            </Fun>
            <Fun signature="dmn_record_eval(model dmnmodel, invocable text, input record DEFAULT NULL) → jsonb">
                "The same, but the input is a composite-type record rather than JSONB, so
                table columns map straight onto model inputs with no JSON in between."
            </Fun>
        </dl>

        <h3 id="typed">"Typed variants"</h3>
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
            <Fun signature="dmn_eval_text(model dmnmodel, invocable text, input jsonb DEFAULT NULL) → text">
                "A decision that returns a string, unquoted."
            </Fun>
            <Fun signature="dmn_eval_numeric(model dmnmodel, invocable text, input jsonb DEFAULT NULL) → numeric">
                "A decision that returns a number, ready for arithmetic without a cast: "
                <code>"round(dmn_eval_numeric(…), 2)"</code>"."
            </Fun>
            <Fun signature="dmn_eval_bool(model dmnmodel, invocable text, input jsonb DEFAULT NULL) → boolean">
                "A decision that returns a boolean — usable directly in a "<code>"WHERE"</code>
                " clause or a "<code>"CHECK"</code>" constraint."
            </Fun>
            <Fun signature="dmn_eval_date(model dmnmodel, invocable text, input jsonb DEFAULT NULL) → date">
                "A decision that returns a date."
            </Fun>
            <Fun signature="dmn_eval_timestamp(model dmnmodel, invocable text, input jsonb DEFAULT NULL) → timestamp">
                "A decision that returns a date and time."
            </Fun>
            <Fun signature="dmn_eval_interval(model dmnmodel, invocable text, input jsonb DEFAULT NULL) → interval">
                "A decision that returns a duration. Both flavours convert: years and months, and
                days and time."
            </Fun>
        </dl>

        <h2>"Introspection functions"</h2>
        <dl class="fn-list">
            <Fun signature="dmn_invocables(model dmnmodel) → setof (name text, kind text)">
                "Every invocable element in the model, as rows. "<code>"kind"</code>" is "
                <code>"decision"</code>", "<code>"business_knowledge_model"</code>", or "
                <code>"decision_service"</code>". Useful for discovering what a model you were
                handed can actually answer."
            </Fun>
            <Fun signature="dmn_info(model dmnmodel) → jsonb">
                "Model metadata: name, namespace, a count of each element type, and the list
                of invocable names."
            </Fun>
            <Fun signature="dmn_xml(model dmnmodel) → text">
                "The original XML source, byte for byte. What you loaded is what you get back
                — which is what makes a stored model auditable."
            </Fun>
            <Fun signature="dmn_name(model dmnmodel) → text">"The model's name."</Fun>
            <Fun signature="dmn_namespace(model dmnmodel) → text">"The model's namespace."</Fun>
        </dl>

        <h2>"FEEL functions"</h2>
        <p>
            "FEEL is the expression language DMN is built on. These evaluate it directly,
            with no model involved."
        </p>
        <dl class="fn-list">
            <Fun signature="feel_eval(expression text, context jsonb DEFAULT NULL) → jsonb">
                "Evaluate a FEEL expression. Keys of the context become variables in scope."
            </Fun>
            <Fun signature="feel_record_eval(expression text, context record DEFAULT NULL) → jsonb">
                "The same, with a composite-type record as the context. Columns become
                variables."
            </Fun>
        </dl>

        <h3>"Typed variants"</h3>
        <p>
            "Each returns a native PostgreSQL type instead of JSONB, so the result drops into
            a typed column or a comparison without a cast. Each raises an error if the
            expression returns something else — asking for a number and getting a string is a
            mistake worth hearing about."
        </p>
        <dl class="fn-list">
            <Fun signature="feel_eval_numeric(expression text, context jsonb DEFAULT NULL) → numeric">
                "A FEEL number, as "<code>"numeric"</code>"."
            </Fun>
            <Fun signature="feel_eval_bool(expression text, context jsonb DEFAULT NULL) → boolean">
                "A FEEL boolean — usable directly in a "<code>"WHERE"</code>" clause or a "
                <code>"CHECK"</code>" constraint."
            </Fun>
            <Fun signature="feel_eval_text(expression text, context jsonb DEFAULT NULL) → text">
                "A FEEL string, unquoted."
            </Fun>
            <Fun signature="feel_eval_date(expression text, context jsonb DEFAULT NULL) → date">
                "A FEEL date, as "<code>"date"</code>"."
            </Fun>
            <Fun signature="feel_eval_timestamp(expression text, context jsonb DEFAULT NULL) → timestamp">
                "A FEEL date and time, as "<code>"timestamp"</code>"."
            </Fun>
            <Fun signature="feel_eval_interval(expression text, context jsonb DEFAULT NULL) → interval">
                "A FEEL duration, as "<code>"interval"</code>". Both flavours convert: years
                and months, and days and time."
            </Fun>
        </dl>

        <h2>"Two things worth knowing"</h2>
        <p>
            "Every function above is "<code>"IMMUTABLE"</code>" and "<code>"PARALLEL SAFE"</code>
            ". PostgreSQL is therefore free to spread a decision table across as many parallel
            workers as it likes, and to skip repeated calls with identical arguments."
        </p>
        <p>
            "A model is not re-parsed per row. Parsed models are cached, keyed by the XML
            itself, so evaluating one model against a whole table parses it once."
        </p>
    }
}
