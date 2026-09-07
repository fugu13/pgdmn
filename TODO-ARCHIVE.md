# TODO Archive

Completed or explicitly-dropped items move here from `TODO.md`, keeping their
original stable ID and header, so the ID stays traceable without cluttering
the active list. This practice starts now — the items pruned from `TODO.md`
in the 2026-08-14 cleanup were deleted outright, not archived, and are not
backfilled here.

### PUBLIC-005: CONTRIBUTING.md, CODEOWNERS, and a Copilot vendor instruction (done)

`.github/CODEOWNERS` routes `vendor/**` and `vendor/CHECKSUMS` to
@fugu13. `CONTRIBUTING.md` has a "Working with vendor/" section
covering the patch discipline (never edit vendor/ in feature PRs; one
commit per change; PGDMN: markers; no reformatting; PATCHES.md entry
per commit). Branch-protection enforcement (`require_code_owner_reviews`)
is live on `main`, verified path-scoped and admin-bypassable — no
self-approval deadlock. `.github/instructions/vendor.instructions.md`
tells Copilot review not to propose reformatting, flag `PGDMN:`
comments, apply pedantic/nursery lint suggestions, or propose inline
fixes anywhere under `vendor/`.

### PUBLIC-010: Enable secret scanning, push protection, and private vulnerability reporting right after going public (done)

All three verified live via a fresh `gh api` fetch after the
visibility flip, not assumed from the settings PATCH response:
`secret_scanning` and `secret_scanning_push_protection` both
`"enabled"`; `private-vulnerability-reporting` returns
`{"enabled": true}` (it 404'd on both GET and PUT while the repo was
private — confirms the endpoint genuinely wasn't available until the
flip, not a bug). `secret_scanning_non_provider_patterns` and
`secret_scanning_validity_checks` were left disabled — narrower,
more advanced sub-features not named in the original ask; enable
separately if wanted.

### PUBLIC-011: Set repo topics and homepage after going public (done)

Topics set: `dmn`, `feel`, `decision`, `postgres`, `pgrx` (mirrors
Cargo.toml's `keywords`). Homepage set to `https://www.pgdmn.com` once
the Route 53 DNS records were wired up and the site actually resolved
— that alone wasn't sufficient, though: DNS pointing at GitHub's
servers doesn't make Pages serve the site by itself, since GitHub's
edge routes by which repo has *claimed* the domain (`cname` on the
Pages API), not just by where DNS points. `www.pgdmn.com` returned
GitHub's generic "there isn't a GitHub Pages site here" page until the
custom domain was explicitly set (`PUT .../pages` with
`cname=www.pgdmn.com`) — after that, the certificate approved
(covers both `www.pgdmn.com` and `pgdmn.com`) and HTTPS enforcement
was turned on. Verified live: both domains serve real content over
HTTP and HTTPS, and the apex redirects (301) to `https://www.pgdmn.com`.

### WEB-006: Mobile hamburger menu (done, different shape than titled)

The site nav overflowed the header on narrow viewports. Resolved without
a hamburger: `.site-nav` wraps onto additional lines below the logo
(`flex-wrap` scoped to `@media (max-width: 40em)`), each item keeps its
label on one line (`white-space: nowrap`), and wrapped items are
underlined on mobile only—matching the footer's existing convention for
inline links—so a multi-line nav still reads as discrete links rather
than run-on prose. Considered and rejected: a CSS-only `:checked`-driven
hamburger disclosure (weaker accessibility semantics than a plain link
list—no live `aria-expanded`—for no real space savings, since the site
only has six nav items). Follow-ups split out: WEB-008 (tokenize the
`40em` breakpoint), WEB-009 (scope the generic `ul, ol` prose rule so
`.site-nav` and its siblings don't each need a `padding-left: 0`
override).

### CHORE-006: Migrate to pgrx 0.18 (done, landed on 0.19)

pgrx and pgrx-tests moved together from 0.16.1 to 0.19.2 (a newer patch
was current by the time this landed than the 0.18.0 the item was
written against). The `pgrx_embed` binary pattern this item flagged as
the first surfaced breakage is gone as of pgrx 0.18: `src/bin/pgrx_embed.rs`
and its `[[bin]]` target are deleted, `crate-type` is `["cdylib"]` (was
`["cdylib", "lib"]`)—pgrx now embeds SQL entity metadata directly in the
compiled shared library and `cargo-pgrx` reads it from the `.pgrx`
linker section instead of running a second compile pass. No manual
`SqlTranslatable` impls existed to port (the custom `DmnModel` type uses
the `PostgresType`/`InOutFuncs` derive path, which the migration guide
confirms is unaffected). `Dockerfile`'s `cargo install cargo-pgrx
--version` pin moved to `~0.19` to match. Full `make test` suite passes
(130 tests), including the `DmnModel` `InOutFuncs` and
`pgrx::datum::Interval` paths flagged as most version-sensitive.
