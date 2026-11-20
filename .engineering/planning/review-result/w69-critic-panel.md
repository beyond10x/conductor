---
format: aep.planning-md/3
id: review-result:w69-critic-panel
kind: review-result
status: active
title: Critic panel, wave 69 (supervisor-watch-command)
summary: Four lenses PASS-WITH-CHANGES, all taken in; split off GREEN and CIEND into supervisor-watch-forge-triggers
relations:
- reviews: story:supervisor-watch-command
- reviews: story:supervisor-watch-forge-triggers
prose_only: a four-lens critic panel whose required edits were taken into the stories directly
revision: 1
---
## Verdicts

1. Cited facts: PASS-WITH-CHANGES. Every cited line holds on the tree. Taken in: the error for an
   unnamed instance exists (`conductor.instance.UnknownInstance`, instance.yaml:384) and is reused
   instead of a new `InstanceNotConfigured`; "reuses its readers" was false for GREEN (the pulls
   reader prints no draft, auto-merge or check end time, watch.rs:403-406, 2878); mailbox names are
   ULIDs (message.rs:619), so the mail cursor is a name, not a time; the manual unpushed check is of
   the product checkout, which no config key names (supervisor.md:203-207).
2. Testability: PASS-WITH-CHANGES. Taken in: the seam is named (a library entry over a
   `Sources`-like struct with a fixed clock, free-bytes reader and a millisecond read interval;
   `GIT_COMMITTER_DATE` in local repositories; fake `gh`/`glab` programs as tests/watch.rs:1350);
   binary cases only for exit codes with `--timeout 1s`; the MAIL and IDLE "waits on a person" rule
   made exact (`waits on`/`waiting for`); the killed-run case made deterministic (commit step not
   called); tests for the lock, a truncated log and rule 7 added.
3. Scope and spec-first: PASS-WITH-CHANGES. Taken in: the config keys as
   `conductor.config.SupervisorWatch` under `globals.supervisor`, `docs/config.md`, the whole
   trigger enum and `--hold` in the first story's spec, `--checkout`, `SupervisorWatchRunning`.
   Too large for one unit: split into the command with eight host-local triggers and
   `story:supervisor-watch-forge-triggers` (GREEN, CIEND), which depends on it.
4. Risk/adversary: PASS-WITH-CHANGES. Taken in: the snapshot cannot carry GREEN (no check end time
   or auto-merge, observation.yaml:1208), so forge reads use the instance's own sources, at most
   every 2 min, only where needed, back off on 403/429, a boxed instance without readable
   credentials is a note; an exclusive lock per host directory; cursor writes atomic, delivery at
   least once; first-run baseline with a note; only existing stores opened, read only; guard and
   `conductor.lease` untouched; profile cap 463 kept and asserted; `example-org` fixtures.
