---
applyTo: "website/**/*.rs"
---

# Leptos view! macro syntax

In Leptos's `view!` macro, all text content must be wrapped in double quotes as Rust string literals. This is NOT a bug — the quotes are part of the macro syntax and do NOT render in the browser.

Correct Leptos code:

```rust
view! {
    <li><code>"dmn_load"</code>" — parse DMN XML"</li>
}
```

This renders as: `dmn_load` — parse DMN XML (no visible quotes).

Do NOT flag quoted string literals inside `view!` as stray or extra quotation marks.

# Leptos meta context

The official Leptos pattern places `<MetaTags/>` in the `shell()` function and `provide_meta_context()` inside the `App` component. This works because `<App/>` renders inside `shell()`, making the context available to `<MetaTags/>` reactively. Do NOT flag this as a scope issue.

# Leptos component shorthand props

`<HydrationScripts options/>` is valid Leptos shorthand for `<HydrationScripts options=options/>`. Do NOT flag this as invalid prop syntax.
