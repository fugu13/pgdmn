use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::sql_block::SqlBlock;
use crate::highlight;

/// One function: its full signature, then what it does.
///
/// A description list is the honest markup here—each signature is a term and
/// each explanation defines it—and it gives screen-reader users a structure
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

/// A block of shell commands. Same presentation and keyboard behaviour as a SQL
/// block, but not syntax-highlighted—shell is not SQL, and these are short.
#[component]
fn Shell(#[prop(into)] label: String, #[prop(into)] code: String) -> impl IntoView {
    view! {
        <pre class="sql-block" role="region" aria-label=label tabindex="0">
            <code>{code}</code>
        </pre>
    }
}

#[component]
pub fn DocsPage() -> impl IntoView {
    view! {
        <Title text="Documentation · pgdmn"/>
        <h1 id="documentation">"Documentation"</h1>
        <p class="lede">
            "Install instructions, and every function pgdmn installs, with its arguments."
        </p>
        <p>
            "Worked examples, with models and datasets you can download and run, are on the "
            <a href="/examples/">"Examples"</a>" page."
        </p>

        <nav class="quicklinks" aria-label="The sections on this page">
            <ul>
                <li>
                    <a href="#install">"Install"</a>
                    <span>"Build pgdmn from source with pgrx, then enable it in a database"</span>
                </li>
                <li>
                    <a href="#dmn-functions">"DMN functions"</a>
                    <span>"Load a DMN model and evaluate a decision, business knowledge model,
                    or decision service"</span>
                </li>
                <li>
                    <a href="#introspection-functions">"Introspection functions"</a>
                    <span>"Inspect a loaded model: its invocables, metadata, and original XML"</span>
                </li>
                <li>
                    <a href="#feel-functions">"FEEL functions"</a>
                    <span>"Evaluate FEEL expressions directly, with no model involved"</span>
                </li>
                <li>
                    <a href="#performance">"Performance"</a>
                    <span>"Why every function is safe to run in parallel, and how model parsing
                    is cached"</span>
                </li>
            </ul>
        </nav>

        <H2 id="install" title="Install"/>
        <p>
            "pgdmn is a PostgreSQL extension. To install it, you build it from source, then copy
            the compiled extension into place. Both are done with "
            <a
                href="https://github.com/pgcentralfoundation/pgrx"
                rel="noopener noreferrer"
                target="_blank"
            >
                "pgrx"
            </a>", the framework pgdmn is built on."
        </p>

        <H3 id="set-up-pgrx" title="Step one: set up pgrx for your database"/>
        <p>
            "Install the pgrx command-line tool. It needs a C compiler and a few build
            libraries; the "
            <a
                href="https://github.com/pgcentralfoundation/pgrx#system-requirements"
                rel="noopener noreferrer"
                target="_blank"
            >
                "pgrx system requirements"
            </a>" list them for each platform."
        </p>
        <Shell
            label="Install the pgrx CLI"
            code="cargo install --locked cargo-pgrx"
        />
        <p class="note">
            "Note: to run pgrx you need Cargo, the Rust package manager. If you do not have
            Cargo, you will need to "
            <a href="https://rust-lang.org/tools/install/" rel="noopener noreferrer" target="_blank">
                "install Rust"
            </a>"."
        </p>
        <p>
            "Then initialise pgrx. Here you choose whether pgrx manages a PostgreSQL for you, or
            uses one you already run."
        </p>

        <p>
            <strong>"If you want pgrx to manage PostgreSQL"</strong>
            ", it downloads and builds its own—the quickest way to try pgdmn end to end. "
            <code>"cargo pgrx run"</code>" builds pgdmn, installs it into that instance, and drops
            you into a "<code>"psql"</code>" shell:"
        </p>
        <Shell
            label="Let pgrx manage PostgreSQL"
            code="# Download and build a PostgreSQL 17 that pgrx manages.
cargo pgrx init --pg17 download

# Build pgdmn, install it into that PostgreSQL, and open psql.
cargo pgrx run pg17"
        />

        <p>
            <strong>"To use a PostgreSQL you already run"</strong>", point pgrx at its "
            <code>"pg_config"</code>", then build and copy the extension into the directories that "
            <code>"pg_config"</code>" reports (you may need write permission there):"
        </p>
        <Shell
            label="Install into an existing PostgreSQL"
            code="# Point pgrx at your server's pg_config.
cargo pgrx init --pg17 $(which pg_config)

# Build pgdmn and copy it into that PostgreSQL's install.
cargo pgrx install --release"
        />

        <H3 id="enable" title="Step two: enable and verify"/>
        <p>"In a database on that server, enable the extension and check it works:"</p>
        <SqlBlock
            label="Enable and verify"
            code="CREATE EXTENSION pgdmn;

-- Execute a simple function from the extension.
SELECT feel_eval('1 + 2');
--  3"
        />

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
                Decisions that depend on other decisions are resolved for you—ask for the
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
            <em>"Approved"</em>" comes back as "<code>"\"Approved\""</code>", a quoted JSONB string. Unwrapping
            that by hand means "<code>"dmn_eval(…) #>> '{}'"</code>", and a numeric decision means
            "<code>"(dmn_eval(…) #>> '{}')::numeric"</code>"."
        </p>
        <p>
            "These take the same arguments and hand back a native PostgreSQL type instead. Each
            errors if the decision returns something else."
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
                "A decision that returns a boolean—usable directly in a "<code>"WHERE"</code>
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
                "The original XML source, byte for byte."
            </Fun>
            <Fun signature="dmn_name(model dmnmodel) RETURNS text">"The model's name."</Fun>
            <Fun signature="dmn_namespace(model dmnmodel) RETURNS text">"The model's namespace."</Fun>
        </dl>

        <H2 id="feel-functions" title="FEEL functions"/>
        <p>
            "FEEL is the expression language DMN is built on. These evaluate it directly,
            with no model involved. These utility functions aren't usually what you should use,
            but are here in case you want to cross check how something evaluates."
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
            expression returns something else."
        </p>
        <dl class="fn-list">
            <Fun signature="feel_eval_numeric(expression text, context jsonb DEFAULT NULL) RETURNS numeric">
                "A FEEL number, as "<code>"numeric"</code>"."
            </Fun>
            <Fun signature="feel_eval_bool(expression text, context jsonb DEFAULT NULL) RETURNS boolean">
                "A FEEL boolean—usable directly in a "<code>"WHERE"</code>" clause or a "
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
    }
}
