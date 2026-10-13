---
format: aep.planning-md/3
id: story:exclude-path-spaces
kind: story
status: draft
title: An exclude names one path space, and sources can split one checkouts root
relations:
- decomposes: epic:forge-neutral
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. One exclude entry is matched against the project path under the group and the checkout path under checkouts.root at once, and spawn and the guard take the union of every source's excludes, so a local source and gitlab sources cannot share a checkouts root.

## Acceptance

- Each source's excludes apply only to that source's repositories, in its own path space; spawn and the guard decide a repository by the source that names it; tests with a local and a gitlab source under one root.
