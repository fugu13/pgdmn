# Bug History

Resolved bugs, recorded so they can be recognised if they reappear. Every entry has: symptom, root cause, fix, files, and a reoccurrence checklist. After any code change, verify no recorded bugs have been reintroduced—pay particular attention to each entry's **Reoccurrence check** section.

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

**Fix:** The Dockerfile chowns `/usr/share/postgresql/17/extension` and `/usr/lib/postgresql/17/lib` to `pgdmn` right after creating that user, then switches to `USER pgdmn` before installing `cargo-pgrx`, adding rustup components, fetching crates, and running `cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config`—every later step is owned by `pgdmn` by construction (CHORE-004 collapsed the former root `base` / non-root `test` stages into a single stage).

**Files:** `Dockerfile`, `Makefile` (`test-image` target)

**Reoccurrence check:**
- [ ] The Dockerfile chowns the PG17 extension and lib directories immediately after `useradd`, before `USER pgdmn`
- [ ] `cargo pgrx init` (and every step after `USER pgdmn`) runs as `pgdmn`, not root
- [ ] `make check` and `make test` succeed from a fresh worktree with no `target/` directory

## BUG-004: FEEL decimal overflow panics in typed numeric conversions

**Symptom:** `feel_eval_numeric` (or `feel_eval_numrange` via a range endpoint) on an expression whose result overflows decimal128—e.g. the product of two ~3100-digit numbers—raises the opaque PG ERROR ``called `Option::unwrap()` on a `None` value`` (a Rust panic converted by pgrx) instead of a real error message.

**Root cause:** dsntk-feel-number's arithmetic (`Mul`/`Add`) is not finiteness-guarded (unlike `pow`/`from_str`), so overflow rounds to ±Inf; `FeelNumber`'s `Display` assumes `bid128_to_string` output contains `'E'` and unwraps, but for ±Inf/NaN it returns `"+Inf"` etc., so `n.to_string()` panics at pgdmn's SQL boundary. Pre-existing on dsntk 0.2; found during the 0.3 migration review.

**Fix:** shared `feel_number_is_finite` guard in `src/convert.rs` (`-Inf < n < Inf`, which also rejects NaN since NaN compares false to everything), applied before stringifying in both `feel_number_to_numeric` (typed paths) and `feel_to_json`'s `Value::Number` arm (JSONB paths, covering numbers nested in lists/contexts via recursion). Residual: a non-finite number inside a `Value::Range` endpoint still reaches the Display catch-all—that closes with the upstream fix tracked as DEPS-002 in TODO.md.

**Files:** `src/functions/feel.rs`, `src/convert.rs`

**Reoccurrence check:**
- [ ] Every path that stringifies a `FeelNumber` checks `feel_number_is_finite` first
- [ ] `test_feel_eval_numeric_rejects_decimal_overflow` and `test_feel_eval_rejects_decimal_overflow` pass

## BUG-006: website build breaks when the wasm-bindgen crate outruns the host CLI

**Symptom:** `make website-build` fails with `wasm-bindgen failed` and a message telling you to either downgrade the crate (`cargo update -p wasm-bindgen --precise <old>`) or reinstall the binary. Triggered by an ordinary `cargo update` of `website/Cargo.lock`, with no source change at all.

**Root cause:** `wasm-bindgen` is two things that must be the *exact* same version: the crate compiled into the wasm bundle, and the `wasm-bindgen-cli` binary installed on the build host that post-processes it. The lockfile pins the crate; nothing pins the binary. Refreshing dependencies moved the crate to 0.2.126 while the host still had 0.2.114, and the build broke. Nothing in the repo could have prevented it, because the binary lives outside the repo.

**Fix:** Removed the coupling rather than patching it. WEB-001 dropped the wasm bundle entirely—the site is prerendered to static HTML and has no `hydrate` feature, no `wasm-bindgen` dependency, no `cargo-leptos`, and therefore no host tool that has to be kept version-matched. Sass is compiled in-process by `grass`. (The immediate unblock at the time was `cargo install -f wasm-bindgen-cli --version 0.2.126`.)

**Files:** `website/Cargo.toml`, `website/src/bin/prerender.rs`, `Makefile`

**Reoccurrence check:**
- [ ] `website/Cargo.toml` declares no `wasm-bindgen`, no `console_error_panic_hook`, no `hydrate` feature, and no `cdylib` crate type
- [ ] The website build invokes no host binary other than `cargo`—in particular not `cargo-leptos`, `wasm-bindgen`, or `sass`
- [ ] Any proposal to reintroduce client-side interactivity is weighed against WEB-001 in CLAUDE.md's Decided section first; reintroducing the wasm bundle reintroduces this bug class

## BUG-005: CI target cache poisons pgrx tests with a non-0700 Postgres data dir

**Symptom:** The first (cold) CI run of `.github/workflows/ci.yml` passes, but the next run that restores the cached `target/` fails **every** `pg_test_*` at once. The first test panics with `pg_ctl: could not start server` / `FATAL: data directory "/pgdmn/target/test-pgdata/17" has invalid permissions` / `DETAIL: Permissions should be u=rwx (0700) or u=rwx,g=rx (0750).`; every subsequent test then panics with `Could not obtain test mutex. A previous test may have hard-aborted while holding it.`

**Root cause:** The `extension` job caches the whole bind-mounted `target/` to skip recompilation. `cargo pgrx test` initialises a throwaway Postgres cluster at `target/test-pgdata/17` with the mode `0700` that Postgres requires. A cache-archival step then ran `chmod -R a+rX target` (so the runner's non-uid-1000 user could tar the tree), which relaxed the data dir to `0755`. That `0755` cluster was cached and restored on the next run; `cargo pgrx test` reuses the existing data dir rather than reinitialising, Postgres rejects the permissions and aborts before releasing the test mutex, and the whole suite cascades to failure. Related to BUG-002 (both are about pgrx's non-root/permission requirements), but distinct: this one is CI-cache-specific and does not reproduce locally, where Docker Desktop does not enforce the mode check the same way.

**Fix:** Never cache the throwaway cluster. `ci.yml` now `rm -rf target/test-pgdata` both after cache restore (so a poisoned cache can't be reused) and before cache save (so it is never archived), and the archival `chmod` runs only on what remains. The `target` cache key prefix was bumped to `-v2-` to retire already-poisoned entries. pgrx reinitialises a fresh `0700` cluster every run.

**Files:** `.github/workflows/ci.yml`

**Reoccurrence check:**
- [ ] `ci.yml` deletes `target/test-pgdata` before `make test` runs (after cache restore) and again before the cache is saved
- [ ] No cache-archival or permission step runs `chmod` over `target/test-pgdata` (or the whole `target/` while the cluster is still present)
- [ ] If the caching scheme changes so the cluster could be archived again, bump the `-vN-` key prefix to abandon poisoned entries
- [ ] A warm CI run (one that restores the `target/` cache) is green, not just the first cold run
