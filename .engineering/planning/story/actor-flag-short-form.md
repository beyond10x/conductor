---
format: aep.planning-md/3
id: story:actor-flag-short-form
kind: story
status: draft
title: --actor takes the human:<name> form the records print
summary: The CLI's --actor accepts the short actor form the records and tick output show, beside the JSON object
tags:
- from-finding
scope:
- confidence: cited
  path: crates/conductor/src/actor.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 1
---
## Why

Agentbox friction log F9 (2026-11-15): the operator copied `human:<name>` from tick output into
`conductor goal propose-goal --actor` and was refused; only the JSON object
`{"kind":"Human","operator":"<name>"}` is read. Read at v1.13.0: `actor::parse` takes only a JSON
object (`crates/conductor/src/actor.rs:202-211`), while the records write an actor as
`human:<name>` (`actor.rs:328-340`), and `website/docs/reference/cli.md:244` documents only the
JSON form. The form the operator reads is not a form the operator can type.

## Outcome

`--actor` also takes the records' short forms (`human:<name>`, `daemon`, and the agent form the
records print), each read to the same `Actor` as its JSON object; the JSON object stays accepted.

## Acceptance

- `--actor human:timo` and `--actor '{"kind":"Human","operator":"timo"}'` record the same actor
  on a goal proposal, and a short form that names no kind is refused naming both accepted forms
  (a CLI test).
