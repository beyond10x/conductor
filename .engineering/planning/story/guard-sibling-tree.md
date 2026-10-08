---
format: aep.planning-md/3
id: story:guard-sibling-tree
kind: story
status: draft
title: A relative cd into a sibling tree of the same repository is not another repository
revision: 1
---
## Why

Observed in a live run: `cd ../<repo>-<other tree>` from a managed tree of a repository into a
sibling managed tree of the same repository was denied as "reaches another repository". Both trees
are under `<trees>/<repo>/`.

## Acceptance

From `<trees>/<repo>/<a>`, `cd ../<b>` with `<b>` a tree of the same repository is allowed;
`cd ../../<other>/x` stays denied.
