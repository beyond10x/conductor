---
format: aep.planning-md/3
id: review-result:autonomy-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, epic conductor-autonomy
relations:
- reviews: epic:conductor-autonomy
revision: 1
---
approve

All eight stories trace to the epic and every outcome clause is claimed by exactly one of them. I found nothing to change.

**Promises extracted from the epic (`.engineering/planning/epic/conductor-autonomy.md`):**
- Seven outcome clauses are claimed:
  - Keeps itself running across restarts: `story:self-arming-restart`.
  - Keeps controllers busy beside long gates: `story:work-beside-long-gate`.
  - Answers status in seconds: `story:status-board`.
  - Reclaims disk before it pauses: `story:gc-before-pause`.
  - Never lets an operator item stall: `story:operator-items-age`.
  - Checks what it announces: `story:claims-cite-source`.
  - Installs from a pre-built binary: `story:prebuilt-binaries`.
- One more promise comes from the Why section: "a long gate restarted from its first step four times". `story:resumable-gates` claims it. The Outcome sentence does not repeat it, but the Why does, so it is not reach beyond the parent.

**No reach beyond the parent:** the epic has no exclusions section, and each story's Why echoes a symptom the epic lists.

**No overlap:** `story:work-beside-long-gate` and `story:resumable-gates` both concern long gates but promise different things. One dispatches parallel work; the other reruns only failed steps. `story:gc-before-pause` extends the implemented `story:conductor-gc` rather than duplicating it.

**Graph:** all eight stories have a `decomposes` edge to `epic:conductor-autonomy`. No other story claims any of the same outcomes.

Out of my lane, and not setting the verdict:
- `story:status-board` promises "under 60 s" where the epic says "seconds". That is an acceptance-checkability matter.
- `story:resumable-gates` and `story:work-beside-long-gate` edit the same three files (`.agents/conductor.md`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`). That is a parallel-safety matter.

What I read: the epic and the eight stories with `aep plan artifact show`, plus `aep plan artifact graph` and `git diff --stat`. I did not run `aep plan artifact kinds` or `relations`.

What I could not establish: nothing.

```findings
[]
```
