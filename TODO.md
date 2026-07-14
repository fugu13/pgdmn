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

Add `docs/ux/{aspect}.md` behavioral descriptions covering the website's existing UI. Descriptions cover the full interaction lifecycle and must not reference implementation details.

Navigation (`docs/ux/navigation.md`) and the Examples page's downloads, code blocks, and tables (`docs/ux/examples-and-code.md`) are written. Still to cover: the home page's hero and quick start, and the function reference's structure on the Docs page.

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

### FEAT-005: Mobile hamburger menu

The site navigation needs a responsive hamburger menu for narrow viewports.

The original framing of this item (focus trapping, an aria-expanded toggle button) assumed JavaScript, which WEB-001 removed. Either implement it without script — a CSS-only disclosure driven by `:checked` or `:focus-within`, which needs no focus trap because nothing is rendered inert — or make the case that this feature alone justifies reintroducing a script bundle. Resolve that before designing the markup, because the two shapes differ.

### PERF-001: Tell people to let the planner parallelize

Evaluating a decision is pure, per-row, self-contained work — the ideal parallel workload — and `make bench-shapes` measures a **3.8×** speedup from parallelism alone, far more than any other change to the query. The functions are already `IMMUTABLE` and `PARALLEL SAFE`, so this needs no code change; it needs a reader who knows to check that their plan actually has a Gather in it.

Document this where someone doing bulk evaluation will read it: the Docs page and the README. Include the deduplication trap alongside it — the obvious "evaluate once per distinct input" CTE silently does nothing unless it is `MATERIALIZED`, because the planner pulls the call back up above the join. Both findings are recorded in `docs/improvements.md`.

### PERF-002: Fingerprint models instead of hashing the XML per call

`cache.rs` keys the evaluator cache on the entire XML string, so every `dmn_eval` hashes the whole model to find its evaluator, then compares the string on a hit. The cost scales with model size rather than with the decision being made. On the benchmark's small models it is a few percent; on a large model it would not be.

Compute a fingerprint once, in `dmn_load`, store it on `DmnModel`, and key the cache on that. Measure before and after with `make bench` — this is worth doing only if the numbers say so, and the numbers currently say it is not the bottleneck (see PERF-001 and `docs/improvements.md`).

### WEB-003: Respect prefers-color-scheme

The site defines its palette once, in light colours, and never consults `prefers-color-scheme`. A visitor whose system is set to dark gets a bright white page regardless. CLAUDE.md already forbids a dark-mode *toggle* on the grounds that the system preference is the right signal — but the site does not currently honour that signal either way.

Add a `prefers-color-scheme: dark` block redefining the custom properties in `style/main.scss`. Verify the dark palette holds 4.5:1 contrast for text and 3:1 for interface elements and focus indicators, which the highlighted table rows and the muted secondary text are the most likely to fail.

### WEB-002: Automated link checking for the website

Nothing verifies that the site's internal links resolve. The prerendered output makes this cheap and exact: every link either corresponds to a file in `website/dist` or it does not. Add a check to the `Website` workflow that walks the generated HTML, resolves each internal `href` against `dist/`, and fails on any that points nowhere.

This would have caught the trailing-slash problem WEB-001 fixed by hand (linking `/why` when the file is `why/index.html`), and it guards the class of breakage a static site is most prone to: a renamed route silently leaving dead links behind.

### WEB-001: Prerender the website to static HTML (done)

The website shipped a wasm hydration bundle (343 KB of wasm plus 19.5 KB of JS) to hydrate pages with no client-side interactivity at all — no signals, no server functions, no client state. The bundle bought nothing while forcing a `wasm-bindgen` crate/CLI version match on the build host and a server-capable host in production.

Done 2026-07-13. Dropped the `hydrate` feature, `wasm-bindgen`, `console_error_panic_hook`, the `cdylib` crate type, the `wasm-release` profile, and `cargo-leptos` itself. Every route is now `SsrMode::Static` and rendered to `website/dist` by a `prerender` binary, which also compiles Sass in-process via `grass` and emits `404.html` and `.nojekyll`. Specification in `docs/specifications/WEB-001-static-prerender.md`; decision recorded in CLAUDE.md.

### FEAT-003: Syntax highlighting in code blocks (done)

Done 2026-07-14. SQL is highlighted at build time by syntect (`website/src/highlight.rs`), emitting CSS classes rather than inline colours — the palette lives in `style/main.scss`, in dark tones chosen to stay legible without colour. The client-side option was closed by WEB-001's no-JavaScript rule. Markdown code fences in blog posts are highlighted through the same path. DMN (XML) highlighting is not implemented: no XML is shown on the site.

### FEAT-004: Markdown-based blog infrastructure (done)

Done 2026-07-14. Posts are markdown files in `website/posts/`, embedded at compile time (`include_dir`) and rendered by `pulldown-cmark` during the prerender. A single `/blog/:slug` route enumerates its slugs from the files themselves via Leptos's `StaticRoute::prerender_params`, so adding a post is adding a file — no Rust change, no route to register. Front matter carries title, date, summary, and the example the post walks through. A malformed post fails the build, naming the file.

The one convention the blog adds: a paragraph reading `Table: …` immediately above a markdown table becomes that table's caption, which is what lets the renderer give tables a caption, `scope`-d column headers, and a keyboard-reachable scroll wrapper that bare markdown would not have.

### FEAT-006: Website CI/CD deployment pipeline (done)

Set up automated builds and deployment for the website. Hosting is GitHub Pages, canonically at `www.pgdmn.com` with the apex redirecting there. The earlier evaluation of SSR-capable hosts (Fly.io, Railway) is closed: Pages serves static files only and cannot run a server process.

Done 2026-07-13. The `Website` workflow (`.github/workflows/website.yml`) lints, prerenders, asserts the output ships no scripts or wasm and that the deployable artifact is complete, then publishes `website/dist` to Pages on every push to `main`. The prerender emits `CNAME`, `404.html`, and `.nojekyll`.

Going live still needs three manual steps, recorded in RELEASEPLAN.md: a paid plan for Pages from a private repo, the Route 53 A/AAAA/CNAME records, and enforcing HTTPS once the certificate is issued.

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
