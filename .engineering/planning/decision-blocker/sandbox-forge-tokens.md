---
format: aep.planning-md/3
id: decision-blocker:sandbox-forge-tokens
kind: decision-blocker
status: open
title: Nobody has created read-only forge tokens for boxed sessions
relations:
- blocks: story:session-forge-credentials
revision: 1
---
## Question

Which read-only forge tokens do boxed sessions get, per role: a fine-grained GitHub token with
read access to the instance's repositories, and a GitLab token with `read_api` on the instance's
group, set in the environment conductor's provider passes (`roles[].forge_token_env`)?

## Why it is the operator's

A credential is class H (design § 5): only the operator creates and stores tokens. Without them a
boxed session has no forge access at all, which is safe but stops controllers reading PRs and CI.

## Default

Until answered, `story:session-forge-credentials` ships its code and doctor warning; its live run
waits.
