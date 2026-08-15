# TODO Archive

Completed or explicitly-dropped items move here from `TODO.md`, keeping their
original stable ID and header, so the ID stays traceable without cluttering
the active list. This practice starts now — the items pruned from `TODO.md`
in the 2026-08-14 cleanup were deleted outright, not archived, and are not
backfilled here.

### WEB-007: OpenGraph and social meta tags, with a share card worth sharing (done)

Every prerendered page now carries a description, the OpenGraph set
(`og:site_name`, `og:locale`, `og:type`, `og:title`, `og:description`,
`og:url`, `og:image` with dimensions and alt text), the Twitter
`summary_large_image` set, and a canonical link. One `PageMeta` component
(`website/src/components/page_meta.rs`) emits all of it; each page passes its
own title, description and path, so a new page that forgets its metadata is
visible as a missing component rather than as a bare unfurl in someone's chat
client. Absolute URLs are built through `site::url`, backed by the same
`DOMAIN` constant the prerender writes to `CNAME` — the constant moved into
`website/src/site.rs` so the pages and the prerender cannot disagree about the
host.

The card is `website/public/share-card.png`, 1200x630, rendered from
`website/card/share-card.html`: the wordmark and tagline beside a small
decision table, which is the one image that says "DMN" at thumbnail size.
Chosen over a plain typographic card (light and dark variants were rendered
and compared in Slack, X, LinkedIn, Discord and iMessage mockups, and at the
220px a phone feed gives a card). The markup source is committed next to the
PNG so the card is edited as markup, not in an image editor; `card/` sits
outside `public/` so the prerender does not publish the source. Icons landed
with it: `favicon.svg` (a decision-table glyph, two columns rather than three
because it is read at 16px), plus `favicon.ico` and `apple-touch-icon.png`
rendered from that same SVG via `card/icons.html`.

Per-page values rather than one sitewide description: Why, Docs, Examples and
Articles each describe themselves, and articles use their front matter —
`og:type=article`, `article:published_time` from `date`, and an optional
`description:` key for the four posts whose index summary runs longer than a
crawler will show (`Article::card_description` falls back to `summary`). A
test fails the build if any article description exceeds
`site::DESCRIPTION_LIMIT`, so the fix is a shorter sentence rather than a
platform truncating mid-word. 404 is `noindex, follow`; `public/dmn-viewer.html`
is `noindex` too and states its own icons, since it renders whatever `?model=`
names and is a static asset rather than a Leptos route.

Guarded in CI: the `Website` workflow now runs `make website-test` (new
target — the website's unit tests had no runner before this) and fails if any
generated page is missing its description, `og:title`, `og:description`,
`og:image`, `twitter:card` or canonical, if the card URL is not absolute, or
if any of the four image assets is missing from `dist/`. Deliberately left for
later: per-article card images, which mean generating images at prerender time
and are a much larger commitment than one sitewide card.

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
