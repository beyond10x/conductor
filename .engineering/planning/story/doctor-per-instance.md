---
format: aep.planning-md/3
id: story:doctor-per-instance
kind: story
status: draft
title: '`conductor doctor --instance` checks one instance'
relations:
- decomposes: epic:multi-instance-host
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. doctor is file-wide: adding one instance's forge adds its logins and hosts to every instance's output, so a public record quoting doctor could leak another organisation's private host.

## Acceptance

- `conductor doctor --instance <name>` (and the active instance by default when the file has several) prints only that instance's forge rows and logins; tests with a two-instance config.
