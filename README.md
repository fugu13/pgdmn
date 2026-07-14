# pgdmn

PostgreSQL extension that brings DMN (Decision Model and Notation) support to Postgres. Built with Rust, pgrx, and dsntk.

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

### Typed variants

`dmn_eval` returns JSONB, so a decision that produces the string `Approved` comes back as `"Approved"` — quoted. Unwrapping that by hand means `dmn_eval(...) #>> '{}'`, and a numeric decision means `(dmn_eval(...) #>> '{}')::numeric`.

The typed variants take the same arguments and return a native PostgreSQL type instead. Each raises an error if the decision returns something else — asking for a number and getting a string is a mistake worth hearing about.

#### dmn_eval_text(model, invocable, input?) -> text

```sql
SELECT dmn_eval_text(
  dmn_load('...'),
  'Eligibility',
  '{"Age": 34, "Income": 82000, "Bankrupt": false}'::jsonb
);
-- Approved
```

#### dmn_eval_numeric(model, invocable, input?) -> numeric

Drops straight into arithmetic, with no unwrap and no cast.

```sql
SELECT round(dmn_eval_numeric(
  dmn_load('...'),
  'Total Price',
  '{"Base Price": 2499.99, "Tax Rate": 0.0825}'::jsonb
), 2);
-- 2706.24
```

#### dmn_eval_bool(model, invocable, input?) -> boolean

Usable directly in a `WHERE` clause or a `CHECK` constraint.

```sql
SELECT dmn_eval_bool(dmn_load('...'), 'Eligible', '{"Age": 30}'::jsonb);
-- true
```

#### dmn_eval_date(model, invocable, input?) -> date

```sql
SELECT dmn_eval_date(dmn_load('...'), 'Due Date');
-- 2024-03-15
```

#### dmn_eval_timestamp(model, invocable, input?) -> timestamp

```sql
SELECT dmn_eval_timestamp(dmn_load('...'), 'Effective From');
-- 2024-03-15 10:30:00
```

#### dmn_eval_interval(model, invocable, input?) -> interval

Both FEEL durations convert: years and months, and days and time.

```sql
SELECT dmn_eval_interval(dmn_load('...'), 'Term');
-- 2 years 3 mons
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

## Build & Test

All builds run in Docker and go through `make`. See [CLAUDE.md](CLAUDE.md) for the full target list and conventions.

```sh
make test-image   # build the Docker image (first time, and after dependency changes)
make check        # fast compilation check
make test         # run the pgrx test suite against PG17
make verify       # fmt + lint + check
```

## Security

pgdmn runs inside your database, so a few properties are worth stating plainly:

- **`dmn_load` parses caller-supplied XML, and does not resolve external entities** — DMN XML is not an XXE vector. A test asserts this so it stays true.
- **FEEL is a decision language, not a general-purpose one** — expressions and decisions have no filesystem, network, or shell access.
- **All evaluation functions are `IMMUTABLE` and `PARALLEL SAFE`** and touch no external state.

A DMN model and a FEEL expression are code: treat one from an untrusted source as you would any SQL you did not write. To report a vulnerability, see [SECURITY.md](SECURITY.md) — please do not open a public issue.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) - How to build, test, and submit changes
- [SECURITY.md](SECURITY.md) - Reporting a vulnerability, and the trust boundary
- [CLAUDE.md](CLAUDE.md) - Development conventions, build targets, and architecture decisions
- [TODO.md](TODO.md) - Tracked work items
- [BUGHISTORY.md](BUGHISTORY.md) - Resolved bugs with reoccurrence checklists
- [website/](website/) - The site: worked examples, a function reference, and walkthroughs in `website/posts/` (Leptos, prerendered to static HTML; `make website-build`, deployed to GitHub Pages at www.pgdmn.com on push to `main`)

There is no `docs/` directory: explanation aimed at users lives on the website, decisions live in CLAUDE.md, and findings and future work live in TODO.md.
