---
format: aep.planning-md/3
id: story:default-branch-collectors
kind: story
status: draft
title: The blockers and specifications collectors read each repository's default branch
relations:
- decomposes: epic:forge-neutral
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. Both collectors hard-code origin/main; repositories whose default is master or another branch get a member failure every snapshot.

## Acceptance

- Both collectors read the default branch the repositories collector already knows (or `git symbolic-ref refs/remotes/origin/HEAD`); a fixture with a `master` default reads without failure.
