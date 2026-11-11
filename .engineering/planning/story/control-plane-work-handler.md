---
format: aep.planning-md/3
id: story:control-plane-work-handler
kind: story
status: draft
title: A conductor instance takes work from a control plane and reports each execution to it
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 2
---
## Why

A consumer control plane records which work handler takes a work item and expects a handler to report progress, checkpoints and the end of an execution to it over HTTP; conductor is named there as one kind of handler, one instance per group of repositories (its design, read by its own session on 2026-11-10). Nobody has mapped that contract to conductor.

UNMAPPED: the contract's commands and fields, the relation between an execution there and a dispatch here (one to one, or one execution to many dispatches), and how the caller proves who it is. The consumer's model declares them; no document in this repository does. This story is not ready to implement until the domain is modelled in `spec/` or cited from the consumer's published contract.

## Acceptance

- A conductor instance configured with a control plane's URL takes work items from it as dispatches and reports each dispatch's progress, checkpoint and end to it; a test runs against a double of that HTTP contract.

## Decision (operator, 2026-11-11)

Where the contract lives. Offered: A, the handler in conductor speaks the consumer's contract
by its own command names; B, conductor models a generic work-source contract in its own
specification and a thin adapter outside this repository maps a control plane's contract to it;
C, the whole handler is a component outside this repository. The operator: "B".

What follows: the domain this story needs is conductor's own (`spec/`): a work source a
conductor instance registers with, takes work items from as dispatches, and reports each
dispatch's progress, checkpoint and end to. It names no consumer and none of a consumer's
commands. The UNMAPPED lines above are settled by that domain, not by citing a consumer's
contract. Still open, and the domain's to answer: whether one execution of a work source is one
dispatch or several, how an instance learns of new work (it polls, or it is called), and how
the caller proves who it is. The consumer's session answered on 2026-11-11 that on its side
the last two are not decided and that a handler cannot sign in yet.
