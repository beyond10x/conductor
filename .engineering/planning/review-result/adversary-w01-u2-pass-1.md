---
format: aep.planning-md/3
id: review-result:adversary-w01-u2-pass-1
kind: review-result
status: active
title: Adversary, wave 01 U2 (instance-records-start), pass 1
relations:
- reviews: story:instance-records-start
revision: 1
---
unit: U2 `story:instance-records-start`, head `539ab74` plus the adversary's uncommitted test additions
verdict: NEEDS-CHANGE
cases: executed 10→16, red 4
origin: introduced 4 / pre-existing 2 / undecided 1

Cases added to `crates/conductor/tests/taskfile.rs`: trailing-slash records key (red), records path
with a line break (red), start refuses without a findable conductor agent (red), start uses the
role's agent (red), agents:link links nothing when only the last adapter is in the way (green,
kills a mutant of the all-or-nothing check), a records path with shell metacharacters (green).

Could not break: empty `config show` output refuses start/attach/restart; restart refuses before
stopping; an unreadable `~/.claude.json` never replaces the file; records under the checkouts root
gives one key; agents:link refuses a real file, a dangling link and a foreign link; quoting.

```findings
- file: Taskfile.yml
  line: 105
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: trust writes the records path verbatim, so a records path ending in / is trusted under a key no session working directory ever has
- file: Taskfile.yml
  line: 107
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a records path containing a line break is split into separate trusted keys, trusting a directory outside the records and the checkouts root
- file: Taskfile.yml
  line: 36
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: conductor:start runs --agent conductor outside the checkout without checking the agent can be found there, so a restart before agents:link starts conductor without its agent
- file: Taskfile.yml
  line: 36
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the conductor role's configured agent is ignored although the spec defines a role's agent as the agent the role starts with
- file: crates/conductor/tests/taskfile.rs
  line: 482
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's agents:link case stays green with the all-or-nothing refusal check at Taskfile.yml:129 deleted
- file: Taskfile.yml
  line: 108
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the mtime check in trust has one-second resolution, so a Claude Code write to ~/.claude.json in the same second is lost
- file: Taskfile.yml
  line: 105
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: a records path through a symlink is trusted by its logical path while the guard matches records by real path; whether Claude keys projects by the physical path is unverified
```
