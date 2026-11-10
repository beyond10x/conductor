---
format: aep.planning-md/3
id: story:dispatch-unknown-repository-named-error
kind: story
status: draft
title: The refusal of a dispatch to an unwatched repository is a named error in the specification
scope:
- confidence: cited
  path: spec/domains
revision: 2
---
## Why

Found by the adversary pass on story:dispatch-refuses-unknown-repository in wave 49: the refusal of a dispatch that names a repository the instance does not watch is printed and tested, but no error in `spec/` names it, so the specification and its conformance suite do not hold it.

## Acceptance

- The refusal is a named error of the dispatch command in `spec/domains/`, the handler returns that name, and `ess specify validate` passes.
