---
title: Loan eligibility, or, only define complex rules once
date: 2026-07-13
summary: Loan eligibility criteria are often very complex and strictly validated. In this simplified example, see how DMN expressing an exact set of rules that would be a bit tricky in SQL encapsulate that complexity, avoiding double implementation.
files: loan-eligibility.dmn, applicants.csv
example: loan
---

A lender has to turn a handful of applicant facts into a single approve-or-decline, and the rules that do it are exactly the sort that read opaquely and break quietly when they live in application code. Additionally, the details of the approval rules are updated frequently.

This example encodes an eligibility policy as a DMN decision table and evaluates it across a table of applicants in SQL.

## The rules

A decision table lists inputs and outputs as headers and rules as rows. This one takes three inputs — how old is the applicant, what do they earn, and have they been bankrupt — and outputs loan eligibility.

Table: Eligibility — hit policy: F (first)

| F   | Age     | Income     | Bankrupt | Eligibility              |
| --- | ------- | ---------- | -------- | ------------------------ |
| 1   | `< 18`  | —          | —        | Denied: underage         |
| 2   | —       | —          | `true`   | Denied: prior bankruptcy |
| 3   | `>= 18` | `>= 50000` | `false`  | Approved                 |
| 4   | `>= 18` | `< 50000`  | `false`  | Denied: low income       |

The dash is a wildcard: the first rule does not care what you earn, and the second does not care how old you are or how much you make.

The hit policy is **first**, which means the rules are tried top to bottom and the first one that matches wins. DMN supports several different hit policies, suitable for different decision scenarios.

## Set up

Load the model, [`loan-eligibility.dmn`](/examples/loan-eligibility.dmn), into a table, and load the applicants from [`applicants.csv`](/examples/applicants.csv) alongside it. The model is an ordinary value, so it lives in a column like anything else. Run this from the directory the two files are in; you will need pgdmn installed first (see [Install](/docs/#install)).

```sql
-- The model, read from the downloaded file into a psql variable.
\set loan `cat loan-eligibility.dmn`

CREATE TABLE models (name text PRIMARY KEY, model dmnmodel NOT NULL);
INSERT INTO models VALUES ('loan', dmn_load(:'loan'));

CREATE TABLE applicants (
  id       int PRIMARY KEY,
  name     text,
  age      int,
  income   numeric,
  bankrupt boolean
);
\copy applicants FROM 'applicants.csv' WITH (FORMAT csv, HEADER true)
```

## One applicant

A short query determines eligibility. Pass the DMN model, the decision to return, and a JSON object containing the inputs, and get an answer back — `dmn_eval_text` unwraps the result, so there are no JSONB quotes to strip.

```sql
SELECT dmn_eval_text(model, 'Eligibility', '{
    "Age": 34, "Income": 82000, "Bankrupt": false
  }'::jsonb) AS decision
FROM models WHERE name = 'loan';

--  decision
-- ----------
--  Approved
```

## Every applicant

The same decision, evaluated against every row of the table. This is the whole point: no export, no service call, no loop in application code — one call per row, and Postgres is free to run it across parallel workers.

```sql
SELECT a.name, a.age, a.income, a.bankrupt,
  dmn_eval_text(m.model, 'Eligibility', jsonb_build_object(
    'Age',      a.age,
    'Income',   a.income,
    'Bankrupt', a.bankrupt
  )) AS decision
FROM applicants a
CROSS JOIN models m
WHERE m.name = 'loan'
ORDER BY a.id;
```

Table: Every applicant, decided

| Name          | Age | Income | Bankrupt | Decision                 |
| ------------- | --- | ------ | -------- | ------------------------ |
| Ada Okafor    | 34  | 82000  | false    | Approved                 |
| Bo Zhang      | 17  | 0      | false    | Denied: underage         |
| Chen Ruiz     | 29  | 41000  | false    | Denied: low income       |
| Dara Singh    | 45  | 120000 | true     | Denied: prior bankruptcy |
| Eli Novak     | 22  | 50000  | false    | Approved                 |
| Fay Mbeki     | 19  | 49999  | false    | Denied: low income       |
| Gus Halvorsen | 64  | 68000  | false    | Approved                 |
| Hana Ito      | 17  | 95000  | false    | Denied: underage         |

A brief explanation follows for how some of the rows evaluated against the DMN model.

### Eli and Fay: the boundary

Eli earns exactly 50000 and is approved. Fay earns 49999 and is not. The rule says `>= 50000`, so the boundary falls between them — and a boundary is the single most common place for a rule to be wrong.

Written as a table, it is one line to check. Buried in application code as `income > 50000` it is a bug nobody notices until Eli complains.

### Dara: one boolean outranks every number

Dara earns 120000 — more than anyone else in the book — and is declined. The bankruptcy rule sits above both income rules, so the moment that boolean is true, nothing about the money matters.

This is the shape most real policies have: a handful of disqualifiers that short-circuit everything, and then the interesting logic underneath. Expressed as a table it is obvious in one glance which is which.

### Hana: why order is the decision

Hana earns 95000 and is turned away. She is 17. The underage rule sits first, and under a _first_ hit policy the engine stops there; the income rule below never runs.

With a decision table, verifying this constraint only involves looking at the underage rule, seeing it is the first in the table, and looking at the hit policy. In code, the details are easily buried.

## So how did we do?

Because the decision is just an expression, the outcome is groupable, aggregatable, joinable — anything SQL can do to a column, it can do to a decision.

```sql
SELECT dmn_eval_text(m.model, 'Eligibility', jsonb_build_object(
    'Age', a.age, 'Income', a.income, 'Bankrupt', a.bankrupt
  )) AS decision,
  count(*)
FROM applicants a
CROSS JOIN models m
WHERE m.name = 'loan'
GROUP BY 1
ORDER BY count(*) DESC, 1;
```

Table: The book of applications, by outcome

| Decision                 | Applicants |
| ------------------------ | ---------- |
| Approved                 | 3          |
| Denied: low income       | 2          |
| Denied: underage         | 2          |
| Denied: prior bankruptcy | 1          |

## Going further

Because the decision is just a column, everything SQL already does to a column works on it. A `boolean` variant reads even more directly in a filter — here, the approval rate as a single number:

```sql
SELECT round(
    100.0 * count(*) FILTER (
      WHERE dmn_eval_text(m.model, 'Eligibility', jsonb_build_object(
        'Age', a.age, 'Income', a.income, 'Bankrupt', a.bankrupt
      )) = 'Approved'
    ) / count(*), 1) AS approval_pct
FROM applicants a
CROSS JOIN models m
WHERE m.name = 'loan';

--  approval_pct
-- --------------
--          37.5
```

Or pull just the declines, with the reason the model gave, straight into a work queue:

```sql
SELECT a.name,
  dmn_eval_text(m.model, 'Eligibility', jsonb_build_object(
    'Age', a.age, 'Income', a.income, 'Bankrupt', a.bankrupt
  )) AS reason
FROM applicants a
CROSS JOIN models m
WHERE m.name = 'loan'
 AND dmn_eval_text(m.model, 'Eligibility', jsonb_build_object(
     'Age', a.age, 'Income', a.income, 'Bankrupt', a.bankrupt
   )) <> 'Approved'
ORDER BY a.id;

--       name      |          reason
-- ---------------+--------------------------
--  Bo Zhang      | Denied: underage
--  Chen Ruiz     | Denied: low income
--  Dara Singh    | Denied: prior bankruptcy
--  Fay Mbeki     | Denied: low income
--  Hana Ito      | Denied: underage
```

When the lending policy changes, you load a new model under the same name and every one of these queries reports against the new rules — no redeploy, and the old model is still a value you can keep for the audit.
