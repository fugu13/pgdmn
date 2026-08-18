# pgdmn

PostgreSQL extension that brings DMN (Decision Model and Notation) support to Postgres. Built with Rust, pgrx, and dsntk. A prerendered static website lives in `website/`.

## Stack

| Component | Choice |
|---|---|
| Language | Rust 2024 edition (extension and website) |
| Extension framework | pgrx 0.16 |
| DMN/FEEL engine | dsntk 0.3 |
| Target | PostgreSQL 17 |
| Website | Leptos 0.8, prerendered to static HTML (`website/`) |

## Build & Run

All extension builds run in Docker (single `pgdmn-test` image: PG17 + pgrx toolchain, built and owned throughout by the non-root user initdb requires). All extension commands go through `make`—do not invoke `cargo` or `docker` directly.

Two host-native exceptions (always with `cargo +stable-aarch64-apple-darwin`—the default host toolchain is x86_64/Rosetta and unusable for measurement):

- `profiling/`—benchmark/profiling harness over the vendored engine; build with `cargo build --release` in that directory, no PostgreSQL needed.
- Vendored dsntk test gates—`CARGO_TARGET_DIR=target-host cargo test -p dsntk-<crate> -- --skip external_functions --skip bif_now --skip dmn_3_0076 --skip "dmn_3_0103::_0017"` from the repo root (the skips are environment-dependent upstream tests; `CARGO_TARGET_DIR=target-host` keeps host artifacts out of the Docker-shared `target/`).

| Command | Purpose |
|---|---|
| `make help` | List all targets |
| `make test-image` | Build/refresh the Docker image (required after `Cargo.lock` changes) |
| `make check` | Fast compilation check |
| `make build` | Build the extension |
| `make test` | pgrx test suite against PG17 |
| `make lint` | clippy (deny warnings) + rustfmt check |
| `make fmt` | Auto-format |
| `make verify` | fmt + lint + vendor integrity + license policy—run after every code change (lint's clippy subsumes check) |
| `make bench` | DMN eval benchmark |
| `make doc-check` | README/website Docs page cover every SQL-facing function (host-native) |
| `make license-check` | Dependency license allowlist via `cargo-deny`, all three cargo workspaces (host-native) |
| `make vendor-status` / `vendor-diff` | Vendored dsntk version, pristine base, carried patch layer, `vendor/CHECKSUMS` drift |
| `make vendor-test` | Vendored engine suites in Docker (env-dependent upstream tests skipped) |
| `make vendor-bench` | Host-native engine benchmarks (canary methodology: Performance section below) |
| `make vendor-upgrade VERSION=x.y.z` | Stage a new pristine upstream tree (drops the patch layer) |
| `make vendor-inspect` | Claude session: audit upstream delta, re-layer patches, gate, measure |
| `make website-build` | Prerender the site to `website/dist` (the deployable artifact) |
| `make website-serve` | Serve `website/dist` the way a static host would |
| `make website-dev` | Prerender, then serve; re-run to pick up changes |
| `make website-test` | the website's unit tests (markdown, metadata limits, prerender) |
| `make website-lint` | clippy + rustfmt check for the website |

`make test-image` builds with `docker buildx build $(DOCKER_BUILD_CACHE) --load -t pgdmn-test .`. `DOCKER_BUILD_CACHE` is empty locally (plain buildx build using Docker's own cache); CI sets it to a GitHub Actions layer cache (`--cache-from`/`--cache-to type=gha`; the exact flags live in `ci.yml`) so the image (apt PostgreSQL 17 + `cargo install cargo-pgrx`) is reused across CI runs instead of rebuilt from scratch. Same `make` target either way—only the cache backend differs.

The website builds on the host rather than in Docker, but needs no host tools beyond cargo: Sass is compiled in-process by the `grass` crate, and there is no wasm step. There is deliberately no hot-reload—it depended on `cargo-leptos`, which cannot survive the removal of the wasm target (see WEB-001).

The website's toolchain is pinned in `website/rust-toolchain.toml` (the extension's lives in the Dockerfile). CI installs no toolchain of its own and honours that pin, so clippy and rustfmt behave identically on a laptop and in CI. Do not pin a toolchain in the workflow instead—CI drifting ahead of developers is what broke the first run of the `Website` workflow.

**Continuous integration:** `.github/workflows/ci.yml` (`CI`) runs `make test-image`, `make lint`, and `make test` on every pull request and push to `main`, gated by a required `CI aggregate` job (plus a non-blocking `cargo audit` job). `.github/workflows/website.yml` (`Website`) builds and deploys the site (see Website below). `.github/dependabot.yml` opens weekly dependency-update PRs for both Cargo workspaces, Docker, and GitHub Actions.

## Architecture

```
src/
  lib.rs            — pg_module_magic, integration tests
  cache.rs          — thread-local ModelEvaluator cache (keyed by XML hash)
  convert.rs        — FEEL value ↔ PG type conversions
  guard.rs          — rejection of external (Java/PMML) function definitions
  types/
    dmn_model.rs    — custom DmnModel PG type (InOutFuncs: XML in, namespace::name out)
  functions/
    feel.rs         — feel_eval (JSONB), feel_record_eval (record), + 7 typed variants (numeric, bool, text, date, timestamp, interval, numrange)
    dmn.rs          — dmn_load, dmn_eval, dmn_record_eval
    introspection.rs — dmn_invocables, dmn_info, dmn_xml, dmn_name, dmn_namespace
website/
  src/
    app.rs          — shell + Router; every route is SsrMode::Static
    bin/prerender.rs — renders all routes to dist/, compiles Sass, writes 404.html
    bin/serve.rs    — local preview of dist/ (not used in production)
    site.rs           — canonical host, share-card constants, absolute-URL helper
    pages/, components/, routes.rs
    components/page_meta.rs — the description/OpenGraph/Twitter set every page emits
  style/main.scss   — Sass styles (compiled by grass, in-process)
  card/             — markup sources for share-card.png and the icons; never served
  public/           — copied verbatim into the site root (card, icons, examples, viewer)
  dist/             — generated static site; the deployable artifact (gitignored)
```

### Decided

- **dsntk is vendored (`vendor/`, 13 crates) and patched in-tree for performance.** The patch set must stay minimal, separable (one commit per change), and upstreamable: `PGDMN:` comments at every change site, no reformatting of surrounding code, repo lint/style conventions do not apply to vendor code (`vendor/rustfmt.toml` disables formatting; cap-lints still surfaces default clippy lints in `make lint`). The user trades ~10% of a speedup for substantially less vendor diff—always report scope alongside speed. Safety net: the vendored crates are workspace members and their test suites (3,600+ evaluator tests, DMN TCK corpus) must stay green after any vendor change. Provenance is durable, not just asserted: `vendor-upgrade` writes `vendor/CHECKSUMS` (per-crate sha256) automatically on every pristine swap, and `make vendor-status` flags drift against it.
- **Dependency licenses are allowlist-gated** (`deny.toml`, `make license-check`, part of `make verify` and a required CI job). MIT/Apache-2.0/Unicode-3.0/BSD-2/3-Clause/ISC/Zlib/Unlicense/BSL-1.0/CC0-1.0 only; `r-efi`'s `MIT OR Apache-2.0 OR LGPL-2.1-or-later` passes via the allowed MIT/Apache-2.0 arms, but LGPL itself is deliberately not allowlisted—a future crate offering only an LGPL license is rejected, not silently accepted through the OR.
- **Perf claims are canary-gated measurements.** Benchmark windows are gated on an untouched-code micro-benchmark canary (Performance section below); sub-microsecond deltas across separate builds are noise below ~8%. Candidate optimization removals are measured at the final tip, not only in isolation—optimizations interact.
- **Evaluation caches are content-addressed by 128-bit double-seeded rapidhash** (DMN: model XML hash; FEEL: expression text + context-shape digest). The shape digest deliberately mirrors the vendored parser's scope derivation—an accepted, test-pinned tradeoff; re-verify on dsntk upgrades.
- **Docker-only extension builds.** The pgrx toolchain and PG17 live in the images; the host never needs them. The Dockerfile copies `Cargo.lock` and fetches `--locked` so the image's crate cache matches the repo (see BUG-001 in BUGHISTORY.md).
- **CI runs the same `make` targets developers do.** No CI-only build path to drift out of sync; the only difference is the Docker image being layer-cached via `type=gha` instead of Docker's local cache (see `DOCKER_BUILD_CACHE` under Build & Run).
- **Error model:** SQL-facing errors are raised with `pgrx::error!` (unwinds into a PostgreSQL ERROR—the idiomatic pgrx mechanism). Internal fallible functions return `Result<_, String>` and callers convert at the SQL boundary with `unwrap_or_else(|e| pgrx::error!(...))`. This is an accepted deviation from the thiserror convention: errors here terminate at the SQL boundary as messages, so enum error types add no value. Revisit if errors ever need programmatic matching.
- **Website is prerendered to static HTML, with no hydration** (WEB-001). Leptos renders every route once at build time; `dist/` is served as plain files by GitHub Pages, with no server process and no JavaScript shipped. The site had no client-side interactivity, so the wasm bundle it used to ship (343 KB) bought nothing while forcing a `wasm-bindgen` crate/CLI version match and a server-capable host. Consequences that bind new work: **no route may depend on request state**, and anything dynamic (e.g. the Articles pages) must render at build time. Reintroducing hydration means reintroducing both couplings—do not do it for a page that merely *looks* interactive.
- **Website uses Leptos**—an accepted deviation from the no-client-framework frontend rule, decided before this convention existed. It is now used purely as a server-side template engine. New pages follow the existing Leptos patterns.
- **JSONB is not the bottleneck** (measured 2026-07-14, `make bench`). `dmn_record_eval` skips the JSONB path entirely and is only **5–9% faster** than `dmn_eval`. The time is in FEEL evaluation itself, so any scheme to bypass serialization—direct datum conversion, SPI batch functions, hstore, variadic inputs—is chasing at most a tenth of the runtime. Do not spend effort there without profiling FEEL first. What *does* pay is letting Postgres parallelize: the functions are already `IMMUTABLE` and `PARALLEL SAFE`, and parallelism measures 3.8× (`make bench-shapes`).
- **External (Java/PMML) function definitions are rejected at the SQL boundary** (`src/guard.rs`). dsntk 0.3's evaluator resolves them with a blocking, untimed HTTP POST from the backend, which would falsify the `immutable, parallel_safe` declaration on every pgdmn function. The vendored evaluator additionally compiles without the external-function machinery entirely (off-by-default `external-functions` feature, DEPS-001), so the guard is defense in depth; details in `src/guard.rs` module docs and TODO.md.
- **No LTO in dev profile** (caused an ICE on Rust 1.85/aarch64; not re-tested since the Docker toolchain moved to 1.95).

### Undecided

- FEEL type compatibility rules for FEAT-002 (`dmn_compat`).

## Conventions

### Error handling

- **Propagate, never crash.** No `unwrap()`, `expect()`, `panic!()`, or fallible indexing in non-test code. Test code may `unwrap()` freely.
- At the SQL boundary, raise errors with `pgrx::error!` (see Decided above). Everywhere else, return `Result`.
- **Validate at boundaries, trust internals.** Check SQL inputs (XML, expressions, contexts) at the point of entry; do not re-validate inside.
- **Error messages are for the user reading them**—a SQL author, not a Rust developer. Include the offending value: `expected FEEL number, got: {other}`.

### Lints

- Lint policy lives in `[lints.clippy]` in each `Cargo.toml` (pedantic + nursery as warnings, justified allows listed there). Enforcement is `make lint` with `-D warnings`—never `#![deny(warnings)]` in source.
- Per-item suppressions only with a comment explaining why. Prefer `#[expect(clippy::...)]`—it self-reports the moment the suppression goes stale. Use `#[allow(clippy::...)]` only when the lint fires inconsistently (e.g. varies with macro expansion or cfg).

### Code quality

- **No dead code.** Nothing without a caller; no commented-out blocks or "for later" helpers.
- **No premature abstractions.** Extract shared code only when it is a clear clarity win.
- **Fix quality issues as found** (naming, missing error handling, unclear logic). If the fix is a substantial standalone effort, add a TODO instead.
- **Name from the domain:** `invocable_names`, not `string_list`.
- **Explicit ordering** on anything a user sees—set-returning functions, `dmn_invocables`, JSONB arrays built from iteration.
- **Serde on boundary types.** Types crossing a boundary (storage, JSONB) implement Serialize/Deserialize.
- **Environment changes are declarative.** Toolchain or dependency changes go in the Dockerfile / `Cargo.toml`, never imperative installs into a running container.

### pgrx / dsntk gotchas

- `dsntk_model::NamedElement` and `DmnElement` traits must be imported for `.name()` / `.namespace()`
- `parse_expression(scope, expr, trace)` takes 3 args
- `pgrx::datum::Interval::new(months, days, micros)`—months first
- `pgrx_embed` binary required: `[[bin]] name = "pgrx_embed_pgdmn"`
- `website/Cargo.toml` and `profiling/Cargo.toml` both need an empty `[workspace]` table (the root `Cargo.toml` excludes them via `exclude = ["profiling", "website"]`). Without it, Cargo's workspace-root search doesn't stop at the excluding manifest and walks further up looking for one—harmless from a normal checkout, but fails with "current package believes it's in a workspace when it's not" whenever the checkout is nested inside another checkout of the same repo, exactly the layout of an agent worktree under `.claude/worktrees/`.

## Performance

The evaluation hot path is per-SQL-row; everything below is load-bearing
knowledge for changing it. Detailed measurements live in the benchmark
outputs (`make bench`, `make vendor-bench`); deferred levers are PERF-* in
TODO.md.

### Caches (per backend thread, content-addressed)

| Cache | Key | Bound |
|---|---|---|
| DMN evaluator | 128-bit content hash of the model XML (computed once at `dmn_load`) + XML length | unbounded; one entry per distinct model |
| FEEL prepared evaluator | full expression text + 128-bit digest of the context *shape* | 1024 entries, then cleared wholesale |

- Collision safety rests on 128-bit double-seeded rapidhash (~2⁻¹²⁸ per pair);
  hot-path probes never re-hash or memcmp the underlying bytes.
- **The context-shape digest mirrors the FEEL parser's scope derivation**
  (`ParsingContext::from(&FeelContext)`): FEEL name tokenization depends on
  which names are in scope, so the same expression parses differently under
  different key sets. The mirror covers entry names, nested-context structure,
  and whether lists contain contexts—never leaf values. It is pinned by unit
  tests in `src/cache.rs` and by `test_parser_scope_derivation_contract`;
  **re-verify it on every dsntk upgrade** (a silent upstream change here means
  wrong answers from stale cached ASTs, not errors).
- The external-function AST guard runs once per parse inside the FEEL cache;
  the DMN guard runs at evaluator build. Cached entries were screened when
  first built.

### Per-row cost model

`dmn_eval` row: model datum CBOR decode (scales with XML size—PERF-001 is
the biggest remaining lever), cache probe, JSONB→FEEL conversion (integer
fast paths, by-value inserts), engine evaluation, FEEL→JSONB conversion.
`feel_eval` row: conversion, shape digest, cache probe, evaluation,
conversion. Parsing never happens per row on warm paths.

### Vendored engine patch inventory (stable IDs)

H4 hit-policy-aware decision tables (short-circuit) · H10 input expressions
once per call, bound to `?` · H5/H11/H12 per-call clone cuts + single-dispatch
BKM · H18 allocation-free invocable lookup · H3 copy-on-write FeelContext
(Arc) · H13 O(n) for-expressions + quantifier early-exit · H14 regex cache for
replace()/split() · H9 stack-buffer number formatting · H19 lazy memoized
builtin resolution · H6 owned coercion path · H20 filter waste cuts · BUG-003
`?`-entry semantics · DEPS-001 external-functions gate · DEPS-002 total
Display · H22 FEEL expression nesting depth cap (500) · H23 now()/today()
removed from builtin dispatch. Constraints: **H20 ships only with H3** (each
alone regresses filters—measured interaction); **H19 must stay lazy**
(eager build-time `Bif::from_str` regressed 3× in both the 0.2 and 0.3
cycles); the H13 AST walker is exhaustive by design and fails the build on
new parser variants.

### Behavioral deviations from pristine upstream

Never-consumed FEEL sub-expressions are not evaluated (hit-policy
short-circuit, quantifier early-exit—observable only via side-effecting
external functions, which are compiled out anyway); one diagnostic null
message no longer embeds the whole input context; BUG-003 spec alignment;
external invocations yield an explained null (DEPS-001); non-finite Display
is total (DEPS-002); FEEL expressions nested past a depth of 500 are rejected
with a parse error instead of being parsed (H22); `now()`/`today()` are no
longer resolvable FEEL function names, so calling either now yields the same
explained null as any unrecognized function name (H23). pgdmn-side: integral
JSON literals normalize on pass-through (`5.0` → `5`; numeric value
preserved, property-tested).

### Measuring

- `make bench` (SQL tier): per-row suite with plain-SQL and jsonb-extraction
  baselines; the pure-PostgreSQL control queries are cross-run canaries.
- `make vendor-bench` (engine tier): host-native harness in `profiling/`;
  it includes `src/convert_core.rs` as shared source so numbers always
  describe shipping conversion code. Hot-loop mode for sampling profilers.
- Cargo `[profile.dev.package.*]` overrides build every measured package at
  opt-level 3 so `cargo pgrx test` benches reflect release-grade codegen.
- Methodology: canary-gate every window (untouched FFI micro within 12% of
  idle baseline); sub-µs cross-build deltas under ~8% are layout noise;
  measure candidate removals at the final tip—optimizations interact.

## Testing

- **Test-first:** write signatures, then tests expressing the behavior, confirm they fail for the right reason, then implement. Do not add untested behavior.
- **Property-based testing is mandatory** for code with algebraic or combinatorial properties (parsing round trips, value conversions). Use `proptest`; commit the persisted `proptest-regressions/` directories (TEST-001 tracks the initial suite).
- Unit tests in `#[cfg(test)]` blocks in the module they test; SQL-level integration tests via `#[pg_test]` in `src/lib.rs`.

## Website

- **Accessibility is a default, not a feature.** WCAG 2.2 AA on every page. Semantic HTML before ARIA; one `<h1>` per page, no skipped heading levels; `<main>`, `<nav>`, and a skip link; visible focus indicators; 4.5:1 text contrast (3:1 large text/UI); keyboard reachability for every interactive element; every image has `alt`.
- **URL as state:** the current view is reproducible from its URL.
- **No JavaScript on the content pages.** Pages are prerendered to static HTML and ship zero JS; links and navigation work with scripting disabled because there is nothing to disable. Anything that would need client-side code has to justify reversing WEB-001 first. CI enforces this: the `Website` workflow fails if a script or wasm reference appears in the output. **The single exception is `public/dmn-viewer.html`**—a deliberate, isolated interactive page that loads dmn-js to render a model in standard DMN tooling. It is a static asset (not a Leptos route), keeps its `.html` name (`prerender.rs` exempts it from `clean_urls` and `strip_scripts`), the CI no-script check excludes it, and it loads dmn-js from a pinned jsDelivr CDN. Do not add a second scripted page without the same explicit exemptions and a good reason.
- **Every page states its own social metadata.** A page is added with a `<PageMeta>` (title, description, site-absolute path, and `published` for articles), which emits the description, canonical link, OpenGraph and Twitter sets; the `Website` workflow fails if any generated page is missing them. Descriptions are one sentence within `site::DESCRIPTION_LIMIT` (160)—articles enforce it as a test, and an article whose index `summary` runs longer adds a shorter `description:` to its front matter. Absolute URLs go through `site::url`, never a hardcoded host: crawlers do not resolve relative ones.
- **A post can be drafted before it's public.** An article whose front matter sets `draft: true` is excluded from the build entirely—no route, no home page listing, no share metadata—so it can be written and committed across sessions without going live half-finished. `make website-draft` scaffolds `website/articles/draft.md` with the required fields stubbed in; `make website-blog` commits everything under `website/articles/` and `website/public/` to a new branch off an up-to-date `main` (message auto-generated from which posts were added/updated and their draft status), pushes it, then builds and previews the site locally. `make website-blog` must be run from `main`, not a worktree branch—it errors otherwise, so the new branch never carries along unrelated commits.
- **The share card and icons are markup, not binaries.** `card/share-card.html` and `card/icons.html` render `public/share-card.png` (1200×630) and the icon set with any headless browser—regeneration commands are in the files. Edit the markup and re-render; never hand-edit the PNGs. Judge a card change at 220px, the size a phone feed gives it, not at full size. Per-article cards would mean generating images during the prerender: a separate decision, not a small extension of this one.
- **Deployment is automatic.** Pushing to `main` publishes `website/dist` to GitHub Pages at `www.pgdmn.com` via `.github/workflows/website.yml`. The site is served from a domain root because its links and stylesheet are absolute paths—a `github.io` subpath would break them.
- **Anti-patterns—do not build:** modals, toasts, skeleton screens, infinite scroll, dark-mode toggles (respect `prefers-color-scheme`), custom cursors.
- Styles in Sass (`style/main.scss`), flat colors, BEM-like modifier names.
- **SQL formatting in site copy (Examples, Docs, articles):** indent in two-space steps. When a `SELECT` list wraps, indent the continuation lines by a fixed two spaces from `SELECT`—do **not** align them to the first select-item column (the column just after `SELECT `). Keep clause keywords (`FROM`, `WHERE`, `GROUP BY`, `ORDER BY`) at column 0, and indent subquery/`LATERAL` bodies and multi-line function arguments in two-space steps from their opener. Every SQL result shown in a comment must be produced by the real engine—assert it in the extension test suite (`src/lib.rs`), not by hand.

## Documentation

| File | Purpose | When to update |
|---|---|---|
| `CLAUDE.md` | Development conventions | When conventions change |
| `README.md` | Project description, quick start, an example for **every** SQL function, doc index | Any SQL function change; any doc added/removed |
| `TODO.md` | Tracked work items with stable IDs | During planning; when items complete |
| `TODO-ARCHIVE.md` | Completed/dropped TODO items, kept for history under their original ID | When an item in `TODO.md` is finished or explicitly dropped |
| `BUGHISTORY.md` | Resolved bugs with reoccurrence checks | Immediately when a bug is fixed |

**This project has no `docs/` directory**—a deliberate departure from the global convention, decided 2026-07-14. Everything that would have lived there lives somewhere with a reader: user-facing explanation goes on the website (`website/`—the Docs page, and the walkthroughs in `website/articles/`), decisions go in this file's *Decided* section, and findings and future work go in `TODO.md`. Do not create `docs/`, `docs/ux/`, `docs/specifications/`, or `docs/plans/`.

**Direction, release, and go-to-market notes live outside this repo.** This repo holds the extension and its site; the thinking about where it is going is kept separately.

- **Documentation is part of the code change, not a follow-up.**
- TODO items: stable IDs with domain-specific prefixes (`TEST-`, `PERF-`, `WEB-`, `CHORE-`, …); avoid catch-all prefixes like `FEAT` for new items (existing `FEAT-*` IDs stay stable). Each item is a header plus paragraphs; when an item is finished or explicitly dropped, move it (same ID, same header) to `TODO-ARCHIVE.md` rather than leaving it in `TODO.md` or deleting it outright.
- BUGHISTORY entries: symptom, root cause, fix, files, reoccurrence checklist. After any change, verify no recorded bug is reintroduced.

## Git

- Main branch is `main` and is never modified directly—all work happens on branches in worktrees, merged via PR (`gh pr merge <number> --merge`).
- Commit discipline, in order: `/simplify` (out-of-scope findings go to `TODO.md`), `make verify` (and `make website-lint` if the website changed), BUGHISTORY reoccurrence check, then commit with a HEREDOC message (`git commit -F - <<'EOF'`), never `$()` substitution.
- Keep PRs focused; split optional enhancements into TODOs.
- When Copilot review is definitively wrong, add a correction under `.github/instructions/` (scoped, with `applyTo` frontmatter) or `.github/copilot-instructions.md` (repo-wide). Ask the user first unless 100% certain.

## Behaviors

- **No praise, no hedging, facts only.** State tradeoffs and a recommendation when one option is clearly better.
- **Match effort to complexity.** Simple tasks get executed immediately, without over-exploration or planning.
- **Simpler first, always.** Implement the minimal direct behavior; split the rest into TODOs.
- **Stop repeating, start diagnosing.** If a change didn't help, investigate before changing anything else; never stack untested hypotheses.
- **When uncertain or when rules conflict, ask**—with a concrete option.
- When the user overrules you, get the reasoning and record it here.
- Flag likely performance problems proactively (unbounded allocations, per-row re-parsing, hot-path bloat).
