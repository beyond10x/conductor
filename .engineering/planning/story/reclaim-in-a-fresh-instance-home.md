---
format: aep.planning-md/3
id: story:reclaim-in-a-fresh-instance-home
kind: story
status: draft
title: Reclaim in a fresh instance home prints no unassessed tree and follows XDG_STATE_HOME
tags:
- from-finding
revision: 1
---
## Outcome

A reclaim pass in an instance home that holds no managed worktree yet prints no `not assessed`
line, and a pass with `XDG_STATE_HOME` set reads the worktree archives where that variable puts
them.

## Why

Two things read on 2026-11-11 and 2026-11-12 while the instance-pods work ran; both were there at
`v1.3.0`.

- In a new instance pod (data volume empty but for the seeded records repository) each reclaim
  pass printed two lines of the form `kept  ? bytes  -  managed tree  <data>/trees: not assessed:
  failed: worktree inspect --json --workspace --max-entries 1 --repo <data>/records: exit status:
  1`, with the tool's `workspace-not-activated` answer inside. Nothing is wrong in that home: no
  workspace was ever activated because no tree was ever made. The lines come from
  `crates/conductor/src/gc.rs:175` and `:317`.
- `crates/conductor/src/gc.rs:590` and `crates/conductor/src/dashboard/waste.rs:497` join
  `.local/state/worktree/archives` to the home directory. With `XDG_STATE_HOME` set, as in the
  container image, the archives are elsewhere and the pass names a directory that does not exist.

## Acceptance

- Given an instance home whose records repository has no activated workspace, when a reclaim pass
  runs, then it prints no `not assessed` line for the managed trees and counts them as none; a
  failure of the inspection for any other reason is still printed.
- Given `XDG_STATE_HOME`, when reclaim or the dashboard's waste panel reads the worktree archives,
  then it reads them under that directory; a test sets the variable and holds the path.
