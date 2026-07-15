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

Add syntax highlighting for SQL and DMN (XML) code examples on the website. The `SqlBlock` component should accept a language parameter to select the grammar.

The client-side option (Prism.js or similar) is closed: WEB-001 ships zero JavaScript, and highlighting prose-level code samples does not justify reversing that. Highlight at build time in Rust — syntect is the obvious candidate — emitting styled markup into the prerendered HTML.

### FEAT-004: Markdown-based blog infrastructure

Build a blog system for the website that reads markdown files from a directory, renders them as pages, and generates an index with titles and dates.

Must render at build time into the prerendered output (WEB-001), not per request: there is no server in production to read files at request time.

### FEAT-005: Mobile hamburger menu

The site navigation needs a responsive hamburger menu for narrow viewports.

The original framing of this item (focus trapping, an aria-expanded toggle button) assumed JavaScript, which WEB-001 removed. Either implement it without script — a CSS-only disclosure driven by `:checked` or `:focus-within`, which needs no focus trap because nothing is rendered inert — or make the case that this feature alone justifies reintroducing a script bundle. Resolve that before designing the markup, because the two shapes differ.

### WEB-002: Automated link checking for the website

Nothing verifies that the site's internal links resolve. The prerendered output makes this cheap and exact: every link either corresponds to a file in `website/dist` or it does not. Add a check to the `Website` workflow that walks the generated HTML, resolves each internal `href` against `dist/`, and fails on any that points nowhere.

This would have caught the trailing-slash problem WEB-001 fixed by hand (linking `/why` when the file is `why/index.html`), and it guards the class of breakage a static site is most prone to: a renamed route silently leaving dead links behind.

### WEB-001: Prerender the website to static HTML (done)

The website shipped a wasm hydration bundle (343 KB of wasm plus 19.5 KB of JS) to hydrate pages with no client-side interactivity at all — no signals, no server functions, no client state. The bundle bought nothing while forcing a `wasm-bindgen` crate/CLI version match on the build host and a server-capable host in production.

Done 2026-07-13. Dropped the `hydrate` feature, `wasm-bindgen`, `console_error_panic_hook`, the `cdylib` crate type, the `wasm-release` profile, and `cargo-leptos` itself. Every route is now `SsrMode::Static` and rendered to `website/dist` by a `prerender` binary, which also compiles Sass in-process via `grass` and emits `404.html` and `.nojekyll`. Specification in `docs/specifications/WEB-001-static-prerender.md`; decision recorded in CLAUDE.md.

### FEAT-006: Website CI/CD deployment pipeline (done)

Set up automated builds and deployment for the website. Hosting is GitHub Pages, canonically at `www.pgdmn.com` with the apex redirecting there. The earlier evaluation of SSR-capable hosts (Fly.io, Railway) is closed: Pages serves static files only and cannot run a server process.

Done 2026-07-13. The `Website` workflow (`.github/workflows/website.yml`) lints, prerenders, asserts the output ships no scripts or wasm and that the deployable artifact is complete, then publishes `website/dist` to Pages on every push to `main`. The prerender emits `CNAME`, `404.html`, and `.nojekyll`.

Going live still needs three manual steps, recorded in RELEASEPLAN.md: a paid plan for Pages from a private repo, the Route 53 A/AAAA/CNAME records, and enforcing HTTPS once the certificate is issued.

## Dependencies

### DEPS-001: Drop the HTTP/TLS stack dsntk 0.3 embeds in the extension

dsntk-feel-evaluator 0.3.0 hard-depends on reqwest 0.13 with the rustls/aws-lc provider, used only by its `evaluator_java.rs` — a blocking HTTP client that calls a local Java RPC server (127.0.0.1:22023) when a model invokes an "external Java function". pgdmn never wants that inside a PostgreSQL backend, but feature unification is additive so it cannot be opted out downstream; the extension `.so` now statically links reqwest, rustls, and the AWS-LC C library, and the Docker image needs cmake to build aws-lc-sys. The fix is upstream (fits the minimal-upstreamable-patch working agreement): propose feature-gating the Java external-function support in dsntk-feel-evaluator, or restoring 0.2's `default-features = false` reqwest with a ring provider. Once merged and adopted, remove cmake from the Dockerfile (comment there points here).

## Chores

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

### CHORE-003: Migrate website to Rust 2024 edition (done)

The website used edition 2021 for cargo-leptos/wasm-bindgen compatibility. Migrated to edition 2024 on 2026-07-13 under rustc 1.95: `cargo fix --edition` reported no source changes, and both `make website-lint` and `make website-build` pass on the new edition.
