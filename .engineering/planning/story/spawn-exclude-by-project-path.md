---
format: aep.planning-md/3
id: story:spawn-exclude-by-project-path
kind: story
status: draft
title: Spawn and the watch read a GitLab repository's name as the snapshot does
scope:
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
revision: 3
---
## Outcome

`conductor spawn --repository <directory>` refuses a GitLab checkout that an `exclude` entry
names by its project path, as it refuses one an entry names by its directory, and the watch
asks a GitLab project by the source that listed it, not by its name alone.

## Why

Since `story:checkout-name-differs-from-project` an `exclude` entry matches a checked-out
GitLab project by its directory name or by its project path in the snapshot and in
`config workspace`. Two callers still read names their own way (adversary pass 1 of that story,
finding 4, and the implementor's round 2 report):

- `spawn::checkout` checks `exclude` against the directory name only, so a direct
  `conductor spawn --repository checks` under `exclude: [example-checks]` is not refused. The
  dispatch loop does not reach it, because the snapshot no longer lists the repository.
- The watch keys its project map by name per forge and takes the first entry, and it decides
  between `gh` and the project map by name alone. Until a new snapshot completes after an
  upgrade, an instance that also has a `github` source asks `gh` for the old project name.

## Acceptance

Given `exclude: [example-checks]` and a checkout `checks` of the project `example-checks`, when
`conductor spawn --repository checks` runs, then it exits 1 and names the `exclude` entry.

Given a snapshot that names a GitLab repository the watch's project map lacks, and a `github`
source in the same instance, when the watch runs a pass, then no `gh` call names that
repository.
