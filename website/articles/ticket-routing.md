---
title: Clarity in support ticket routing rules
date: 2026-07-11
summary: Routing rules as a DMN decision table behind a view: every ticket is classified in one place, and a table generated from the same model embeds in the wiki, so the docs never drift from the rules that actually run.
files: ticket-routing.dmn, tickets.csv
example: routing
---

Support triage rules tend to live in people's heads or scattered through application code. They're also documented in the internal wiki, but that gradually goes stale and out of date.

This example puts the routing rules in the database as a DMN model behind a view, so every ticket is classified in one place. Also, a table automatically generated from the DMN can be embedded in the wiki, guaranteeing the rules match.

## The rules

Two string inputs—how urgent, and who is asking—and a queue to land in. Same *first* hit policy, with a catch-all at the bottom so nothing falls through the floor.

This is the model itself, drawn the way a DMN tool draws it: the hit policy in the corner, one numbered row per rule, inputs on the left and the output on the right. It is not a picture *of* the rules—it is the rules, and it is what runs.

Table: Queue—hit policy: F (first)

| F | Priority | Customer Tier | Queue |
| --- | --- | --- | --- |
| 1 | `"critical"` | — | pager |
| 2 | `"high"` | `"enterprise"` | pager |
| 3 | `"high"` | — | tier-2 |
| 4 | — | `"enterprise"` | tier-2 |
| 5 | — | — | tier-1 |

This is a standard DMN file—[open it in dmn-js →](/dmn-viewer.html?model=ticket-routing.dmn), or in any DMN tool.

Read it as a policy and it is legible to someone who does not write SQL: wake somebody for anything critical; wake somebody for an enterprise customer with an urgent problem; everything else queues by severity, and enterprise jumps the line. That is a conversation you can have with the person who owns the support budget.

## Set up

Load the model, [`ticket-routing.dmn`](/examples/ticket-routing.dmn), and the tickets from [`tickets.csv`](/examples/tickets.csv). Run this from the directory the two files are in; you will need pgdmn installed first (see [Install](/docs/#install)).

```sql
\set routing `cat ticket-routing.dmn`

CREATE TABLE models (name text PRIMARY KEY, model dmnmodel NOT NULL);
INSERT INTO models VALUES ('routing', dmn_load(:'routing'));

CREATE TABLE tickets (
  id            int PRIMARY KEY,
  subject       text,
  priority      text,
  customer_tier text
);
\copy tickets FROM 'tickets.csv' WITH (FORMAT csv, HEADER true)
```

## The view is the point

Rather than run the decision by hand each time, make it part of the schema. A view routes every ticket, and everything downstream—dashboards, alerting, the on-call rota—reads the view rather than knowing the rules.

```sql
CREATE VIEW routed_tickets AS
SELECT t.id, t.subject, t.priority, t.customer_tier,
  dmn_eval_text(m.model, 'Queue', jsonb_build_object(
    'Priority',      t.priority,
    'Customer Tier', t.customer_tier
  )) AS queue
FROM tickets t
CROSS JOIN models m
WHERE m.name = 'routing';

SELECT subject, priority, customer_tier, queue
FROM routed_tickets ORDER BY id;
```

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

The startup with the login outage gets woken up for, and the enterprise customer with a billing question does too—but the enterprise password reset does not. The rules say so, in five lines, and you can hand those five lines to someone and ask whether they agree.

## Who is on the hook tonight?

Because the routing is a column in a view, the questions you actually ask are ordinary SQL. Group by queue and the pager list falls out:

```sql
SELECT queue, count(*), string_agg(subject, '; ' ORDER BY id) AS work
FROM routed_tickets
GROUP BY queue
ORDER BY count(*) DESC, queue;

--  queue  | count |                        work
-- --------+-------+------------------------------------------------------
--  pager  |     3 | Cannot log in after SSO change; Billing discrepan...
--  tier-1 |     3 | Feature request: dark mode; Typo in the docs; Onb...
--  tier-2 |     2 | API latency spike in eu-west; Password reset not ...
```

## Change the policy, not the code

When the routing changes, you `UPDATE` one row. No migration, no deploy, no redeploying the consumers, and no window during which half the system triages by the old rules and half by the new. The next query against the view sees the new policy, because the policy *is* data.

```sql
-- Swap in a revised model; every consumer of routed_tickets follows at once.
\set routing_v2 `cat ticket-routing-v2.dmn`
UPDATE models SET model = dmn_load(:'routing_v2') WHERE name = 'routing';
```

And because the old model is still just a value, you can keep it. Version the rows, timestamp them, and you can answer the question every audit eventually asks: not *what would this ticket be routed to now*, but *what rule routed it that night, and who changed it*.
