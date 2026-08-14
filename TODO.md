# TODO

## PGXN

Packaging and distribution work for listing pgdmn on PGXN. Not required to
flip the repo public — a separate, later distribution milestone.

### PGXN-001: PGXN readiness—packaging, META.json, and pgrx precedent

META.json already exists and targets PGXN's meta-spec (pgxn.org/meta/spec.txt),
but has a FIXME placeholder maintainer contact and has never been validated
against PGXN's actual distribution tooling (`pgxn-utils`/`pgxn_meta` validate,
building a real PGXN zip bundle). The open question is packaging: PGXN's
traditional distribution model assumes a PGXS-based `make && make install`
build, which doesn't match how pgrx extensions are normally built
(`cargo pgrx package`/`install`)—and this project's build is Docker-only by
design (see CLAUDE.md's Decided section), in further tension with PGXN's
assume-it-builds-from-source model. Before shipping: survey existing pgrx-based
extensions that list on PGXN (if any) for how they bridge this—a Makefile
wrapper that shells out to cargo-pgrx, prebuilt artifacts, or something else—and
decide whether pgdmn follows suit or PGXN isn't the right distribution channel
given the toolchain, with crates.io/GitHub releases as the alternative. Fix the
FIXME contact and validate META.json either way.

### PGXN-002: Add PGXN badge to README once published

Add a PGXN badge to the README's badge row once the extension is
actually published there (see the PGXN Setup steps in PGXN-001). A
crates.io badge is a separate, lower-priority call — PGXN-only
distribution is the current plan, not crates.io, so a crates.io badge
only makes sense if that plan changes; don't add one just to have one.

## Safety

### SAFETY-001: Evaluate whether pgdmn should be a trusted extension

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

## Upstream

### UPSTREAM-001: Open the upstream PRs and link them (executes PERF-006)

BUG-003 first, then the DEPS-001 feature gate, DEPS-002, H4, H14, H9,
and H3+H20 as a pair. Link each PR from TODO.md and vendor/PATCHES.md
so the public repo visibly demonstrates the upstream-first posture.
(DEPS-001/DEPS-002 are vendor/PATCHES.md patch IDs — the vendor-side
work is done; this item tracks only opening the upstream PRs.)

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
call—one Box per comparison, two per range, a Vec per list—paid
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

## Features

### FEAT-002: DMN compatibility checking functions

SQL functions inspired by Kafka Schema Registry compatibility semantics, applied to DMN invocable input and output definitions. These enable versioned evolution of DMN models while verifying that consumers and producers remain compatible.

**Core function:** `dmn_compat(new_model DmnModel, old_models DmnModel[], invocable text) → JSONB`

Compares the input and output definitions of a named invocable across a new model and one or more old models, producing a structured JSONB report.

**Compatibility directions (checked separately for inputs and outputs):**

- **BACKWARD (inputs):** The new invocable accepts all inputs the old one could. Adding optional inputs (with defaults) is allowed; removing or narrowing required inputs is not.
- **FORWARD (inputs):** The old invocable could accept all inputs the new one can. Adding required inputs breaks forward compatibility; removing optional inputs is allowed.
- **FULL (inputs):** Both backward and forward—only optional-with-default fields may be added or removed.
- **BACKWARD (outputs):** Consumers of the old outputs can still consume the new outputs. Removing output fields or widening types breaks backward output compatibility.
- **FORWARD (outputs):** Consumers of the new outputs could consume the old outputs. Adding output fields or narrowing types breaks forward output compatibility.
- **FULL (outputs):** Both directions for outputs.

**Report structure (JSONB):** Per old-model entry, per field: field name, direction (input/output), change type (added/removed/type-changed/unchanged), whether the change is backward-compatible, forward-compatible, and a human-readable detail string. Top-level summary booleans for each compatibility mode.

**Array input enables transitive checks:** By passing all historical model versions as the array, the caller gets a report covering compatibility against every prior version, not just the immediately previous one.

**Decided (2026-07-14):** the type-compatibility relation is dsntk's own `FeelType::is_conformant`—the same relation `dmn_eval` applies at coercion time—not a pgdmn-invented widening lattice.

Direction mapping: inputs are BACKWARD compatible iff the old input type conforms to the new one (everything the old invocable accepted is still accepted), FORWARD iff the new conforms to the old. Outputs are BACKWARD compatible iff the new output type conforms to the old one (new outputs remain readable by existing consumers), FORWARD mirrored. Exact equality reports `unchanged`; conformance in the required direction reports a compatible `type-changed`; neither reports incompatible.

The mapping must be locked in with property tests (reflexivity, Any-as-top, Null-conforms-to-everything, context width subtyping in both directions). `is_conformant` is lenient about Null—a field becoming nullable never reads as a narrowing—which the report semantics must document.

Scoping note: no typed-introspection columns get added to `dmn_invocables`/`dmn_info` ahead of this work—dsntk 0.3 added no model-introspection capability, and the typeRef resolver should be built once, here.

**Open design questions:**

- Whether to support checking multiple invocables in one call or keep it single-invocable
- How to handle invocables that exist in one model but not the other (report as incompatible vs. skip with warning)

### WEB-008: Tokenize the site's mobile breakpoint

`main.scss` centralizes every other reusable dimension as a `:root` custom property (`--max-width`, the `--color-*`/`--font-*` tokens), but the `@media (max-width: 40em)` query added for the mobile nav (WEB-006) hardcodes the literal instead of referencing a token. CSS custom properties can't be read inside a media feature query, so this needs a compile-time Sass `$variable`—not yet a pattern used anywhere in this file—defined once and referenced from every breakpoint. Worth doing once a second responsive rule needs the same breakpoint; premature with only one caller.

### WEB-009: Scope the generic `ul, ol` prose-list rule to content containers

The sitewide `ul, ol { padding-left: 1.5rem; margin-bottom: 1rem; }` rule (meant for prose lists in article/docs body content) is not scoped to content—its element-selector specificity beats the universal reset for `padding-left` on *any* `<ul>`/`<ol>` on the site, so `.site-nav`, `.quicklinks ul`, `.download-list`, and `.post-list` each carry their own `padding-left: 0` override to cancel it back out. Scoping the base rule to something like `main ul, main ol` would let all four overrides be dropped in one pass. Low urgency—the current per-component overrides are an established, working pattern, not a bug.

### FEAT-007: Accept PG range types as inputs in the record-eval path

Extend `pg_datum_to_feel` with arms for `NUMRANGEOID`, `INT4RANGEOID`, `INT8RANGEOID`, `DATERANGEOID`, `TSRANGEOID`: read via pgrx `Range<T>`, convert bounds with the existing scalar arms, and build `Value::Range` with `IntervalType::Closed`/`Opened`/`OpenedUndef` (infinite bound → `OpenedUndef`). No SQL signature changes—`feel_record_eval` and `dmn_record_eval` start accepting range-typed columns, the only inbound channel for binding a FEEL range variable (JSON cannot carry one). Decide the policy for PG `empty` ranges (error is more consistent with the mixed-interval precedent in convert.rs) and defer `TSTZRANGEOID` with the timezone question. Round-trip property tests (PG range → FEEL → `feel_eval_numrange` → PG range) pair with the existing numrange output path. Discrete-range canonicalization (`int4range '[1,4]'` arrives as `[1,5)`) is same-set but changes `.end`/`.end included` property values—document it.

### FEAT-008: feel_eval_daterange and feel_eval_tsrange typed variants

Stage 2 of the numrange work: map FEEL date and date-time ranges onto `daterange`/`tsrange` via pgrx `Range<Date>`/`Range<Timestamp>`, mirroring `feel_eval_numrange`. Note PG canonicalizes discrete `daterange` (`[a..b]` becomes `[a..b+1)`)—same set, different rendering; document alongside the implementation.

### FEAT-009: feel_unary_test—evaluate decision-table-style unary tests against a value

`feel_unary_test(tests text, value jsonb, context jsonb DEFAULT NULL) → boolean`: parse `tests` with `parse_unary_tests` (the exact grammar of a decision-table input entry, including `-`, `not(...)`, ranges, and comma lists), bind `value` as the FEEL `?` placeholder, build the `In` node exactly as dsntk's own decision-table evaluator does, and evaluate. Enables rules-stored-in-tables matching (`WHERE feel_unary_test(r.quantity_test, to_jsonb(42))`). Needs a `/spec` first: the null-result policy (error like `feel_eval_bool` vs. decision-table-style false) and the temporal-typing story (a JSONB string stays a FEEL string, so `< today()` needs context-passed dates or later typed overloads) are behavior decisions.

### FEAT-010: Surface explained FEEL nulls the JSONB paths discard

dsntk 0.3 attaches explanations to many nulls (`null(position must not be zero)`) that `feel_eval`/`dmn_eval` currently flatten to bare JSON null (convert.rs maps `Null(_)` dropping the message). Candidate shapes: `feel_eval_detail`/`dmn_eval_detail` returning `(result jsonb, null_reason text)`, or emitting the explanation as a DEBUG-level notice inside the existing functions (zero new API). The explanation text is upstream-owned prose that rewords between releases—whatever ships must document that it is diagnostic, not a stable contract. Decide the shape before implementing.

### WEB-002: Automated link checking for the website

Nothing verifies that the site's internal links resolve. The prerendered output makes this cheap and exact: every link either corresponds to a file in `website/dist` or it does not. Add a check to the `Website` workflow that walks the generated HTML, resolves each internal `href` against `dist/`, and fails on any that points nowhere.

This would have caught a trailing-slash problem fixed by hand early on (linking `/why` when the file is `why/index.html`), and it guards the class of breakage a static site is most prone to: a renamed route silently leaving dead links behind.

### WEB-007: OpenGraph and social meta tags

Add og:title, og:description, og:image, and Twitter card meta tags to the website shell and per-page overrides via leptos_meta.

## CI

### CI-001: Publish the extension test image to GHCR as a cache fallback

Today `ci.yml` rebuilds `pgdmn-test` with a `type=gha` buildx layer cache injected via the Makefile's `DOCKER_BUILD_CACHE` variable. If cold-cache runs (e.g. after `Cargo.lock` changes) get too slow, publish a prebuilt image to GitHub Container Registry whenever the Dockerfile or `Cargo.lock` changes, and pull it in CI as a fallback. This would cap the worst-case build time without changing the normal cache path.

### CI-002: Scheduled DMN eval benchmark with regression tracking

`make bench` is gated behind `PGDMN_BENCH=1` and deliberately excluded from PR CI because microbenchmark numbers are noisy on shared runners (see the canary-gated benchmarking notes). Add a nightly or otherwise scheduled workflow that runs the benchmark and records results over time, so drift is caught without flaking PRs.

### CI-003: Path-based job skipping for docs-only and website-only PRs

`ci.yml` intentionally has no `paths:` filter so the required `CI aggregate` check always reports; a path-filtered required check stays pending forever and blocks merges. Add a `dorny/paths-filter`-style gate (or equivalent) that skips the extension lint/test work for docs-only and website-only PRs while still letting the aggregate job run unconditionally, restoring the savings without reintroducing that failure mode.

### CI-005: `pgdmn-test` buildx GHA cache is near the 10GB repo quota (partially done)

`gh api repos/fugu13/pgdmn/actions/caches --paginate` showed the `type=gha` buildx layer cache (`DOCKER_BUILD_CACHE`, `mode=max`) plus the `target` cache sitting at ~9.9GB of GitHub's 10GB per-repo Actions cache quota. The dominant contributor turned out to be dead weight: every closed/merged PR's `refs/pull/<N>/merge`-scoped caches (buildx blobs + `target/`) stay in the store — unreachable forever, since that ref can never be restored again — until GitHub's LRU eviction reclaims them, which can evict a still-useful cache instead and force a full from-scratch image rebuild in a single job (fresh apt install, fresh `cargo install cargo-pgrx`, fresh full compile); that was a likely contributor to the `No space left on device` failures fixed by the `Free disk space` step in `ci.yml`. Three closed PRs (`refs/pull/42`, `44`, `45`) accounted for ~3.9GB by themselves; pruned manually via `gh cache delete --all --ref refs/pull/<N>/merge` (a dozen entries deletes in ~4s, cheap enough to automate), and `.github/workflows/cache-cleanup.yml` now runs the same deletion on every `pull_request: closed` event so this doesn't reaccumulate.

What's still open: `refs/heads/main`'s own cache (buildx blobs regenerate on every push that touches `Cargo.lock`, since the Dockerfile `COPY`s it before `cargo fetch --locked`) isn't pruned by anything — old main-branch blob generations just sit until eviction. `mode=min` would not help here regardless (this Dockerfile is single-stage, so every layer already ends up in the final image). CI-001 (publish the image to GHCR instead of layer-caching through Actions cache) would sidestep the quota entirely and is the more thorough fix if main's cache growth becomes a problem again.

## Dependencies

### ADOPT-002: Migrate to rapidhash 4.x

rapidhash 1.4.0 → 4.5.1 is a breaking API rename Dependabot cannot land on its own (rejected PR #30): `RapidInlineHasher`, `rapidhash_seeded`, and `RAPID_SEED` no longer exist in the crate, and all three are load-bearing in `src/cache.rs` — the 128-bit content hash is two independently seeded 64-bit passes (`rapidhash_seeded(bytes, RAPID_SEED)` and `rapidhash_seeded(bytes, SECOND_SEED)`), and its collision reasoning depends on the algorithm being well-mixed. Porting to 4.x means finding the renamed API, confirming an equivalent seeded 64-bit primitive exists, and re-validating the double-seeded 128-bit collision argument and the cache-key contract test before trusting cached ASTs — a stale/weaker hash means wrong answers, not errors. The algorithm change itself is safe for the caches (per-backend, never persisted, so no stored hashes to invalidate). Do this as a dedicated, tested change, not a triage-merge; keep the `[profile.dev.package.rapidhash]` opt-level-3 override.

## Chores

### FEAT-001: `dmn_create_input_type` helper

A convenience function that inspects a DMN model's input requirements for a given invocable and creates a matching PostgreSQL composite type automatically. This would eliminate the manual `CREATE TYPE` step when using `dmn_record_eval`.

Example usage:
```
SELECT dmn_create_input_type(dmn_load('<xml>'), 'Eligibility', 'eligibility_input');
-- Creates: CREATE TYPE eligibility_input AS ("Age" numeric, "Income" numeric)
```

### CHORE-006: Migrate to pgrx 0.18

pgrx and pgrx-tests 0.16.1 → 0.18.0 is a breaking framework major that Dependabot cannot land on its own (rejected PRs #31/#32): the embed entrypoint moved — `::pgrx::pgrx_embed!()` in `src/bin/pgrx_embed.rs` no longer resolves (`cannot find pgrx_embed in pgrx`, `main function not found in crate pgrx_embed_pgdmn`), which is only the first surfaced breakage before the SQL-entity/schema-generation and datum-API changes across two minor cycles (0.16 → 0.17 → 0.18). Do this as a dedicated migration: bump both crates together (they are a matched pair and must move in lockstep), rebuild the test image (`make test-image`, Cargo.lock changed), and run the full `make test` suite plus the custom `DmnModel` InOutFuncs and `pgrx::datum::Interval` paths that are the most version-sensitive. Update the `pgrx_embed` gotcha in CLAUDE.md if the embed API shape changes. Not a triage-merge.
