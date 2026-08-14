---
applyTo: "vendor/**"
---

# vendor/ is deliberately unformatted

`vendor/rustfmt.toml` disables `make fmt` under `vendor/` on purpose — a diff against pristine upstream dsntk must stay legible. Do NOT propose reformatting, style cleanup, or idiom modernization (e.g. `?`-operator rewrites, iterator-chain simplification, import reordering) anywhere under `vendor/`, even in a line adjacent to a real change.

# PGDMN: comments mark a deliberate, minimal patch

A `PGDMN:` comment marks a change site in an otherwise-pristine upstream file, cataloged in `vendor/PATCHES.md` with what it does and its measured effect. The surrounding minimalism is intentional — these patches are designed to be small enough to offer upstream to DecisionToolkit/dsntk. Do NOT suggest rewording or removing a `PGDMN:` comment, and do NOT propose a broader or "cleaner" rewrite of the logic it touches.

# vendor/ is exempt from this repo's pedantic/nursery lint policy

CLAUDE.md's Lints section scopes clippy's `pedantic`/`nursery` levels to the `pgdmn` package only (`make lint` runs `cargo clippy -p pgdmn`); vendor/ code is held only to default-level clippy. Do NOT flag pedantic- or nursery-style suggestions (missing `# Errors`/`# Panics` docs, `needless_pass_by_value`, `must_use_candidate`, and similar) anywhere under `vendor/`.

# Do not fix vendor/ bugs inline

If something under `vendor/` looks wrong, say so, but do not propose an inline fix as part of a feature PR — vendor/ changes are their own PR, one commit per logical change, reviewed on their own terms (`.github/CODEOWNERS` routes them automatically). See `CONTRIBUTING.md`'s "Working with vendor/" section.
