---
title: Change pricing policies on the fly
date: 2026-07-12
summary: Store each pricing policy as a dated row, and let one query price every order under whichever version is in effect on the day—so starting a promotion is an INSERT with a future date, not a deploy.
description: Store each pricing policy as a dated row, and let one query price every order under the version in effect—starting a promotion is an INSERT, not a deploy.
files: order-pricing.dmn, order-pricing-promo.dmn, orders.csv
example: pricing
---

Pricing rules change all the time. If those rules are in code, that means frequent deployments and tricky price tracking. You could build a dedicated pricing policy system, but that's assuming a large maintenance burden.

Taking base item attributes and turning them into final prices fits DMN perfectly. This example prices orders with a DMN model where decisions chain one on another. Both pricing policies take the same inputs and return the same outputs, so they can be changed on the fly, such as having one start applying on a certain date.

## A model is a graph, not a list

The standard pricing model holds two decisions. `Tax Amount` multiplies the base price by the tax rate. `Total Price` adds that tax to the base price—so it depends on the other decision, not merely on the inputs.

You never tell pgdmn about that dependency. You ask for `Total Price`, and it works backwards: to answer that, it needs `Tax Amount`; to answer that, it needs the two inputs you supplied. The order of evaluation falls out of the model.

Table: The standard pricing model

| Decision | Needs | Expression |
| --- | --- | --- |
| Tax Amount | Base Price, Tax Rate | `Base Price * Tax Rate` |
| Total Price | Base Price, Tax Amount | `Base Price + Tax Amount` |

Both models are standard DMN files—open them in dmn-js, or any DMN tool: [standard →](/dmn-viewer.html?model=order-pricing.dmn), [promotional →](/dmn-viewer.html?model=order-pricing-promo.dmn). These models are visualized as graphs, not tables, here. Many DMN models are best shown as one, the other, or a mix of both, in order to provide maximum clarity to business stakeholders.

## Set up

Load both models—[`order-pricing.dmn`](/examples/order-pricing.dmn) and [`order-pricing-promo.dmn`](/examples/order-pricing-promo.dmn)—into a `pricing_policies` table as two dated versions of one `retail` policy: the standard model, in effect since the start of the year, and the promotional one, set to take effect on 1 July. Then load the orders from [`orders.csv`](/examples/orders.csv). Run this from the directory the three files are in; you will need pgdmn installed first (see [Install](/docs/#install)).

```sql
\set standard `cat order-pricing.dmn`
\set promo    `cat order-pricing-promo.dmn`

CREATE TABLE pricing_policies (
  name         text NOT NULL,
  takes_effect date NOT NULL,
  model        dmnmodel NOT NULL,
  PRIMARY KEY (name, takes_effect)
);
INSERT INTO pricing_policies VALUES
  ('retail', DATE '2026-01-01', dmn_load(:'standard')),
  ('retail', DATE '2026-07-01', dmn_load(:'promo'));

CREATE TABLE orders (
  id         int PRIMARY KEY,
  customer   text,
  base_price numeric,
  tax_rate   numeric
);
\copy orders FROM 'orders.csv' WITH (FORMAT csv, HEADER true)
```

A single order prices in one call, under whichever version of the policy is in effect on the day you ask about—the latest one whose start date has arrived. `dmn_eval_numeric` hands back a `numeric`, so it drops straight into `round` with no cast:

```sql
SELECT round(dmn_eval_numeric(model, 'Total Price', '{
    "Base Price": 100.00, "Tax Rate": 0.10
  }'::jsonb), 2) AS total
FROM pricing_policies
WHERE name = 'retail' AND takes_effect <= DATE '2026-06-30'
ORDER BY takes_effect DESC
LIMIT 1;

--  total
-- --------
--  110.00
```

Asked as of 30 June, that is the standard model—the promotional row is dated 1 July, so its start date has not arrived yet.

## The promotion is a different model, not a different query

The promotional policy takes ten percent off first, and taxes the discounted price. It is a *three*-decision graph rather than two.

Table: The promotional pricing model

| Decision | Needs | Expression |
| --- | --- | --- |
| Net Price | Base Price | `Base Price * 0.9` |
| Tax Amount | Net Price, Tax Rate | `Net Price * Tax Rate` |
| Total Price | Net Price, Tax Amount | `Net Price + Tax Amount` |

Here is the part that matters: **both models answer to the name `Total Price`**. The caller asks the same question of both. The shape of the graph behind that question is entirely the model's business—one has an extra decision in the middle, and no query has to know.

So both live in the `pricing_policies` table as two dated versions of the same `retail` policy, and choosing a version is choosing a row by its date. One query prices the whole book under each, and the only thing that differs between the two columns is which version it reached for—the one in effect through June, and the one that takes over in July:

```sql
SELECT o.customer, o.base_price,
  round(dmn_eval_numeric(thru_jun.model, 'Total Price', p.input), 2) AS thru_jun,
  round(dmn_eval_numeric(from_jul.model, 'Total Price', p.input), 2) AS from_jul
FROM orders o
CROSS JOIN LATERAL (
  SELECT jsonb_build_object(
    'Base Price', o.base_price,
    'Tax Rate',   o.tax_rate
  ) AS input
) p
JOIN pricing_policies thru_jun
  ON thru_jun.name = 'retail' AND thru_jun.takes_effect = DATE '2026-01-01'
JOIN pricing_policies from_jul
  ON from_jul.name = 'retail' AND from_jul.takes_effect = DATE '2026-07-01'
ORDER BY o.id;
```

Table: The same orders, before and after the switch

| Customer | Base price | Through June | From July |
| --- | --- | --- | --- |
| Northwind Traders | 100.00 | 110.00 | 99.00 |
| Globex | 2499.99 | 2706.24 | 2435.62 |
| Initech | 45.50 | 54.60 | 49.14 |
| Umbrella Corp | 1000.00 | 1000.00 | 900.00 |
| Acme Supply | 19.99 | 21.49 | 19.34 |

Umbrella pays no tax either way—their rate is zero—so the promotion is the whole of their saving. Globex saves 270.62, which is not ten percent of anything they were quoted: the discount comes off before the tax, so the tax shrinks too. That interaction is *in the model*, where somebody can argue with it, rather than distributed across whichever queries happened to price an order.

Rolling the promotion out to everyone is not an `UPDATE` at all—it is the row you already inserted, dated 1 July. When that day arrives, the everyday query starts returning the July column on its own. No migration, no deploy, no coordinated release.

## Globex, and what a penny is

Globex's standard tax is not 206.25. It is **206.249175**, and their total is 2706.239175. FEEL arithmetic is exact decimal arithmetic to 34 significant digits. You'll never see binary arithmetic problems like 0.10 + 0.20 = 0.30000000000000004, because in FEEL 0.10 + 0.20 always equals 0.3.

pgdmn's typed numeric functions—like `dmn_eval_numeric`, used above—return PostgreSQL `numeric`, which can exactly represent every number FEEL handles. The query rounds the result to 2 decimal places in this example, but that decision is left up to the calling SQL.

## Going further

What does the promotion cost the business? The saving is the difference between the two columns, summed. Here's how you might calculate that.

```sql
SELECT round(sum(dmn_eval_numeric(thru_jun.model, 'Total Price', p.input)), 2) AS thru_jun_book,
  round(sum(dmn_eval_numeric(from_jul.model, 'Total Price', p.input)), 2) AS from_jul_book,
  round(sum(dmn_eval_numeric(thru_jun.model, 'Total Price', p.input)
    - dmn_eval_numeric(from_jul.model, 'Total Price', p.input)), 2) AS given_away
FROM orders o
CROSS JOIN LATERAL (
  SELECT jsonb_build_object('Base Price', o.base_price, 'Tax Rate', o.tax_rate) AS input
) p
JOIN pricing_policies thru_jun
  ON thru_jun.name = 'retail' AND thru_jun.takes_effect = DATE '2026-01-01'
JOIN pricing_policies from_jul
  ON from_jul.name = 'retail' AND from_jul.takes_effect = DATE '2026-07-01';

--  thru_jun_book | from_jul_book | given_away
-- ---------------+---------------+------------
--        3892.33 |       3503.10 |     389.23
```

To produce a price report for the current day, no need to specify the pricing model by name or ID. A query can look up the policy in effect—so on 1 July it begins pricing at the promotional rate, with nothing deployed and nothing updated in between:

```sql
SELECT o.customer,
  round(dmn_eval_numeric(pol.model, 'Total Price', jsonb_build_object(
    'Base Price', o.base_price, 'Tax Rate', o.tax_rate)), 2) AS total
FROM orders o
CROSS JOIN LATERAL (
  SELECT model FROM pricing_policies
  WHERE name = 'retail' AND takes_effect <= CURRENT_DATE
  ORDER BY takes_effect DESC
  LIMIT 1
) pol
ORDER BY o.id;
```

Run it in June and every row is the standard price; run it on 1 July and every row is the promotional one. The query uses the current date to pick the applicable pricing policy. It runs backwards just as well: to see what an order would have cost on any past day, ask as of that date. The older version is still right there, dated, so last quarter's prices stay reproducible exactly.

