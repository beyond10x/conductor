---
format: aep.planning-md/3
id: story:context-line-names-nested-repository
kind: story
status: draft
title: The context line names a nested repository, not its group folder
summary: a three-deep checkout is labelled by its first folder
revision: 2
---
## Why

Observed 2026-11-07: the watch line read `context: <group> 303k tokens (session …)` for the
controller of `<group>/<sub>/<repo>`, a checkout three folders deep, reached through a symlink
`<root>/<sub>/<repo>`. `repository_of` (`crates/conductor/src/watch.rs:2411`) labels a session by
the first folder of its working directory below the checkouts root or the trees directory, so a
nested repository is labelled by its group folder, and every controller under that group gets the
same label.

## Acceptance

- The context line of a controller session names its repository as the instance knows it (its
  path under `checkouts.root`, symlinks resolved), for checkouts one, two and three folders deep
  and for their managed trees.
- A session whose directory is no checkout is labelled by its session name, as today.
