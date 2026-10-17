---
format: aep.planning-md/3
id: review-result:parallel-attach-tui-quickstart-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1 (session-attach, tui-frontend, quick-start)
relations:
- reviews: story:session-list-attachable
- reviews: story:session-attach-refuses-sessions
- reviews: story:tui-open-session
- reviews: story:install-one-liner
- reviews: story:demo-mode
- reviews: story:quick-start-page
- reviews: story:tui-overview
- reviews: story:tui-decisions-answer
revision: 1
---
needs-revision

story:session-list-attachable — it adds `conductor.session.ListSessions` to `spec/domains/session.yaml` and `[session, list]` to `crates/conductor/cli.yaml`, the same two files `story:session-attach-command` changes (cited, both bodies), and no edge or body line orders the two stories; both depend only on `story:session-pty-relay`. Name an ordering edge with the shared files as its reason, or split the surface — .engineering/planning/story/session-list-attachable.md:26, .engineering/planning/story/session-attach-command.md:27
story:session-attach-refuses-sessions — it and `story:session-attach-watch` both depend only on `story:session-attach-command` and both change the relay's `spec/domains/session.yaml` model (the `inside-session` refusal against the `watch` mode and refusal frame) and the relay's connection handling (the peer-cgroup refusal against the read-only enforcement). Neither body mentions the other (cited, both bodies; the relay code is not yet written). Name an ordering edge or split the surface — .engineering/planning/story/session-attach-refuses-sessions.md:27, .engineering/planning/story/session-attach-refuses-sessions.md:30, .engineering/planning/story/session-attach-watch.md:26, .engineering/planning/story/session-attach-watch.md:28
story:tui-open-session — acceptance 6 reads the `inside-session` refusal that `story:session-attach-refuses-sessions` creates, and the story's three `depends_on` edges are to `story:tui-overview`, `story:session-attach-watch` and `story:session-list-attachable`, so no edge runs to the producer. Add `depends_on story:session-attach-refuses-sessions`, or merge the two — .engineering/planning/story/tui-open-session.md:33, .engineering/planning/story/session-attach-refuses-sessions.md:27
story:install-one-liner — acceptance 7 requires the command text on `website/docs/quick-start.md`, which `story:quick-start-page` creates, while the only edge runs the other way (the page depends on this story), so the acceptance cannot be checked when this story is worked. Move the three-way comparison into `story:quick-start-page`, or merge the two — .engineering/planning/story/install-one-liner.md:32, .engineering/planning/story/quick-start-page.md:28
story:demo-mode — acceptance 8 requires `website/docs/quick-start.md` to show the demo's real output, a file `story:quick-start-page` creates, while the only edge runs the other way (the page depends on this story). Move that line into `story:quick-start-page`, or merge the two — .engineering/planning/story/demo-mode.md:35, .engineering/planning/story/quick-start-page.md:28
story:quick-start-page — acceptance 4 reads a comparison that `story:quick-start-docs-test` creates ("the docs test of `story:quick-start-docs-test` compares them"), while `story:quick-start-docs-test` depends on this story, so a depends_on edge cannot fix it. Take the clause out of this acceptance and leave the comparison to the docs-test story, or merge the two — .engineering/planning/story/quick-start-page.md:31, .engineering/planning/story/quick-start-docs-test.md:27
story:tui-overview — acceptance 2 moves only "the overview's data" out of the HTTP server's `Sources`, but `story:tui-decisions-read` (`/api/decisions`), `story:tui-dispatches-activity` (`/api/events`, `/api/waiting-on`, `/api/blockers`) and `story:tui-host-disk-builds` (the `disk` object) each call a read-model function that is still behind that server, and no body says who moves it. All four therefore edit `crates/conductor/src/dashboard.rs` and `dashboard/*.rs` with no edge between the three siblings (cited, all four bodies). State that this story extracts all the screens' functions, or have each sibling say it extracts its own and order them — .engineering/planning/story/tui-overview.md:19, .engineering/planning/story/tui-overview.md:27, .engineering/planning/story/tui-decisions-read.md:22
story:tui-overview — it creates the TUI's screen list, navigation keys and header/footer, and `story:tui-decisions-read` (`2`/`tab`), `story:tui-dispatches-activity`, `story:tui-host-disk-builds` (header and the overview's attention lines) and `story:tui-open-session` (the sessions table) each add to them without saying where, so those four collide on one module (inferred: the TUI module does not exist yet, and `git grep tui -- crates/conductor/src` finds nothing). Name the module and its registration point in this body, or order the siblings — .engineering/planning/story/tui-overview.md:26, .engineering/planning/story/tui-decisions-read.md:27, .engineering/planning/story/tui-host-disk-builds.md:30
story:tui-decisions-answer — acceptance 6 adds a guard rule and a `docs/design/conductor.md` § 7 table entry, the same guard code and table `story:session-attach-refuses-sessions` (acceptance 3 and 5) and `story:demo-mode` (acceptance 7) change, and no body mentions another or has an edge to it. Name an ordering edge with the guard table as its reason, or give each command's rule its own entry — .engineering/planning/story/tui-decisions-answer.md:32, .engineering/planning/story/session-attach-refuses-sessions.md:29, .engineering/planning/story/demo-mode.md:34
story:demo-mode — it adds `[demo]` to `crates/conductor/cli.yaml`, regenerates `crates/conductor-model` and `website/docs/reference/cli.md`, as do `story:tui-overview` (`[tui]`), `story:session-attach-command` and `story:session-list-attachable`, and nothing orders or mentions them across the three epics. Say that the four share these files, then either order them or accept the merge cost — .engineering/planning/story/demo-mode.md:28, .engineering/planning/story/tui-overview.md:26, .engineering/planning/story/session-attach-command.md:27

What I read: 16 stories, the 3 epics, and `aep plan artifact waves`, run as separate `aep plan artifact show <id>` calls; I also read the critic rubric and the parallel-safety procedure. Surface counts: cited 16, inferred 0, unplaced 0. The inferred part is the TUI screen module in the second `story:tui-overview` finding, which does not exist yet. I traced 19 acceptance reads to a producer in the set and 15 have a `depends_on` edge in the right direction. The four without one are the `inside-session` read, the quick-start-page read, and the two reads of `quick-start.md`.

What I could not establish:
- All 16 stories have no `scope` field, so `aep plan artifact waves` lists every one as `unassessed` and reports no collisions for them. The findings come from the bodies, not from the scheduler.
- Out of my lane (the acceptance critic's): `story:quick-start-docs-test` acceptance 4 depends on a "documented variable" that puts the branch binary in the one-liner's place, and `story:install-one-liner` defines none.
- Out of my lane (the design critic's): `story:init-defaults` is not among the page's three commands, and the page reaches it only through `story:demo-mode`.
- Collisions with stories outside the set, such as `story:sessions-systemd-kind` (named in `story:session-list-attachable`'s Ambiguities) and `story:dashboard-build-queue`, were not assessed.

```findings
- file: .engineering/planning/story/session-list-attachable.md
  line: 26
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds ListSessions to spec/domains/session.yaml and [session, list] to crates/conductor/cli.yaml, the same files story:session-attach-command changes (.engineering/planning/story/session-attach-command.md:27), and no edge or body line orders the two; both depend only on story:session-pty-relay; add an ordering edge naming the shared files or split the surface"
- file: .engineering/planning/story/session-attach-refuses-sessions.md
  line: 27
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this story and story:session-attach-watch (session-attach-watch.md:26, :28) both depend only on story:session-attach-command and both change the relay's spec/domains/session.yaml model (inside-session refusal against watch mode and refusal frame) and the relay's connection handling (peer-cgroup refusal at :30 against read-only enforcement), and neither body mentions the other; add an ordering edge naming the shared surface or split it"
- file: .engineering/planning/story/tui-open-session.md
  line: 33
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 reads the inside-session refusal that story:session-attach-refuses-sessions creates (session-attach-refuses-sessions.md:27) and no depends_on edge runs from this story to it; add the edge or merge the two"
- file: .engineering/planning/story/install-one-liner.md
  line: 32
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7 requires the command text on website/docs/quick-start.md, which story:quick-start-page creates (quick-start-page.md:28), while the only edge runs the other way (the page depends on this story); move the three-way comparison into story:quick-start-page or merge the two"
- file: .engineering/planning/story/demo-mode.md
  line: 35
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 8 requires website/docs/quick-start.md to show the demo's real output, a file story:quick-start-page creates (quick-start-page.md:28), while the only edge runs the other way (the page depends on this story); move that line into story:quick-start-page or merge the two"
- file: .engineering/planning/story/quick-start-page.md
  line: 31
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 4 reads the comparison that story:quick-start-docs-test creates (quick-start-docs-test.md:27) while that story depends on this one, so no depends_on edge can fix it; remove the clause from this acceptance and leave it to the docs-test story, or merge the two"
- file: .engineering/planning/story/tui-overview.md
  line: 27
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2 extracts only the overview's data from the HTTP server's Sources, but story:tui-decisions-read, story:tui-dispatches-activity and story:tui-host-disk-builds each call a read-model function still behind that server and no body says who moves it, so all four edit crates/conductor/src/dashboard.rs and dashboard/*.rs with no edge between the three siblings; state that this story extracts them all or have each sibling extract its own and order them"
- file: .engineering/planning/story/tui-overview.md
  line: 26
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred surface (the TUI module does not exist yet): this story creates the screen list, navigation keys and header/footer that story:tui-decisions-read (2/tab), story:tui-dispatches-activity, story:tui-host-disk-builds and story:tui-open-session each add to without saying where, so the four siblings collide on one module; name the module and its registration point in this body or order the siblings"
- file: .engineering/planning/story/tui-decisions-answer.md
  line: 32
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 adds a guard rule and a docs/design/conductor.md section 7 table entry, the same guard code and table that story:session-attach-refuses-sessions (acceptance 3 and 5) and story:demo-mode (acceptance 7) change, and no body mentions another or has an edge to it; add an ordering edge naming the guard table or give each command's rule its own entry"
- file: .engineering/planning/story/demo-mode.md
  line: 28
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds [demo] to crates/conductor/cli.yaml and regenerates crates/conductor-model and website/docs/reference/cli.md, as do story:tui-overview ([tui]), story:session-attach-command and story:session-list-attachable, and nothing orders or mentions them across the three epics; say that the four share these files and either order them or accept the merge cost"
```