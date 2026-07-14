# WEB-001: Prerender the website to static HTML

Status: Implemented (2026-07-13)

## Summary

The pgdmn website is rendered once at build time into static HTML files and served as plain files by a static host, with no server process and no client-side JavaScript.

## Problem

The site was built as a Leptos SSR application that also shipped a WebAssembly hydration bundle — 343 KB of wasm plus 19.5 KB of JavaScript — to every visitor. It had nothing to hydrate. There are no signals, no server functions, no client state, and no interactive components: every page is prose, navigation links, and code examples.

That bundle was not merely wasted bandwidth. It imposed three costs:

| Cost | Consequence |
|---|---|
| Version coupling | The `wasm-bindgen` crate and the `wasm-bindgen-cli` host binary must match exactly. A routine dependency refresh broke the build until the host tool was reinstalled. |
| Build toolchain | `cargo-leptos` was required to orchestrate the wasm build, and with it a second host tool to keep current. |
| Hosting shape | A running Axum process was needed to serve requests, restricting hosting to platforms that run containers. |

The third cost became decisive: hosting is GitHub Pages, which serves static files and cannot run a server process at all.

## Requirements

The site must render every route to a file at build time, with no request-time rendering, and must therefore not depend on request state anywhere.

Navigation must work with JavaScript entirely disabled — which prerendering makes literally true rather than aspirational.

The accessibility guarantees the site already makes must survive rendering to disk: a skip link targeting the main landmark, `header`/`nav`/`main`/`footer` landmarks, a single `h1` per page with no skipped heading levels, and a document language.

Unknown paths must produce a not-found page carrying a genuine 404 status, since a static host has no server-side routing to fall back on.

Stylesheets must compile without a host tool that has to be installed and version-matched separately, because eliminating exactly that class of coupling is the point of the change.

The URL a page is served from must address its file directly, rather than relying on a particular host's implicit resolution rules, so that the local preview and the production host agree.

## Non-goals

Changing any page's content, wording, or visual design. The rendered output is the same site.

Restoring hot-reload. The previous dev loop depended on `cargo-leptos watch`, which cannot survive the removal of the wasm target. Rebuilding to see changes is accepted.

Deployment itself. Publishing the generated output to GitHub Pages, and the DNS records pointing at it, are FEAT-006.

## Tradeoffs accepted

Losing hot-reload is a real regression in developer experience, accepted because the site is five static pages and the rebuild is fast.

Content that must vary per request becomes impossible without revisiting this decision. Nothing on the site does, and the blog (FEAT-004) will render its markdown at build time into the same static output.

## Verification

Every route renders to a file; the generated HTML references no JavaScript or wasm; the compiled stylesheet is present; navigation links resolve directly without redirects; an unknown path returns 404; and the accessibility landmarks are present in the emitted markup.
