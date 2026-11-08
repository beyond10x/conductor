---
format: aep.planning-md/3
id: story:federated-views
kind: story
status: active
title: the global API federates sessions, decisions, unanswered, kpis, events and ask across instances
summary: union views with an instance column; an unreachable instance is a row, not an error
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:instance-api
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:24Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:24Z", actor: "human:timo", revision: 3}
---
## Why

Design §4.2: the operator sees every instance in one place; `conductor ask` of `epic:host-bridge` becomes the route `/v1/ask` delivered to the supervisor.

## Acceptance

- `GET /v1/sessions|decisions|unanswered|kpis` return the union of every instance's rows, each row carrying `instance`; `GET /v1/events` streams every instance's events as SSE with the same column.
- An instance that does not answer within `daemon.timeout` is one row with `state: unreachable` and its last mirror time; the view's status is 200.
- `POST /v1/ask` delivers the text to the supervisor session and returns its answer, whose first line names the route (`→ <instance>/<repository>`); `--to <instance>` delivers to that instance's controller.
- `conductor status` with no instance prints the federated view.
