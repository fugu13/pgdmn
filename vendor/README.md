# Vendored dependencies

## dsntk 0.2.0 (12 crates)

The DMN/FEEL engine ([DecisionToolkit / dsntk](https://github.com/DecisionToolkit)),
vendored so pgdmn can profile and optimize the evaluation hot path in-tree.

- **Source:** published crates.io tarballs (`https://static.crates.io/crates/{name}/{name}-0.2.0.crate`),
  extracted verbatim. Each tarball's sha256 was verified against the checksum
  recorded in `Cargo.lock` before extraction.
- **Wiring:** `[patch.crates-io]` entries in the root `Cargo.toml` redirect every
  dsntk crate to its `vendor/` path, so the whole dependency graph (including
  dsntk-internal cross-dependencies) resolves to these sources.
- **License:** MIT (see each crate's metadata; upstream author Dariusz Depta,
  Engos Software). The `Cargo.toml.orig` / `Cargo.lock` files shipped in the
  tarballs are kept for provenance.
- **Local modifications:** tracked in git history of this directory; the
  vendoring commit itself contains pristine upstream sources. Performance
  changes are documented in the perf report and marked with `PGDMN:` comments
  at the change sites.

Vendored crates are third-party code: repo lint/style conventions
(clippy pedantic, rustfmt, error-model rules) do not apply to `vendor/`.
`make lint` / `make fmt` operate on the pgdmn package only.
