---
format: aep.planning-md/3
id: story:control-plane-work-handler
kind: story
status: draft
title: A conductor instance takes work from a control plane and reports each execution to it
relations:
- decomposes: epic:deployable-in-kubernetes
- depends_on: story:pod-source-checkouts
revision: 3
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

## Shape (2026-11-12, reversible)

The three open points, answered for the first version after a consumer control plane asked for
a way to hand one unit of work to an instance with nothing but the control token:

- The instance is called; it does not poll. The work source is whoever holds the control token.
- One unit of work is one dispatch.
- The caller proves who it is with the control token, as for the instance routes.

The calls, on the daemon, forwarded to the instance:

- `POST /v1/instances/{name}/work` with `repository` (as the instance's rows name it), `brief`
  (text), `reference` (the caller's id for the unit, unique per instance) and optionally a ref
  or an issue. A repeated `reference` answers what the first call answered and creates nothing.
- `GET /v1/instances/{name}/work/{reference}` answers the dispatch's `state` by its own names
  (Sent, Started, Blocked, Done, Failed, Cancelled), when it started and ended, the last
  report, a `detail` where the state is Blocked or Failed, and `links`: the merge or pull
  requests the session reported, as a list.

What it needs that is not there on 2026-11-12 (read at v1.4.0):

- The control token may write nothing but the instance routes; every other write answers `403`
  `ActorNotHuman`. This call is one more write it may make.
- A dispatch needs an open goal (`dispatch send-dispatch --goal-id`); a unit of work brings
  none. The call creates what a dispatch needs or the dispatch stops needing it.
- The loop starts a controller only for a repository with a charter file in the records, once
  per cycle tick (2 h by default). The call brings a default charter where there is none and
  starts the controller at the call.
- A dispatch has no link to a merge request; a link is free text in a report. `links` needs a
  place in the dispatch or in the report.
- No agent session has been shown running end to end in an instance pod.

Before it: `story:pod-source-checkouts` (an agent has no checkout to work in without it).
