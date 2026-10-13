---
format: aep.planning-md/3
id: review-result:autonomy-parallel-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic conductor-autonomy
relations:
- reviews: story:claims-cite-source
- reviews: story:gc-before-pause
- reviews: story:operator-items-age
- reviews: story:resumable-gates
- reviews: story:work-beside-long-gate
- reviews: story:self-arming-restart
- reviews: story:status-board
revision: 1
---
needs-revision

All six stories on `.agents/conductor.md` collide, and `story:self-arming-restart` and `story:status-board` collide on `Taskfile.yml`. No body names any of these collisions, and none has a `depends_on` edge for them. Your plan to run units that share a file one after another is not recorded in the stories. For each collision the options are an ordering edge that names the shared file as its reason, or splitting the surface. I am not choosing between them.

- **`.agents/conductor.md` (cited, all six bodies):** the six stories are `claims-cite-source`, `gc-before-pause`, `operator-items-age`, `resumable-gates`, `self-arming-restart` and `work-beside-long-gate`.
- **Also shared (cited):**
  - `crates/conductor/tests/profiles.rs`: `claims-cite-source`, `resumable-gates` and `work-beside-long-gate`.
  - `.agents/repo-controller.md`: `resumable-gates` and `work-beside-long-gate`.
  - `Taskfile.yml` and `crates/conductor/tests/taskfile.rs`: `self-arming-restart` and `status-board`. Only the frontmatter of `status-board` lists them. Its body `## Scope` omits them, so its body is the one to fix.
- **`story:prebuilt-binaries`:** no collision inside this set. `aep plan artifact waves` reports one on `README.md` with `story:session-writes-bwrap`, which is outside this set, so I make no finding on it.

**What I read:** 8 stories, via `aep plan artifact show` and `cat -n` of each body. I also ran `aep plan artifact waves --kind story --status draft` and `aep plan artifact graph`. Surfaces established: 8 cited, 0 inferred, 0 unplaced. Acceptance reads traced to a producer inside the set: 0, so 0 are recorded by an edge.

**Not established:** `crates/conductor/tests/status.rs` and `STATUS.md` do not exist yet, and both are named in `status-board`. Whether `task status` exists was not checked. Whether the config that `self-arming-restart` reads from has a producer was not checked. That config is not in this set.

```findings
- file: .engineering/planning/story/claims-cite-source.md
  line: 11
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this and story:gc-before-pause, story:operator-items-age, story:resumable-gates, story:self-arming-restart and story:work-beside-long-gate all land on .agents/conductor.md, and on crates/conductor/tests/profiles.rs with resumable-gates and work-beside-long-gate (cited, all bodies); none says so and no depends_on edge orders them; options are an ordering edge naming the file, or splitting the surface"
- file: .engineering/planning/story/gc-before-pause.md
  line: 12
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this lands on .agents/conductor.md with five other stories in the set (claims-cite-source, operator-items-age, resumable-gates, self-arming-restart, work-beside-long-gate) (cited) and its body does not say so; options are an ordering edge naming the file, or splitting the surface"
- file: .engineering/planning/story/operator-items-age.md
  line: 11
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this lands on .agents/conductor.md with five other stories in the set (claims-cite-source, gc-before-pause, resumable-gates, self-arming-restart, work-beside-long-gate) (cited) and its body does not say so; options are an ordering edge naming the file, or splitting the surface"
- file: .engineering/planning/story/resumable-gates.md
  line: 11
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this and story:work-beside-long-gate both land on .agents/conductor.md, .agents/repo-controller.md and crates/conductor/tests/profiles.rs, and on conductor.md also with four other stories in the set (cited); the body does not say so; options are an ordering edge naming the files, or splitting the surface"
- file: .engineering/planning/story/work-beside-long-gate.md
  line: 11
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this and story:resumable-gates both land on .agents/conductor.md, .agents/repo-controller.md and crates/conductor/tests/profiles.rs, and on conductor.md also with four other stories in the set (cited); the body does not say so; options are an ordering edge naming the files, or splitting the surface"
- file: .engineering/planning/story/self-arming-restart.md
  line: 16
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this and story:status-board both land on Taskfile.yml and crates/conductor/tests/taskfile.rs, and this also lands on .agents/conductor.md with five other stories (cited); the body does not say so; options are an ordering edge naming the files, or splitting the surface"
- file: .engineering/planning/story/status-board.md
  line: 11
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this and story:self-arming-restart both land on Taskfile.yml and crates/conductor/tests/taskfile.rs (cited, frontmatter scope); the body's own Scope section omits both files and does not mention the other story; options are an ordering edge naming the files, or splitting the surface"
```
