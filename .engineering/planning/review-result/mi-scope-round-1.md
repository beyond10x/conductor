---
format: aep.planning-md/3
id: review-result:mi-scope-round-1
kind: review-result
status: active
title: Scope critic, multi-instance-host, round 1
relations:
- reviews: epic:multi-instance-host
- reviews: story:substrate-session-spike
- reviews: story:instance-session-names
revision: 1
---
needs-revision

instance-session-names — it and instance-scoped-tasks both claim "Taskfile lookups use the instance's full session names", so both would be marked done for one outcome; drop "every Taskfile lookup" from this story and leave it to instance-scoped-tasks — .engineering/planning/story/instance-session-names.md:22 (the same promise is at .engineering/planning/story/instance-scoped-tasks.md:19)
substrate-session-spike — the spec's design item 5(b), per-instance cgroup limits for cpu, memory and pids, is "without touching each other's … resources" in the epic, and no story claims it or says it is deferred pending the spike; the acceptance names only "the stories it would take" — .engineering/planning/story/substrate-session-spike.md:23 (promise: .engineering/planning/specification/multi-instance-host.md:45, epic at .engineering/planning/epic/multi-instance-host.md:14)

**What you read:** 10 artifacts (the epic, the specification, 7 stories and the decision-blocker), with `aep plan artifact show <id>` for each plus `aep plan artifact graph`. I extracted 9 promises from the epic and specification design and traced 8 to an item. The untraced one is the 5(b) limits, which only the spike touches. The 9: sessions untouched, messages untouched, resources untouched, operator sees cost, instance-qualified names, scoped tasks and a per-instance dashboard port, host ledger, guard, and storage quota. The usage metrics (5a) and the storage quota are counted among them. Nothing in the set reaches beyond the parent. The `conductor doctor` report in instance-storage-quota is only loosely traceable to design 6, which I did not count as a finding.

**What I could not establish:** I was not given the drafter's report, so I cannot tell whether the 5(b) deferral was recorded anywhere outside the artifacts.

**Out of my lane:** instance-session-names and instance-scoped-tasks both edit `Taskfile.yml` and `config.rs`, which is for parallel-safety. The storage-quota story is blocked by an open decision, and the decision-blocker has no acceptance by design.

```findings
- file: .engineering/planning/story/instance-session-names.md
  line: 22
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "instance-session-names and instance-scoped-tasks both claim that Taskfile lookups use the instance's full session names, so both will be marked done for one outcome; drop 'every Taskfile lookup' from this story and leave it to instance-scoped-tasks"
- file: .engineering/planning/story/substrate-session-spike.md
  line: 23
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "specification design 5(b), per-instance cgroup limits for cpu, memory and pids, is promised by the epic's 'without touching each other's resources' and no story claims it or says it is deferred pending the spike; the acceptance names only 'the stories it would take'"
```
