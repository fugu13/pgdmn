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
