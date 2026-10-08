---
format: aep.planning-md/3
id: review-result:mi-parallel-round-1
kind: review-result
status: active
title: Parallel-safety critic, multi-instance-host, round 1
relations:
- reviews: epic:multi-instance-host
- reviews: story:host-grants
- reviews: story:instance-usage-metrics
- reviews: story:instance-storage-quota
revision: 1
---
needs-revision

- story:host-grants — shares `spec/domains/config.yaml` and `.agents/conductor.md` with story:instance-session-names, and `config.yaml` with story:instance-scoped-tasks, story:instance-usage-metrics and story:instance-storage-quota (cited, all Scope sections). Neither this body nor the others mention it, and no edge orders any of these pairs. Fix per pair: an ordering edge that records the shared file as its reason, or split the surface (for example, put the `host:` section in its own spec domain file). — .engineering/planning/story/host-grants.md:29-30, .engineering/planning/story/instance-session-names.md:29-30
- story:instance-usage-metrics — shares `crates/conductor/src/watch.rs` and `spec/domains/config.yaml` with story:instance-session-names (cited, both Scope sections). Neither body says so and no edge orders them. Fix: an ordering edge that records the shared files as its reason, or split the surface (for example, give the watch threshold line a separate module). — .engineering/planning/story/instance-usage-metrics.md:27-28, .engineering/planning/story/instance-session-names.md:29-31
- story:instance-storage-quota — its Scope is the whole `crates/conductor/src/` directory plus unnamed "docs", so it collides with every code-touching item in the set (all except the spike and the guard story). The `conductor doctor` command it requires has no file or symbol anywhere (`git grep -n doctor -- crates spec` returns nothing). Narrow the Scope to the files and the new module it will create. — .engineering/planning/story/instance-storage-quota.md:25, .engineering/planning/story/instance-storage-quota.md:21
- story:host-grants — its Scope names whole directories (`crates/conductor/src/` and `crates/conductor/tests/`). That includes `tests/taskfile.rs` (instance-scoped-tasks), `tests/guard_rules.rs` (cross-instance-message-guard) and `tests/` (instance-usage-metrics). The Scope claims more than the "new `host` module" it describes, so it collides with every other item. Narrow it to the new module, its test files and the specific existing files it edits. — .engineering/planning/story/host-grants.md:30

**What I read:** 7 stories plus decision-blocker:project-quota-filesystem and the epic edges. I ran `aep plan artifact waves --kind story --status draft`, `graph`, `relations`, `show` on all eight, and `git grep` and `ls` on the tree.

Surfaces established:
- Cited: 7.
- Inferred: 0.
- Unplaceable: 0. The quota story's surface is only at directory level.

Acceptance reads traced to a producer in the set:
- 3 traced: instance-scoped-tasks and cross-instance-message-guard read the names that instance-session-names creates; instance-usage-metrics reads the spike's go/no-go.
- 2 of the 3 are recorded by `depends_on`. The third is `informed_by`, which I judged adequate because the metrics acceptance reads cgroup files that exist without the spike. The `depends_on` pairs are the ones I did not flag. They share files (`Taskfile.yml`, `config.yaml`, `config.rs`), but the edge orders them.

**What I could not establish:**
- `aep plan artifact waves` places no wave. None of the 7 declares a scope, and `aep artifact scope` records none.
- Out of my lane (design or scope critic): instance-session-names retypes `Controller.session_name`, which also touches `crates/conductor/src/controller.rs:305` and `crates/conductor/src/store/controller.rs:87`. Its Scope omits both files, and no other item in the set touches them.
- Out of my lane: instance-storage-quota is blocked by the open decision-blocker:project-quota-filesystem, so it cannot start in any wave until the operator decides.
- Weak collision, not flagged: the quota story's "docs" and the spike's new `docs/analysis/` page might land in the same directory. The quota story names no path, so I cannot tell.

```findings
- file: .engineering/planning/story/host-grants.md
  line: 29
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "host-grants shares spec/domains/config.yaml and .agents/conductor.md with instance-session-names and config.yaml with instance-scoped-tasks, instance-usage-metrics and instance-storage-quota (cited, all Scope sections); no body names the overlap and no edge orders them. Remedies: an ordering edge recording the shared file as reason, or split the surface."
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 27
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "instance-usage-metrics shares crates/conductor/src/watch.rs and spec/domains/config.yaml with instance-session-names (cited, both Scope sections); neither body names the overlap and no edge orders them. Remedies: an ordering edge recording the shared files as reason, or split the surface."
- file: .engineering/planning/story/instance-storage-quota.md
  line: 25
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Scope is the whole crates/conductor/src/ directory plus unnamed 'docs', so it collides with every code-touching item in the set; the conductor doctor command it requires has no existing file or symbol (git grep -n doctor -- crates spec returns nothing). Narrow the Scope to the files and the new module it will create."
- file: .engineering/planning/story/host-grants.md
  line: 30
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Scope names whole directories crates/conductor/src/ and crates/conductor/tests/, which contain files other items in the set edit (tests/taskfile.rs, tests/guard_rules.rs, tests/); narrow to the new host module, its test files and the specific existing files edited."
```
