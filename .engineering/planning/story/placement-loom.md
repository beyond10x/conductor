---
format: aep.planning-md/3
id: story:placement-loom
kind: story
status: draft
title: 'placement: loom runs a run-to-exit worker as a LoomAgent Command through Agency or the kube API'
summary: takt or an implementer on one unit as a Loom Job; exit code and events as the report; refused for interactive roles
relations:
- decomposes: epic:architecture-next
- depends_on: story:placement-substrate-local
- informed_by: design:conductor-on-agency
revision: 1
---
## Why

ADR 0105 (atlas): conductor starts runs through Agency's API and remote, cluster and swarm runs stay in Loom; ADR 0108: Loom owns the agent spec and the `LoomAgent` controller. A `LoomAgent` `Command` is a Job that runs to an exit, takes no message and has no attach, so it fits a worker on one unit (takt on one case, an implementer on one story), not a controller. Design §5 and §9a; `docs/design/conductor-on-agency.md` §4 D2 is the interim direct runner.

## Acceptance

- `placement: {loom: {context: <kube context>, namespace: <ns>}}` on a worker role makes the controller start that worker as a `LoomAgent` of the `Command` kind (program takt or the implementer, `secretRef`/`secretFiles` only, a workspace claim) through Agency's API when the config names an Agency, else through the kube API; the resource is written by conductor, nothing of conductor's own schema (ADR 0108).
- The controller reads `status.state`, `status.run.exitCode` and takt's `events.jsonl` from the workspace volume as the worker's report; exit 4 (`ApprovalRequired`) becomes a decision request and, once answered, a new generation.
- The repository goes in as a claim and the branch comes out through the box's forge route, recorded like any push; a test runs one story on a local kind cluster end to end.
- A `loom` placement on an interactive role (controller, supervisor) is refused at config validation with the reason.
