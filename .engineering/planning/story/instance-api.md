---
format: aep.planning-md/3
id: story:instance-api
kind: story
status: draft
title: the instance runtime serves /v1/ on its socket and the daemon proxies and serves one dashboard
summary: today's dashboard routes become the instance API; one dashboard app served by the daemon; the per-instance dashboard unit retired
relations:
- decomposes: epic:architecture-next
- depends_on: story:daemon-reconcile
revision: 1
---
## Why

Operator, 2026-11-07: "not every instance should come with its own dashboard ... Each instance comes with an HTTP API ... but not with its own UI deployment." Design §4.1 and §4.3.

## Acceptance

- `conductor instance serve` serves every route of `crates/conductor/src/dashboard.rs` under `/v1/` on `<runtime>/api.sock`, with `config` renamed `settings` and the three writing routes keeping their token rule.
- `GET /v1/instances/{name}/<route>` on the daemon answers with the instance's response; the browser holds the daemon's token only.
- The daemon serves the dashboard app at `/`; its data calls go to the daemon; the instance picker reads `instances`.
- `conductor_<instance>-dashboard.service` is no longer installed; `dashboard_port` in a `conductor.config/1` file is ignored with one warning line.
