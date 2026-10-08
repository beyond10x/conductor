---
format: aep.planning-md/3
id: review-result:mi-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, multi-instance-host, round 1
relations:
- reviews: epic:multi-instance-host
- reviews: story:instance-session-names
- reviews: story:host-grants
- reviews: story:instance-usage-metrics
- reviews: story:instance-storage-quota
revision: 1
---
needs-revision

story:instance-session-names — the acceptance says "The sessions collector and the watch place a session by its prefix as well as its directory" and "the separator follows it", but names no output or test that tells placed from unplaced. The separator is already fixed as `-` in the bullets above it, so "the separator follows" reads the same whatever the spike finds — .engineering/planning/story/instance-session-names.md:24-25

story:host-grants — "safe for two processes at once" and "releases it when the session leaves" name no check. The only test, "two instances' grants against one floor; the second is refused", is sequential and never exercises concurrency or release — .engineering/planning/story/host-grants.md:22,25

story:instance-usage-metrics — the acceptance claims a second source (substrate's `GET /v1/metrics` where a session runs under substrate), but "Tests read fixture cgroup files" covers only the cgroup v2 path. If the spike's go/no-go is no-go, that branch can never be observed — .engineering/planning/story/instance-usage-metrics.md:19-23

story:instance-storage-quota — "Per-instance project quota ids ... set through substrate or `setquota -P`, with the limit from config" names no observable check. The either/or tool is unresolved, there is no refusal or EDQUOT to observe, and this host's `/` has no `prjquota`, so nothing can be run against it. Only the `conductor doctor` bullet is checkable — .engineering/planning/story/instance-storage-quota.md:19-20

epic:multi-instance-host — the epic has an Outcome but no Acceptance section. Its single sentence joins two outcomes with "and", and "without touching each other's ... resources" cannot be observed: the specification says storage is "policed ... not enforced" (design 6) and the blocker is open. The epic could not be shown done under blocker option C — .engineering/planning/epic/multi-instance-host.md:14

**What I read:** all 10 artifacts, using `aep plan artifact show <id>` for each. I also ran `aep plan artifact kinds` and `aep plan artifact lifecycle` for specification, epic, story and decision-blocker. I read the store files for line citations and `spec/drafts/host.yaml` in full. I grepped the tree for `build_size`, `docs/analysis` and `guard_rules.rs`.

**What I could not establish:**
- `docs/analysis/` does not exist yet. That is fine for a new page, so I did not count it.
- Whether the other critics' lanes cover the points below. These are out of my lane and did not set the verdict:
  - `story:instance-session-names` carries an in-story "spike first" on the `claude --bg -n` characters. It has no relation to `story:substrate-session-spike`, and the two are different spikes (design and scope).
  - `story:instance-storage-quota` is blocked by an operator decision. If the decision is option C, the story has nothing to do (scope).
- I did not flag multi-bullet acceptance lists. They follow the store's precedent in `story:portable-trust`, even though SKILL.md:426 says "one acceptance statement per story".
- `decision-blocker:project-quota-filesystem` and `specification:multi-instance-host` carry no acceptance section by kind, and I found no defect in either.
- `story:instance-scoped-tasks`, `story:cross-instance-message-guard` and `story:substrate-session-spike` have checkable acceptances, so I raised no finding on them.

```findings
- file: .engineering/planning/story/instance-session-names.md
  line: 24
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says 'The sessions collector and the watch place a session by its prefix as well as its directory' and 'the separator follows it', but names no output or test that tells placed from unplaced; the separator is already fixed as `-` in the bullets above, so 'follows the spike' reads the same whatever the spike finds"
- file: .engineering/planning/story/host-grants.md
  line: 22
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'safe for two processes at once' and 'releases it when the session leaves' name no check; the only test, 'two instances' grants against one floor; the second is refused', is sequential and never exercises concurrency or release"
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 19
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance claims a second source (substrate's `GET /v1/metrics`) but 'Tests read fixture cgroup files' covers only the cgroup v2 path, so that branch can never be observed, and it is dead if the spike's go/no-go is no-go"
- file: .engineering/planning/story/instance-storage-quota.md
  line: 19
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'Per-instance project quota ids ... set through substrate or `setquota -P`, with the limit from config' names no observable check: the tool is an unresolved either/or, no refusal or EDQUOT is named, and this host's `/` has no prjquota so nothing can be run against it; only the doctor bullet is checkable"
- file: .engineering/planning/epic/multi-instance-host.md
  line: 14
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic has an Outcome but no Acceptance section; its one sentence joins two outcomes with 'and', and 'without touching each other's ... resources' cannot be observed when the specification says storage is policed, not enforced, and the quota blocker is open"
```
