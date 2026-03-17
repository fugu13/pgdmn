# Release Plan

## Info Website

Standalone site for pgdmn at pgdmn.com.

**Pages:**

- **Home** -- tagline, one-paragraph pitch, quick-start SQL snippet, install command
- **Why pgdmn** -- the case for running decisions inside the database: no network hop, transactional consistency, auditable, co-located with the data
- **Docs** -- function reference (mirror of README), FEEL expression guide, DMN model authoring tips, composite-type record eval walkthrough
- **Examples** -- curated real-world scenarios (loan eligibility, pricing tiers, compliance checks, routing rules)
- **Blog** -- launch post, deep dives, integration guides (dbt, Supabase, pgAdmin)
- **GitHub link** -- prominent

**Tech:** Static site generator (Hugo, Astro, or similar). Hosted on Netlify/Vercel/GitHub Pages.

---

## PGXN Setup

Following David E. Wheeler's auto-release workflow (https://justatheory.com/2025/05/release-on-pgxn/ and the 2024 pgrx-specific guide at https://justatheory.com/2024/04/pgxn-tools-pgrx/).

### Steps

1. **Register a PGXN Manager account** at https://manager.pgxn.org/account/register. A volunteer approves; once confirmed, uploads publish immediately with no further review.

2. **Create `META.json.in` template.** Because pgrx has no static SQL file, use a template that extracts the version from `Cargo.toml` at build time. Required fields:

   | Field | Value |
   |---|---|
   | name | `pgdmn` |
   | version | extracted from Cargo.toml |
   | abstract | `DMN (Decision Model and Notation) support for PostgreSQL` |
   | maintainer | TBD |
   | license | `mit` (or `apache_2_0` -- pick one for PGXN) |
   | provides | `{"pgdmn": {"file": "src/lib.rs", "version": "..."}}` |
   | meta-spec | `{"version": "1.0.0"}` |

   Optional but recommended: `description`, `tags` (`["dmn", "feel", "decision", "business-rules"]`), `prereqs` (PostgreSQL 17), `resources` (GitHub repo, issue tracker).

3. **Add a Makefile target** to generate `META.json` from the template. Add `META.json` to `.gitignore`.

4. **Configure `.gitattributes`** to exclude CI/dev files from the release archive:
   ```
   .gitignore export-ignore
   .github export-ignore
   .gitattributes export-ignore
   ```

5. **Add GitHub Actions secrets:** `PGXN_USERNAME` and `PGXN_PASSWORD`.

6. **Create `.github/workflows/pgxn-release.yml`:**
   - Trigger on semver tags (`v[0-9]+.[0-9]+.[0-9]+`)
   - Uses `pgxn/pgxn-tools` container
   - Steps: checkout, `make META.json`, `pgxn-bundle`, `pgxn-release`
   - Optionally create a GitHub Release in a second step (PGXN first, since it has stricter validation)

7. **Release process:** Tag and push.
   ```
   git tag v0.1.0 -sm 'Tag v0.1.0'
   git push --follow-tags
   ```

---

## Promotion Plan

### PostgreSQL Channels

| Channel | How | URL |
|---|---|---|
| **Postgres Weekly** | Submit link via form | https://cooperpress.com/submit |
| **Planet PostgreSQL** | Register blog, write launch post, get syndicated | https://planet.postgresql.org |
| **pgsql-announce** | Post announcement | https://lists.postgresql.org |
| **pgsql-general** | Share for broader user reach | https://lists.postgresql.org |
| **r/PostgreSQL** | Post announcement + demo | https://reddit.com/r/PostgreSQL |
| **Hacker News** | "Show HN" post | https://news.ycombinator.com |
| **PGXN** | Publish the extension | https://pgxn.org |
| **Citus Community Slack** | Share in relevant channel | https://slack.citusdata.com |

### General Dev Channels

- **Hacker News** -- "Show HN: pgdmn -- Run DMN decision tables inside PostgreSQL"
- **Lobste.rs** -- if someone with an invite can post
- **Dev.to / Hashnode** -- cross-post the launch blog post
- **X/Twitter** -- tag PostgreSQL and Rust communities

### DMN-Specific Channels

See the DMN Communities section below for where to post.

### Blog Posts to Write

1. **Launch post** -- what pgdmn is, why, quick demo
2. **"Why run decisions in your database"** -- the architectural argument
3. **"FEEL expressions as a PostgreSQL query language"** -- for the SQL-curious
4. **Integration guides** -- dbt, Supabase, pgAdmin, application-layer patterns

---

## DMN Communities

Places where DMN practitioners gather. These are the targets for announcing pgdmn and participating in ongoing discussions.

### Forums and Q&A

- **Camunda Forum** (most active open DMN forum) -- https://forum.camunda.io/
- **Stack Overflow [dmn] tag** -- https://stackoverflow.com/questions/tagged/dmn
- **bpmn.io Forum** -- https://forum.bpmn.io/
- **dmn-tck GitHub** (spec compliance discussions) -- https://github.com/dmn-tck/tck
- **Drools / KIE community** (Red Hat's DMN engine) -- https://www.drools.org/community/

### Chat

- **Camunda Community Slack** -- https://camunda.com/developers/
- **DecisionCAMP Discord** -- year-round channel for the annual decision modeling conference

### LinkedIn Groups

- "Decision Model and Notation (DMN)"
- "Business Rules"
- "BPM, BPMN & DMN"
- "Camunda BPM"

### Reddit

- **r/bpm** -- https://reddit.com/r/bpm
- **r/businessanalysis** -- https://reddit.com/r/businessanalysis

### Conferences

- **DecisionCAMP** -- primary annual decision modeling conference, heavily DMN-focused -- https://decisioncamp.org/
- **CamundaCon** -- annual, has DMN sessions -- https://camunda.com/events/camundacon/
- **bpmNEXT** -- showcase conference for BPM/DMN innovations -- https://bpmnext.com/
- **Building Business Capability (BBC)** -- business rules and decisions tracks -- https://buildingbusinesscapability.com/
- **OMG Quarterly Technical Meetings** -- where the DMN spec is developed

### Key Individuals / Blogs

- **Bruce Silver** (DMN spec author) -- https://methodandstyle.com/blog/
- **James Taylor** (Decision Management Solutions) -- https://jtonedm.com/
- **Denis Gagne** (Trisotech, DMN spec co-chair) -- active on LinkedIn and at conferences
- **Trisotech blog** -- https://www.trisotech.com/blog

### Specification Body

- **OMG DMN spec page** -- https://www.omg.org/spec/DMN
- **OMG BPM+ Health** (DMN in healthcare vertical) -- https://www.omg.org/bpm-health/

---

## Snazzy Demo

A self-contained, copy-paste demo that shows pgdmn solving a real problem in under 30 seconds of reading.

### Concept: Loan Eligibility Engine in Pure SQL

Walk through a complete loan eligibility scenario:

1. Load a DMN model with a multi-hit decision table (age, income, credit score -> eligibility + rate)
2. Run it against a table of applicants in one query using a lateral join
3. Show the results -- each applicant gets a decision with no application code

This demonstrates: decision logic separated from application code, batch evaluation, transactional consistency, auditability (the model XML is stored right there).

### Format

- **README GIF/asciicast** -- record a psql session with asciinema, embed in README and website
- **Live playground** -- if feasible, a web page where visitors paste DMN XML and run it (backed by a Supabase instance with pgdmn installed, or a serverless PG)
- **Conference talk version** -- 5-minute live-coding demo that builds up from `feel_eval('1+1')` to a full decision table evaluation

### Secondary Demo Ideas

- **Compliance checker** -- GDPR data-handling rules as a decision table, evaluated per-row on a customers table
- **Dynamic pricing** -- pricing rules that change by swapping the DMN model, no code deploy needed
- **Feature flags via DMN** -- decision table controlling feature rollout by user segment

---

## Thorough Documentation

### Audience Tiers

1. **Quick start** (existing, in README) -- install, first query, see a result
2. **Function reference** (existing, in README) -- every function with signature and example
3. **Guides** (to write):
   - FEEL expression language primer for SQL users
   - Authoring DMN models (tools: Camunda Modeler, Trisotech, bpmn.io)
   - Using composite types with `dmn_record_eval` and `feel_record_eval`
   - Storing and versioning DMN models in PostgreSQL tables
   - Batch evaluation patterns (lateral joins, CTEs)
   - Error handling and debugging (what happens when evaluation fails)
4. **Integration guides** (to write):
   - dbt: using pgdmn in transformations
   - Supabase: calling pgdmn from Edge Functions / PostgREST
   - Application layer: calling from Python/Node/Go via standard PG drivers
5. **Architecture / internals** (to write):
   - How the ModelEvaluator cache works
   - FEEL value to PG type conversion rules
   - Performance characteristics and when to use typed vs. JSONB variants

### Where

- Short docs stay in README
- Guides go on the info website (and/or in `docs/`)
- API reference auto-generated if possible (pgdoc or similar)

---

## Product Upsell: Schema Management

**Pitch:** pgdmn already treats DMN models as first-class database values. The natural next step is managing the lifecycle of those models -- versioning, compatibility checking, migration.

### Story

"You store your DMN models in PostgreSQL. You version your database schema with migrations. Why not version your decision logic the same way?"

### Components

- **`dmn_compat`** (FEAT-002 in TODO.md) -- Kafka-style compatibility checking for DMN invocables. Backward, forward, and full compatibility against historical model versions.
- **Model registry pattern** -- a `dmn_models` table with version, created_at, and compatibility metadata. Provide a reference schema and helper functions.
- **Migration integration** -- guidance for using dmn_compat in CI to gate deployments: "does the new model break any consumers?"
- **Audit trail** -- every model version stored, every evaluation traceable to a specific model version.

### Positioning

This bridges the gap between "I have a DMN model" and "I have a governed, versioned decision management system" -- without leaving PostgreSQL. Targets teams that currently use Camunda, Drools, or similar platforms and want to simplify their stack.

---

## Product Upsell: LLM + DMN Tools

**Pitch:** LLMs are good at generating structured output but bad at deterministic rule application. DMN is the inverse. Combine them.

### Story

"Use an LLM to extract facts from unstructured data. Use DMN to make the decision. Get the best of both worlds: flexibility in understanding, rigor in deciding."

### Use Cases

1. **LLM extracts, DMN decides** -- LLM reads a document (insurance claim, support ticket, resume) and extracts structured fields. Those fields feed into a DMN decision table for a deterministic, auditable outcome.
2. **LLM generates DMN** -- Given a natural-language description of business rules, an LLM generates a DMN XML model. pgdmn validates and evaluates it. The human reviews the decision table, not the code.
3. **Guardrails for LLM output** -- DMN decision tables as post-processing validators for LLM responses. "If the LLM says X but the rules say Y, flag it."
4. **Explainability** -- DMN decisions are inherently explainable (the table is the explanation). Pair with LLM-generated natural-language summaries for non-technical stakeholders.

### Positioning

Targets teams building AI-powered workflows who need auditability and determinism for the decision layer. The database is already the system of record -- pgdmn makes it the system of decision too.

---

## Consulting Upsell

**Pitch:** pgdmn is open source and free. Adopting decision management well is a design problem.

### Services

1. **Decision architecture review** -- Assess which business rules should move into DMN, how to structure models, and how to integrate with existing data pipelines. Fixed-scope engagement.
2. **Migration from BRMS** -- Help teams migrating from Camunda, Drools, IBM ODM, or FICO Blaze Advisor to a pgdmn-based stack. Includes model conversion, testing, and deployment patterns.
3. **Custom integration** -- Build the glue between pgdmn and the team's stack (dbt pipelines, event-driven architectures, API layers, CI/CD for model versioning).
4. **Training** -- Half-day or full-day workshop on DMN modeling, FEEL expressions, and pgdmn usage. Targeted at data engineers, backend developers, and business analysts.
5. **Decision model authoring** -- For organizations that know what rules they want but lack DMN expertise. Deliver tested, versioned DMN models ready to load into pgdmn.

### Positioning

The open-source extension gets people in the door. Consulting addresses the gap between "I installed the extension" and "my organization runs on auditable, versioned decision logic in the database." Lead generation comes from the info website, blog posts, conference talks, and community participation in the DMN channels listed above.
