# Vendored dependencies

## dsntk 0.3.0 (13 crates)

The DMN/FEEL engine ([DecisionToolkit / dsntk-rs](https://github.com/DecisionToolkit/dsntk-rs)),
vendored so pgdmn can profile, optimize, and dependency-slice the evaluation
hot path in-tree.

- **Source:** published crates.io tarballs
  (`https://static.crates.io/crates/{name}/{name}-0.3.0.crate`), extracted
  verbatim. Each tarball's sha256 was verified against the checksum recorded
  in the pre-vendoring `Cargo.lock` (which pinned the registry versions).
- **Wiring:** `[patch.crates-io]` entries in the root `Cargo.toml` redirect
  every dsntk crate to its `vendor/` path, so the whole dependency graph
  (including dsntk-internal cross-dependencies) resolves to these sources.
  The crates are also workspace members so their test suites run in-tree
  (`make vendor-test`).
- **License:** MIT (see each crate's metadata; upstream author Dariusz Depta,
  Engos Software). The `Cargo.toml.orig` / `Cargo.lock` files shipped in the
  tarballs are kept for provenance.
- **Layering model:** git history under this directory is structured as
  *pristine upstream commit* followed by a minimal, separable patch layer —
  one commit per logical change, each marked with `PGDMN:` comments at the
  change sites. Upgrades replace the pristine base and re-apply the layer
  (`make vendor-upgrade` / `make vendor-inspect` drive this). The performance
  report documents each patch's measured effect and upstream-PR viability.
- **Dependency slicing:** vendored crates may gate upstream functionality we
  do not want in a PostgreSQL backend behind off-by-default cargo features
  (e.g. the external Java/PMML evaluators and their HTTP/TLS stack —
  DEPS-001). Gates are designed to be upstreamable.

Vendored code is third-party: repo lint/style conventions do not apply to
`vendor/`, and `vendor/rustfmt.toml` disables formatting so `make fmt` leaves
upstream formatting byte-identical. `make lint` still surfaces default-level
clippy findings in vendored code (cap-lints does not apply to path
dependencies) — our own additions must stay clippy-clean at the default level.
