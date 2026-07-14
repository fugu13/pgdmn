# Evaluation Performance

How pgdmn keeps per-row DMN/FEEL evaluation fast: what is amortized, what each
row still pays, how performance is measured, and which engine-level
optimizations the vendored dsntk carries. Detailed numbers, charts, and the
scope-vs-speed breakdown live in the session performance report; this document
describes the durable architecture.

## What a user can rely on

- Repeated `dmn_eval`/`dmn_record_eval` calls with the same model pay model
  parsing and compilation once per PostgreSQL backend, not per row.
- Repeated `feel_eval` (and typed variants) with the same expression pay FEEL
  parsing and evaluator construction once per backend per context *shape*,
  not per row — values may change freely between rows.
- Per-row cost is dominated by actual decision logic, not by re-parsing;
  moderately complex decision tables evaluate in single-digit microseconds
  per row on current hardware, comparable to hand-written jsonb-extraction
  SQL for the same logic.

## Amortization architecture

| Cache | Key | Invalidation | Bound |
|---|---|---|---|
| DMN evaluator (per backend thread) | 128-bit content hash of the model XML (computed once at `dmn_load`) plus XML length | never (content-addressed) | unbounded; one entry per distinct model |
| FEEL prepared evaluator (per backend thread) | full expression text plus a 128-bit digest of the context *shape* | never (content-addressed) | 1024 entries, then cleared wholesale |

The context-shape digest exists because FEEL name tokenization depends on
which names are in scope (multi-word names like `monthly salary`): the same
expression can parse differently under different key sets. The digest covers
entry names, nested-context structure, and whether lists contain contexts —
never leaf values — mirroring exactly what the FEEL parser derives from a
scope. This mirror is an accepted tradeoff (documented and pinned by unit
tests) so the vendored parser needs no API change; re-verify it if dsntk is
upgraded.

Collision safety in both caches rests on 128-bit double-seeded hashing
(probability of any false hit ~2⁻¹²⁸), so hot-path probes never re-hash or
compare the underlying XML/expression bytes.

## Per-row costs that remain

Sequence for one `dmn_eval` row: model datum deserialization (scales with
model XML size — see PERF-001 in TODO.md), cache probe, JSONB-to-FEEL context
conversion (integer fast paths, by-value inserts), invocable evaluation in the
engine, FEEL-to-JSONB result conversion. For `feel_eval`: context conversion,
shape digest, cache probe, evaluation, result conversion.

## Vendored engine optimizations

The dsntk crates under `vendor/` carry a deliberately minimal, separable
patch set (one commit per change, `PGDMN:` markers at every site; see
`vendor/README.md`). The main ones:

| Area | Change |
|---|---|
| Decision tables | Hit-policy-aware evaluation: input entries short-circuit per rule, output entries evaluate only for matching rules, FIRST stops at the first match |
| Decision tables | Input expressions evaluate once per call (bound to `?`), not once per rule×column; `?`-referencing entries evaluate per the DMN spec (BUG-003) |
| Model evaluation | Fewer per-call context/value clones; allocation-free invocable lookup; single-dispatch BKM invocation |
| FEEL contexts | Copy-on-write storage behind an atomic reference count: cloning a context (every scope lookup) is O(1) |
| FEEL iteration | For-expressions accumulate in place (previously quadratic in list length); some/every stop once decided |
| Regex built-ins | replace()/split() reuse compiled regexes via a small per-thread cache |
| Number formatting | Stack-buffer rendering instead of a 1 KiB zeroed heap buffer per to-string |

Behavioral deviations from pristine upstream are limited to: never-consumed
FEEL sub-expressions are no longer evaluated (hit-policy short-circuiting,
quantifier early-exit — observable only through side-effecting external
functions), one diagnostic null message no longer embeds the entire input
context, and the BUG-003 spec-alignment fix.

## Measurement infrastructure

- `make bench` — SQL-level per-row benchmark suite (PGDMN_BENCH-gated test):
  trivial and complex models against plain-SQL and jsonb-extraction baselines,
  FEEL per-row scenarios, and an 85 KB model that exposes size-proportional
  per-row costs. Results land in `benchmark_results.txt`; the pure-PostgreSQL
  control queries double as cross-run comparability canaries.
- `profiling/` — host-native benchmark and profiling harness over the vendored
  engine (no PostgreSQL): engine, conversion, parser, and construct-specific
  scenarios (iteration, filters, regex), with JSON output and a hot-loop mode
  for sampling profilers. It includes the extension's conversion code as
  shared source, so its numbers always describe shipping code.
- Cargo profile overrides build every measured package at full optimization
  even under the dev-profile test runner, so benchmark numbers reflect
  release-grade code.
- Methodology on shared machines: gate measurement windows on an
  untouched-code canary micro-benchmark and treat sub-microsecond deltas
  across separate builds as noise below roughly eight percent (code-layout
  effects). Measure candidate removals at the final tip, not only in
  isolation — optimizations interact.

## Deferred work

PERF-001 (zero-copy model datum), PERF-002 (regex cache for matches()),
PERF-003 (Arc payloads for lists/strings/functions), PERF-004 (decision-service
lock), PERF-005 (cross-backend cache), PERF-006 (upstreaming the patch set) —
see TODO.md for details.
