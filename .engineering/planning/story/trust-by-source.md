---
format: aep.planning-md/3
id: story:trust-by-source
kind: story
status: draft
title: '`conductor trust` trusts only the instance''s repositories'
relations:
- decomposes: epic:multi-instance-host
revision: 1
---
## Why

A second instance on a self-hosted GitLab (2026-10-13, conductor 0.10.4) found this gap. `conductor trust` trusts every git checkout under checkouts.root, ignoring sources and excludes: about 330 checkouts for 6 repositories.

## Acceptance

- trust marks only the checkouts of the instance's sources after excludes (and its records); a test counts the trusted paths.
