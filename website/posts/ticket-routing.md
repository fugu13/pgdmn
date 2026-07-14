---
title: Put the rules in the schema
date: 2026-07-11
summary: A decision model is a value. Store it in a column, wrap it in a view, and changing how every ticket is triaged becomes an UPDATE of one row — with the old rules still there to audit.
example: routing
---

A decision model is a value. Store it in a column, wrap it in a view, and triage stops being something your application does.

## The rules

Two string inputs — how urgent, and who is asking — and a queue to land in. Same *first* hit policy, with a catch-all at the bottom so nothing falls through the floor.

Table: The Queue decision table

| Priority | Customer Tier | Queue |
| --- | --- | --- |
| "critical" | — | pager |
| "high" | "enterprise" | pager |
| "high" | — | tier-2 |
| — | "enterprise" | tier-2 |
| — | — | tier-1 |

Read it as a policy and it is legible to someone who does not write SQL: wake somebody for anything critical; wake somebody for an enterprise customer with an urgent problem; everything else queues by severity, and enterprise jumps the line. That is a conversation you can have with the person who owns the support budget.

## Every ticket

Table: Every ticket, routed

| Subject | Priority | Tier | Queue |
| --- | --- | --- | --- |
| Cannot log in after SSO change | critical | startup | pager |
| Billing discrepancy on invoice 4471 | high | enterprise | pager |
| Feature request: dark mode | low | startup | tier-1 |
| API latency spike in eu-west | high | startup | tier-2 |
| Password reset not arriving | normal | enterprise | tier-2 |
| Typo in the docs | low | free | tier-1 |
| Data export failing silently | critical | enterprise | pager |
| Onboarding question | normal | free | tier-1 |

The startup with the login outage gets woken up for, and the enterprise customer with a billing question does too — but the enterprise password reset does not. The rules say so, in five lines, and you can hand those five lines to someone and ask whether they agree.

## The view is the point

The model sits in a column of a `models` table, and `routed_tickets` is a view over `tickets` that evaluates it per row. Everything downstream — dashboards, alerting, the on-call rota — reads the view.

When the policy changes, you `UPDATE` one row. No migration, no deploy, no redeploying the consumers, and no window during which half the system triages by the old rules and half by the new. The next query sees the new policy, because the policy *is* data.

And because the old model is still just a value, you can keep it. Version the rows, timestamp them, and you can answer the question every audit eventually asks: not *what would this ticket be routed to now*, but *what rule routed it that night, and who changed it*.
