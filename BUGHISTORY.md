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

**Fix:** The Dockerfile's `test` stage chowns `/usr/share/postgresql/17/extension` and `/usr/lib/postgresql/17/lib` to `pgdmn`, then runs `cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config` as `pgdmn`, creating `/home/pgdmn/.pgrx`. CHORE-004 tracks the deeper fix (build the whole image as the non-root user).

**Files:** `Dockerfile` (`test` stage), `Makefile` (`test-image` target)

**Reoccurrence check:**
- [ ] The Dockerfile `test` stage chowns the PG17 extension and lib directories before `USER pgdmn`
- [ ] The Dockerfile `test` stage runs `cargo pgrx init` after `USER pgdmn`
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
