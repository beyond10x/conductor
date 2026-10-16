# Wave 39 (levels, roles and tool surfaces)

Coordinator: conductor-dev. Approved up front (rules item 99). Target: v0.17.0, after v0.16.x.

Sources:
- the operator, 2026-10-16: one level above the instances, clearer roles, re-reviewed instance isolation, and workers that never run arbitrary Bash;
- [`../design/roles-and-surfaces.md`](../design/roles-and-surfaces.md), approved in full by the operator on 2026-10-16.

Epics: multi-instance-host (step 0), role-names, host-bridge, agent-tools. Decisions taken while planning, each recorded on its cleared blocker:
- per-host roles live in `host.yaml` in the host directory;
- a worker's grant ends with its lead's session;
- the director gets its own `tool` verb group.

## Units, in order

| unit | story | note |
|---|---|---|
| U1 | instance-names-unique | step 0: required prefix, the host registry, collisions refused across config files |
| U2 | host-fall-per-instance | step 0: a disk fall per instance |
| U3 | scratch-per-instance | step 0: scratch under the instance's cache |
| U4 | role-name-aliases | director, maintainer and lead, with the old names as aliases for one release |
| U5 | maintainer-host-config | `host.yaml` holds the maintainer and the bridge |
| U6 | tool-broker-read | the broker, grants, the `tool` binary, read verbs, `tool mcp` |
| U7 | tool-build-commit-report | test, check, fmt, commit, send, report through the broker |
| U8 | implementers-on-tool | implementers with Bash denied, measured before and after |
| U9 | host-instances-list, status-all-instances | the view across instances |
| U10 | role-names-in-files, message-levels | the names in every file; messages one level up or down |
| U11 | profile-slimming | director profile under 250 lines, measured context per session |

Later in the wave, or in wave 40:
- the bridge session and `conductor ask`;
- the host dashboard;
- worker agents;
- the remaining verbs, grants by `agent run`, the director's verbs and aliases;
- dropping session commands from `conductor`.

## Measures

- **Context per conductor session:** before is 200k reached in about 35–70 min (2026-10-16). After U8 and U11, the same reading from transcripts.
- **Guard refusals and round trips per day** from transcripts (48 h baseline: 212 refusals, 24,647 calls).
