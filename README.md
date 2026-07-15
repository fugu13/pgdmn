# pgdmn

PostgreSQL extension that brings DMN (Decision Model and Notation) support to Postgres. Built with Rust, pgrx, and dsntk.

[![CI](https://github.com/fugu13/pgdmn/actions/workflows/ci.yml/badge.svg)](https://github.com/fugu13/pgdmn/actions/workflows/ci.yml) [![Website](https://github.com/fugu13/pgdmn/actions/workflows/website.yml/badge.svg)](https://github.com/fugu13/pgdmn/actions/workflows/website.yml)

## Quick Start

```sql
-- Install the extension
CREATE EXTENSION pgdmn;

-- Evaluate a FEEL expression directly
SELECT feel_eval('1 + 2');
-- Returns: 3

-- Load a DMN model and evaluate a decision
SELECT dmn_eval(
  dmn_load('<your DMN XML here>'),
  'Decision Name',
  '{"input": "value"}'::jsonb
);
```

## DMN Functions

### dmn_load(xml) -> dmnmodel

Parse DMN XML into a `dmnmodel` value. This is the entry point for all DMN operations.

```sql
SELECT dmn_load('<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             namespace="https://example.com/greeting"
             name="greeting-model">
  <decision id="greeting" name="Greeting">
    <literalExpression>
      <text>"Hello, " + name</text>
    </literalExpression>
  </decision>
</definitions>');
-- https://example.com/greeting::greeting-model
```

The display format is `namespace::name`. The model can be stored in a column for reuse:

```sql
CREATE TABLE models (model dmnmodel);
INSERT INTO models VALUES (dmn_load('<your DMN XML>'));
```

### dmn_eval(model, invocable, input?) -> jsonb

Evaluate a named invocable (decision, BKM, or decision service) from a DMN model.

```sql
-- Simple decision with no inputs
SELECT dmn_eval(dmn_load('...'), 'Greeting');

-- Decision table with JSONB inputs
SELECT dmn_eval(
  dmn_load('...'),
  'Eligibility',
  '{"Age": 30, "Income": 75000}'::jsonb
);
-- "Approved"

-- Chained decisions (dependencies resolved automatically)
SELECT dmn_eval(
  dmn_load('...'),
  'Total Price',
  '{"Base Price": 100, "Tax Rate": 0.1}'::jsonb
);
-- 110
```

### dmn_record_eval(model, invocable, input?) -> jsonb

Same as `dmn_eval`, but accepts a composite-type record instead of JSONB for the input.

```sql
CREATE TYPE loan_input AS ("Age" int, "Income" numeric);

SELECT dmn_record_eval(
  dmn_load('...'),
  'Eligibility',
  ROW(30, 75000)::loan_input
);
-- "Approved"
```

## Introspection Functions

### dmn_invocables(model) -> setof (name text, kind text)

List all invocable elements in a DMN model as a table.

```sql
SELECT * FROM dmn_invocables(dmn_load('...'));
--     name     |   kind
-- -------------+----------
--  Tax Amount  | decision
--  Total Price | decision
```

### dmn_info(model) -> jsonb

Return model metadata as JSONB, including counts of each element type and invocable names.

```sql
SELECT dmn_info(dmn_load('...'));
-- {"name": "greeting-model", "namespace": "https://example.com/greeting",
--  "decisions": 1, "business_knowledge_models": 0,
--  "decision_services": 0, "invocables": ["Greeting"]}
```

### dmn_xml(model) -> text

Extract the raw XML source from a DMN model.

```sql
SELECT dmn_xml(dmn_load('<definitions ...>...</definitions>'));
-- Returns the original XML
```

### dmn_name(model) -> text

Get the model name.

```sql
SELECT dmn_name(dmn_load('...'));
-- greeting-model
```

### dmn_namespace(model) -> text

Get the model namespace.

```sql
SELECT dmn_namespace(dmn_load('...'));
-- https://example.com/greeting
```

## FEEL Functions

### feel_eval(expression, context?) -> jsonb

Evaluate a FEEL expression. Optionally pass a JSONB context for variable bindings.

```sql
-- Simple arithmetic
SELECT feel_eval('1 + 2');
-- 3

-- With context variables
SELECT feel_eval('x * 2', '{"x": 21}'::jsonb);
-- 42

-- List comprehension
SELECT feel_eval('for i in [1,2,3] return i * i');
-- [1, 4, 9]

-- String operations
SELECT feel_eval('"Hello " + name', '{"name": "World"}'::jsonb);
-- "Hello World"
```

### feel_record_eval(expression, context?) -> jsonb

Same as `feel_eval`, but accepts a composite-type record instead of JSONB for the context. Columns map directly to FEEL variables.

```sql
CREATE TYPE calc_input AS (x numeric, y numeric);

SELECT feel_record_eval('x + y', ROW(3, 4)::calc_input);
-- 7
```

### feel_eval_numeric(expression, context?) -> numeric

Evaluate a FEEL expression that returns a number. Returns a native PG `numeric`.

```sql
SELECT feel_eval_numeric('x * 2', '{"x": 21}'::jsonb);
-- 42
```

### feel_eval_bool(expression, context?) -> bool

Evaluate a FEEL expression that returns a boolean.

```sql
SELECT feel_eval_bool('5 > 3');
-- true

SELECT feel_eval_bool('x > 100', '{"x": 50}'::jsonb);
-- false
```

### feel_eval_text(expression, context?) -> text

Evaluate a FEEL expression that returns a string.

```sql
SELECT feel_eval_text('"Hello " + name', '{"name": "World"}'::jsonb);
-- Hello World
```

### feel_eval_date(expression, context?) -> date

Evaluate a FEEL expression that returns a date.

```sql
SELECT feel_eval_date('date("2024-03-15")');
-- 2024-03-15
```

### feel_eval_timestamp(expression, context?) -> timestamp

Evaluate a FEEL expression that returns a date-time.

```sql
SELECT feel_eval_timestamp('date and time("2024-03-15T10:30:00")');
-- 2024-03-15 10:30:00
```

### feel_eval_interval(expression, context?) -> interval

Evaluate a FEEL expression that returns a duration.

```sql
SELECT feel_eval_interval('duration("P2Y3M")');
-- 2 years 3 mons

SELECT feel_eval_interval('duration("PT4H30M")');
-- 04:30:00
```

### feel_eval_numrange(expression, context?) -> numrange

Evaluate a FEEL expression that returns a range of numbers, as a native PG
`numrange`. Open/closed endpoints are preserved, and an unbounded FEEL endpoint
becomes an infinite range bound.

```sql
SELECT feel_eval_numrange('range("[18..65)")');
-- [18,65)

SELECT feel_eval_numrange('range("[1..)")');
-- [1,)

-- Compose with native PG range operators
SELECT feel_eval_numrange('[low..high)', '{"low": 18, "high": 65}'::jsonb) @> 42::numeric;
-- true
```

### FEEL language notes

- Ranges returned through the JSONB functions (`feel_eval`, `dmn_eval`, …)
  appear as FEEL-syntax strings, e.g. `"[18..65)"`; an unbounded end renders
  with nothing after the `..` (`"[1..)"`). Use `feel_eval_numrange` for a
  structured value.
- `in` works with unary-test lists (`5 in (=4, =5)`), variables may share names
  with temporal builtins (a context key or column named `date` or `duration` is
  fine), and multi-word range properties parse (`[1..10].end included`).
- Malformed `\u` escapes in FEEL string literals are parse errors.
- FEEL `external` function definitions (Java/PMML) are not supported: the
  machinery that would evaluate them (a blocking HTTP client) is compiled out
  of the extension entirely, so any external invocation yields an explained
  null — and definitions detectable at load time are rejected with a clear
  error before evaluation.

## Vendored dsntk engine

pgdmn vendors the [dsntk](https://github.com/DecisionToolkit/dsntk) DMN/FEEL
engine (by Dariusz Depta / Engos Software) under `vendor/`: verbatim,
checksum-verified crates.io releases plus a minimal, deliberately upstreamable
patch layer (performance work, one bug fix, and an off-by-default feature gate
that keeps an HTTP/TLS stack out of the PostgreSQL backend). Each change is
its own commit with `PGDMN:` markers. See [vendor/README.md](vendor/README.md)
for provenance and conventions, and [vendor/PATCHES.md](vendor/PATCHES.md) for
what each patch does; `make vendor-status` shows the carried layer at any time.

## Build & Test

All builds run in Docker and go through `make`. See [CLAUDE.md](CLAUDE.md) for the full target list and conventions.

```sh
make test-image   # build the Docker image (first time, and after dependency changes)
make check        # fast compilation check
make test         # run the pgrx test suite against PG17
make verify       # fmt + lint + check
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Documentation

- [CLAUDE.md](CLAUDE.md) - Development conventions, build targets, and architecture decisions
- [TODO.md](TODO.md) - Tracked work items
- [BUGHISTORY.md](BUGHISTORY.md) - Resolved bugs with reoccurrence checklists
- [RELEASEPLAN.md](RELEASEPLAN.md) - Release, promotion, and go-to-market plan
- [docs/improvements.md](docs/improvements.md) - Investigation of approaches to bypass JSONB for more efficient PG-to-DMN data passing
- [vendor/README.md](vendor/README.md) - Provenance and conventions for the vendored dsntk engine
- [docs/specifications/](docs/specifications/) - Specifications describing what a feature must do, written before implementation
- [docs/ux/](docs/ux/) - Behavioral descriptions of the website's UI
- [website/](website/) - Marketing and documentation site (Leptos, prerendered to static HTML; `make website-build`, deployed to GitHub Pages at www.pgdmn.com on push to `main`)

### Third-party content

- `vendor/` — the dsntk engine, copyright Dariusz Depta / Engos Software,
  licensed MIT OR Apache-2.0 at your option; the upstream license texts and
  NOTICE are included in that directory. The sources carry local
  modifications, each marked with a `PGDMN:` comment.
- `examples/` — DMN example models from the DMN TCK (DMN TCK Contributors)
  and Camunda Services GmbH, licensed Apache-2.0 (see file headers).

pgdmn's own license applies to everything else in the repository.
