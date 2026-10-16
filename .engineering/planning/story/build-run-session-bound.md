---
format: aep.planning-md/3
id: story:build-run-session-bound
kind: story
status: implemented
title: A build request ends when the session that made it stops
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T11:24:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T11:24:26Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T11:24:27Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: build-run-session-bound

## Outcome

A session that stops or pauses, for example on a usage limit, gives up its queued disk requests at once, so other repositories' builds run and its tree can be cleaned.

## Context

On 2026-10-15 at ~07:40Z, three conductor-dev implementors had paused on a usage limit until 11:00Z. Their `conductor build run` requests (`--reserve 8G` cargo test, `--reserve 4G` docs-generate, `--reserve 1G` cargo fmt) stayed queued in the host ledger.
- Their `;`-chained shells started the next command each time one was killed.
- A poll loop (`until grep -q … ; do sleep 10; done`) waited for output that would never come.
- These processes held the trees, so `worktree discard-cache` refused with `worktree-in-use`, while builds of substrate, cortex and canon were refused for disk (conductor's DEC-20261015-27).

Related: story:reserve-from-peak, story:build-run-priority.

## Acceptance

- `conductor build run` records the session id of its caller (from the environment the harness sets) on its request.
- A request in state `Requested` whose session is no longer running is withdrawn at the next ledger pass, and the waiting process exits 2 with `withdrawn: session <id> stopped`. Test with a fake session list.
- `conductor host grants` shows each request's session.

## Out of Scope

Killing a command that is already running. Its watchdog and peak recording stay as they are.

## Ambiguities

- How a paused but live session (a usage-limit pause) is told apart from a stopped one: `inferable` from the session list's state field, if it carries one. Otherwise only stopped sessions are covered.
