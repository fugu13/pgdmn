# TODO

## Performance

### PERF-001: Zero-copy DmnModel datum layout

`dmn_eval` still CBOR-decodes the whole DmnModel struct (including the
full XML string) from the datum on every row; only the cache probe was
made O(1) via the stored content hash. A manual varlena layout
(`[hash][ns][name][xml]`) with borrowed `&str` views would make per-row
cost O(1) in model size, materializing the XML only on cache miss.
Breaks the on-disk format (acceptable pre-1.0, needs a migration note).

### PERF-002: Cache compiled regexes in dsntk-feel-regex (matches BIF)

`replace()`/`split()` now reuse compiled regexes via a thread-local
cache in dsntk-feel-evaluator, but `matches()` compiles inside the
dsntk-feel-regex crate on every call. Needs a small change in that
crate to route through the same cache.

### PERF-003: Arc payloads for Value::List / Value::String / FunctionDefinition

FeelContext is Arc-backed copy-on-write, but list, string, and function
payloads still deep-clone on every scope resolution. Requires a
compiler-guided sweep across dsntk-model-evaluator (~43 construction and
match sites), which was out of scope for the minimal patch set. Expected
to matter for list-heavy and BKM-heavy models.

### PERF-004: DecisionServiceEvaluator per-call read lock

Two-phase construction forces an RwLock read per decision-service call;
`Arc::new_cyclic` would remove it. Minor cost, larger refactor.

### PERF-005: Shared or serialized evaluator cache across backends

The evaluator cache is thread-local, so every new PostgreSQL backend
re-parses and re-builds each model on first use (~ms per model). For
connection-churn workloads without a pooler, consider a shared-memory
cache or a serialized precompiled-evaluator representation.

### PERF-006: Upstream the vendored dsntk performance patch set

The vendor/ changes are deliberately minimal and separable (one commit
per fix, `PGDMN:` markers). Offer them upstream to dsntk; each accepted
PR shrinks the maintained delta. The perf report's scope-vs-speed table
is the negotiation sheet.

## Conversions

### CONVERT-001: Integers above i64::MAX lose precision in feel_to_json

`feel_to_json` converts a FEEL number by trying `parse::<i64>()` and falling
back to `parse::<f64>()`. A JSON integer above `i64::MAX` (e.g. `2^63`)
survives the trip into FEEL exactly (decimal128) but comes back as a lossy
f64 (`9.223372036854776e18`). Either serialize such values as JSON strings,
or use serde_json's arbitrary-precision feature. The property tests in
`src/convert_props.rs` deliberately generate only within i64 until this is
fixed.

## Testing

### TEST-001: Property-based tests for DMN round trips

Write property-based tests (proptest) for DMN round trips: parse -> serialize -> parse should be identity. Commit the persisted `proptest-regressions/` directories.

### TEST-002: Automated accessibility testing for the website

Integrate axe-core into the website's test suite via Playwright: launch the SSR server, run axe-core against each page, fail on any violation at the "critical" or "serious" level.

### TEST-003: Flaky timing assertion in cache test (done)

`test_cache_different_models_independent` asserted the warm (cached) evaluation is at least 2x faster than the cold one and failed spuriously under load (observed again 2026-07-14 after the evaluator got faster: cold=464µs, warm=367µs). Fixed by exposing a test-only `dmn_evaluator_builds()` counter (`src/cache.rs`, gated to test builds) and asserting build counts instead of wall-clock ratios.

## Documentation

### DOCS-001: Feature docs for major features

Create one file per major feature in `docs/`: the FEEL evaluation functions, the DMN model type and evaluation functions, and the website. Each file needs a one-sentence summary and a list of user actions the feature accomplishes, described from the user's needs rather than mechanically. No code blocks in `docs/` — use tables, prose, and mermaid diagrams.

### DOCS-002: UI behavioral descriptions for the website

Add `docs/ux/{aspect}.md` behavioral descriptions covering the website's existing UI (navigation, page structure, code example presentation). Descriptions cover the full interaction lifecycle and must not reference implementation details.

## Accessibility

### A11Y-001: Install accessibility review agents

Install the core agents from Community-Access/accessibility-agents into `.claude/agents/` with `install.sh --project`, so UI changes to the website get structured accessibility review before commit.

## Features

### FEAT-002: DMN compatibility checking functions

SQL functions inspired by Kafka Schema Registry compatibility semantics, applied to DMN invocable input and output definitions. These enable versioned evolution of DMN models while verifying that consumers and producers remain compatible.

**Core function:** `dmn_compat(new_model DmnModel, old_models DmnModel[], invocable text) → JSONB`

Compares the input and output definitions of a named invocable across a new model and one or more old models, producing a structured JSONB report.

**Compatibility directions (checked separately for inputs and outputs):**

- **BACKWARD (inputs):** The new invocable accepts all inputs the old one could. Adding optional inputs (with defaults) is allowed; removing or narrowing required inputs is not.
- **FORWARD (inputs):** The old invocable could accept all inputs the new one can. Adding required inputs breaks forward compatibility; removing optional inputs is allowed.
- **FULL (inputs):** Both backward and forward — only optional-with-default fields may be added or removed.
- **BACKWARD (outputs):** Consumers of the old outputs can still consume the new outputs. Removing output fields or widening types breaks backward output compatibility.
- **FORWARD (outputs):** Consumers of the new outputs could consume the old outputs. Adding output fields or narrowing types breaks forward output compatibility.
- **FULL (outputs):** Both directions for outputs.

**Report structure (JSONB):** Per old-model entry, per field: field name, direction (input/output), change type (added/removed/type-changed/unchanged), whether the change is backward-compatible, forward-compatible, and a human-readable detail string. Top-level summary booleans for each compatibility mode.

**Array input enables transitive checks:** By passing all historical model versions as the array, the caller gets a report covering compatibility against every prior version, not just the immediately previous one.

**Open design questions:**

- Exact FEEL type compatibility/widening rules (strict equality vs. some form of type promotion)
- Whether to support checking multiple invocables in one call or keep it single-invocable
- How to handle invocables that exist in one model but not the other (report as incompatible vs. skip with warning)

### FEAT-003: Syntax highlighting in code blocks

Add syntax highlighting for SQL and DMN (XML) code examples on the website. Evaluate Rust-side highlighting (e.g. syntect) vs client-side (Prism.js or similar) and choose based on SSR compatibility and bundle size. The SqlBlock component should accept a language parameter to select the grammar.

### FEAT-004: Markdown-based blog infrastructure

Build a blog system for the website that reads markdown files from a directory, renders them as pages, and generates an index with titles and dates.

### FEAT-005: Mobile hamburger menu with focus trapping

The site navigation needs a responsive hamburger menu for narrow viewports. Must include focus trapping when open and proper aria attributes for the toggle button.

### FEAT-006: Website CI/CD deployment pipeline

Set up automated builds and deployment for the website. Evaluate hosting options (Fly.io, Railway, or similar SSR-capable hosts).

## Chores

### CHORE-003: Migrate website to Rust 2024 edition

The website uses edition 2021 for cargo-leptos/wasm-bindgen compatibility. Once the toolchain supports it, migrate to edition 2024.

### CHORE-002: OpenGraph and social meta tags

Add og:title, og:description, og:image, and Twitter card meta tags to the website shell and per-page overrides via leptos_meta.

### CHORE-004: Build the Docker image as the non-root user from the start (done)

The base image installed the toolchain, fetched crates, and (formerly) initialized pgrx as root, while tests must run as the non-root `pgdmn` user — the mismatch caused BUG-001 and BUG-002 and was patched with a `chmod`, a `chown`, and a second `cargo pgrx init` in a separate `test` stage. The Dockerfile now creates `pgdmn` first and runs `cargo install cargo-pgrx`, `rustup component add`, `cargo fetch --locked`, and `cargo pgrx init` as that user (with `CARGO_HOME` under its home), making ownership correct by construction in a single stage.

### FEAT-001: `dmn_create_input_type` helper

A convenience function that inspects a DMN model's input requirements for a given invocable and creates a matching PostgreSQL composite type automatically. This would eliminate the manual `CREATE TYPE` step when using `dmn_record_eval`.

Example usage:
```
SELECT dmn_create_input_type(dmn_load('<xml>'), 'Eligibility', 'eligibility_input');
-- Creates: CREATE TYPE eligibility_input AS ("Age" numeric, "Income" numeric)
```
