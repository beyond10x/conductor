---
format: aep.planning-md/3
id: story:gitlab-repo-naming
kind: story
status: draft
title: A repository has one name across the collectors, spawn, the guard and session names
relations:
- decomposes: epic:forge-neutral
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. For checkouts at <root>/<group>/<repo>, the gitlab collector names `repo` while spawn, sessions, the guard and charters use `group/repo`, so rows and sessions do not join and a session name could contain `/`.

## Acceptance

- One naming rule (the path under checkouts.root, with `/` mapped to `-` in session names) used by every collector and by spawn, the guard and charters; a test over a nested gitlab fixture joins snapshot rows and sessions.
