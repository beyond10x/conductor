---
format: aep.planning-md/3
id: review-result:conf-scope-round-1
kind: review-result
status: active
title: Scope critic, session-confinement, round 1
relations:
- reviews: epic:session-confinement
- reviews: story:session-envelope-model
- reviews: story:substrate-gate-provider
- reviews: story:instance-storage-quota
- reviews: story:quota-support-report
- reviews: story:instance-usage-metrics
- reviews: story:substrate-session-provider
- reviews: story:session-writes-bwrap
- reviews: architecture-decision-record:external-confinement
revision: 1
---
needs-revision

Findings (each is `artifact — reason — citation`):

1. epic:session-confinement — Two promises have no claiming item. The epic promises "a session that cannot be boxed either not started or started and reported as unboxed" and checks it with "`confinement.mode: strict` and no provider available, `conductor:start` refuses". `session-envelope-model` only parses the config and records a measurement, and `session-limits-systemd` never mentions a refusal or an unboxed start. The natural owner is `session-limits-systemd`, which already scopes `Taskfile.yml`. The ADR's "strict mode refuses a child whose envelope evidence does not match the request" is claimed nowhere either. — .engineering/planning/epic/session-confinement.md:15
2. epic:session-confinement — "and still answers SendMessage and appears in `claude agents --json`" is checked only by the spike, a go/no-go page. No implementing story's acceptance carries it, and the recorded live run in `session-limits-systemd` names none of these checks. — .engineering/planning/epic/session-confinement.md:22
3. story:session-envelope-model — The epic promises limits "per instance and session", but the request defines `limits` per role only and `session-limits-systemd` gives each instance a slice with no ceiling of its own. The superseded `instance-cgroup-limits` carried `instances[].limits`, so the per-instance cap was dropped without a note. — .engineering/planning/story/session-envelope-model.md:31
4. story:session-envelope-model — The ADR's request has "limits, writable and readable paths, programs, network", plus lifetime in its list. The story carries limits, writable paths and network (open only). Readable paths, programs and lifetime are dropped with nothing saying so. — .engineering/planning/architecture-decision-record/external-confinement.md:48
5. story:session-envelope-model — The ADR's "envelope provider seam" is claimed by no item. The seam is the request, a provider that starts the harness in it, and the measurement. `substrate-session-provider` also promises "selectable per instance", yet no `provider` config key exists in `session-envelope-model`'s spec. — .engineering/planning/architecture-decision-record/external-confinement.md:48
6. story:substrate-gate-provider — Running gates as substrate execs is not traceable to the epic or the ADR. Both concern harness sessions, and the epic names substrate only as a later provider for sessions. Either the epic gains the promise, or the story, and with it `upstream-blocker:substrate-exec-in-checkout`, leaves the epic. — .engineering/planning/story/substrate-gate-provider.md:20
7. story:instance-storage-quota — Storage quotas are in neither the epic outcome nor the ADR. The only reason the story sits here is `epic:multi-instance-host`'s "quotas … moved to epic `session-confinement`", which the receiving epic never promises. The story still depends on `host-grants`, which belongs to the other epic. `decision-blocker:project-quota-filesystem` follows it. — .engineering/planning/epic/multi-instance-host.md:30
8. story:quota-support-report — The same reach as finding 7: a `conductor doctor` filesystem and quota report has no sentence in the epic or the ADR to trace to. — .engineering/planning/story/quota-support-report.md:17
9. story:instance-usage-metrics — The watch threshold line ("prints one line when an instance's resident memory or CPU passes a config threshold") is alerting. The epic promises only "usage attributed per session", and its acceptance is the dashboard showing CPU and memory. — .engineering/planning/story/instance-usage-metrics.md:43
10. story:substrate-session-provider — "usage" in its acceptance and the "reads `GET /v1/metrics`" source in `instance-usage-metrics` claim the same substrate usage reading. Both would be marked done for one outcome. — .engineering/planning/story/substrate-session-provider.md:24
11. docs/analysis/2026-10-09-substrate-benefits.md — The operator's ask was "what are the benefits we would gain by using substrate", "same way … as metaharness". The page evaluates only `claude` itself inside substrate. It never assesses the route the ADR names, where the harness runs unconfined outside and its tools execute inside substrate through an MCP server (`mcp-serve --workspace --writable --allow-program`). That route sidesteps the page's own blockers (AF_UNIX, credentials, the 1 h session limit). The word "mcp-serve" appears nowhere on the page, and § 4 does not mention it. — .engineering/planning/architecture-decision-record/external-confinement.md:36
12. story:session-writes-bwrap — The story rewrites the README Security model and design § 7 on what the box enforces and what the guard still does. It does not say what becomes of draft `story:guard-bash-write-targets`, which extends the Bash heuristics the ADR says "stop mattering for safety" and edits the same README and design § 7 sections. — .engineering/planning/story/session-writes-bwrap.md:31

**What I read:** 16 artifacts (epic, ADR, nine stories, the two upstream blockers and the decision blocker, plus `epic:multi-instance-host`, `story:instance-cgroup-limits`, `story:bash-sandbox`, `story:guard-bash-write-targets`, `story:host-grants` and `story:codex-controller-profile` for coverage elsewhere), the benefits page, and the metaharness `launch.rs` and `AGENTS.md` lines the ADR cites. Commands: `aep plan artifact show`, `graph`, `kinds`, `relations`. I extracted 16 promises from the epic, the ADR and the operator's words. 10 are traced to an item, 3 are narrowed (findings 2, 3 and 4), and 3 are untraced (findings 1, 5 and 11).

**What I could not establish:**
- Whether "every harness session" includes Codex sessions. All items are Claude-only, and `story:codex-controller-profile` has its own sandbox.
- The epic's first outcome sentence reads "runs unconfined inside and inside a box". That is a wording defect, not a scope one.
- Out of my lane and not counted in the verdict:
  - `instance-usage-metrics` Why still points at the archived `instance-cgroup-limits`, and it has no `depends_on` on `session-envelope-model`.
  - The two substrate stories have no `depends_on` on the spike or the envelope model.
  - `session-limits-systemd`'s "one recorded live run" names no checks. This one is for the acceptance critic.
  - The substrate stories' scope omits `spec/domains/config.yaml`, although spec-first applies. This one is for the design critic.

```findings
- file: .engineering/planning/epic/session-confinement.md
  line: 15
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the promise that a session which cannot be boxed is refused under strict mode or started and reported as unboxed, and the acceptance that conductor:start refuses with no provider, are claimed by no item; session-limits-systemd (it scopes Taskfile.yml) would most naturally take it"
- file: .engineering/planning/epic/session-confinement.md
  line: 22
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance that the boxed controller still answers SendMessage and appears in claude agents --json is checked only by the spike go/no-go; no implementing story's acceptance carries it"
- file: .engineering/planning/story/session-envelope-model.md
  line: 31
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the epic promises limits per instance and session, but the request defines limits per role only and session-limits-systemd gives the instance slice no ceiling; the superseded instance-cgroup-limits carried instances[].limits and the drop is not recorded"
- file: .engineering/planning/architecture-decision-record/external-confinement.md
  line: 48
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the ADR's envelope request has readable paths, programs and lifetime, and session-envelope-model carries limits, writable paths and network only, with nothing recording that readable paths, programs and lifetime are dropped"
- file: .engineering/planning/architecture-decision-record/external-confinement.md
  line: 48
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the ADR's envelope provider seam (request, provider, measurement) is claimed by no item, and substrate-session-provider's 'selectable per instance' has no provider key in session-envelope-model's config; session-envelope-model would most naturally take both"
- file: .engineering/planning/story/substrate-gate-provider.md
  line: 20
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "running gates as substrate execs is not traceable to any sentence of the epic or ADR, which concern harness sessions and name substrate only as a later session provider; the epic must gain the promise or the story and upstream-blocker:substrate-exec-in-checkout must leave it"
- file: .engineering/planning/epic/multi-instance-host.md
  line: 30
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "storage quotas were handed to session-confinement ('quotas ... moved to epic session-confinement') but that epic's outcome and the ADR promise no storage quota, so instance-storage-quota and decision-blocker:project-quota-filesystem decompose an epic that does not ask for them"
- file: .engineering/planning/story/quota-support-report.md
  line: 17
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a doctor report on project-quota support has no sentence in the epic or ADR to trace to, for the same reason as instance-storage-quota"
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 43
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the watch threshold line on instance memory or CPU is alerting that the epic does not ask for; it promises only usage attributed per session, and its acceptance is the dashboard showing CPU and memory"
- file: .engineering/planning/story/substrate-session-provider.md
  line: 24
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the usage in its acceptance and the substrate GET /v1/metrics source in instance-usage-metrics claim the same substrate usage reading, so both would be marked done for one outcome"
- file: .engineering/planning/architecture-decision-record/external-confinement.md
  line: 36
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the benefits page, the operator's specific ask, evaluates only claude itself inside substrate and never the ADR's metaharness route of an unconfined harness whose tools run inside substrate through an MCP server, which avoids the AF_UNIX, credentials and 1 h blockers the page cites; add it to docs/analysis/2026-10-09-substrate-benefits.md section 2/3/4"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 31
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story rewrites README Security model and design section 7 on box versus guard but does not say what becomes of draft story:guard-bash-write-targets, which extends the Bash heuristics the ADR says stop mattering and edits the same two sections"
```
