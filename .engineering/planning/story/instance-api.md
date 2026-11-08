---
format: aep.planning-md/3
id: story:instance-api
kind: story
status: active
title: the instance runtime serves /v1/ on its socket; the daemon proxies it and serves the one dashboard, locally and for a remote deployment
summary: today's dashboard routes become the instance API; one dashboard app served by the daemon at / and on a TLS listener; the per-instance dashboard server retired
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:daemon-reconcile
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:24Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:24Z", actor: "human:timo", revision: 4}
---
## Why

Operator, 2026-11-07: "not every instance should come with its own dashboard ... Each instance comes with an HTTP API ... but not with its own UI deployment"; 2026-11-08: served by the daemon where the operator controls conductord and, at the same time, deployed remotely in a cluster for colleagues. Design §5.

## Acceptance

- `conductor instance serve` serves every route of `crates/conductor/src/dashboard.rs` under `/v1/` on `<runtime>/api.sock`, with `config` renamed `settings` and the writing routes keeping their token rule; the per-instance dashboard server and its unit are deleted.
- `GET /v1/instances/{name}/<route>` on the daemon answers with the instance's response; a browser holds the daemon's token only.
- The daemon serves the dashboard app at `/` on `127.0.0.1:<daemon_port>` and, when `daemon.listen` names a TLS listener, on it with Identity bearers; the same `dist/` deployed elsewhere works against the listener's URL with a read-only bearer; both run at once in a test.
- Every write on the global API carries an operator (`conductor.operator`); a bearer without the grant for a write is refused by name, and the record of an accepted write names the operator.
