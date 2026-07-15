---
title: The obligation is a column
date: 2026-07-10
summary: Data-handling rules as a decision table, evaluated per row. What must we encrypt, what may not leave the EU — answered by a query rather than by a policy document nobody has read since the audit.
example: compliance
---

Every row of customer data carries an obligation. Where it may live, how long you may keep it, whether it must be encrypted. Usually that obligation lives in a PDF, and the code that enforces it lives somewhere else, and the two agree for about a quarter.

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

That changes the sort of question you can answer. *Who must we encrypt?* is a `WHERE` clause. *What breaks if we tighten the retention rule?* is the same query against a second model row, diffed against the first. *What were we obliged to do on the day of the breach?* is answerable too, if you version the model rows — because the old rules are still just values in a table.

None of that requires exporting a single customer record to anywhere.
