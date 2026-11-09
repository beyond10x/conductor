---
format: aep.planning-md/3
id: story:sessions-systemd-kind
kind: story
status: archived
title: task sessions recognises a session started under the systemd provider
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:28:17Z", actor: "human:timo", revision: 4}
---
# Story: sessions-systemd-kind

## Outcome

`task sessions` and the conductor-dev profile's restart check recognise a conductor session started under the systemd provider, so a correct restart is not read as a missing background session.

## Context

On 2026-10-16 at 11:2xZ, `task conductor:restart` started conductor through the systemd provider (unit `conductor_conductor.service`, session 43bb6568). `claude agents --json` lists it with kind `interactive` and a pid, not `background`. The profile's step 5 ("check `task sessions` shows one background `conductor`") and the `ambiguous` filter of `task sessions` (Taskfile.yml, which selects `kind=="background"`) both assume the old provider.

## Acceptance

1. `task sessions` prints the provider of each role's session (`systemd <unit>` or `background`), taken from the instance's session record rather than from the harness's kind.
2. The ambiguity check counts the sessions of a name under either provider.
3. The conductor-dev profile's restart check names the systemd form.
4. A test or task check covers a session record of each provider.

## Out of Scope

Changing how the harness labels the session.

## Ambiguities

None.

## Archived 2026-11-09 (draft sweep)

The systemd session provider and confine/systemd.rs are removed (docs/design/architecture-next.md section 11).
