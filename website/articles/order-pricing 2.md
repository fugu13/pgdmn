---
title: Two pricing policies, one query
date: 2026-07-12
summary: The rules are a row in a table. Put two pricing models side by side, price the same orders under both, and switching everybody to the promotion becomes an UPDATE rather than a deploy.
example: pricing
---

Ask for the total and the tax computes itself. Then keep two pricing policies in the same table and choose between them by name.

## A model is a graph, not a list

The standard model holds two decisions. `Tax Amount` multiplies the base price by the tax rate. `Total Price` adds that tax to the base price — so it depends on the other decision, not merely on the inputs.

Table: The standard pricing model

| Decision | Needs | Expression |
| --- | --- | --- |
| Tax Amount | Base Price, Tax Rate | `Base Price * Tax Rate` |
| Total Price | Base Price, Tax Amount | `Base Price + Tax Amount` |

You never tell pgdmn about that dependency. You ask for `Total Price`, and it works backwards: to answer that, it needs `Tax Amount`; to answer that, it needs the two inputs you supplied. The order of evaluation falls out of the model.

## The promotion is a different model, not a different query

The promotional policy takes ten percent off first, and taxes the discounted price. It is a *three*-decision graph rather than two.

Table: The promotional pricing model

| Decision | Needs | Expression |
| --- | --- | --- |
| Net Price | Base Price | `Base Price * 0.9` |
| Tax Amount | Net Price, Tax Rate | `Net Price * Tax Rate` |
| Total Price | Net Price, Tax Amount | `Net Price + Tax Amount` |

Here is the part that matters: **both models answer to the name `Total Price`**. The caller asks the same question of both. The shape of the graph behind that question is entirely the model's business — one has an extra decision in the middle, and no query has to know.

So both live in the `models` table under different names, and choosing a policy is choosing a row.

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
