---
format: aep.planning-md/3
id: story:guard-admits-numeric-project-id
kind: story
status: draft
title: The guard admits an own forge write that names the project by its numeric id
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
revision: 2
---
## Why

story:guard-admits-own-mr-auto-merge admits the merge PUT only for a project written as its URL-encoded path. The same request with the project's numeric id is denied, because the guard cannot tell from the line which project the number is. The story's acceptance named the numeric form; that half was not built.

## Acceptance

- A source in the config file may state its forge project id; `conductor config validate` reads it and the reference names it.
- Under `forge.route: direct` the merge PUT and the pipeline POST with that number are admitted, and any other number is denied; cases in `tests/guard_rules.rs` hold both.
