---
format: aep.planning-md/3
id: review-result:conf-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, session-confinement, round 1
relations:
- reviews: epic:session-confinement
- reviews: story:substrate-session-provider
- reviews: story:substrate-gate-provider
- reviews: story:instance-usage-metrics
- reviews: story:session-envelope-model
- reviews: story:session-writes-bwrap
- reviews: story:quota-support-report
- reviews: decision-blocker:project-quota-filesystem
revision: 1
---
needs-revision

- story:substrate-session-provider — the acceptance "meets the epic's acceptance (limits, writes, usage, SendMessage, `claude agents`)" borrows checks that are systemd-specific (`systemctl --user show`, "the instance's slice", "its own cgroup"), so it cannot be passed or failed under substrate as written. It also adds a second outcome, "selectable per instance" — .engineering/planning/story/substrate-session-provider.md:23 (epic checks at .engineering/planning/epic/session-confinement.md:22-27)
- story:substrate-gate-provider — the acceptance does not say whether a test or a recorded run checks it. "`task check` (or `cargo test`)" leaves the command open. It joins two outcomes, running as an exec with usage recorded and a refusal reported by name — .engineering/planning/story/substrate-gate-provider.md:26
- story:instance-usage-metrics — the dashboard display and the watch threshold line are independent outcomes in one acceptance. The fixtures at line 41 test only the three data sources, so nothing checks that the dashboard shows the values — .engineering/planning/story/instance-usage-metrics.md:37-43
- story:session-envelope-model — the bullet "`conductor doctor` reports which providers this host supports and why not" has no check. The Tests bullet lists only config parsing, refusal and the snapshot measurement — .engineering/planning/story/session-envelope-model.md:37-38
- story:session-writes-bwrap — "the network and the harness's own state stay usable" has no check. The recorded live run covers only writes outside and inside the writable paths — .engineering/planning/story/session-writes-bwrap.md:27-30
- story:quota-support-report — the acceptance does not state what makes `project quotas: yes` versus `no`, so the fixture mount table cannot show whether the rule is right. The acceptance should name the mount option, for example `prjquota` or `pquota` — .engineering/planning/story/quota-support-report.md:22 (blocker options at .engineering/planning/decision-blocker/project-quota-filesystem.md:11)
- decision-blocker:project-quota-filesystem — the body has a Question but no statement of what clears it or where the operator's choice is recorded. The upstream blockers each have "What would clear it". `story:instance-storage-quota` line 29 ("Once the operator chose option A or B") has no recorded decision to point at — .engineering/planning/decision-blocker/project-quota-filesystem.md:11

What I read: all 15 ids (14 artifacts), each with `aep plan artifact show`, plus `aep plan artifact kinds` and `aep plan artifact lifecycle` for epic, story, architecture-decision-record, upstream-blocker and decision-blocker. I read both evidence pages. I ran `git grep` for `conductor:start`, `snapshot sessions`, `doctor` and `"pid"`, and all exist. `story:substrate-session-spike` exists and is implemented; `story:instance-cgroup-limits` and `story:bash-sandbox` exist and are archived.

What I could not establish:
- Whether `claude agents --json` entries carry the pid that `story:instance-usage-metrics` relies on. Only `collect/sessions.rs:258` and `guard/rules.rs:1435` read `pid`, and I did not run the command.
- Two items belong to other lanes and did not set the verdict.
  - Scope: `story:instance-usage-metrics` Why still names the archived `instance-cgroup-limits`, and `story:instance-storage-quota` and `story:quota-support-report` decompose a session-confinement epic whose acceptance does not mention storage.
  - Design: the epic's Outcome opens with a garbled "unconfined inside and inside a box".
- The epic, ADR and spike acceptances, `session-limits-systemd`, `instance-storage-quota` and `upstream-blocker:substrate-exec-in-checkout` and `substrate-long-sessions` raised no acceptance finding.

```findings
- file: .engineering/planning/story/substrate-session-provider.md
  line: 23
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance 'meets the epic's acceptance' borrows systemd-specific checks (systemctl --user show, instance slice, own cgroup) that cannot be passed under substrate as written, and adds a second outcome, 'selectable per instance'"
- file: .engineering/planning/story/substrate-gate-provider.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names no test or recorded run, leaves the command open ('task check (or cargo test)'), and joins two outcomes: running as an exec with usage recorded, and a refusal reported by name"
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the dashboard display and the watch threshold line are independent outcomes in one acceptance, and the fixtures test only the data sources, so nothing checks that the dashboard shows the values"
- file: .engineering/planning/story/session-envelope-model.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the bullet 'conductor doctor reports which providers this host supports and why not' has no check; the Tests bullet lists only config parsing, refusal and the snapshot measurement"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'the network and the harness's own state stay usable' has no check; the recorded live run covers only writes outside and inside the writable paths"
- file: .engineering/planning/story/quota-support-report.md
  line: 22
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance does not state what makes 'project quotas: yes' versus 'no', so the fixture mount table cannot show whether the rule is right"
- file: .engineering/planning/decision-blocker/project-quota-filesystem.md
  line: 11
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body holds a Question but no statement of what clears it or where the operator's choice is recorded, so story:instance-storage-quota's 'once the operator chose option A or B' has no observable trigger"
```
