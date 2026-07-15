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

All extension builds run in Docker (single `pgdmn-test` image: PG17 + pgrx toolchain, built and owned throughout by the non-root user initdb requires). All commands go through `make` — do not invoke `cargo` or `docker` directly.

| Command | Purpose |
|---|---|
| `make help` | List all targets |
| `make test-image` | Build/refresh the Docker image (required after `Cargo.lock` changes) |
| `make check` | Fast compilation check |
| `make build` | Build the extension |
| `make test` | pgrx test suite against PG17 |
| `make lint` | clippy (deny warnings) + rustfmt check |
| `make fmt` | Auto-format |
| `make verify` | fmt + lint — run after every code change (lint's clippy subsumes check) |
| `make bench` | DMN eval benchmark |
| `make website-build` | Prerender the site to `website/dist` (the deployable artifact) |
| `make website-serve` | Serve `website/dist` the way a static host would |
| `make website-dev` | Prerender, then serve; re-run to pick up changes |
| `make website-lint` | clippy + rustfmt check for the website |

`make test-image` builds with `docker buildx build $(DOCKER_BUILD_CACHE) --load -t pgdmn-test .`. `DOCKER_BUILD_CACHE` is empty locally (plain buildx build using Docker's own cache); CI sets it to a GitHub Actions layer cache (`--cache-from`/`--cache-to type=gha`; the exact flags live in `ci.yml`) so the image (apt PostgreSQL 17 + `cargo install cargo-pgrx`) is reused across CI runs instead of rebuilt from scratch. Same `make` target either way — only the cache backend differs.

The website builds on the host rather than in Docker, but needs no host tools beyond cargo: Sass is compiled in-process by the `grass` crate, and there is no wasm step. There is deliberately no hot-reload — it depended on `cargo-leptos`, which cannot survive the removal of the wasm target (see WEB-001).

The website's toolchain is pinned in `website/rust-toolchain.toml` (the extension's lives in the Dockerfile). CI installs no toolchain of its own and honours that pin, so clippy and rustfmt behave identically on a laptop and in CI. Do not pin a toolchain in the workflow instead — CI drifting ahead of developers is what broke the first run of the `Website` workflow.

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
    pages/, components/, routes.rs
  style/main.scss   — Sass styles (compiled by grass, in-process)
  dist/             — generated static site; the deployable artifact (gitignored)
```

### Decided

- **Docker-only extension builds.** The pgrx toolchain and PG17 live in the images; the host never needs them. The Dockerfile copies `Cargo.lock` and fetches `--locked` so the image's crate cache matches the repo (see BUG-001 in BUGHISTORY.md).
- **CI runs the same `make` targets developers do.** No CI-only build path to drift out of sync; the only difference is the Docker image being layer-cached via `type=gha` instead of Docker's local cache (see `DOCKER_BUILD_CACHE` under Build & Run).
- **Error model:** SQL-facing errors are raised with `pgrx::error!` (unwinds into a PostgreSQL ERROR — the idiomatic pgrx mechanism). Internal fallible functions return `Result<_, String>` and callers convert at the SQL boundary with `unwrap_or_else(|e| pgrx::error!(...))`. This is an accepted deviation from the thiserror convention: errors here terminate at the SQL boundary as messages, so enum error types add no value. Revisit if errors ever need programmatic matching.
- **Website is prerendered to static HTML, with no hydration** (WEB-001). Leptos renders every route once at build time; `dist/` is served as plain files by GitHub Pages, with no server process and no JavaScript shipped. The site had no client-side interactivity, so the wasm bundle it used to ship (343 KB) bought nothing while forcing a `wasm-bindgen` crate/CLI version match and a server-capable host. Consequences that bind new work: **no route may depend on request state**, and anything dynamic (e.g. the FEAT-004 blog) must render at build time. Reintroducing hydration means reintroducing both couplings — do not do it for a page that merely *looks* interactive.
- **Website uses Leptos** — an accepted deviation from the no-client-framework frontend rule, decided before this convention existed. It is now used purely as a server-side template engine. New pages follow the existing Leptos patterns.
- **JSONB is not the bottleneck** (measured 2026-07-14, `make bench`). `dmn_record_eval` skips the JSONB path entirely and is only **5–9% faster** than `dmn_eval`. The time is in FEEL evaluation itself, so any scheme to bypass serialization — direct datum conversion, SPI batch functions, hstore, variadic inputs — is chasing at most a tenth of the runtime. Do not spend effort there without profiling FEEL first. What *does* pay is letting Postgres parallelize: the functions are already `IMMUTABLE` and `PARALLEL SAFE`, and parallelism measures 3.8× (`make bench-shapes`). See PERF-001 and PERF-002.
- **External (Java/PMML) function definitions are rejected at the SQL boundary** (`src/guard.rs`). dsntk 0.3's evaluator resolves them with a blocking, untimed HTTP POST from the backend, which would falsify the `immutable, parallel_safe` declaration on every pgdmn function. Coverage limits and the upstream endgame (DEPS-001) are documented in the module docs of `src/guard.rs`.
- **No LTO in dev profile** (caused an ICE on Rust 1.85/aarch64; not re-tested since the Docker toolchain moved to 1.95).

### Undecided

- FEEL type compatibility rules for FEAT-002 (`dmn_compat`).

## Conventions

### Error handling

- **Propagate, never crash.** No `unwrap()`, `expect()`, `panic!()`, or fallible indexing in non-test code. Test code may `unwrap()` freely.
- At the SQL boundary, raise errors with `pgrx::error!` (see Decided above). Everywhere else, return `Result`.
- **Validate at boundaries, trust internals.** Check SQL inputs (XML, expressions, contexts) at the point of entry; do not re-validate inside.
- **Error messages are for the user reading them** — a SQL author, not a Rust developer. Include the offending value: `expected FEEL number, got: {other}`.

### Lints

- Lint policy lives in `[lints.clippy]` in each `Cargo.toml` (pedantic + nursery as warnings, justified allows listed there). Enforcement is `make lint` with `-D warnings` — never `#![deny(warnings)]` in source.
- Per-item suppressions only with a comment explaining why. Prefer `#[expect(clippy::...)]` — it self-reports the moment the suppression goes stale. Use `#[allow(clippy::...)]` only when the lint fires inconsistently (e.g. varies with macro expansion or cfg).

### Code quality

- **No dead code.** Nothing without a caller; no commented-out blocks or "for later" helpers.
- **No premature abstractions.** Extract shared code only when it is a clear clarity win.
- **Fix quality issues as found** (naming, missing error handling, unclear logic). If the fix is a substantial standalone effort, add a TODO instead.
- **Name from the domain:** `invocable_names`, not `string_list`.
- **Explicit ordering** on anything a user sees — set-returning functions, `dmn_invocables`, JSONB arrays built from iteration.
- **Serde on boundary types.** Types crossing a boundary (storage, JSONB) implement Serialize/Deserialize.
- **Environment changes are declarative.** Toolchain or dependency changes go in the Dockerfile / `Cargo.toml`, never imperative installs into a running container.

### pgrx / dsntk gotchas

- `dsntk_model::NamedElement` and `DmnElement` traits must be imported for `.name()` / `.namespace()`
- `parse_expression(scope, expr, trace)` takes 3 args
- `pgrx::datum::Interval::new(months, days, micros)` — months first
- `pgrx_embed` binary required: `[[bin]] name = "pgrx_embed_pgdmn"`

## Testing

- **Test-first:** write signatures, then tests expressing the behavior, confirm they fail for the right reason, then implement. Do not add untested behavior.
- **Property-based testing is mandatory** for code with algebraic or combinatorial properties (parsing round trips, value conversions). Use `proptest`; commit the persisted `proptest-regressions/` directories (TEST-001 tracks the initial suite).
- Unit tests in `#[cfg(test)]` blocks in the module they test; SQL-level integration tests via `#[pg_test]` in `src/lib.rs`.

## Website

- **Accessibility is a default, not a feature.** WCAG 2.2 AA on every page. Semantic HTML before ARIA; one `<h1>` per page, no skipped heading levels; `<main>`, `<nav>`, and a skip link; visible focus indicators; 4.5:1 text contrast (3:1 large text/UI); keyboard reachability for every interactive element; every image has `alt`.
- **URL as state:** the current view is reproducible from its URL.
- **No JavaScript on the content pages.** Pages are prerendered to static HTML and ship zero JS; links and navigation work with scripting disabled because there is nothing to disable. Anything that would need client-side code has to justify reversing WEB-001 first. CI enforces this: the `Website` workflow fails if a script or wasm reference appears in the output. **The single exception is `public/dmn-viewer.html`** — a deliberate, isolated interactive page that loads dmn-js to render a model in standard DMN tooling. It is a static asset (not a Leptos route), keeps its `.html` name (`prerender.rs` exempts it from `clean_urls` and `strip_scripts`), the CI no-script check excludes it, and it loads dmn-js from a pinned jsDelivr CDN. Do not add a second scripted page without the same explicit exemptions and a good reason.
- **Deployment is automatic.** Pushing to `main` publishes `website/dist` to GitHub Pages at `www.pgdmn.com` via `.github/workflows/website.yml`. The site is served from a domain root because its links and stylesheet are absolute paths — a `github.io` subpath would break them.
- **Anti-patterns — do not build:** modals, toasts, skeleton screens, infinite scroll, dark-mode toggles (respect `prefers-color-scheme`), custom cursors.
- Styles in Sass (`style/main.scss`), flat colors, BEM-like modifier names.
- **SQL formatting in site copy (Examples, Docs, articles):** indent in two-space steps. When a `SELECT` list wraps, indent the continuation lines by a fixed two spaces from `SELECT` — do **not** align them to the first select-item column (the column just after `SELECT `). Keep clause keywords (`FROM`, `WHERE`, `GROUP BY`, `ORDER BY`) at column 0, and indent subquery/`LATERAL` bodies and multi-line function arguments in two-space steps from their opener. Every SQL result shown in a comment must be produced by the real engine — assert it in the extension test suite (`src/lib.rs`), not by hand.

## Documentation

| File | Purpose | When to update |
|---|---|---|
| `CLAUDE.md` | Development conventions | When conventions change |
| `README.md` | Project description, quick start, an example for **every** SQL function, doc index | Any SQL function change; any doc added/removed |
| `TODO.md` | Tracked work items with stable IDs | During planning; when items complete |
| `BUGHISTORY.md` | Resolved bugs with reoccurrence checks | Immediately when a bug is fixed |

**This project has no `docs/` directory** — a deliberate departure from the global convention, decided 2026-07-14. Everything that would have lived there lives somewhere with a reader: user-facing explanation goes on the website (`website/` — the Docs page, and the walkthroughs in `website/posts/`), decisions go in this file's *Decided* section, and findings and future work go in `TODO.md`. Do not create `docs/`, `docs/ux/`, `docs/specifications/`, or `docs/plans/`.

**Direction, release, and go-to-market notes live outside this repo.** This repo holds the extension and its site; the thinking about where it is going is kept separately.

- **Documentation is part of the code change, not a follow-up.**
- TODO items: stable IDs with domain-specific prefixes (`TEST-`, `DOCS-`, `A11Y-`, `CHORE-`, …); avoid catch-all prefixes like `FEAT` for new items (existing `FEAT-*` IDs stay stable). Each item is a header plus paragraphs; completed items move to the end of their section.
- BUGHISTORY entries: symptom, root cause, fix, files, reoccurrence checklist. After any change, verify no recorded bug is reintroduced.

## Git

- Main branch is `main` and is never modified directly — all work happens on branches in worktrees, merged via PR (`gh pr merge <number> --merge`).
- Commit discipline, in order: `/simplify` (out-of-scope findings go to `TODO.md`), `make verify` (and `make website-lint` if the website changed), BUGHISTORY reoccurrence check, then commit with a HEREDOC message (`git commit -F - <<'EOF'`), never `$()` substitution.
- Keep PRs focused; split optional enhancements into TODOs.
- When Copilot review is definitively wrong, add a correction under `.github/instructions/` (scoped, with `applyTo` frontmatter) or `.github/copilot-instructions.md` (repo-wide). Ask the user first unless 100% certain.

## Behaviors

- **No praise, no hedging, facts only.** State tradeoffs and a recommendation when one option is clearly better.
- **Match effort to complexity.** Simple tasks get executed immediately, without over-exploration or planning.
- **Simpler first, always.** Implement the minimal direct behavior; split the rest into TODOs.
- **Stop repeating, start diagnosing.** If a change didn't help, investigate before changing anything else; never stack untested hypotheses.
- **When uncertain or when rules conflict, ask** — with a concrete option.
- When the user overrules you, get the reasoning and record it here.
- Flag likely performance problems proactively (unbounded allocations, per-row re-parsing, hot-path bloat).
