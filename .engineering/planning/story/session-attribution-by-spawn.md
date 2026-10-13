---
format: aep.planning-md/3
id: story:session-attribution-by-spawn
kind: story
status: draft
title: Sessions belong to an instance by its spawn records, not by name prefix
relations:
- decomposes: epic:multi-instance-host
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. Sessions are attributed by name prefix; an unrelated live session whose name began with the instance's prefix would be counted and messageable as the instance's.

## Acceptance

- A session is the instance's when a spawn or start record of the instance names its id; the prefix is only a fallback for sessions started before; tests with a colliding name.
