---
format: aep.planning-md/3
id: review-result:mi-scope-round-2
kind: review-result
status: active
title: Scope critic, multi-instance-host, round 2
relations:
- reviews: epic:multi-instance-host
- reviews: story:instance-session-names
revision: 1
---
needs-revision

instance-scoped-tasks — the Taskfile's own `-n conductor` and `-n conductor-dev` start commands (design 1: "Every session an instance starts is named `<prefix>-<role or repository>`") sit in neither story's scope. `instance-session-names` lists them in its acceptance but only reaches `.agents/conductor.md`, and it hands "Taskfile lookups" to this story. This story's "resolves sessions by the active instance's full names" says nothing about naming them at start. Add the `conductor:start`, `conductor:restart` and conductor-dev start naming (`<prefix>-conductor`, `<prefix>-conductor-dev`) to this story, and drop conductor and conductor-dev from `instance-session-names`' bullet. — `Taskfile.yml:42` and `Taskfile.yml:103` (the `claude --bg -n …` starts), `.engineering/planning/story/instance-session-names.md:28` (the claim), and `.engineering/planning/story/instance-session-names.md:36` (the Taskfile hand-off)

Round-1 findings:
- **Duplicate Taskfile lookup claim: fixed.** `instance-session-names` now ends "Taskfile lookups are story `instance-scoped-tasks`", and `instance-scoped-tasks` is the only story that claims them.
- **Design 5(b) gap: fixed.** `instance-cgroup-limits` claims it (`instances[].limits`, depends on the spike and `instance-scoped-tasks`). `substrate-session-spike` stays a no-code spike.

**What I read:**
- 14 artifacts: the epic, the specification, 9 stories, the decision-blocker and `review-result:mi-scope-round-1`.
- I ran `aep plan artifact show` on each and `aep plan artifact graph`.
- I also checked `Taskfile.yml` and `.agents/conductor.md` for where the session names are set.
- I extracted 10 promises and traced all 10 to an item: sessions untouched, messages untouched, one ledger with refusal numbers, operator sees cost, qualified names, scoped tasks and per-instance port, guard, 5(a) usage, 5(b) limits and storage quota. Only the ownership of the Taskfile start names (design 1) is unclear, as the finding says.
- Nothing reaches beyond the parent. The `systemd-run` fallback in `instance-cgroup-limits` and the `conductor doctor` report in `quota-support-report` are loosely traceable to design 5(b) and 6. I did not count either as a finding.

**What I could not establish:**
- Whether `install-prerequisites` (status `active`) delivers the `conductor doctor` that `quota-support-report` depends on. That is not my lane.
- Out of my lane: `decision-blocker:project-quota-filesystem` has no acceptance, by design, and `instance-storage-quota` stays blocked on it. If the operator picks option C, no story claims "storage policed by the host ledger and the watch" (design 6). That is a decision outcome, not a gap in this set.
- I wrote one scratch file, `~/.cache/claude-tmp/scopecrit-r2-4117389.txt`, outside the store and the repository, to read truncated output. I did not delete it.

```findings
- file: .engineering/planning/story/instance-session-names.md
  line: 28
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Taskfile's own start commands (Taskfile.yml:42 `-n conductor`, Taskfile.yml:103 `-n conductor-dev`) are claimed in this story's acceptance but outside its scope (.agents/conductor.md only), and the Taskfile is handed to instance-scoped-tasks, whose acceptance only says tasks resolve sessions by full names; move the conductor and conductor-dev start naming into instance-scoped-tasks and drop it from this bullet"
```
