# Security Policy

## Reporting a vulnerability

Please report security issues privately, not in a public issue or pull request.

Use GitHub's **[private vulnerability reporting](https://github.com/fugu13/pgdmn/security/advisories/new)** ("Report a vulnerability" under the repository's Security tab). This keeps the report confidential until a fix is available.

Please include enough to reproduce it: a minimal DMN model or FEEL expression, the SQL that triggers it, and what you observed versus expected. You will get an acknowledgement, and we will keep you informed as it is addressed.

## Supported versions

pgdmn is pre-1.0. Fixes land on the latest release; there are no backports yet.

## What pgdmn does with untrusted input

pgdmn evaluates decisions inside your database, so its trust boundary is worth stating plainly.

- **`dmn_load` parses caller-supplied XML.** External entities are **not** resolved — a `SYSTEM` entity pointing at a file does not read that file — so DMN XML is not an XXE vector. This is asserted by a test (`test_dmn_load_does_not_resolve_external_entities`) so it stays true.
- **`feel_eval` and `dmn_eval` evaluate FEEL expressions and decisions.** FEEL is a decision language, not a general-purpose one: it has no filesystem, network, or shell access, and cannot call out of the evaluator.
- **The functions are `IMMUTABLE` and `PARALLEL SAFE`** and touch no external state.

The practical guidance: a DMN model and a FEEL expression are code. Treat a model from an untrusted source the way you would treat any SQL you did not write — review it before running it — and grant `EXECUTE` on these functions no more widely than you would any other.
