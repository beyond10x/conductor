---
format: aep.planning-md/3
id: review-result:conf-parallel-round-1
kind: review-result
status: active
title: Parallel-safety critic, session-confinement, round 1
relations:
- reviews: epic:session-confinement
- reviews: story:session-envelope-model
- reviews: story:quota-support-report
- reviews: story:instance-usage-metrics
- reviews: story:instance-storage-quota
- reviews: story:instance-scoped-tasks
- reviews: story:session-limits-systemd
- reviews: story:substrate-gate-provider
- reviews: story:substrate-session-provider
revision: 1
---
needs-revision

Findings, one line each (`artifact — reason — citation`). Paths below are under `.engineering/planning/story/`.

1. story:session-envelope-model — it shares `crates/conductor-model`, `config.rs` and `spec/domains/config.yaml` with `instance-storage-quota` and `instance-scoped-tasks`. It shares `conductor-model` and `config.rs` with `host-grants`. It shares `sessions.rs` too with `instance-session-names`. It has no `depends_on` edge and its body names none of them. `waves` only separates them by tie-break. An ordering edge recording the shared files, or splitting the surface, would fix it. — session-envelope-model.md:8,42-44; `aep plan artifact waves --kind story --status draft` collision lines (cited)
2. story:session-envelope-model — its acceptance reads which providers the host supports. Those providers are created by `session-limits-systemd` and `session-writes-bwrap`, and by `substrate-gate-provider` and `substrate-session-provider`. The edges run the other way, so the line can only be built against stand-ins. The body must say which provider probes it owns, or the line moves to the provider stories. — session-envelope-model.md:37; session-limits-systemd.md:9-10
3. story:quota-support-report — it and `session-envelope-model` both change `crates/conductor/src/doctor.rs`. Neither body names the other and no edge exists. An ordering edge or a split would fix it. — quota-support-report.md:28; session-envelope-model.md:43 (cited)
4. story:instance-usage-metrics — its body names only `instance-session-names` as the sharer of `config.yaml` and the generated crate. It also shares `config.yaml` and `conductor-model` with `instance-storage-quota`, `config.yaml` with `instance-scoped-tasks`, and `conductor-model` with `host-grants`. None of those has an edge. — instance-usage-metrics.md:48-51 (cited)
5. story:instance-storage-quota — its body names only `host-grants`. It also shares `config.rs` and `config.yaml` with `instance-scoped-tasks`, which has no edge to it and no sentence about it. — instance-storage-quota.md:39-40; instance-scoped-tasks.md:37-39 (cited)
6. story:instance-scoped-tasks — it shares `config.rs` with `host-grants`. Both depend on `instance-session-names` and neither is ordered against the other. It also changes spec, so `task regen` rewrites `crates/conductor-model`, but its scope omits that directory, as round 2 already required of every spec-changing story. — instance-scoped-tasks.md:37-39; review-result:mi-parallel-round-2 (cited)
7. story:session-limits-systemd — it rewrites the `claude --bg` start lines in `Taskfile.yml` (lines 54 and 136) and the controller start and resume commands in `.agents/conductor.md` (lines 128 and 172). `instance-session-names` edits the same lines to add `-n <prefix>-…`. It also shares `Taskfile.yml` with `instance-scoped-tasks` and `.agents/conductor.md` with `host-grants`. It names only its edges to the spike and the envelope story, so there is no `instance-session-names` edge. — session-limits-systemd.md:9-11,38-39; Taskfile.yml:54,136; .agents/conductor.md:128,172 (cited)
8. story:substrate-gate-provider — it changes `task check` in `Taskfile.yml`, which `instance-session-names`, `instance-scoped-tasks` and `session-limits-systemd` also change. It has no `depends_on` edge and no sentence about any of them. — substrate-gate-provider.md:26,31 (cited)
9. story:substrate-session-provider — it and `substrate-gate-provider` both create `crates/conductor/src/substrate.rs`. That file is not in the tree yet. Neither body names the other and no edge exists. An ordering edge or a split would fix it. — substrate-session-provider.md:29; substrate-gate-provider.md:31 (cited)
10. story:substrate-session-provider — its acceptance meets the epic's limits, writes and usage, and makes the provider selectable per instance. That reads the config and observation model that `session-envelope-model` creates, and it has no `depends_on`. Its scope also omits `spec/domains/config.yaml` and `config.rs`, where a selection field must go, and `Cargo.toml`, where the substrate SDK dependency goes. Both omissions are inferred (the SDK from `substrate-gate-provider.md:26`). The edge to `session-envelope-model` is missing. — substrate-session-provider.md:23-29; session-envelope-model.md:31-36
11. story:substrate-gate-provider — "with the usage recorded" writes into the measured-envelope record that `session-envelope-model` creates in `observation.yaml`. There is no `depends_on` edge to `session-envelope-model`. — substrate-gate-provider.md:26; session-envelope-model.md:34-36

**What I read:** 12 artifacts (the 9 set stories plus `instance-session-names`, `instance-scoped-tasks`, `host-grants`) with `aep plan artifact show`. I also ran `graph`, `relations` and `waves --kind story --status draft`. I read `review-result:mi-parallel-round-2`, `epic:session-confinement`, `upstream-blocker:substrate-exec-in-checkout` and `story:spawn`. In the tree I read `Taskfile.yml`, `.agents/conductor.md`, `lib.rs`, `doctor.rs`, `config.rs` and `docs/config.md`.

Surfaces established:

| Basis | Items |
|---|---|
| Cited | 12 |
| Inferred | 0 |
| Unplaceable | 0 |

Two inferred additions sit inside finding 10. Acceptance reads traced to a producer in the set: 13. Eight are recorded by a direct `depends_on` and one by a transitive chain. Four are unrecorded (findings 2, 10 and 11, plus the `GET /v1/metrics` read below).

**What I could not establish:**
- **Controller start surface:** `session-limits-systemd` and `session-writes-bwrap` must wrap a controller start. That start lives in `.agents/conductor.md` and in `story:spawn` (`spawn.rs`, a stub, outside the set). `session-writes-bwrap` lists neither `Taskfile.yml` nor `.agents/conductor.md`. I could not tell whether `confine` builds the command or the Taskfile does, so I raised no finding.
- **Module lines:** new modules (`confine`, `substrate`, `quota`, `host`) each need a `pub mod` line in `lib.rs` (lines 15-39). Round 2 treated that as a trivial merge and I did too.
- **Metrics read:** `instance-usage-metrics` reads `GET /v1/metrics` from a substrate session, but its acceptance tests only a fixture body and the producer is upstream-blocked. I left it unflagged.
- **Out of lane:**
  - `instance-usage-metrics` still names the archived `instance-cgroup-limits` (lines 28-32), which `session-limits-systemd` supersedes.
  - `docs/config.md` is hand-written and nobody but `substrate-session-provider` lists it, although `session-envelope-model` and the others change config.
- **Ordering:** `waves` currently separates the collisions only by tie-break. None of it is recorded as an edge, which is what findings 1 and 3 to 9 ask for.

```findings
- file: .engineering/planning/story/session-envelope-model.md
  line: 42
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "session-envelope-model shares crates/conductor-model, config.rs and spec/domains/config.yaml with instance-storage-quota and instance-scoped-tasks, conductor-model and config.rs with host-grants, and collect/sessions.rs with instance-session-names; it has no depends_on edge and no body names any of them (waves separates them only by tie-break). Remedies: ordering edges recording the shared files, or split the surface."
- file: .engineering/planning/story/session-envelope-model.md
  line: 37
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads which providers the host supports, state created by session-limits-systemd, session-writes-bwrap and the substrate stories whose edges run toward this story, so the line can only be built against stand-ins; the body must say which provider probes it owns or the line moves to the provider stories."
- file: .engineering/planning/story/quota-support-report.md
  line: 28
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "quota-support-report and session-envelope-model both change crates/conductor/src/doctor.rs and neither body names the other nor is ordered by an edge. Remedies: an ordering edge recording doctor.rs, or split the doctor output."
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 51
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body names only instance-session-names as sharer of config.yaml and the generated crate, but it also shares config.yaml and conductor-model with instance-storage-quota, config.yaml with instance-scoped-tasks and conductor-model with host-grants, none ordered by an edge. Remedies: ordering edges recording the shared files, or split the surface."
- file: .engineering/planning/story/instance-storage-quota.md
  line: 40
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body names only host-grants as sharer, but it also shares config.rs and spec/domains/config.yaml with instance-scoped-tasks, which has no edge to it and no sentence about it. Remedies: an ordering edge recording the shared files, or split the surface."
- file: .engineering/planning/story/instance-scoped-tasks.md
  line: 37
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "instance-scoped-tasks shares crates/conductor/src/config.rs with host-grants (both depend only on instance-session-names, no edge between them), and it changes spec but omits crates/conductor-model/ from its scope, which task regen rewrites. Remedies: an ordering edge recording config.rs, or split the surface; add the generated crate to the scope."
- file: .engineering/planning/story/session-limits-systemd.md
  line: 38
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "session-limits-systemd rewrites the claude --bg start lines in Taskfile.yml (54, 136) and the controller start and resume commands in .agents/conductor.md (128, 172) that instance-session-names rewrites for -n <prefix>, and shares Taskfile.yml with instance-scoped-tasks and .agents/conductor.md with host-grants; it names only its edges to the spike and the envelope story. Remedies: ordering edges recording those files, or split the surface."
- file: .engineering/planning/story/substrate-gate-provider.md
  line: 31
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "substrate-gate-provider changes task check in Taskfile.yml, which instance-session-names, instance-scoped-tasks and session-limits-systemd also change; it has no depends_on edge and no sentence about any of them. Remedies: ordering edges recording Taskfile.yml, or split the surface."
- file: .engineering/planning/story/substrate-session-provider.md
  line: 29
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "substrate-session-provider and substrate-gate-provider both create crates/conductor/src/substrate.rs (the file is not in the tree yet) and neither body names the other nor is ordered by an edge. Remedies: an ordering edge recording the file, or split into separate modules."
- file: .engineering/planning/story/substrate-session-provider.md
  line: 23
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance (limits, writes, usage, provider selectable per instance) reads the config and observation model session-envelope-model creates and it has no depends_on to it; its scope also omits spec/domains/config.yaml and config.rs for the selection field and Cargo.toml for the SDK dependency (the omitted surfaces are inferred, the SDK from substrate-gate-provider.md:26). Missing edge: depends_on story:session-envelope-model."
- file: .engineering/planning/story/substrate-gate-provider.md
  line: 26
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance records usage into the measured-envelope record that session-envelope-model creates in spec/domains/observation.yaml and it has no depends_on to it. Missing edge: depends_on story:session-envelope-model."
```
