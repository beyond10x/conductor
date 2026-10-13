---
format: aep.planning-md/3
id: story:gitlab-forge-write-route
kind: story
status: draft
title: A sanctioned route for GitLab merge requests
relations:
- decomposes: epic:forge-neutral
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. The guard denies glab writes and the bot route is GitHub-only, so a controller has no sanctioned way to open a merge request on a GitLab forge even with the operator's go.

## Acceptance

- A route for merge requests (create, update, merge) on a gitlab source through an identity the instance config names, gated like the GitHub bot route; the guard admits it and nothing else; tests with a fake glab. The identity is the operator's to provide (class H).
