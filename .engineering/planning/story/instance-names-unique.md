---
format: aep.planning-md/3
id: story:instance-names-unique
kind: story
status: implemented
title: No two instances on a host share a session name, unit, port or state directory
relations:
- decomposes: epic:multi-instance-host
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T02:05:01Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T02:05:01Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T02:05:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: instance-names-unique

## Outcome

No two instances on a host share a session name, a systemd unit, a dashboard port or a state or records directory, whether they are in one config file or in several.

## Context

docs/design/roles-and-surfaces.md § 5a, items I1 to I5. These are the findings of an inventory of runtime paths taken on 2026-10-16:
- I1: a prefix-less session name matches another file's session host-wide (confine/mod.rs:789), so a start is refused or the wrong session is stopped.
- I2: messages are addressed by bare name, and nothing checks the receiver's instance (`repo-controller.md:82`, dashboard.rs:1419).
- I3: a systemd unit or slice is named by prefix only (Taskfile.yml:275; confine/systemd.rs:92, 110), so a second bare instance's dashboard is silently not started.
- I4: every file's first instance gets dashboard port 7313 (config.rs:1115).
- I5: the uniqueness of `state` and `records` is not validated.

This is migration step 0 of the design. It needs no new level.

## Acceptance

1. `session_prefix` is required for every instance; `config validate` refuses a missing one, and `config migrate` (or the validate notes) writes a suggested prefix.
2. A host registry lists every config file that has run on the host: `<host>/instances.json`, written by `config validate` and by session start. `config validate` refuses a prefix, port, state or records directory that another registered instance holds, naming both.
3. Unit, slice and dashboard unit names always carry the prefix.
4. A message whose receiver is not a session of the sender's instance is refused and recorded.
5. Tests show two config files with colliding names refused, and two distinct ones both start their dashboards.

## Out of Scope

The bridge (design § 4).

## Ambiguities

None.
