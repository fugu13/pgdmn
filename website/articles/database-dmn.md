---
title: DMN in the database
date: 2026-08-14
summary: An introduction to DMN, why it matters for software developers, and the natural fit between DMN and the database.
---

The large majority of software developers have no familiarity with DMN, BPMN, and most other standards of OMG (yes, that's the name). Real quick, those are [Decision Model & Notation](https://www.omg.org/dmn/), [Business Process Model & Notation](https://www.omg.org/bpmn/), and [Object Management Group](https://www.omg.org/).

The two standards from OMG most developers may have encountered are often the subject of various mixes of revulsion and confusion—UML and CORBA. Whether or not that's deserved, DMN and BPMN are very different and more modern standards.

DMN and BPMN are in wide, productive use in real organizations, and they provide a critical capability: separating business rules and processes from software processes.

## Caveat emptor

There's no such thing as a complete separation of business rules and processes from software processes. Software processes _are_ business processes. But! It is often the case that there are certain rules and processes that are some mix of

- owned by business teams that do not do software development
- updated on schedules very different than the systems that apply them (e.g. they are often stable and rarely updated)
- subject to regulations or strict internal controls that are only concerned with these rules and processes
- require historical auditability of decisions and executions even after the rules have been changed

And none of these are a good fit for rule and process definitions that live in code.

## What is DMN?

DMN models decision rules. Decision rules are a lot more than a bunch of if statements. Using DMN means decision rules are

- precisely specified, not ambiguous language in a document
- formally verifiable for important properties such as covering all cases
- automatically comparable against previous versions for compatibility
- renderable as tables or variable dependency diagrams to swiftly answer business questions
- executable in exactly the form they were developed instead of requiring an additional and error-prone translation step

Decision rules also support a number of niceties that align better with real business rules than if statements do, such as specifying that exactly one rule must match, or that if multiple rules match they must have identical outputs.

## Where does DMN live?

Right now, DMN mostly lives in specialized systems, often ones that other standards such as BPMN, or "rules engines". Software developers may remember [Drools](https://kie.apache.org/components/drools/), an open source rules engine ("business rule management system") that predates DMN but now primarily uses DMN.

There's a lot of messy history as to why things worked out that way, and part of it is that the tighter relationship of software development and business teams needed to support DMN-in-software is a fairly recent development. I've worked on multiple projects to include formal decision specifications inside software platforms that aren't rules engines, from fraud engines to auth rules.

A good related example is the use of [Rego and Open Policy Agent](https://www.openpolicyagent.org/docs/policy-language) to enforce policies about Kubernetes entities.

But so far I haven't seen DMN show up in one place I think it is an excellent and natural fit: the database.

## DMN in the database

To figure out if a DMN rule applies to an entity, a system gathers the relevant data, executes the DMN model on the data, then provides the output. Right now that often involves a round trip to some other system, or is embedded deep within an application where it is hard to ask alternative questions like "give me a table summarizing recent decisions". But with DMN in the database, it already lives where the data lives, and alternative questions are just a query away.

Yes, database queries can look somewhat like declarative rules already, but in practice they have most of the same issues as any other code in comparison to the benefits of DMN I listed above.

This project provides DMN in the database via a PostgreSQL extension. Keep your model in the database and execute any decision the model supports in any query.

Want to keep an audit trail? Materialize the decisions and keep the old models while changing which models new queries point at.

Additionally, the models are introspectable in the database, so if you have a library of models you can answer questions such as "what decisions does this support?" and "what inputs and outputs are provided for this decision?"

## Next steps: DMN management in the database

Related to that introspection capability, one of the next big features for pgdmn will be a set of (entirely optional) functions for managing the storage of DMN models and validating models for compatibility with the models they replace.

This is inspired by Kafka's [Schema Registry](https://docs.confluent.io/platform/current/schema-registry/index.html).

The Schema Registry supports important checks such as "will the new schema support current data?" and "will the new schema support historical data from all previous versions of the schema?"

pgdmn will support similar checks, such as "will the new DMN provide compatible output?" and "will the new DMN be compatible with all inputs accepted by previous models?"

With these functions, developers can confidently allow business teams to provide updated DMN rules without risking the stability of the platform executing the rules.
