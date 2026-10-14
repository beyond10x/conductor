---
format: aep.planning-md/3
id: initiative:adoption
kind: initiative
status: draft
title: Engineers adopt conductor because it runs confined by default
revision: 1
---
## Why

The operator, 2026-10-14: "we really need to push the bwrap sandboxing ... for conductor ...
engineers are now requesting it. we need it. otherwise no adoption. make adoption a goal!" Engineers
will not hand a repository to sessions that run with full permissions as their own user, and the
public site's guide says conductor has no operating-system sandbox yet.

## Outcome

An engineer outside the founding team installs conductor from its public site on Linux, runs it
with every session confined (limits, a write fence, read-only forge credentials) by default, and
can see that it is. macOS and Windows confinement come later.
