---
format: aep.planning-md/3
id: review-result:adversary-w01-u2-pass-2
kind: review-result
status: active
title: Adversary, wave 01 U2 (instance-records-start), pass 2
relations:
- reviews: story:instance-records-start
revision: 1
---
unit: U2 wave 01 (story:instance-records-start), commit 539ab74 plus the pass-1 correction
verdict: NEEDS-CHANGE
cases: executed 20→23, red 2
origin: introduced 5 / pre-existing 1 / undecided 0

Cases added: start refuses an agent found only outside the agents directories (red); restart
stops nothing when start would refuse (red); restart checks the role's agent (green, kills a
mutant of restart's AGENT). Could not break: records without search bit, records path ending in
a newline, records path that is a file, checkouts root with a trailing slash, restart order of
the records and agent checks; the changed pass-1 cases keep their meaning.

```findings
- file: Taskfile.yml
  line: 35
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the agent precondition joins the agent name into a path unconfined, so the config-valid agent ../../stray is satisfied by ~/stray.md and start runs claude --agent ../../stray
- file: Taskfile.yml
  line: 71
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: conductor:restart stops and removes the running conductor before conductor:start checks the harness, so a role conductor on codex leaves no conductor running
- file: Taskfile.yml
  line: 66
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: restart's duplicated AGENT variable had no case with a non-default role agent, so replacing it by the constant conductor left the suite green
- file: docs/config.md
  line: 134
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the config reference still says the start tasks pass --agent conductor and lists roles[].agent as read only by the conductor profile
- file: README.md
  line: 106
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: README says task conductor:start starts the session in this checkout, while it now starts in the instance's records directory
- file: Taskfile.yml
  line: 35
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the agent check tests for a file named after the agent, so a plugin-namespaced agent the config's alphabet admits is always refused although claude matches agents by frontmatter name
```
