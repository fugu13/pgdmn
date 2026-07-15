---
title: The obligation is a column
date: 2026-07-10
summary: Data-handling rules as a decision table, evaluated per row. What must we encrypt, what may not leave the EU — answered by a query rather than by a policy document nobody has read since the audit.
files: compliance.dmn, customers.csv
example: compliance
---

What you are obliged to do with a row of customer data — where it may live, how long you may keep it, whether it must be encrypted — is usually written in a document nobody has opened since the audit, while the code that actually enforces it lives somewhere else and drifts away from it over time.

This example expresses those data-handling obligations as a DMN decision table and computes the obligation for every customer as a column of a view, so the enforced rule and the written rule are the same artifact. It covers region and data-class handling under a first-hit policy; it is a worked illustration, not a complete regulatory model, and the specific rules here are invented for the example.

## The rules

Table: Handling — hit policy: F (first)

| F | Region | Data Class | Handling |
| --- | --- | --- | --- |
| 1 | — | `"special"` | encrypt, restrict access, retain 6 months |
| 2 | `"EU"` | `"personal"` | store in EU, retain 24 months |
| 3 | — | `"personal"` | retain 24 months |
| 4 | — | — | standard handling |

Read top to bottom, because the hit policy is *first* and the order carries meaning.

Special-category data is caught by the first rule **regardless of region** — a stricter obligation wins before anything more specific gets a look in. Only then does region matter, and only for personal data: EU personal data has to stay in the EU, personal data elsewhere merely has a retention clock. Everything else is ordinary.

That ordering is the policy. Move the special-category rule below the EU rule and you have quietly decided that an EU customer's special-category data is governed by residency rather than by sensitivity — which is very likely not what your legal team said.

## Set up

Load the model, [`compliance.dmn`](/examples/compliance.dmn), and the customers from [`customers.csv`](/examples/customers.csv). Run this from the directory the two files are in; you will need pgdmn installed first (see [Install](/docs/#install)).

```sql
\set compliance `cat compliance.dmn`

CREATE TABLE models (name text PRIMARY KEY, model dmnmodel NOT NULL);
INSERT INTO models VALUES ('compliance', dmn_load(:'compliance'));

CREATE TABLE customers (
    id         int PRIMARY KEY,
    name       text,
    region     text,
    data_class text
);
\copy customers FROM 'customers.csv' WITH (FORMAT csv, HEADER true)
```

The obligation for every row becomes a column, computed from the rules in force whenever anyone looks — so make it a view:

```sql
CREATE VIEW obligations AS
SELECT c.id, c.name, c.region, c.data_class,
    dmn_eval_text(m.model, 'Handling', jsonb_build_object(
        'Region',     c.region,
        'Data Class', c.data_class
    )) AS handling
FROM customers c
CROSS JOIN models m
WHERE m.name = 'compliance';

SELECT name, region, data_class, handling FROM obligations ORDER BY id;
```

## Every customer

Table: What we owe each customer

| Customer | Region | Class | Handling |
| --- | --- | --- | --- |
| Northwind Traders | EU | personal | store in EU, retain 24 months |
| Globex | US | personal | retain 24 months |
| Initech | US | special | encrypt, restrict access, retain 6 months |
| Umbrella Corp | EU | special | encrypt, restrict access, retain 6 months |
| Acme Supply | UK | public | standard handling |
| Soylent Industries | EU | public | standard handling |
| Tyrell Corp | US | public | standard handling |
| Wayne Enterprises | UK | personal | retain 24 months |

Umbrella is the row worth pausing on. They are an EU customer with special-category data — and the answer is *not* "store in EU". The special-category rule is listed first, and it wins. Whether that is right is a question for a lawyer, and the point is that the question is now askable: the rule is four lines you can put in front of one.

## Why this belongs in the database

The obligation is a `view` over the customers table. It is not a report that was true last Tuesday; it is computed from the current rules and the current rows, every time anyone looks.

That changes the sort of question you can answer. *Who must we encrypt?* is a `WHERE` clause on the view:

```sql
SELECT handling, count(*), string_agg(name, ', ' ORDER BY id) AS who
FROM obligations
WHERE handling <> 'standard handling'
GROUP BY handling
ORDER BY count(*) DESC, handling;

--            handling            | count |          who
-- -------------------------------+-------+------------------------
--  encrypt, restrict access, ... |     2 | Initech, Umbrella Corp
--  retain 24 months              |     2 | Globex, Wayne Enterp...
--  store in EU, retain 24 months |     1 | Northwind Traders
```

*What data may not leave the EU?* is another:

```sql
SELECT name, region FROM obligations
WHERE handling LIKE 'store in EU%'
ORDER BY id;

--        name        | region
-- -------------------+--------
--  Northwind Traders | EU
```

*What breaks if we tighten the retention rule?* is the same query against a second model row, diffed against the first. *What were we obliged to do on the day of the breach?* is answerable too, if you version the model rows — because the old rules are still just values in a table.

None of that requires exporting a single customer record to anywhere.
