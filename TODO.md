# TODO

## Public release readiness

### PUBLIC-001: Strip RELEASEPLAN.md before flipping public (user decision)

The panel flagged RELEASEPLAN.md's promotion-target names and upsell
strategy as the one item that cannot be un-seen after publication (it is
linked from the README). Move internal marketing content to private
notes, and decide explicitly whether the git history containing it is
acceptable or the repository should be published with fresh history.

### PUBLIC-002: SECURITY.md and private vulnerability reporting

Add SECURITY.md (report via GitHub private security advisories; trust
model: DMN XML and FEEL expressions are untrusted SQL input evaluated
in-process; no network stack linked — external evaluation compiled out;
vendored-engine issues coordinated with upstream). Enable private
vulnerability reporting when the repo goes public.

### PUBLIC-003: Extension CI workflow

.github/workflows currently covers only the website. Add a workflow for
PRs touching src/, vendor/, Cargo.*, Makefile, Dockerfile: at minimum
`make vendor-check` + `cargo check` + the vendored test gate, plus
cargo-audit (advisories against vendored versions do not surface via
Dependabot for path deps) and a guard against stray duplicate files
(a "* 2.toml" incident occurred once).

### PUBLIC-004: Durable vendor manifests

Commit a CHECKSUMS manifest (per-crate sha256 of the vendored tarballs,
written by vendor-upgrade). vendor/PATCHES.md exists (summaries +
measured effects; update it as part of the one-commit-per-change
discipline) — extend it with upstream-PR links as UPSTREAM-001 executes,
teach vendor-status to reconcile the git layer against it, and disable
squash-merge for vendor PRs so patch-layer separability survives GitHub
merges.

### PUBLIC-005: CONTRIBUTING.md, CODEOWNERS, and a Copilot vendor instruction

Document the vendor/ contribution rules where outsiders will look
(never edit vendor/ in feature PRs; one commit per change; PGDMN:
markers; no reformatting), route vendor/ changes via CODEOWNERS, and
add .github/instructions/vendor.instructions.md so Copilot review stops
proposing stylistic rewrites of vendored code.

### PUBLIC-007: Commit a benchmark baseline for the vendor-inspect flow

vendor-inspect step 4 needs a committed reference (the detailed
measurement report is a private session artifact). Commit a per-scenario baseline table
(median, machine, toolchain, date — e.g. profiling/baselines/) and
point the prompt at it, so someone other than the original author can
run the regression gate.

### PUBLIC-008: Decide the lint-churn policy for vendored code

`make lint`'s -D warnings covers vendored code (path deps escape
cap-lints), so toolchain bumps force edits to pristine upstream files
(rustc 1.95 already did). Either scope the hard gate to the pgdmn
package (`cargo clippy -p pgdmn`) or record lint-only vendor commits as
fold-into-next-pristine-swap churn in PATCHES.md. Also decide whether
the personal content-preference rule in CLAUDE.md's Behaviors section
should ship in a public repo, and add a DMN trademark acknowledgment
(OMG) to the README if counsel thinks it worthwhile.

### PUBLIC-009: Evaluate whether pgdmn should be a trusted extension

`pgdmn.control` now sets `superuser = true` explicitly (matches
Postgres's documented default; every install-script command is `CREATE
FUNCTION ... LANGUAGE c`, which requires superuser regardless of this
flag). Separately, and only after a deliberate security review, decide
whether to also set `trusted = true`. Postgres runs a trusted
extension's install script *as* superuser even when invoked by a
non-superuser, so `trusted = true` would let any role with `CREATE`
privilege on the database install this natively-compiled, unsandboxed
shared library — a real privilege-escalation surface, not a follow-on
to the superuser fix. Needs a pgvector-style audit (review every
installed function/type for anything a non-superuser could turn into
more access than they started with) before it's safe to flip.

### PUBLIC-010: Enable secret scanning and push protection right after going public

GitHub's native secret-scanning and push-protection are free for public
repos and near-zero setup, but only apply once the repo is public — do
this immediately after the visibility flip, not as an afterthought. A
point-in-time preflight sweep finding no secrets today isn't a
substitute for an ongoing control on every future push.

### PUBLIC-011: Set repo topics and homepage after going public

The GitHub repo currently has no topics and no homepage URL set. Add
DMN/FEEL/PostgreSQL-relevant topics for discoverability and set the
homepage to www.pgdmn.com once the site is live — cosmetic, but worth
doing at the same moment as the visibility flip rather than forgetting
it.

## Upstream

### UPSTREAM-001: Open the upstream PRs and link them (executes PERF-006)

BUG-003 first, then the DEPS-001 feature gate, DEPS-002, H4, H14, H9,
and H3+H20 as a pair. Link each PR from TODO.md and vendor/PATCHES.md
so the public repo visibly demonstrates the upstream-first posture.

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

### PERF-007: Memoize literal-only unary-test values in decision tables

The 0.3 opportunity hunt (N4) found that decision-table entries are
overwhelmingly literals, yet build_in evaluates the right-hand side per
call — one Box per comparison, two per range, a Vec per list — paid
rules x inputs x SQL-rows. A build-time AST walk detecting
scope-independent right-hand sides could evaluate once into a cached
Value and match by reference. Measure-first on a wide table of
range/list entries.

### PERF-008: Cache range() bif construction

range("[18..65)") re-parses its literal and rebuilds an evaluator tree
on every call (hunt finding N6); pgdmn's expression cache only covers
the outer expression, so feel_eval_numrange with a literal range() pays
a full FEEL parse per row. Either extend the vendored regex-cache
pattern to range(), or document that direct '[low..high)' syntax
compiles once. Measure-first.

### PERF-006: Upstream the vendored dsntk performance patch set

The vendor/ changes are deliberately minimal and separable (one commit
per fix, `PGDMN:` markers). Offer them upstream to dsntk; each accepted
PR shrinks the maintained delta. The perf report's scope-vs-speed table
is the negotiation sheet.

Audited against upstream 0.3.0 (released 2026-04-29) by source-diffing
the published crates against pristine 0.2.0: it contains none of this
performance work and does not fix BUG-003 (the `?` input-entry bug —
lead with that PR). It does fix the H21 latent defect (FeelNumber
integer comparisons via FFI string round trips) that we left untouched.
H20 and H3 must be offered as a pair (measured interaction).

### CHORE-005: Evaluate re-vendoring on dsntk 0.3.0 (done)

0.3.0 is mostly FEEL range/interval rework (`IntervalType` replaces
bools in AST/Value variants), new built-ins, expanded `in` semantics,
and Rust 2024 let-chain restyling. Port costs identified: the H13 AST
walker must add the new variants (fails the build by design), H14/H19
need manual rebases over heavily-restyled builders/bifs files, and the
Docker toolchain must move from Rust 1.85 to ≥1.88 for let-chains
(re-test the dev-profile LTO ICE while at it). Everything else rebases
near-clean — decision_table.rs, model_definitions.rs, context.rs and
the model-evaluator files are functionally unchanged upstream.

Executed 2026-07-14: pristine 0.3.0 vendored (13 crates, recognizer
joined the graph), full patch layer re-applied commit-by-commit with
gates, H13 walker extended for IntervalType, H19 reworked around 0.3's
allocation-free Name::as_str (lazy memoization retained after the eager
variant measurably regressed), BUG-003 re-verified absent upstream and
re-applied. Engine wins on 0.3 match the 0.2-era measurements.

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

**Decided (2026-07-14):** the type-compatibility relation is dsntk's own `FeelType::is_conformant` — the same relation `dmn_eval` applies at coercion time — not a pgdmn-invented widening lattice.

Direction mapping: inputs are BACKWARD compatible iff the old input type conforms to the new one (everything the old invocable accepted is still accepted), FORWARD iff the new conforms to the old. Outputs are BACKWARD compatible iff the new output type conforms to the old one (new outputs remain readable by existing consumers), FORWARD mirrored. Exact equality reports `unchanged`; conformance in the required direction reports a compatible `type-changed`; neither reports incompatible.

The mapping must be locked in with property tests (reflexivity, Any-as-top, Null-conforms-to-everything, context width subtyping in both directions). `is_conformant` is lenient about Null — a field becoming nullable never reads as a narrowing — which the report semantics must document.

Scoping note: no typed-introspection columns get added to `dmn_invocables`/`dmn_info` ahead of this work — dsntk 0.3 added no model-introspection capability, and the typeRef resolver should be built once, here.

**Open design questions:**

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

Done 2026-07-13. Dropped the `hydrate` feature, `wasm-bindgen`, `console_error_panic_hook`, the `cdylib` crate type, the `wasm-release` profile, and `cargo-leptos` itself. Every route is now `SsrMode::Static` and rendered to `website/dist` by a `prerender` binary, which also compiles Sass in-process via `grass` and emits `404.html` and `.nojekyll`. Specification in `docs/specifications/WEB-001-static-prerender.md`; decision recorded in CLAUDE.md.

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

### DEPS-001: Drop the HTTP/TLS stack dsntk 0.3 embeds in the extension (done in vendor; upstream PR pending)

dsntk-feel-evaluator 0.3.0 hard-depends on reqwest 0.13 with the rustls/aws-lc provider, used only by its `evaluator_java.rs` — a blocking HTTP client that calls a local Java RPC server (127.0.0.1:22023) when a model invokes an "external Java function". pgdmn never wants that inside a PostgreSQL backend, but feature unification is additive so it cannot be opted out downstream; the extension `.so` now statically links reqwest, rustls, and the AWS-LC C library, and the Docker image needs cmake to build aws-lc-sys. The fix is upstream (fits the minimal-upstreamable-patch working agreement): land a small PR on DecisionToolkit/dsntk putting `evaluator_java`/`evaluator_pmml` and the reqwest dependency behind an off-by-default cargo feature (e.g. `external-functions`) that returns an explained null when disabled; 0.2's `default-features = false` reqwest with a ring provider is the fallback shape. pgdmn then consumes dsntk-feel-evaluator with the feature off, removing the HTTP/TLS stack from the backend entirely and demoting the `src/guard.rs` boundary rejection from load-bearing to belt-and-suspenders (the guard is inherently partial for DMN models — see the module docs in `src/guard.rs`). Once merged and adopted, remove cmake from the Dockerfile (comment there points here). No local `[patch]`/fork in the interim.

Resolved in the vendored copy (the vendoring decision superseded the
no-fork constraint): `vendor/dsntk-feel-evaluator` now has an
off-by-default `external-functions` cargo feature gating the evaluators
and the optional reqwest dependency; disabled builds return an explained
null for external invocations. reqwest/rustls/aws-lc/hyper/quinn are
unreachable in the extension graph (verified with cargo tree) and cmake
is out of the Dockerfile. The feature gate is the shape to offer
upstream; `src/guard.rs` remains as belt-and-suspenders and its FEEL
literal-expression gap is now closed at the build level.

### DEPS-002: Upstream fix for non-finite FeelNumber Display panic (done in vendor; upstream PR pending)

dsntk-feel-number's `Display` unwraps on the assumption that `bid128_to_string` output contains `'E'`; ±Inf/NaN — reachable because `Mul`/`Add` results are not finiteness-guarded, unlike `pow`/`from_str` — panic instead of printing. pgdmn guards its direct number conversions (BUG-004), but a non-finite number inside a `Value::Range` endpoint still panics via the Display catch-all in `feel_to_json`, and every other dsntk consumer stays exposed. Propose upstream either guarding arithmetic like `pow` does (overflow → FEEL null) or making `Display` total (print `+Inf`/`-Inf`/`NaN`). Minimal upstreamable patch per the working agreement.

Fixed in the vendored copy: `Display` is now total (non-finite values
print the Intel library's textual form instead of panicking on a missing
exponent). pgdmn's BUG-004 SQL-error guards stay in place — the SQL
boundary still rejects non-finite results with a clear error.

### ADOPT-001: Migrate to pgrx 0.18

pgrx and pgrx-tests 0.16.1 → 0.18.0 is a breaking framework major that Dependabot cannot land on its own (rejected PRs #31/#32): the embed entrypoint moved — `::pgrx::pgrx_embed!()` in `src/bin/pgrx_embed.rs` no longer resolves (`cannot find pgrx_embed in pgrx`, `main function not found in crate pgrx_embed_pgdmn`), which is only the first surfaced breakage before the SQL-entity/schema-generation and datum-API changes across two minor cycles (0.16 → 0.17 → 0.18). Do this as a dedicated migration: bump both crates together (they are a matched pair and must move in lockstep), rebuild the test image (`make test-image`, Cargo.lock changed), and run the full `make test` suite plus the custom `DmnModel` InOutFuncs and `pgrx::datum::Interval` paths that are the most version-sensitive. Update the `pgrx_embed` gotcha in CLAUDE.md if the embed API shape changes. Not a triage-merge.

### ADOPT-002: Migrate to rapidhash 4.x

rapidhash 1.4.0 → 4.5.1 is a breaking API rename Dependabot cannot land on its own (rejected PR #30): `RapidInlineHasher`, `rapidhash_seeded`, and `RAPID_SEED` no longer exist in the crate, and all three are load-bearing in `src/cache.rs` — the 128-bit content hash is two independently seeded 64-bit passes (`rapidhash_seeded(bytes, RAPID_SEED)` and `rapidhash_seeded(bytes, SECOND_SEED)`), and its collision reasoning depends on the algorithm being well-mixed. Porting to 4.x means finding the renamed API, confirming an equivalent seeded 64-bit primitive exists, and re-validating the double-seeded 128-bit collision argument and the cache-key contract test before trusting cached ASTs — a stale/weaker hash means wrong answers, not errors. The algorithm change itself is safe for the caches (per-backend, never persisted, so no stored hashes to invalidate). Do this as a dedicated, tested change, not a triage-merge; keep the `[profile.dev.package.rapidhash]` opt-level-3 override.

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
