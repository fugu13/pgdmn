//! The content-hashed stylesheet href every rendered page links.
//!
//! The prerender pass computes the hash once, before any route is rendered,
//! and every subsequent route render reads the same value—so all pages link
//! one fingerprinted CSS URL without threading it through every route.

use std::sync::OnceLock;

static HREF: OnceLock<String> = OnceLock::new();

/// Set once by the prerender pass, before any route is rendered.
///
/// # Panics
/// If called more than once—a build-tool invariant, not a runtime path.
pub fn set_href(href: String) {
    HREF.set(href)
        .expect("stylesheet::set_href called more than once");
}

/// The content-hashed stylesheet path every rendered page links.
///
/// # Panics
/// If called before `set_href`. In the current build, `set_href` always runs
/// before any route renders, so this can only fire if that ordering is
/// broken by a future refactor—failing loudly beats silently linking a
/// stylesheet path `compile_stylesheet` never wrote.
pub fn href() -> &'static str {
    HREF.get().expect("stylesheet::href called before set_href")
}
