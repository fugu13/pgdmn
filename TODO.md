# TODO

## Testing

- [ ] Write property-based tests for DMN round trips (parse -> serialize -> parse should be identity)

## Features

### FEAT-001: `dmn_create_input_type` helper

A convenience function that inspects a DMN model's input requirements for a given invocable and creates a matching PostgreSQL composite type automatically. This would eliminate the manual `CREATE TYPE` step when using `dmn_record_eval`.

Example usage:
```
SELECT dmn_create_input_type(dmn_load('<xml>'), 'Eligibility', 'eligibility_input');
-- Creates: CREATE TYPE eligibility_input AS ("Age" numeric, "Income" numeric)
```
