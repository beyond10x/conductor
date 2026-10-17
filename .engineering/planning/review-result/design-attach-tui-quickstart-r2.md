---
format: aep.planning-md/3
id: review-result:design-attach-tui-quickstart-r2
kind: review-result
status: active
title: Design critic, round 2 (session-attach, tui-frontend, quick-start)
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
approve

I found no defect in the shape of the set, and all five round 1 design findings are fixed. I checked each one against the revised bodies.

- **`story:install-one-liner`:** acceptance 7 is gone. Acceptance 6 now says the story changes no workflow file or spec, and the three-way text comparison sits in `story:quick-start-page` acceptance 7.
- **`story:demo-mode`:** the acceptance 8 that required the quick-start page to show the demo's output is gone. The page now owns it in acceptance 8, and `story:quick-start-page` already declares `depends_on story:demo-mode`.
- **`story:quick-start-page`:** acceptance 4 now checks its own printed lines locally through `--help` and `--version`. It says the run of the whole page in CI belongs to `story:quick-start-docs-test`, so the two stories no longer wait on each other.
- **`story:tui-open-session`:** it now declares `depends_on story:session-attach-refuses-sessions`, and the context says why.
- **Guard ownership:** `story:tui-decisions-answer` acceptance 6 is the sole owner of keeping sessions out of `conductor tui`, and `story:tui-open-session` declares `depends_on story:tui-decisions-answer` and adds no refusal of its own.

I did not repeat the round 1 unease about the window between `story:session-attach-command` and `story:session-attach-refuses-sessions`. The set now states in both stories that they ship in the same release, and `story:tui-open-session` depends on the refusal, so I do not treat it as a finding.

What I read: the 3 epics and 16 stories with `aep plan artifact show`, plus the round 1 design `review-result`. I also ran `aep plan artifact relations`, `aep plan artifact graph` and `aep plan artifact validate`. Validate returned `valid`. Its two reports (five stories closed on an assertion, and `story:sweep-legacy-waiters` with no scope) are not in this set and not mine. I walked about 50 declared edges, including `decomposes` and `informed_by` to artifacts outside the set (`initiative:adoption`, `epic:session-confinement`, `epic:dashboard-decisions`, `epic:onboarding-from-url`) and `depends_on` to `story:watch-stalled-sessions`. I found no cycle. I traced about 30 acceptance reads of state that a sibling in the set produces, and a `depends_on` edge, direct or transitive, covers every one.

What I could not establish:
- Out of my lane (parallel safety): `story:session-list-attachable` and `story:session-attach-command` edit the same Taskfile lines that `story:sessions-systemd-kind`, outside the set, also edits, and no edge records it.
- Out of my lane (parallel safety): `story:session-attach-watch` and `story:session-attach-refuses-sessions` both change the relay's connection-accept path. Their edge, with its written reason, covers that.
- Out of my lane (parallel safety): the guard table in `docs/design/conductor.md` § 7 is shared by `story:session-attach-refuses-sessions`, `story:tui-decisions-answer` and `story:demo-mode`, and so is `screens.rs` among the TUI screens. The bodies name these shared surfaces.
- Out of my lane (acceptance): `story:session-attach-refuses-sessions` acceptance 2 does not name the environment variable the provider sets. `story:quick-start-docs-test` acceptance 1 relies on blocks "marked as commands", but `story:quick-start-page` acceptance 2 does not state the marker.

```findings
[]
```