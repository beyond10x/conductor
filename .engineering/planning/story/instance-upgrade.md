---
format: aep.planning-md/3
id: story:instance-upgrade
kind: story
status: active
title: POST /v1/instances/{name}/upgrade installs a released binary at the placement with rollback
summary: replaces conductor self-install; one instance at a time under the supervisor's grant
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:daemon-reconcile
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:29Z", actor: "human:timo", revision: 3}
---
## Why

Design §8: after a release of conductor, the daemon installs it where each instance runs; the supervisor runs it, the operator sees it in the brief.

## Acceptance

- `POST /v1/instances/{name}/upgrade {tag}` fetches the release, installs the binary at the instance's placement, restarts the runtime, runs `conductor doctor` inside, and rolls back to the previous binary when the runtime does not answer within `daemon.timeout`; every step is a record.
- `conductor instance upgrade --all` goes one instance at a time and stops at the first rollback.
- `GET /v1/instances` shows the binary version per instance.
