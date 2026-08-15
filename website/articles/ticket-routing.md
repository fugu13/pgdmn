---
title: Clarity in support ticket routing rules
date: 2026-07-11
summary: Routing rules as a DMN decision table behind a view: every ticket is classified in one place, and a table generated from the same model embeds in the wiki, so the docs never drift from the rules that actually run.
description: Routing rules as a decision table behind a view, with a table generated from the same model for the wiki—so the docs never drift from the rules that run.
files: ticket-routing.dmn, tickets.csv
example: routing
---

Support triage rules tend to live in people's heads or scattered through application code. They're also documented in the internal wiki, but that gradually goes stale and out of date.

This example puts the routing rules in the database as a DMN model behind a view, so every ticket is classified in one place. Also, a table automatically generated from the DMN can be embedded in the wiki, guaranteeing the rules match.

## The rules

The ticket routing rules are a decision table. The table exactly models the rules, it does not need to be translated into code. It is the rules, and it is what runs.

Table: Queue—hit policy: F (first)

| F | Priority | Customer Tier | Queue |
| --- | --- | --- | --- |
| 1 | `"critical"` | — | pager |
| 2 | `"high"` | `"enterprise"` | pager |
| 3 | `"high"` | — | tier-2 |
| 4 | — | `"enterprise"` | tier-2 |
| 5 | — | — | tier-1 |

This is a standard DMN file—[open it in dmn-js →](/dmn-viewer.html?model=ticket-routing.dmn), or in any DMN tool.

Read it as a policy and it is legible to someone who does not write SQL. Wake somebody for anything critical. Wake somebody for an enterprise customer with an urgent problem. Everything else queues by severity, and enterprise jumps the line.

That is a conversation you can have with the person who owns the support budget.

## Displaying the rules

The same standard DMN file can be shown as a table by more than one tool, and none of them change the file.

**dmn-js.** A short HTML file loads [dmn-js](https://bpmn.io/toolkit/dmn-js/) from a CDN and draws the decision table the way a DMN tool does; the hosted copy is the [dmn-js viewer](/dmn-viewer.html?model=ticket-routing.dmn).

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/dmn-js@17.9.0/dist/assets/dmn-js-shared.css" />
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/dmn-js@17.9.0/dist/assets/dmn-js-decision-table.css" />
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/dmn-js@17.9.0/dist/assets/dmn-js-decision-table-controls.css" />
    <style>html, body, #canvas { margin: 0; height: 100%; }</style>
  </head>
  <body>
    <div id="canvas"></div>
    <script src="https://cdn.jsdelivr.net/npm/dmn-js@17.9.0/dist/dmn-navigated-viewer.production.min.js"></script>
    <script>
      const viewer = new DmnJS({ container: "#canvas" });
      fetch("ticket-routing.dmn")
        .then((response) => response.text())
        .then((xml) => viewer.importXML(xml));
    </script>
  </body>
</html>
```

**XSLT, right in the browser.** A four-line wrapper names the model and points at a stylesheet, so opening it in a browser (served over http) renders the table: [see it rendered](/display/view.xml).

```xml
<?xml version="1.0" encoding="UTF-8"?>
<?xml-stylesheet type="text/xsl" href="render.xslt"?>
<render dmn="../examples/ticket-routing.dmn"/>
```

The stylesheet reads the DMN read-only with `document()` and builds the table itself: [render.xslt](/display/render.xslt).

```xslt
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0"
    xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    xmlns:dmn="https://www.omg.org/spec/DMN/20191111/MODEL/">
  <xsl:output method="html" indent="yes"/>

  <!-- Entry: the wrapper names the DMN; we load it read-only via document(). -->
  <xsl:template match="/render">
    <html lang="en">
      <head>
        <meta charset="UTF-8"/>
        <title>Decision table</title>
        <style>
          table.decision-table { border-collapse: collapse; }
          .decision-table th, .decision-table td { border: 1px solid #c9ced6; padding: .35rem .75rem; text-align: left; }
          .decision-table thead th { background: #eef2f7; }
          .decision-table th:last-child { background: #d9e3ee; }
        </style>
      </head>
      <body>
        <xsl:apply-templates select="document(@dmn)//dmn:decisionTable[1]"/>
      </body>
    </html>
  </xsl:template>

  <xsl:template match="dmn:decisionTable">
    <table class="decision-table">
      <caption>
        <xsl:value-of select="ancestor::dmn:decision/@name"/>
        <xsl:text> (hit policy: </xsl:text>
        <xsl:value-of select="translate(@hitPolicy, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')"/>
        <xsl:text>)</xsl:text>
      </caption>
      <thead>
        <tr>
          <th scope="col"><xsl:value-of select="substring(@hitPolicy, 1, 1)"/></th>
          <xsl:for-each select="dmn:input">
            <th scope="col"><xsl:value-of select="dmn:inputExpression/dmn:text"/></th>
          </xsl:for-each>
          <xsl:for-each select="dmn:output">
            <th scope="col"><xsl:value-of select="@name | ancestor::dmn:decision/dmn:variable/@name"/></th>
          </xsl:for-each>
        </tr>
      </thead>
      <tbody>
        <xsl:for-each select="dmn:rule">
          <tr>
            <td><xsl:value-of select="position()"/></td>
            <xsl:for-each select="dmn:inputEntry | dmn:outputEntry">
              <td><xsl:call-template name="cell"><xsl:with-param name="t" select="dmn:text"/></xsl:call-template></td>
            </xsl:for-each>
          </tr>
        </xsl:for-each>
      </tbody>
    </table>
  </xsl:template>

  <xsl:template name="cell">
    <xsl:param name="t"/>
    <xsl:choose>
      <xsl:when test="$t = '-'">any</xsl:when>
      <xsl:otherwise><xsl:value-of select="translate($t, '&quot;', '')"/></xsl:otherwise>
    </xsl:choose>
  </xsl:template>
</xsl:stylesheet>
```

That stylesheet runs anywhere XSLT does: paste both files into [xsltransform.net](https://xsltransform.net/), or run it with `xsltproc`, which ships on macOS and most Linux systems.

```sh
xsltproc render.xslt view.xml > table.html
```

## Try it in SQL

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

Rather than run the decision by hand each time, make it part of the schema. A view routes every ticket, and everything downstream reads the view rather than knowing the rules.

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

Because the routing is a column in a view, anyone comfortable with SQL can quickly answer questions about ticket routing. Group by queue and the pager list falls out:

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

And because the old model is still just a value, you can keep it. Version the rows, timestamp them, and in addition to "what would this ticket be routed to now" you can answer audit questions such as "what rule routed it two weeks ago, and who changed it?"
