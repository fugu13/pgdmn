# Carried patches

One entry per logical change in the patch layer on top of pristine upstream
(see README.md in this directory for the layering model; `make vendor-status`
lists the live commits, `make vendor-diff` the full delta). Engine effects
below are canary-gated medians against pristine 0.3.0 on the benchmark
scenarios named; "sub-floor" means individually below the ~8% cross-build
noise floor, kept on mechanistic grounds.

| ID | Crate | What it does | Measured effect |
|---|---|---|---|
| H4 | model-evaluator | Decision tables evaluate hit-policy-aware: input entries short-circuit per rule, output entries only for matching rules, FIRST stops at the first match, UNIQUE at the second | −40% on a 6-rule table (largest single win) |
| H10 | model-evaluator | Input expressions evaluate once per call and bind to `?`, instead of once per rule×column | structural (M instead of N×M evaluations) |
| H5/H11 | model-evaluator | Per-call context/value clone cuts in decision, input-data, and decision-service evaluation | risk −13%, lending −15% (on top of H4) |
| H12 | model-evaluator | BKM invocation: fewer FunctionDefinition clones, single dispatch | lending ≈−3% |
| H18 | model-evaluator | Allocation-free invocable lookup (borrowed nested-map probe instead of a 3-String tuple key) | sub-floor |
| — | model-evaluator | Component names borrowed during decision-table evaluation | minor |
| BUG-003 | model-evaluator (+ walker export from feel-evaluator) | `?`-referencing input entries evaluate directly per the DMN spec (they never matched upstream — verified still broken in 0.3.0) | correctness |
| H3 | feel | Copy-on-write FeelContext behind an Arc: cloning a context (every scope lookup) is O(1); all mutation through one `Arc::make_mut` funnel | lending −30%, loan-comparison −56% |
| H6 | feel | Owned coercion path skips deep-cloning already-conformant values | −2–4% on DMN scenarios |
| H13 | feel-evaluator | For-expressions accumulate in place (upstream is quadratic in list length); some/every stop once decided; exhaustive AST walker detects `partial` references (fails the build on new parser variants by design) | for-loop ×100 −90% |
| H14 | feel-evaluator | replace()/split() reuse compiled regexes via a small per-thread LRU (matches() needs a dsntk-feel-regex change — PERF-002) | replace() −94% |
| H19 | feel-evaluator | Builtin-function fallback memoized lazily per name evaluator (must stay lazy: eager build-time resolution regressed 3× in both the 0.2 and 0.3 cycles) | sub-floor per call; guards the uncached path |
| H20 | feel-evaluator | Filter expressions: per-element context reuse, no wasted probe. **Ships only with H3** — each alone regresses filters, together −33% (measured interaction) | filter ×100 −33% (with H3) |
| H9 | feel-number | Number formatting via a 64-byte stack buffer bound directly to the Intel library call, replacing a 1 KiB zeroed heap path; output byte-identical | number→string −16% |
| DEPS-001 | feel-evaluator | Off-by-default `external-functions` cargo feature gates the Java/PMML evaluators and their reqwest/rustls/aws-lc dependency tree; disabled builds answer external invocations with an explained null | removes the HTTP/TLS stack from the extension |
| DEPS-002 | feel-number | `Display` is total: ±Inf/NaN print the library's textual form instead of panicking on a missing exponent | closes BUG-004's root cause |
| lint | several | Minimal edits keeping `make lint -D warnings` green under newer clippy (vendored path deps escape cap-lints); includes one pristine-0.3 site rewritten with `?` | none (non-functional) |
| H22 | feel-parser | Caps FEEL expression nesting depth at 500. Parsing itself is LALR/table-driven (no native-stack recursion regardless of input nesting), but the *AstNode tree* it builds is a Box-linked recursive structure that every downstream consumer (Debug/Clone/derived Drop, the feel-evaluator's evaluator builder and every invocation of the built evaluator, this crate's own ClosureBuilder, pgdmn's guard.rs) walks with ordinary native recursion — deep enough input drives a native stack overflow there, which aborts the whole Postgres backend process. A depth counter threaded through every AST-constructing reduce action rejects the parse before such a tree is ever built; left-recursive constructs (chained `+`, `.`, comparisons, ...) are capped identically to explicit nesting since they build an equally deep left-nested AST despite the shift-reduce parse stack staying flat. Flat accumulators (context/list/parameter entries) track depth as the deepest element, not the count, so wide-but-shallow expressions are unaffected | closes a confirmed remote stack-overflow DoS: any role with EXECUTE on `feel_eval`/`dmn_eval` (granted to PUBLIC by default) could crash the backend with a deeply nested expression |
| H23 | feel, feel-evaluator | `now()`/`today()` removed from the `Bif` name→implementation dispatch (dsntk-feel `Bif` enum, `FromStr`, `is_built_in_function_name`); both wall-clock builtins are now unreachable as FEEL function calls, falling through to the same "unresolved function" explained null as any unknown name | closes an `IMMUTABLE` contract violation — no pgdmn-side guard needed |

Upstream status: tracked per patch in TODO.md (PERF-006 / UPSTREAM-001).
BUG-003, the DEPS-001 gate, and DEPS-002 are the lead candidates; H14
implements an upstream TODO; H20+H3 must be proposed as a pair.
