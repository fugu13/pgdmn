# pgdmn

PostgreSQL extension that brings DMN (Decision Model and Notation) support to Postgres. Built with Rust, pgrx, and dsntk. A Leptos SSR website lives in `website/`.

## Stack

| Component | Choice |
|---|---|
| Language | Rust 2024 edition (extension); Rust 2021 (website, pending CHORE-003) |
| Extension framework | pgrx 0.16 |
| DMN/FEEL engine | dsntk 0.2 |
| Target | PostgreSQL 17 |
| Website | Leptos 0.8 + Axum SSR (`website/`) |

## Build & Run

All extension builds run in Docker (images: `pgdmn-base` = PG17 + pgrx toolchain, `pgdmn-test` adds the non-root user initdb requires). All commands go through `make` — do not invoke `cargo` or `docker` directly.

| Command | Purpose |
|---|---|
| `make help` | List all targets |
| `make test-image` | Build/refresh the Docker images (required after `Cargo.lock` changes) |
| `make check` | Fast compilation check |
| `make build` | Build the extension |
| `make test` | pgrx test suite against PG17 |
| `make lint` | clippy (deny warnings) + rustfmt check |
| `make fmt` | Auto-format |
| `make verify` | fmt + lint — run after every code change (lint's clippy subsumes check) |
| `make bench` | DMN eval benchmark |
| `make website-dev` | Website dev server with hot-reload |
| `make website-build` | Production website build |
| `make website-lint` | clippy + rustfmt check for the website |

## Architecture

```
src/
  lib.rs            — pg_module_magic, integration tests
  cache.rs          — thread-local ModelEvaluator cache (keyed by XML hash)
  convert.rs        — FEEL value ↔ PG type conversions
  types/
    dmn_model.rs    — custom DmnModel PG type (InOutFuncs: XML in, namespace::name out)
  functions/
    feel.rs         — feel_eval (JSONB), feel_record_eval (record), + 6 typed variants (numeric, bool, text, date, timestamp, interval)
    dmn.rs          — dmn_load, dmn_eval, dmn_record_eval
    introspection.rs — dmn_invocables, dmn_info, dmn_xml, dmn_name, dmn_namespace
website/
  src/              — Leptos app (pages, components, routes)
  style/main.scss   — Sass styles
```

### Decided

- **Docker-only extension builds.** The pgrx toolchain and PG17 live in the images; the host never needs them. The Dockerfile copies `Cargo.lock` and fetches `--locked` so the image's crate cache matches the repo (see BUG-001 in BUGHISTORY.md).
- **Error model:** SQL-facing errors are raised with `pgrx::error!` (unwinds into a PostgreSQL ERROR — the idiomatic pgrx mechanism). Internal fallible functions return `Result<_, String>` and callers convert at the SQL boundary with `unwrap_or_else(|e| pgrx::error!(...))`. This is an accepted deviation from the thiserror convention: errors here terminate at the SQL boundary as messages, so enum error types add no value. Revisit if errors ever need programmatic matching.
- **Website uses Leptos SSR** — an accepted deviation from the no-client-framework frontend rule, decided before this convention existed. New pages follow the existing Leptos patterns.
- **No LTO in dev profile** (causes ICE on Rust 1.85/aarch64).

### Undecided

- Efficient PG→DMN data passing beyond JSONB and records — candidate approaches tracked in `docs/improvements.md`.
- FEEL type compatibility rules for FEAT-002 (`dmn_compat`).

## Conventions

### Error handling

- **Propagate, never crash.** No `unwrap()`, `expect()`, `panic!()`, or fallible indexing in non-test code. Test code may `unwrap()` freely.
- At the SQL boundary, raise errors with `pgrx::error!` (see Decided above). Everywhere else, return `Result`.
- **Validate at boundaries, trust internals.** Check SQL inputs (XML, expressions, contexts) at the point of entry; do not re-validate inside.
- **Error messages are for the user reading them** — a SQL author, not a Rust developer. Include the offending value: `expected FEEL number, got: {other}`.

### Lints

- Lint policy lives in `[lints.clippy]` in each `Cargo.toml` (pedantic + nursery as warnings, justified allows listed there). Enforcement is `make lint` with `-D warnings` — never `#![deny(warnings)]` in source.
- Per-item `#[allow(clippy::...)]` only with a comment explaining why.

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
- **Property-based testing is mandatory** for code with algebraic or combinatorial properties (parsing round trips, value conversions). Use `proptest`; commit `*.proptest-regressions` (TEST-001 tracks the initial suite).
- Unit tests in `#[cfg(test)]` blocks in the module they test; SQL-level integration tests via `#[pg_test]` in `src/lib.rs`.

## Website

- **Accessibility is a default, not a feature.** WCAG 2.2 AA on every page. Semantic HTML before ARIA; one `<h1>` per page, no skipped heading levels; `<main>`, `<nav>`, and a skip link; visible focus indicators; 4.5:1 text contrast (3:1 large text/UI); keyboard reachability for every interactive element; every image has `alt`.
- **URL as state:** the current view is reproducible from its URL.
- **Progressive enhancement:** pages render server-side; links and navigation work without JavaScript.
- **Anti-patterns — do not build:** modals, toasts, skeleton screens, infinite scroll, dark-mode toggles (respect `prefers-color-scheme`), custom cursors.
- Styles in Sass (`style/main.scss`), flat colors, BEM-like modifier names.
- UI changes get a behavioral description in `docs/ux/{aspect}.md` in the same change (see DOCS-002 for the backfill).

## Documentation

| File | Purpose | When to update |
|---|---|---|
| `CLAUDE.md` | Development conventions | When conventions change |
| `README.md` | Project description, quick start, an example for **every** SQL function, doc index | Any SQL function change; any doc added/removed |
| `TODO.md` | Tracked work items with stable IDs | During planning; when items complete |
| `BUGHISTORY.md` | Resolved bugs with reoccurrence checks | Immediately when a bug is fixed |
| `RELEASEPLAN.md` | Release and go-to-market plan | As the plan changes |
| `docs/{feature}.md` | Feature descriptions (user actions, no code blocks; mermaid for diagrams) | When feature behavior changes |
| `docs/ux/{aspect}.md` | Website UI behavioral descriptions | When UI changes |
| `docs/specifications/{ID}-{slug}.md` | Specifications (via `/spec`; historical artifacts) | Created before planning |
| `docs/plans/{ID}-{slug}.md` | Implementation plans (via `/blueprint`; status header kept current) | When work completes or is superseded |

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
- Never reference Harry Potter or anything related to JK Rowling.
