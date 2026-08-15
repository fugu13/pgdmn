//! The site's own identity: where it is served from, and what represents it
//! when a link to it is pasted somewhere else.
//!
//! Crawlers do not resolve relative URLs in social metadata, so every
//! `og:url`/`og:image` has to be absolute. Building them here means the host
//! is written down once—the same constant the prerender pass writes to
//! `CNAME`—rather than repeated as a literal on every page.

/// The canonical host. GitHub Pages reads this from the published output and
/// redirects the apex (pgdmn.com) here.
pub const DOMAIN: &str = "www.pgdmn.com";

/// What social platforms name as the source of a shared link.
pub const NAME: &str = "pgdmn";

/// The share card: 1200×630, the size every platform crops toward.
/// Regenerated from `card/share-card.html`; see that file for how.
pub const CARD: &str = "/share-card.png";
pub const CARD_WIDTH: &str = "1200";
pub const CARD_HEIGHT: &str = "630";

/// Describes the card for anyone who cannot see it.
///
/// Screen readers announce this on the platforms that expose it, so it says
/// what the card says.
pub const CARD_ALT: &str = "pgdmn: run DMN decision tables inside PostgreSQL. \
     A decision table with an Age and Income column, deciding Approve, Review, or Decline.";

/// The longest description a crawler shows without truncating it.
///
/// Search engines and social platforms both cut past this, and a sentence
/// ending mid-word reads as neglect. The fix is always a shorter sentence,
/// never letting the crawler choose where it ends.
pub const DESCRIPTION_LIMIT: usize = 160;

/// An absolute URL for a site-absolute path (`/why/` → `https://…/why/`).
pub fn url(path: &str) -> String {
    format!("https://{DOMAIN}{path}")
}

#[cfg(test)]
mod tests {
    use super::{CARD, url};

    #[test]
    fn builds_absolute_urls_crawlers_can_follow() {
        assert_eq!(url("/"), "https://www.pgdmn.com/");
        assert_eq!(url("/why/"), "https://www.pgdmn.com/why/");
        assert_eq!(url(CARD), "https://www.pgdmn.com/share-card.png");
    }
}
