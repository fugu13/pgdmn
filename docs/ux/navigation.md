# Navigation

How a visitor moves between pages on the pgdmn website, and what they see when a page does not exist.

## Moving between pages

Every page carries the same header: the site name on the left, acting as a link home, and a navigation list containing Home, Why pgdmn, Docs, Examples, Blog, and a link out to the project on GitHub.

Home appears in the list as well as being reachable by the site name, because a visitor should not have to know that a wordmark is clickable to get back to the start.

Choosing any of these loads that page whole. The browser performs an ordinary page load — the address in the location bar changes to the page's own address, the back button returns to the previous page, and the page can be reloaded, bookmarked, or opened in a new tab and will show the same content. Nothing is fetched or swapped in behind the scenes.

Because each page is a complete document delivered as-is, navigation continues to work with JavaScript turned off entirely. The site never depends on scripting to render or to move between pages.

The link for the page the visitor is currently on is marked as the current page, so assistive technology announces it as such rather than presenting it as somewhere else to go.

## Reaching the content directly

The first thing reachable by keyboard on every page is a skip link. It is invisible until focused; pressing Tab on arrival reveals it. Activating it moves focus past the header and navigation, directly to the page's main content, so a keyboard or screen-reader user does not have to travel through the same navigation list on every page.

Focus lands on the main content region itself, so the next Tab press continues from there rather than returning to the top of the page.

## When a page does not exist

Requesting an address that does not correspond to a page shows a not-found page. It is a normal page of the site — same header, same navigation, same footer — so the visitor can immediately continue somewhere useful rather than reaching a dead end.

The response also reports, at the protocol level, that the page was not found, so search engines and other automated visitors are told the truth rather than being shown an apparently valid page.
