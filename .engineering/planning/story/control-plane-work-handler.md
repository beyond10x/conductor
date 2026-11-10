---
format: aep.planning-md/3
id: story:control-plane-work-handler
kind: story
status: draft
title: A conductor instance takes work from a control plane and reports each execution to it
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

A consumer control plane records which work handler takes a work item and expects a handler to report progress, checkpoints and the end of an execution to it over HTTP; conductor is named there as one kind of handler, one instance per group of repositories (its design, read by its own session on 2026-11-10). Nobody has mapped that contract to conductor.

UNMAPPED: the contract's commands and fields, the relation between an execution there and a dispatch here (one to one, or one execution to many dispatches), and how the caller proves who it is. The consumer's model declares them; no document in this repository does. This story is not ready to implement until the domain is modelled in `spec/` or cited from the consumer's published contract.

## Acceptance

- A conductor instance configured with a control plane's URL takes work items from it as dispatches and reports each dispatch's progress, checkpoint and end to it; a test runs against a double of that HTTP contract.
