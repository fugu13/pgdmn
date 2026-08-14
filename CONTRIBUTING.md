# Contributing to pgdmn

Thanks for your interest. This guide covers how to build, test, and submit changes.

## Building requires Docker, not a local toolchain

The extension builds **only** inside Docker. The pgrx toolchain and PostgreSQL 17 live in the image; your host needs nothing but Docker and `make`. `cargo test` on the host will not work, and is not meant to—reach for `make` instead.

```sh
make help          # list every target
make test-image    # build the Docker images (first time, and after Cargo.lock changes)
make check         # fast compilation check
make test          # the pgrx test suite against PostgreSQL 17
make lint          # clippy (warnings denied) + rustfmt check
make verify        # fmt + lint—run this after every change
```

Run `make test-image` again whenever `Cargo.lock` changes; the image fetches crates `--locked`, so it must be rebuilt to match.

## Before you open a pull request

- `make verify` passes (formatting and lint are enforced; warnings are errors).
- `make test` passes.
- If you fixed a bug, add an entry to `BUGHISTORY.md`—symptom, root cause, fix, and a check for recognising it again.
- If you changed or added an SQL function, update `README.md`; it carries an example for every function.
- New behaviour is tested. This project writes the test first: the signature, then a test expressing the behaviour, confirm it fails for the right reason, then implement.

## Conventions

The full set lives in [CLAUDE.md](CLAUDE.md)—error handling, lint policy, naming, and the pgrx and dsntk gotchas that are easy to trip over. The essentials:

- **Propagate, never crash.** No `unwrap`, `expect`, `panic!`, or fallible indexing in non-test code; return `Result` and convert to a SQL error at the boundary with `pgrx::error!`. Test code may `unwrap` freely.
- **Validate at boundaries, trust internals.** Check SQL inputs where they enter; do not re-check them deeper in.
- **Error messages are for the SQL author who reads them**, and include the offending value.

## Working with vendor/

`vendor/` holds the DMN/FEEL engine (dsntk), patched in-tree. It has its own rules, stricter than the rest of the repo:

- **Never touch `vendor/` in a feature PR.** Vendor changes are their own PR, reviewed on their own terms—`.github/CODEOWNERS` routes any PR touching `vendor/**` through its owner automatically.
- **One commit per logical change**, each carrying a `PGDMN:` comment at every site it touches. This keeps the patch layer separable from the pristine upstream base—see `vendor/README.md` and `vendor/PATCHES.md` for the full model.
- **Don't reformat.** `vendor/rustfmt.toml` disables `make fmt` there on purpose, so a diff against pristine upstream stays legible. Resist the urge to clean up surrounding style even when touching a line right next to it.
- Every carried patch is logged in `vendor/PATCHES.md` with what it does and its measured effect—add an entry as part of the same commit, not a follow-up.

## Work items

Tracked in `TODO.md` with stable prefixed IDs. If you want to take one on, or propose one, that is the place.

## Reporting a vulnerability

Please do not open a public issue for a security problem. See [SECURITY.md](SECURITY.md).
