---
format: aep.planning-md/3
id: story:instance-isolation
kind: story
status: draft
title: Every conductor process and session acts for exactly one instance, pinned, never by default
revision: 1
---
# Story: instance-isolation

## Outcome

Two instances on one host never read or write each other's records, state, leases or messages, whatever environment a session, hook or command starts with. The host-wide disk ledger is one ledger for both.

## Context

The operator, 2026-10-15: the instances were meant to be isolated. An audit by conductor-dev the same day found no data mixed, but it found isolation that rests on hand-written settings rather than on conductor:
- **D1:** session.rs:644-654, spawn.rs:367-379 and confine/mod.rs:213-218 set `CONDUCTOR_INSTANCE` and `CONDUCTOR_CONFIG`, but the harness daemon forks the session, so neither reaches it. Only a settings file's `env` pins the instance, and a hook without it acts on the default instance.
- **D2:** config.rs:536-544. A missing `CONDUCTOR_INSTANCE` silently selects the default instance, even in another instance's checkouts or records.
- **D3:** state.rs:34-45 against records.rs:305-317, status.rs:160-161 and snapshot.rs:335-337. `--state-dir` changes the store but not the records.
- **D4:** host.rs:13-17, 77-78 and 121-134. The host ledger and the reclaim lock live in each instance's state, so the instances do not see each other's reservations.
- **D7:** cli.rs:116-126 and config.rs:665-686. When the config fails to load, the guard silently uses built-in paths.
- **D8:** dashboard.rs:347. `dashboard serve` defaults to one fixed port whatever the instance.
- **D10:** the profile's section on other instances (conductor.md:255-264) can make one instance's conductor a second writer for an instance that has its own conductor.

## Acceptance

1. **Hook commands.** Every hook command conductor writes or checks names its instance (`CONDUCTOR_INSTANCE=<name> conductor guard …`).
   - `session start`, `spawn` and `config validate` refuse a role whose settings do not pin `env.CONDUCTOR_INSTANCE` to the role's instance.
   - Tests cover a pinned role, an unpinned one, and one pinned to the wrong instance.
2. **Instance selection.** With no `CONDUCTOR_INSTANCE`, a command whose working directory is under an instance's records, state or checkouts root selects that instance. Under another instance's tree than the default, it refuses rather than using the default. Tests use two fixture instances.
3. **`--state-dir`.** A record-writing command (publish-board, snapshot rollups, the decision, dispatch and message logs) refuses a `--state-dir` other than its instance's `state`. Test.
4. **Host ledger.** The host ledger, its runs and the reclaim lock and marks (`host.lock`, `host-runs/`, `host.reclaim*`, `host.fall`) move to one host-wide directory beside the config, `<config dir>/host/`.
   - Existing files are migrated once.
   - A test runs two instances' `build run` and has them queue in one ledger.
5. **Guard on a bad config.** When the config fails to load, the guard denies and names the reason. Test.
6. **Dashboard port.** `dashboard serve` defaults to the instance's `dashboard_port`. Test.
7. **Profile.** The profile's other-instance section applies only to instances that name no `conductor` role. tests/profiles.rs holds the sentence.

## Out of Scope

Relays between instances that the instance's rules sanction. Adding an instance field to every log line is a separate story, if wanted.

## Ambiguities

None. The audit notes are kept outside the repository, because they name the second instance.
