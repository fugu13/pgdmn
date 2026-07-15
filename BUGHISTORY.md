# Bug History

Resolved bugs, recorded so they can be recognised if they reappear. Every entry has: symptom, root cause, fix, files, and a reoccurrence checklist. After any code change, verify no recorded bugs have been reintroduced — pay particular attention to each entry's **Reoccurrence check** section.

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

## BUG-003: website build breaks when the wasm-bindgen crate outruns the host CLI

**Symptom:** `make website-build` fails with `wasm-bindgen failed` and a message telling you to either downgrade the crate (`cargo update -p wasm-bindgen --precise <old>`) or reinstall the binary. Triggered by an ordinary `cargo update` of `website/Cargo.lock`, with no source change at all.

**Root cause:** `wasm-bindgen` is two things that must be the *exact* same version: the crate compiled into the wasm bundle, and the `wasm-bindgen-cli` binary installed on the build host that post-processes it. The lockfile pins the crate; nothing pins the binary. Refreshing dependencies moved the crate to 0.2.126 while the host still had 0.2.114, and the build broke. Nothing in the repo could have prevented it, because the binary lives outside the repo.

**Fix:** Removed the coupling rather than patching it. WEB-001 dropped the wasm bundle entirely — the site is prerendered to static HTML and has no `hydrate` feature, no `wasm-bindgen` dependency, no `cargo-leptos`, and therefore no host tool that has to be kept version-matched. Sass is compiled in-process by `grass`. (The immediate unblock at the time was `cargo install -f wasm-bindgen-cli --version 0.2.126`.)

**Files:** `website/Cargo.toml`, `website/src/bin/prerender.rs`, `Makefile`

**Reoccurrence check:**
- [ ] `website/Cargo.toml` declares no `wasm-bindgen`, no `console_error_panic_hook`, no `hydrate` feature, and no `cdylib` crate type
- [ ] The website build invokes no host binary other than `cargo` — in particular not `cargo-leptos`, `wasm-bindgen`, or `sass`
- [ ] Any proposal to reintroduce client-side interactivity is weighed against WEB-001 in CLAUDE.md's Decided section first; reintroducing the wasm bundle reintroduces this bug class

## BUG-004: CI target cache poisons pgrx tests with a non-0700 Postgres data dir

**Symptom:** The first (cold) CI run of `.github/workflows/ci.yml` passes, but the next run that restores the cached `target/` fails **every** `pg_test_*` at once. The first test panics with `pg_ctl: could not start server` / `FATAL: data directory "/pgdmn/target/test-pgdata/17" has invalid permissions` / `DETAIL: Permissions should be u=rwx (0700) or u=rwx,g=rx (0750).`; every subsequent test then panics with `Could not obtain test mutex. A previous test may have hard-aborted while holding it.`

**Root cause:** The `extension` job caches the whole bind-mounted `target/` to skip recompilation. `cargo pgrx test` initialises a throwaway Postgres cluster at `target/test-pgdata/17` with the mode `0700` that Postgres requires. A cache-archival step then ran `chmod -R a+rX target` (so the runner's non-uid-1000 user could tar the tree), which relaxed the data dir to `0755`. That `0755` cluster was cached and restored on the next run; `cargo pgrx test` reuses the existing data dir rather than reinitialising, Postgres rejects the permissions and aborts before releasing the test mutex, and the whole suite cascades to failure. Related to BUG-002 (both are about pgrx's non-root/permission requirements), but distinct: this one is CI-cache-specific and does not reproduce locally, where Docker Desktop does not enforce the mode check the same way.

**Fix:** Never cache the throwaway cluster. `ci.yml` now `rm -rf target/test-pgdata` both after cache restore (so a poisoned cache can't be reused) and before cache save (so it is never archived), and the archival `chmod` runs only on what remains. The `target` cache key prefix was bumped to `-v2-` to retire already-poisoned entries. pgrx reinitialises a fresh `0700` cluster every run.

**Files:** `.github/workflows/ci.yml`

**Reoccurrence check:**
- [ ] `ci.yml` deletes `target/test-pgdata` before `make test` runs (after cache restore) and again before the cache is saved
- [ ] No cache-archival or permission step runs `chmod` over `target/test-pgdata` (or the whole `target/` while the cluster is still present)
- [ ] If the caching scheme changes so the cluster could be archived again, bump the `-vN-` key prefix to abandon poisoned entries
- [ ] A warm CI run (one that restores the `target/` cache) is green, not just the first cold run
