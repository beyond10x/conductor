---
format: aep.planning-md/3
id: epic:loom-agent-runner
kind: epic
status: draft
title: conductor dispatches a story to a takt LoomAgent run on a local kind cluster
summary: An interim runner that writes a LoomAgent Command resource, records the run from its status and reserves its disk; forge writes stay fenced on the host
relations:
- depends_on: epic:dispatch-runner
revision: 1
---
# Epic: loom-agent-runner

## Outcome

conductor dispatches one story of one repository as a takt `LoomAgent` `Command` run on a local
kind cluster: it writes the resource through the cluster's command line (a program
`conductor doctor` checks), reads the resource's status for started and done or failed, and
reserves the run's disk in the host ledger. Forge writes stay fenced: the bot route on the host
pushes the run's branch and opens the pull request.

## Acceptance

For one dispatch, the dispatch log holds `sent`, `started` and `done` lines written by
`conductor run` from the resource's status; the pull request is the bot's; one host-ledger
reservation is taken and released at the run's end.

## Depends on outside this store

A `Command` run that runs to completion and reports its exit code in its status (loom), and a clean
repeat of the 2026-10-26 trial with no hand patch (loom, takt, llm, metaharness): design § 4 step 2.

## Source

`docs/design/conductor-on-agency.md` § 4 step 3, § 6 D2, D6, D7.
