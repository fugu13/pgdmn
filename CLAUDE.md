# pgdmn

PostgreSQL extension that brings DMN (Decision Model and Notation) support to Postgres. Built with Rust, pgrx, and dsntk.

## Stack

- **Rust 2024 edition**, pgrx 0.16, dsntk 0.2
- **Target:** PostgreSQL 17

## Project Structure

```
src/
  lib.rs            — pg_module_magic, integration tests
  cache.rs          — thread-local ModelEvaluator cache (keyed by XML hash)
  convert.rs        — FEEL value ↔ PG type conversions
  types/
    dmn_model.rs    — custom DmnModel PG type (InOutFuncs: XML in, namespace::name out)
  functions/
    feel.rs         — feel_eval (JSONB) + 6 typed variants (numeric, bool, text, date, timestamp, interval)
    dmn.rs          — dmn_load, dmn_eval
    introspection.rs — dmn_invocables, dmn_info, dmn_xml, dmn_name, dmn_namespace
```

## Build & Test

All builds run in Docker. Two images:

- `pgdmn-base` — PG17 + pgrx toolchain
- `pgdmn-test` — adds non-root user (required by initdb)

Build the test image if it doesn't exist:
```sh
docker build -t pgdmn-base .
docker build -t pgdmn-test --build-arg BASE=pgdmn-base -<<'EOF'
FROM pgdmn-base
RUN useradd -ms /bin/bash pgdmn
USER pgdmn
EOF
```

Run tests:
```sh
docker run --rm -e USER=pgdmn -v "$(pwd)":/pgdmn -w /pgdmn pgdmn-test cargo pgrx test pg17
```

Check compilation:
```sh
docker run --rm -e USER=pgdmn -v "$(pwd)":/pgdmn -w /pgdmn pgdmn-test cargo check
```

## Key Conventions

- No LTO in dev profile (causes ICE on Rust 1.85/aarch64)
- `dsntk_model::NamedElement` and `DmnElement` traits must be imported for `.name()` / `.namespace()`
- `parse_expression(scope, expr, trace)` takes 3 args
- `pgrx::datum::Interval::new(months, days, micros)` — months first
