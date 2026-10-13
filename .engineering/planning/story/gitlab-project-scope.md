---
format: aep.planning-md/3
id: story:gitlab-project-scope
kind: story
status: draft
title: A gitlab source can name projects, not only a whole group
relations:
- decomposes: epic:forge-neutral
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. A gitlab source lists the whole group and its subgroups; scoping a few projects in a large group needs an exclude list of every other project, and a project created later joins silently.

## Acceptance

- `sources: [{gitlab: <group>, host: …, projects: [<path>, …]}]` (spec first) limits the source to those projects; a project created later is not added; collector tests with a fake glab.
