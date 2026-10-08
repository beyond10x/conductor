---
format: aep.planning-md/3
id: review-result:mi-parallel-round-2
kind: review-result
status: active
title: Parallel-safety critic, multi-instance-host, round 2
relations:
- reviews: epic:multi-instance-host
- reviews: story:host-grants
revision: 1
---
needs-revision

All four round-1 findings are fixed. Typed scopes are written, the broad directory scopes are narrowed, and `waves` now places every item in five waves with no same-wave collision on a declared scope. Three new findings remain, all from surfaces the declared scopes leave out.

story:host-grants — the `host:` section it requires needs an edit to `crates/conductor/src/config.rs` (`CONFIG_KEYS = ["version","default","instances"]` at config.rs:172 and the unknown-key refusal at config.rs:755), and its Scope omits that file. Story:instance-storage-quota needs the same file for `instances[].storage_quota` (`INSTANCE_KEYS`, config.rs:173), and `waves` puts both in wave 2. Neither body names the overlap and no edge orders them. Either an ordering edge records `config.rs` as the reason, or the two keys move into separate modules — .engineering/planning/story/host-grants.md:33,44 (cited, from the body and the tree; the second story is at .engineering/planning/story/instance-storage-quota.md:27)

story:host-grants — it and story:instance-storage-quota both change `spec/`, so each `task regen` rewrites the digest header of every file in `crates/conductor-model/src/`, plus `plan.json` and `PLAN.md`. `task docs-generate` also rewrites `website/docs/reference/{config,cli}.md` and `website/docs/status.md`. No story in the set lists any of these in its typed scope, so `waves` cannot see the collision and puts both in wave 2. Either an ordering edge records the generated crate as the reason, or the typed scopes of every spec-changing story gain `crates/conductor-model/` so the waves serialize them — .engineering/planning/story/host-grants.md:44 (cited, AGENTS.md:30,57,59,74 and the digest headers in `crates/conductor-model/src/*.rs`)

story:host-grants — its typed scope lists five entries, but its Scope prose also names `spec/ess-inputs.yaml`, `spec/system.yaml` and `crates/conductor/tests/host.rs` (new). The surface that `aep artifact scope` records is narrower than the surface the body claims. Nothing else in the set touches those three files today, so this is a warning — .engineering/planning/story/host-grants.md:44 (cited)

**What I read:** 9 stories, `review-result:mi-parallel-round-1`, `story:install-prerequisites` and the epic edges. I ran `aep plan artifact show` on each, plus `graph` and `waves --kind story --status draft`. I also read `crates/conductor/src/{config.rs,lib.rs}`, `AGENTS.md`, `Taskfile.yml` and `spec/` with `git ls-files` and `grep`.

Surfaces established:

| Basis | Items |
|---|---|
| Cited | 9 |
| Inferred | 0 |
| Unplaceable | 0 |

Acceptance reads traced to a producer in the set: 6. Five are recorded by `depends_on`:
- session-names to guard, scoped-tasks, host-grants and usage-metrics
- the spike and scoped-tasks to cgroup-limits

The sixth is the spike's go/no-go read by usage-metrics, recorded as `informed_by`; I accepted that in round 1.

**What I could not establish:**
- Out of my lane: `story:install-prerequisites` is outside the set, and quota-support-report depends on it for `doctor.rs`. Its Scope (`Taskfile.yml`, `spec/`, `crates/conductor/cli.yaml`, `tests/`) overlaps set members, and it is `active`. I did not judge that.
- `waves` only serializes on declared scopes. Usage-metrics (watch thresholds, `THRESHOLD_KEYS`) and cgroup-limits (`instances[].limits`) also need `config.rs` but do not list it. Both already sit in later waves than every other `config.rs` holder, so no concurrency finding follows. Adding the file to their scopes would keep that true if the waves change.
- New modules (`host.rs`, `quota.rs`, `usage_metrics.rs`, `doctor.rs`) each need a `pub mod` line in `crates/conductor/src/lib.rs`. No story lists it. I counted this as a trivial merge and did not raise it.
- I did not check whether quota-support-report's `project quotas: yes|no` output needs a spec change. If it does, it joins the generated-crate collision above.

```findings
- file: .engineering/planning/story/host-grants.md
  line: 33
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "host-grants needs an edit to crates/conductor/src/config.rs for its host: section (CONFIG_KEYS at config.rs:172 refuses unknown keys, config.rs:755) and omits it from Scope; instance-storage-quota edits the same file for instances[].storage_quota (INSTANCE_KEYS, config.rs:173) and waves puts both in wave 2; neither body names it and no edge orders them. Remedies: an ordering edge recording config.rs as reason, or split the surface."
- file: .engineering/planning/story/host-grants.md
  line: 44
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "host-grants and instance-storage-quota both change spec/, so each task regen rewrites the digest header of every file in crates/conductor-model/src plus plan.json and PLAN.md, and docs-generate rewrites website/docs/reference/config.md, cli.md and status.md (cited, AGENTS.md:30,57,59,74); no story in the set lists these in its typed scope, so waves puts both in wave 2. Remedies: an ordering edge recording the generated crate as reason, or add crates/conductor-model/ to the typed scope of every spec-changing story so the waves serialize them."
- file: .engineering/planning/story/host-grants.md
  line: 44
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the typed scope has five entries but the Scope prose also names spec/ess-inputs.yaml, spec/system.yaml and crates/conductor/tests/host.rs (new), so the recorded scope is narrower than the surface the body claims; nothing else in the set touches those files today."
```
