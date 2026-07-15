---
title: Two pricing policies, one query
date: 2026-07-12
summary: The rules are a row in a table. Put two pricing models side by side, price the same orders under both, and switching everybody to the promotion becomes an UPDATE rather than a deploy.
files: order-pricing.dmn, order-pricing-promo.dmn, orders.csv
example: pricing
---

Pricing rules change — a promotion starts, a tax treatment moves — and the change usually arrives as a code deploy, with the old and new prices awkward to reconcile and easy to apply inconsistently across the queries that happen to price an order.

This example prices orders with a DMN model whose decisions chain one on another, and keeps two pricing policies side by side under the same output name so a single unchanged query can serve either. It covers swapping the live policy as a data change and reading money back to the penny you intend; discounting beyond a flat percentage, and rounding conventions per line versus per invoice, are only touched on, not built.

## A model is a graph, not a list

The standard model holds two decisions. `Tax Amount` multiplies the base price by the tax rate. `Total Price` adds that tax to the base price — so it depends on the other decision, not merely on the inputs.

Table: The standard pricing model

| Decision | Needs | Expression |
| --- | --- | --- |
| Tax Amount | Base Price, Tax Rate | `Base Price * Tax Rate` |
| Total Price | Base Price, Tax Amount | `Base Price + Tax Amount` |

You never tell pgdmn about that dependency. You ask for `Total Price`, and it works backwards: to answer that, it needs `Tax Amount`; to answer that, it needs the two inputs you supplied. The order of evaluation falls out of the model.

Both models are standard DMN files — open them in dmn-js, or any DMN tool: [standard →](/dmn-viewer.html?model=order-pricing.dmn), [promotional →](/dmn-viewer.html?model=order-pricing-promo.dmn).

## Set up

Load both models — [`order-pricing.dmn`](/examples/order-pricing.dmn) and [`order-pricing-promo.dmn`](/examples/order-pricing-promo.dmn) — into the `models` table under different names, and load the orders from [`orders.csv`](/examples/orders.csv). Run this from the directory the three files are in; you will need pgdmn installed first (see [Install](/docs/#install)).

```sql
\set standard `cat order-pricing.dmn`
\set promo    `cat order-pricing-promo.dmn`

CREATE TABLE models (name text PRIMARY KEY, model dmnmodel NOT NULL);
INSERT INTO models VALUES
  ('pricing-standard', dmn_load(:'standard')),
  ('pricing-promo',    dmn_load(:'promo'));

CREATE TABLE orders (
  id         int PRIMARY KEY,
  customer   text,
  base_price numeric,
  tax_rate   numeric
);
\copy orders FROM 'orders.csv' WITH (FORMAT csv, HEADER true)
```

A single order prices in one call. `dmn_eval_numeric` hands back a `numeric`, so it drops straight into `round` with no cast:

```sql
SELECT round(dmn_eval_numeric(model, 'Total Price', '{
    "Base Price": 100.00, "Tax Rate": 0.10
  }'::jsonb), 2) AS total
FROM models WHERE name = 'pricing-standard';

--  total
-- --------
--  110.00
```

## The promotion is a different model, not a different query

The promotional policy takes ten percent off first, and taxes the discounted price. It is a *three*-decision graph rather than two.

Table: The promotional pricing model

| Decision | Needs | Expression |
| --- | --- | --- |
| Net Price | Base Price | `Base Price * 0.9` |
| Tax Amount | Net Price, Tax Rate | `Net Price * Tax Rate` |
| Total Price | Net Price, Tax Amount | `Net Price + Tax Amount` |

Here is the part that matters: **both models answer to the name `Total Price`**. The caller asks the same question of both. The shape of the graph behind that question is entirely the model's business — one has an extra decision in the middle, and no query has to know.

So both live in the `models` table under different names, and choosing a policy is choosing a row. One query prices the whole book under both, and the only thing that differs between the two columns is which model row it reached for:

```sql
SELECT o.customer, o.base_price,
  round(dmn_eval_numeric(s.model, 'Total Price', p.input), 2) AS standard,
  round(dmn_eval_numeric(t.model, 'Total Price', p.input), 2) AS promo
FROM orders o
CROSS JOIN LATERAL (
  SELECT jsonb_build_object(
    'Base Price', o.base_price,
    'Tax Rate',   o.tax_rate
  ) AS input
) p
JOIN models s ON s.name = 'pricing-standard'
JOIN models t ON t.name = 'pricing-promo'
ORDER BY o.id;
```

Table: The same orders, under both policies

| Customer | Base price | Standard | Promo |
| --- | --- | --- | --- |
| Northwind Traders | 100.00 | 110.00 | 99.00 |
| Globex | 2499.99 | 2706.24 | 2435.62 |
| Initech | 45.50 | 54.60 | 49.14 |
| Umbrella Corp | 1000.00 | 1000.00 | 900.00 |
| Acme Supply | 19.99 | 21.49 | 19.34 |

Umbrella pays no tax either way — their rate is zero — so the promotion is the whole of their saving. Globex saves 270.62, which is not ten percent of anything they were quoted: the discount comes off before the tax, so the tax shrinks too. That interaction is *in the model*, where somebody can argue with it, rather than distributed across whichever queries happened to price an order.

Running the promotion for everyone is an `UPDATE` of one row. No migration, no deploy, no coordinated release.

## Globex, and what a penny is

Globex's standard tax is not 206.25. It is **206.249175**, and their total is 2706.239175. FEEL arithmetic is decimal and keeps every digit; the engine will not quietly round money on your behalf.

That is the right default. Rounding is a business rule — half-up, banker's, per line or per invoice — and a decision engine guessing at it is a decision engine introducing a bug. So the rounding lives in the query, in `round(…, 2)`, where you can see it and argue about it.

It matters more than it looks. Round each line and sum, and you get one number; sum and then round, and you can get another. Which one you want is a decision. Make it deliberately, somewhere a reviewer can find it.

## Going further

What does the promotion cost the business? The saving is just the difference between the two columns, summed — a single query, no application code:

```sql
SELECT round(sum(dmn_eval_numeric(s.model, 'Total Price', p.input)), 2) AS standard_book,
  round(sum(dmn_eval_numeric(t.model, 'Total Price', p.input)), 2) AS promo_book,
  round(sum(dmn_eval_numeric(s.model, 'Total Price', p.input)
    - dmn_eval_numeric(t.model, 'Total Price', p.input)), 2) AS given_away
FROM orders o
CROSS JOIN LATERAL (
  SELECT jsonb_build_object('Base Price', o.base_price, 'Tax Rate', o.tax_rate) AS input
) p
JOIN models s ON s.name = 'pricing-standard'
JOIN models t ON t.name = 'pricing-promo';

--  standard_book | promo_book | given_away
-- ---------------+------------+------------
--        3892.33 |    3503.10 |     389.23
```

Making the promotion the live policy is one statement — and because the old model is still a value in the table, last quarter's prices remain reproducible exactly:

```sql
-- Point the everyday name at the promotional model.
UPDATE models
SET model = (SELECT model FROM models WHERE name = 'pricing-promo')
WHERE name = 'pricing-standard';
```

