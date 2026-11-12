---
format: aep.planning-md/3
id: story:daemon-version-json
kind: story
status: draft
title: The daemon's version in one JSON answer
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read on 2026-11-12 at v1.4.0: the daemon's version is readable over HTTP only as the label of
`conductor_build_info` on `/metrics`. `/healthz` answers `{"state":"serving"}`, no route under
`/v1` carries it, and no response header does. A control plane that holds the control token
parses the metrics text to learn which daemon it talks to.

A consumer control plane asked for a JSON answer on 2026-11-12.

## Outcome

A caller of the daemon's API reads the daemon's version from one JSON answer.

## Decisions (2026-11-12, reversible)

- `GET /v1/version`, authenticated as every other read under `/v1`, answers
  `{"version": "<the binary's version>", "config": "<the config format it reads>"}`: the two
  values `conductor_build_info` carries. It answers before the first reconcile has published.
- `/healthz` and the instance rows are unchanged.

## Acceptance

- `GET /v1/version` with the control token answers `200` and the two fields; without a bearer
  it answers as every other `/v1` read does.
- The specification declares the route; the generated HTTP description and the guide's list of
  calls carry it.
