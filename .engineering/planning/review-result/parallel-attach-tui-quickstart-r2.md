---
format: aep.planning-md/3
id: review-result:parallel-attach-tui-quickstart-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2 (session-attach, tui-frontend, quick-start)
relations:
- reviews: epic:session-attach
- reviews: epic:tui-frontend
- reviews: epic:quick-start
- reviews: story:session-attach-command
- reviews: story:session-attach-watch
- reviews: story:session-attach-refuses-sessions
- reviews: story:session-list-attachable
- reviews: story:tui-overview
- reviews: story:tui-decisions-read
- reviews: story:tui-decisions-answer
- reviews: story:tui-dispatches-activity
- reviews: story:tui-host-disk-builds
- reviews: story:tui-open-session
- reviews: story:install-one-liner
- reviews: story:demo-mode
- reviews: story:init-defaults
- reviews: story:quick-start-page
- reviews: story:quick-start-docs-test
revision: 1
---
needs-revision

Round 1's ten findings are resolved. Three narrower collisions remain.

story:session-attach-refuses-sessions — its shared-surface line says it shares `spec/domains/session.yaml` with `story:session-attach-watch`, but watch says it adds nothing there; the other story that edits that file is `story:session-list-attachable` (`ListSessions`), and no edge or body line orders the two, since both sit after `story:session-attach-command` on different branches (cited, both bodies). Name list-attachable here and either order the pair or accept the merge cost, or drop the false watch claim — .engineering/planning/story/session-attach-refuses-sessions.md:25, .engineering/planning/story/session-attach-watch.md:23, .engineering/planning/story/session-list-attachable.md:28

story:init-defaults — it extends `conductor.config.InitRecords` in `spec/domains/config.yaml` (:538) and regenerates `crates/conductor-model` and the CLI reference, the same file and generated files `story:demo-mode` changes (`RunDemo` "beside `InitRecords`"). It mentions demo-mode only to say it does not depend on it, no edge orders the two, and demo-mode's shared-surface line lists four other stories but not this one (cited, both bodies). Add a shared-surface line and either order the two or accept the merge cost — .engineering/planning/story/init-defaults.md:27, .engineering/planning/story/demo-mode.md:28

story:tui-overview — acceptance 8 says the four siblings add their screen "there and nowhere else" (the registry in `crates/conductor/src/tui/screens.rs`). But `story:tui-host-disk-builds` acceptance 4 adds a line to the overview's attention lines, and `story:tui-open-session` acceptance 2 adds the attachable column to the overview's sessions table. Those two both edit the overview screen's code with no edge between them, and no body names the overview screen's file (inferred: that file does not exist yet). Name the overview screen's file and either order the two siblings or say that the overview exposes extension points for them — .engineering/planning/story/tui-overview.md:33, .engineering/planning/story/tui-host-disk-builds.md:32, .engineering/planning/story/tui-open-session.md:33

What I read: 3 epics and 16 stories, each with `aep plan artifact show <id>`, plus `review-result:parallel-attach-tui-quickstart-r1` and `aep plan artifact waves`. Surface counts: cited 15, inferred 1 (the overview screen's file in the third finding), unplaced 0. I traced 21 acceptance reads to a producer in the set, and 21 are recorded by a `depends_on` edge. Three of those run through a chain: `story:quick-start-docs-test` reaches install-one-liner and demo-mode through `story:quick-start-page`, and `story:tui-open-session` reaches `story:session-attach-command` through `story:session-attach-watch`.

What I could not establish:
- All 16 stories have no `scope` field, so `aep plan artifact waves` lists them as unassessed and reports no collisions for them. The findings come from the bodies.
- The relay and attach-client source files are named by no body and do not exist yet. I could not check whether `story:session-list-attachable`'s socket probe lands in the same file as the attach client. Both are ordered by an edge, so no finding rests on it.
- The read-model module that `story:tui-overview` extracts has no path in any body. Its surface is cited only as `dashboard.rs` and `dashboard/*.rs`.
- Collisions with stories outside the set (`story:dashboard-build-queue`, `story:sessions-systemd-kind`, `story:watch-stalled-sessions`) were not assessed.
- Out of my lane: the round 1 note on `story:quick-start-docs-test` acceptance 4, a "documented variable" for the branch binary, now reads as `CONDUCTOR_INSTALL_BASE` defined by `story:install-one-liner`, so I treat it as fixed. `story:session-attach-command` and `story:session-list-attachable` both edit `Taskfile.yml`, in different recipes, so I did not call that a collision.

```findings
- file: .engineering/planning/story/session-attach-refuses-sessions.md
  line: 25
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its shared-surface line says it shares spec/domains/session.yaml with story:session-attach-watch, but watch adds none there (session-attach-watch.md:23); the other editor of that file is story:session-list-attachable (ListSessions, session-list-attachable.md:28) and no edge or body line orders the two; name list-attachable and either order the pair or accept the merge cost, or drop the false watch claim"
- file: .engineering/planning/story/init-defaults.md
  line: 27
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it extends conductor.config.InitRecords in spec/domains/config.yaml (:538) and regenerates crates/conductor-model and the CLI reference, the same file and generated files story:demo-mode changes (demo-mode.md:28); init-defaults mentions demo-mode only to say it does not depend on it, no edge orders the two, and demo-mode's shared-surface line (demo-mode.md:24) omits this story; add a shared-surface line and either order the two or accept the merge cost"
- file: .engineering/planning/story/tui-overview.md
  line: 33
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred surface (the overview screen's file does not exist yet): acceptance 8 says the siblings add their screen and key to the screen registry and nowhere else, but story:tui-host-disk-builds acceptance 4 (tui-host-disk-builds.md:32) adds a line to the overview's attention lines and story:tui-open-session acceptance 2 (tui-open-session.md:33) adds a column to the overview's sessions table, so both edit the overview screen with no edge between them and no body names its file; name the overview screen's file and either order the two siblings or state the extension points the overview provides"
```