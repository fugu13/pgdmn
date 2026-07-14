# Bug History

Resolved bugs, recorded so they can be recognised if they reappear. Every entry has: symptom, root cause, fix, files, and a reoccurrence checklist. After any code change, verify no recorded bugs have been reintroduced — pay particular attention to each entry's **Reoccurrence check** section.

## BUG-003: decision-table input entries referencing `?` never match

**Symptom:** A DMN decision-table rule whose input entry references the tested input value as `?` (e.g. `? >= 18`, valid DMN unary-test syntax) never matches; evaluation silently falls through to later rules or the default, misclassifying rows without any error.

**Root cause:** Two layered gaps in the vendored dsntk engine. (1) The FEEL parser only recognises names that are in scope at parse time, and nothing registered `?` before input entries were parsed, so `?`-entries failed to parse. (2) Even with `?` parseable, every entry was compiled into the implicit membership form `? in (entry)`; a `?`-referencing entry evaluates to a boolean, and testing the input value for membership *in a boolean* is always false. Per the DMN spec, an entry referencing `?` is itself the test.

**Fix:** In `vendor/dsntk-model-evaluator/src/decision_table.rs`, the table parse pushes a scope context with `?` registered (popped on all paths), and entry compilation classifies each parsed unary-test item with `ast_references_name` (exported from `dsntk-feel-evaluator`): `?`-referencing items are evaluated directly, `?`-free items keep the membership form, and comma-list items OR-combine. String literals containing `?` are unaffected because the check is AST-based, not textual.

**Files:** `vendor/dsntk-model-evaluator/src/decision_table.rs`, `vendor/dsntk-feel-evaluator/src/builders.rs`, `vendor/dsntk-feel-evaluator/src/lib.rs`, `src/lib.rs` (test)

**Reoccurrence check:**
- [ ] `test_dmn_decision_table_question_mark_binding` passes (ages 16/18/20 against `? >= 18`)
- [ ] `parse_decision_table` is still wrapped in the pushed/popped `?` scope context (including error paths)
- [ ] Entry compilation still routes through `input_entry_test_node`, not a bare `? in (...)` wrap
- [ ] If dsntk is re-vendored/upgraded: verify upstream includes an equivalent fix before dropping ours

## BUG-001: Docker image crate cache diverges from Cargo.lock

**Symptom:** `make lint` / `make check` fails inside the container with `failed to open /usr/local/cargo/registry/cache/.../<crate>.crate ... Permission denied (os error 13)` immediately after any `Cargo.toml` change.

**Root cause:** The Dockerfile ran `cargo fetch` without copying `Cargo.lock`, so the image's registry cache was populated with the *latest* resolvable versions at image-build time, not the versions the repo's lockfile pins. At runtime cargo (running as the non-root `pgdmn` user) tried to download the pinned versions into the root-owned `/usr/local/cargo/registry` and was denied.

**Fix:** Copy `Cargo.lock` into the image alongside `Cargo.toml` and fetch with `cargo fetch --locked`, keeping the image cache aligned with the repo's pinned versions.

**Files:** `Dockerfile`

**Reoccurrence check:**
- [ ] `Dockerfile` copies `Cargo.lock` next to `Cargo.toml` before `cargo fetch`
- [ ] The fetch step uses `--locked`
- [ ] After updating dependencies (`Cargo.lock` changes), `make test-image` is re-run before `make check`/`make lint`/`make test`

## BUG-002: non-root test user cannot use root-initialized pgrx/PostgreSQL

**Symptom:** Cold builds (fresh worktree, empty `target/`) fail inside the container with `Error: $PGRX_HOME does not exist` while building `pgrx-pg-sys`. After fixing that, `cargo pgrx test` fails with `failed writing /pgdmn/pgdmn.control to /usr/share/postgresql/17/extension/pgdmn.control: Permission denied`, cascading into every test failing (`Could not initialize test framework` / `Could not obtain test mutex`).

**Root cause:** Everything pgrx needs at test time was set up as root: `cargo pgrx init` wrote `/root/.pgrx` (invisible to the `pgdmn` user), and the system PostgreSQL's extension install directories are root-owned, so `cargo pgrx install --test` cannot copy the built extension in. Warm builds masked the first half by reusing cached `pgrx-pg-sys` artifacts from a populated `target/`.

**Fix:** The Dockerfile chowns `/usr/share/postgresql/17/extension` and `/usr/lib/postgresql/17/lib` to `pgdmn` right after creating that user, then switches to `USER pgdmn` before installing `cargo-pgrx`, adding rustup components, fetching crates, and running `cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config` — every later step is owned by `pgdmn` by construction (CHORE-004 collapsed the former root `base` / non-root `test` stages into a single stage).

**Files:** `Dockerfile`, `Makefile` (`test-image` target)

**Reoccurrence check:**
- [ ] The Dockerfile chowns the PG17 extension and lib directories immediately after `useradd`, before `USER pgdmn`
- [ ] `cargo pgrx init` (and every step after `USER pgdmn`) runs as `pgdmn`, not root
- [ ] `make check` and `make test` succeed from a fresh worktree with no `target/` directory
