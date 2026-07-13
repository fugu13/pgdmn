# TODO

## Testing

### TEST-001: Property-based tests for DMN round trips

Write property-based tests (proptest) for DMN round trips: parse -> serialize -> parse should be identity. Commit the persisted `proptest-regressions/` directories.

### TEST-003: Flaky timing assertion in cache test

`test_cache_different_models_independent` asserts the warm (cached) evaluation is at least 2x faster than the cold one; under light load the cold path can complete in ~700µs and the assertion fails spuriously (observed 2026-07-08: cold=720µs, warm=603µs). Replace the wall-clock comparison with a deterministic signal — e.g. a cache-hit counter exposed for tests, or assert on repeated-call stability rather than a fixed speedup ratio.

### TEST-002: Automated accessibility testing for the website

Integrate axe-core into the website's test suite via Playwright: launch the SSR server, run axe-core against each page, fail on any violation at the "critical" or "serious" level.

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

### WEB-001: Prerender the website to static HTML

The website ships a wasm hydration bundle but has no client-side interactivity: no signals, no server functions, no client state. Every page is prose, navigation links, and code blocks. The hydration bundle therefore buys nothing, while costing a `cdylib` target, a `wasm-release` profile, and a hard version coupling between the `wasm-bindgen` crate in `website/Cargo.lock` and the `wasm-bindgen-cli` binary on the build host — a coupling that broke `make website-build` during the dependency refresh.

Drop the `hydrate` feature, the `wasm-bindgen` and `console_error_panic_hook` dependencies, the `cdylib` crate type, and the `wasm-release` profile. Prerender all routes to static HTML at build time using Leptos static route generation, keeping the Axum SSR path only as the rendering engine that produces those files. Verify the emitted HTML preserves the accessibility guarantees the site already makes (skip link, landmarks, heading order) and that navigation works with JavaScript disabled — which prerendering makes literally true rather than aspirational.

Blocks FEAT-006 (deployment) and interacts with FEAT-004 (blog): a markdown blog must render at build time into the static output rather than at request time.

### FEAT-005: Mobile hamburger menu with focus trapping

The site navigation needs a responsive hamburger menu for narrow viewports. Must include focus trapping when open and proper aria attributes for the toggle button.

### FEAT-006: Website CI/CD deployment pipeline

Set up automated builds and deployment for the website. Hosting is GitHub Pages, with the custom domain pointed at it from Route 53. The earlier evaluation of SSR-capable hosts (Fly.io, Railway) is closed: Pages serves static files only and cannot run a server process, which makes the static prerender in WEB-001 a prerequisite rather than an optimization.

Depends on WEB-001. The pipeline is a GitHub Actions workflow that builds the prerendered output and publishes it to Pages. Two Pages-specific details the build must produce: a `CNAME` file carrying the custom domain, and a `404.html` for the not-found route, since Pages has no server-side routing to fall back on.

## Chores

### CHORE-004: Build the Docker image as the non-root user from the start

The base image installs the toolchain, fetches crates, and (formerly) initialized pgrx as root, while tests must run as the non-root `pgdmn` user — the mismatch caused BUG-001 and BUG-002 and is currently patched with a `chmod`, a `chown`, and a second `cargo pgrx init` in the `test` stage. Restructure the Dockerfile to create `pgdmn` first and run `cargo install cargo-pgrx`, `rustup component add`, `cargo fetch --locked`, and `cargo pgrx init` as that user (with `CARGO_HOME` under its home), making ownership correct by construction and collapsing the two stages into one.

### CHORE-002: OpenGraph and social meta tags

Add og:title, og:description, og:image, and Twitter card meta tags to the website shell and per-page overrides via leptos_meta.

### FEAT-001: `dmn_create_input_type` helper

A convenience function that inspects a DMN model's input requirements for a given invocable and creates a matching PostgreSQL composite type automatically. This would eliminate the manual `CREATE TYPE` step when using `dmn_record_eval`.

Example usage:
```
SELECT dmn_create_input_type(dmn_load('<xml>'), 'Eligibility', 'eligibility_input');
-- Creates: CREATE TYPE eligibility_input AS ("Age" numeric, "Income" numeric)
```

### CHORE-003: Migrate website to Rust 2024 edition (done)

The website used edition 2021 for cargo-leptos/wasm-bindgen compatibility. Migrated to edition 2024 on 2026-07-13 under rustc 1.95: `cargo fix --edition` reported no source changes, and both `make website-lint` and `make website-build` pass on the new edition.
