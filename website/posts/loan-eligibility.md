---
title: Loan eligibility, row by row
date: 2026-07-13
summary: Four rules and eight applicants. Two sit either side of the boundary, one is a 17-year-old earning 95000, and one out-earns the lot and is declined anyway — because the order of the rules is part of the decision.
example: loan
---

Four rules, eight applicants, and three of them show you why the order of the rules is part of the decision.

## The rules

A decision table takes inputs across the top and tries each rule in turn. This one asks three questions — how old is the applicant, what do they earn, and have they been bankrupt — and answers a fourth.

Table: The Eligibility decision table

| Age | Income | Bankrupt | Eligibility |
| --- | --- | --- | --- |
| < 18 | — | — | Denied: underage |
| — | — | true | Denied: prior bankruptcy |
| ≥ 18 | ≥ 50000 | false | Approved |
| ≥ 18 | < 50000 | false | Denied: low income |

The dash is a wildcard: the first rule does not care what you earn, and the second does not care how old you are or how much you make. The third input is a plain boolean, and it is treated exactly like the numbers — a column in, a rule matched.

The hit policy is **first**, which means the rules are tried top to bottom and the first one that matches wins. No rule further down gets a say. That is a design decision, not an implementation detail, and it is visible right here in the model.

## Every applicant

Table: Every applicant, decided

| Name | Age | Income | Bankrupt | Decision |
| --- | --- | --- | --- | --- |
| Ada Okafor | 34 | 82000 | false | Approved |
| Bo Zhang | 17 | 0 | false | Denied: underage |
| Chen Ruiz | 29 | 41000 | false | Denied: low income |
| Dara Singh | 45 | 120000 | true | Denied: prior bankruptcy |
| Eli Novak | 22 | 50000 | false | Approved |
| Fay Mbeki | 19 | 49999 | false | Denied: low income |
| Gus Halvorsen | 64 | 68000 | false | Approved |
| Hana Ito | 17 | 95000 | false | Denied: underage |

### Eli and Fay: the boundary

Eli earns exactly 50000 and is approved. Fay earns 49999 and is not. The rule says `≥ 50000`, so the boundary falls between them — and a boundary is the single most common place for a rule to be wrong.

Written as a table, it is one line to check. Buried in application code as `income > 50000` it is a bug nobody notices until Eli complains.

### Dara: one boolean outranks every number

Dara earns 120000 — more than anyone else in the book — and is declined. The bankruptcy rule sits above both income rules, so the moment that boolean is true, nothing about the money matters.

This is the shape most real policies have: a handful of disqualifiers that short-circuit everything, and then the interesting logic underneath. Expressed as a table it is obvious in one glance which is which.

### Hana: why order is the decision

Hana earns 95000 and is turned away. She is 17. The underage rule sits first, and under a *first* hit policy the engine stops there; the income rule below never runs.

Move the underage rule to the bottom and Hana gets approved. The rules would look identical in a summary, and the system would behave differently. This is exactly the kind of change you want a reviewer to see in a diff of the model, rather than discover in production.

## So how did we do?

Because the decision is just an expression, the outcome is groupable, aggregatable, joinable — anything SQL can do to a column, it can do to a decision.

Table: The book, by outcome

| Decision | Applicants |
| --- | --- |
| Approved | 3 |
| Denied: low income | 2 |
| Denied: underage | 2 |
| Denied: prior bankruptcy | 1 |

Nothing left the database to work that out.
