---
format: aep.planning-md/3
id: story:dashboard-port-explicit
kind: story
status: draft
title: An instance's dashboard port does not shift when others are added or removed
relations:
- decomposes: epic:multi-instance-host
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. The dashboard_port default is positional, so removing or reordering an instance shifts every later instance's port.

## Acceptance

- The default port is derived from the instance's name (stable) or `config validate` warns when an instance relies on a positional default in a file with several instances; tests.
