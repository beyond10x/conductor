---
format: aep.planning-md/3
id: review-result:mi-design-round-1
kind: review-result
status: active
title: Design critic, multi-instance-host, round 1
relations:
- reviews: epic:multi-instance-host
- reviews: story:instance-session-names
- reviews: story:host-grants
- reviews: story:instance-storage-quota
revision: 1
---
needs-revision

- story:instance-session-names — its acceptance needs the `SessionName` type "of `spec/drafts/host.yaml`" and types `Controller.session_name` with it, but that type lives in the `conductor.host` domain, and `spec/domains/host.yaml` does not exist yet. story:host-grants creates that file, and no edge or scope line records the order. Either this story defines `SessionPrefix` and `SessionName` itself, in the config or dispatch domain and in its scope, and the draft imports them. Or it takes a `depends_on` edge to story:host-grants. Only the first fits, because host-grants needs the type too (next finding). — `.engineering/planning/story/instance-session-names.md:18-19`, `.engineering/planning/story/host-grants.md:18,29`, `ls spec/domains` (no `host.yaml`)
- story:host-grants — `HostGrant.session_name` and `TakeHostGrant.session_name` are typed `conductor.host.SessionName`, whose meaning (`<prefix>-<repository>`) story:instance-session-names defines. Its test of "two instances' grants" also needs distinct prefixes. It names no `depends_on` edge to story:instance-session-names, so `aep plan artifact waves` can place the two side by side. Add the edge once the type has one owner. — `spec/drafts/host.yaml:51-52,110-111`, `aep plan artifact graph` (no edge from story:host-grants)
- story:instance-storage-quota — two things in one item, split on the open decision. The first is the `conductor doctor` report on project-quota support, which the decision does not affect. The second is setting per-instance quota ids, which the decision blocks. Under option (C), "no quotas", the whole story would lapse, including the doctor check. Split the doctor check into its own unblocked story, leaving enforcement behind `decision-blocker:project-quota-filesystem`. — `.engineering/planning/story/instance-storage-quota.md:20-21`, `aep plan artifact graph` (`blocks` edge covers the whole story)

What I read: 10 artifacts, with `aep plan artifact show` on each, plus `relations`, `graph` and `validate` (valid, only unrelated scope warnings on other stories). I also read `spec/drafts/host.yaml` and grepped `spec/domains` and `crates/conductor/src/collect/sessions.rs`.

Edges and acceptance reads:
- I walked all 15 edges in the store's graph, including the 2 `reviews` edges outside the set. There is no cycle and no serialising chain; the longest `depends_on` chain is 2.
- I traced 5 acceptance reads to a producer inside the set. Edges record 3 of them: the two `depends_on` edges to instance-session-names, plus usage-metrics `informed_by` the spike. The 2 unrecorded reads are the two findings above.
- usage-metrics reading session-to-instance placement was ruled out. The sessions collector already places by directory, so it does not need the prefix work.

What I could not establish:
- Whether instance-scoped-tasks is two things in one item. It covers session-scoped tasks and a per-instance dashboard port, and the port does not obviously depend on sessions. I left it as an unease, not a finding.
- Out of my lane, for the parallel-safety critic: five stories edit `spec/domains/config.yaml` and `crates/conductor/src/config.rs`. They are instance-session-names, instance-scoped-tasks, host-grants, usage-metrics and storage-quota. Only the edge between instance-session-names and instance-scoped-tasks records any of that.
- Out of my lane, for the scope critic: the spike's go/no-go leads to the limits step (5b) of the specification, and no story covers it.

```findings
- file: .engineering/planning/story/instance-session-names.md
  line: 18
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance needs the SessionName type of spec/drafts/host.yaml for Controller.session_name, but that type lives in the conductor.host domain whose file spec/domains/host.yaml is created by story:host-grants, and no edge or scope line records the order; define SessionPrefix and SessionName in this story's own domain and scope, or add a depends_on edge to story:host-grants"
- file: spec/drafts/host.yaml
  line: 51
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:host-grants types HostGrant.session_name and TakeHostGrant.session_name with the SessionName that story:instance-session-names defines, and no depends_on edge runs from story:host-grants to story:instance-session-names"
- file: .engineering/planning/story/instance-storage-quota.md
  line: 20
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "two things in one item: the conductor doctor project-quota report (line 21) is independent of the open decision while quota enforcement (line 20) is blocked by it, so the blocks edge holds back deliverable work and option C would lapse the doctor check too; split the doctor check into its own unblocked story"
```
