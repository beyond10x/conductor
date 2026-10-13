---
format: aep.planning-md/3
id: story:gitlab-user-namespace
kind: story
status: draft
title: A gitlab source can be a user namespace
relations:
- decomposes: epic:forge-neutral
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. `gitlab: <user>` reads groups/<user>/projects, which a user namespace does not answer, and the whole collector fails.

## Acceptance

- A source naming a user namespace reads users/<user>/projects; the collector tells the two apart and a failure of one source does not fail the others; tests with a fake glab.
