# TODO

## Testing

### TEST-001: Property-based tests for DMN round trips

Write property-based tests (proptest) for DMN round trips: parse -> serialize -> parse should be identity. Commit the persisted `proptest-regressions/` directories.

### TEST-003: Flaky timing assertion in cache test

`test_cache_different_models_independent` asserts the warm (cached) evaluation is at least 2x faster than the cold one; under light load the cold path can complete in ~700µs and the assertion fails spuriously (observed 2026-07-08: cold=720µs, warm=603µs). Replace the wall-clock comparison with a deterministic signal — e.g. a cache-hit counter exposed for tests, or assert on repeated-call stability rather than a fixed speedup ratio.

### TEST-002: Automated accessibility testing for the website

Integrate axe-core into the website's test suite via Playwright: launch the SSR server, run axe-core against each page, fail on any violation at the "critical" or "serious" level.

## Documentation

### DOCS-001: Feature docs for major features (dropped)

Dropped 2026-07-14 with the `docs/` directory. Explanation aimed at users belongs where users are — the website's Docs page and the walkthroughs in `website/posts/` — rather than in a parallel set of files inside the repo that only contributors would ever open. Conventions live in CLAUDE.md; findings and future work live here.

### DOCS-002: UI behavioral descriptions for the website (dropped)

Dropped 2026-07-14 with the `docs/` directory, for the same reason. The behaviour of the site is described by the site.

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

**Decided (2026-07-14):** the type-compatibility relation is dsntk's own `FeelType::is_conformant` — the same relation `dmn_eval` applies at coercion time — not a pgdmn-invented widening lattice.

Direction mapping: inputs are BACKWARD compatible iff the old input type conforms to the new one (everything the old invocable accepted is still accepted), FORWARD iff the new conforms to the old. Outputs are BACKWARD compatible iff the new output type conforms to the old one (new outputs remain readable by existing consumers), FORWARD mirrored. Exact equality reports `unchanged`; conformance in the required direction reports a compatible `type-changed`; neither reports incompatible.

The mapping must be locked in with property tests (reflexivity, Any-as-top, Null-conforms-to-everything, context width subtyping in both directions). `is_conformant` is lenient about Null — a field becoming nullable never reads as a narrowing — which the report semantics must document.

Scoping note: no typed-introspection columns get added to `dmn_invocables`/`dmn_info` ahead of this work — dsntk 0.3 added no model-introspection capability, and the typeRef resolver should be built once, here.

**Open design questions:**

- Whether to support checking multiple invocables in one call or keep it single-invocable
- How to handle invocables that exist in one model but not the other (report as incompatible vs. skip with warning)

### FEAT-005: Mobile hamburger menu

The site navigation needs a responsive hamburger menu for narrow viewports.

The original framing of this item (focus trapping, an aria-expanded toggle button) assumed JavaScript, which WEB-001 removed. Either implement it without script — a CSS-only disclosure driven by `:checked` or `:focus-within`, which needs no focus trap because nothing is rendered inert — or make the case that this feature alone justifies reintroducing a script bundle. Resolve that before designing the markup, because the two shapes differ.

### PERF-001: Tell people to let the planner parallelize

Evaluating a decision is pure, per-row, self-contained work — the ideal parallel workload — and parallelism is worth more than every other change to the query put together. The functions are already `IMMUTABLE` and `PARALLEL SAFE`, so this needs no code change; it needs a reader who knows to check that their plan actually has a Gather in it.

Measured with `make bench-shapes` (2026-07-14), same model and rows under different SQL, against a naive per-row baseline of 18 µs/row simple and 68 µs/row complex:

| Query shape | Speedup |
|---|---|
| **Permit parallelism** | **3.8×** |
| Deduplicate inputs, `MATERIALIZED` | 1.35–1.5× |
| Deduplicate inputs, plain CTE | 1.00× — no effect |
| Parallel *and* deduplicated | 1.3× |
| Model from a table column vs. an inline literal | 1.0× — no penalty |

Two findings worth writing down for users, on the Docs page and in the README.

**The deduplication trap.** The obvious way to write "evaluate once per distinct input, then join the answers back" is a plain CTE, and it does *nothing*: the planner inlines the CTE and pulls `dmn_eval` back up above the join, so it evaluates once per output row exactly as before. It looks like an optimization and measures like the baseline. `WITH … AS MATERIALIZED` is what holds the evaluation down at the distinct-row count.

**They do not compose.** A `MATERIALIZED` CTE is scanned serially, throwing the parallelism away. Given the choice, take the parallelism.

### PERF-002: Fingerprint models instead of hashing the XML per call

`cache.rs` keys the evaluator cache on the entire XML string, so every `dmn_eval` hashes the whole model to find its evaluator, then compares the string on a hit. The cost scales with model size rather than with the decision being made. On the benchmark's small models it is a few percent; on a large model it would not be.

Compute a fingerprint once, in `dmn_load`, store it on `DmnModel`, and key the cache on that. Measure before and after with `make bench` — worth doing only if the numbers say so, and the numbers currently say it is not the bottleneck.

**What is the bottleneck: FEEL evaluation itself.** `dmn_record_eval` skips the JSONB path entirely and is only **5–9% faster** than `dmn_eval` (16.4 vs 18.1 µs/row simple, 65.5 vs 69.0 complex). Any future work aimed at the serialization hops — direct datum conversion, SPI batch functions, hstore or variadic inputs — is chasing at most a tenth of the runtime. Profile FEEL evaluation before spending effort there.

### WEB-003: Respect prefers-color-scheme

The site defines its palette once, in light colours, and never consults `prefers-color-scheme`. A visitor whose system is set to dark gets a bright white page regardless. CLAUDE.md already forbids a dark-mode *toggle* on the grounds that the system preference is the right signal — but the site does not currently honour that signal either way.

Add a `prefers-color-scheme: dark` block redefining the custom properties in `style/main.scss`. Verify the dark palette holds 4.5:1 contrast for text and 3:1 for interface elements and focus indicators, which the highlighted table rows and the muted secondary text are the most likely to fail.
### RANGE-002: Accept PG range types as inputs in the record-eval path

Extend `pg_datum_to_feel` with arms for `NUMRANGEOID`, `INT4RANGEOID`, `INT8RANGEOID`, `DATERANGEOID`, `TSRANGEOID`: read via pgrx `Range<T>`, convert bounds with the existing scalar arms, and build `Value::Range` with `IntervalType::Closed`/`Opened`/`OpenedUndef` (infinite bound → `OpenedUndef`). No SQL signature changes — `feel_record_eval` and `dmn_record_eval` start accepting range-typed columns, the only inbound channel for binding a FEEL range variable (JSON cannot carry one). Decide the policy for PG `empty` ranges (error is more consistent with the mixed-interval precedent in convert.rs) and defer `TSTZRANGEOID` with the timezone question. Round-trip property tests (PG range → FEEL → `feel_eval_numrange` → PG range) pair with the existing numrange output path. Discrete-range canonicalization (`int4range '[1,4]'` arrives as `[1,5)`) is same-set but changes `.end`/`.end included` property values — document it.

### RANGE-003: feel_eval_daterange and feel_eval_tsrange typed variants

Stage 2 of the numrange work: map FEEL date and date-time ranges onto `daterange`/`tsrange` via pgrx `Range<Date>`/`Range<Timestamp>`, mirroring `feel_eval_numrange`. Note PG canonicalizes discrete `daterange` (`[a..b]` becomes `[a..b+1)`) — same set, different rendering; document alongside the implementation.

### UNARY-001: feel_unary_test — evaluate decision-table-style unary tests against a value

`feel_unary_test(tests text, value jsonb, context jsonb DEFAULT NULL) → boolean`: parse `tests` with `parse_unary_tests` (the exact grammar of a decision-table input entry, including `-`, `not(...)`, ranges, and comma lists), bind `value` as the FEEL `?` placeholder, build the `In` node exactly as dsntk's own decision-table evaluator does, and evaluate. Enables rules-stored-in-tables matching (`WHERE feel_unary_test(r.quantity_test, to_jsonb(42))`). Needs a `/spec` first: the null-result policy (error like `feel_eval_bool` vs. decision-table-style false) and the temporal-typing story (a JSONB string stays a FEEL string, so `< today()` needs context-passed dates or later typed overloads) are behavior decisions.

### NULLS-001: Surface explained FEEL nulls the JSONB paths discard

dsntk 0.3 attaches explanations to many nulls (`null(position must not be zero)`) that `feel_eval`/`dmn_eval` currently flatten to bare JSON null (convert.rs maps `Null(_)` dropping the message). Candidate shapes: `feel_eval_detail`/`dmn_eval_detail` returning `(result jsonb, null_reason text)`, or emitting the explanation as a DEBUG-level notice inside the existing functions (zero new API). The explanation text is upstream-owned prose that rewords between releases — whatever ships must document that it is diagnostic, not a stable contract. Decide the shape before implementing.

### WEB-002: Automated link checking for the website

Nothing verifies that the site's internal links resolve. The prerendered output makes this cheap and exact: every link either corresponds to a file in `website/dist` or it does not. Add a check to the `Website` workflow that walks the generated HTML, resolves each internal `href` against `dist/`, and fails on any that points nowhere.

This would have caught the trailing-slash problem WEB-001 fixed by hand (linking `/why` when the file is `why/index.html`), and it guards the class of breakage a static site is most prone to: a renamed route silently leaving dead links behind.

### WEB-001: Prerender the website to static HTML (done)

The website shipped a wasm hydration bundle (343 KB of wasm plus 19.5 KB of JS) to hydrate pages with no client-side interactivity at all — no signals, no server functions, no client state. The bundle bought nothing while forcing a `wasm-bindgen` crate/CLI version match on the build host and a server-capable host in production.

Done 2026-07-13. Dropped the `hydrate` feature, `wasm-bindgen`, `console_error_panic_hook`, the `cdylib` crate type, the `wasm-release` profile, and `cargo-leptos` itself. Every route is now `SsrMode::Static` and rendered to `website/dist` by a `prerender` binary, which also compiles Sass in-process via `grass` and emits `404.html` and `.nojekyll`. The decision is recorded in CLAUDE.md.

### FEAT-003: Syntax highlighting in code blocks (done)

Done 2026-07-14. SQL is highlighted at build time by syntect (`website/src/highlight.rs`), emitting CSS classes rather than inline colours — the palette lives in `style/main.scss`, in dark tones chosen to stay legible without colour. The client-side option was closed by WEB-001's no-JavaScript rule. Markdown code fences in blog posts are highlighted through the same path. DMN (XML) highlighting is not implemented: no XML is shown on the site.

### FEAT-004: Markdown-based blog infrastructure (done)

Done 2026-07-14. Posts are markdown files in `website/posts/`, embedded at compile time (`include_dir`) and rendered by `pulldown-cmark` during the prerender. A single `/blog/:slug` route enumerates its slugs from the files themselves via Leptos's `StaticRoute::prerender_params`, so adding a post is adding a file — no Rust change, no route to register. Front matter carries title, date, summary, and the example the post walks through. A malformed post fails the build, naming the file.

The one convention the blog adds: a paragraph reading `Table: …` immediately above a markdown table becomes that table's caption, which is what lets the renderer give tables a caption, `scope`-d column headers, and a keyboard-reachable scroll wrapper that bare markdown would not have.

### FEAT-006: Website CI/CD deployment pipeline (done)

Set up automated builds and deployment for the website. Hosting is GitHub Pages, canonically at `www.pgdmn.com` with the apex redirecting there. The earlier evaluation of SSR-capable hosts (Fly.io, Railway) is closed: Pages serves static files only and cannot run a server process.

Done 2026-07-13. The `Website` workflow (`.github/workflows/website.yml`) lints, prerenders, asserts the output ships no scripts or wasm and that the deployable artifact is complete, then publishes `website/dist` to Pages on every push to `main`. The prerender emits `CNAME`, `404.html`, and `.nojekyll`.

Going live still needs three manual steps, recorded in RELEASEPLAN.md: a paid plan for Pages from a private repo, the Route 53 A/AAAA/CNAME records, and enforcing HTTPS once the certificate is issued.

## CI

### CI-001: Publish the extension test image to GHCR as a cache fallback

Today `ci.yml` rebuilds `pgdmn-test` with a `type=gha` buildx layer cache injected via the Makefile's `DOCKER_BUILD_CACHE` variable. If cold-cache runs (e.g. after `Cargo.lock` changes) get too slow, publish a prebuilt image to GitHub Container Registry whenever the Dockerfile or `Cargo.lock` changes, and pull it in CI as a fallback. This would cap the worst-case build time without changing the normal cache path.

### CI-002: Scheduled DMN eval benchmark with regression tracking

`make bench` is gated behind `PGDMN_BENCH=1` and deliberately excluded from PR CI because microbenchmark numbers are noisy on shared runners (see the canary-gated benchmarking notes). Add a nightly or otherwise scheduled workflow that runs the benchmark and records results over time, so drift is caught without flaking PRs.

### CI-003: Path-based job skipping for docs-only and website-only PRs

`ci.yml` intentionally has no `paths:` filter so the required `CI aggregate` check always reports; a path-filtered required check stays pending forever and blocks merges. Add a `dorny/paths-filter`-style gate (or equivalent) that skips the extension lint/test work for docs-only and website-only PRs while still letting the aggregate job run unconditionally, restoring the savings without reintroducing that failure mode.

### CI-004: Documentation-integrity check in CI

Add a CI check that enforces this project's documentation conventions automatically: the README has a SQL example for every function, and `docs/` structure invariants hold. The sibling `datasend` repo runs a `scripts/doc_check.py` in CI for the same purpose; pgdmn's doc conventions are currently enforced only by review.

## Dependencies

### DEPS-001: Drop the HTTP/TLS stack dsntk 0.3 embeds in the extension

dsntk-feel-evaluator 0.3.0 hard-depends on reqwest 0.13 with the rustls/aws-lc provider, used only by its `evaluator_java.rs` — a blocking HTTP client that calls a local Java RPC server (127.0.0.1:22023) when a model invokes an "external Java function". pgdmn never wants that inside a PostgreSQL backend, but feature unification is additive so it cannot be opted out downstream; the extension `.so` now statically links reqwest, rustls, and the AWS-LC C library, and the Docker image needs cmake to build aws-lc-sys. The fix is upstream (fits the minimal-upstreamable-patch working agreement): land a small PR on DecisionToolkit/dsntk putting `evaluator_java`/`evaluator_pmml` and the reqwest dependency behind an off-by-default cargo feature (e.g. `external-functions`) that returns an explained null when disabled; 0.2's `default-features = false` reqwest with a ring provider is the fallback shape. pgdmn then consumes dsntk-feel-evaluator with the feature off, removing the HTTP/TLS stack from the backend entirely and demoting the `src/guard.rs` boundary rejection from load-bearing to belt-and-suspenders (the guard is inherently partial for DMN models — see the module docs in `src/guard.rs`). Once merged and adopted, remove cmake from the Dockerfile (comment there points here). No local `[patch]`/fork in the interim.

### DEPS-002: Upstream fix for non-finite FeelNumber Display panic

dsntk-feel-number's `Display` unwraps on the assumption that `bid128_to_string` output contains `'E'`; ±Inf/NaN — reachable because `Mul`/`Add` results are not finiteness-guarded, unlike `pow`/`from_str` — panic instead of printing. pgdmn guards its direct number conversions (BUG-004), but a non-finite number inside a `Value::Range` endpoint still panics via the Display catch-all in `feel_to_json`, and every other dsntk consumer stays exposed. Propose upstream either guarding arithmetic like `pow` does (overflow → FEEL null) or making `Display` total (print `+Inf`/`-Inf`/`NaN`). Minimal upstreamable patch per the working agreement.

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
