---
format: aep.planning-md/3
id: review-result:design-attach-tui-quickstart-r1
kind: review-result
status: active
title: Design critic, round 1 (session-attach, tui-frontend, quick-start)
relations:
- reviews: story:install-one-liner
- reviews: story:demo-mode
- reviews: story:quick-start-page
- reviews: story:tui-open-session
- reviews: story:tui-decisions-answer
revision: 1
---
needs-revision
story:install-one-liner — acceptance 7 reads `website/docs/quick-start.md`, which story:quick-start-page creates, while story:quick-start-page declares `depends_on story:install-one-liner`; the install story can't be checked until the page it must land first exists, so drop acceptance 7 here and leave the three-way text comparison to story:quick-start-page — .engineering/planning/story/install-one-liner.md:32, .engineering/planning/story/quick-start-page.md:28
story:demo-mode — acceptance 8 requires `website/docs/quick-start.md` to show the demo's real output, but that page is created by story:quick-start-page, which `depends_on story:demo-mode`; the same reverse read, so move the sentence to story:quick-start-page and keep only the generated CLI reference here — .engineering/planning/story/demo-mode.md:35, .engineering/planning/story/quick-start-page.md:28
story:quick-start-page — acceptance 4 makes its printed-lines check the comparison that story:quick-start-docs-test performs, and that story declares `depends_on story:quick-start-page`, so each waits on the other; the page should state how it checks its own printed lines, and docs-test should own the CI comparison — .engineering/planning/story/quick-start-page.md:31, .engineering/planning/story/quick-start-docs-test.md:27
story:tui-open-session — acceptance 6 reads the `inside-session` refusal that story:session-attach-refuses-sessions creates, and no `depends_on` edge, direct or through story:session-attach-watch or story:session-attach-command, leads to it (the edge runs from refuses-sessions to attach-command only); add `depends_on story:session-attach-refuses-sessions` — .engineering/planning/story/tui-open-session.md:33, .engineering/planning/story/session-attach-refuses-sessions.md:28
story:tui-decisions-answer — it and story:tui-open-session both claim to keep sessions out of `conductor tui` (a guard rule here, a client self-refusal there), with no edge between the two, so the rule has two owners; assign it to one story and have the other depend on it — .engineering/planning/story/tui-decisions-answer.md:32, .engineering/planning/story/tui-open-session.md:33

**What you read:** 16 stories and the 3 epics (`aep plan artifact show` on each), `aep plan artifact relations`, `aep plan artifact graph` and `aep plan artifact validate`. I walked about 36 edges, including the `decomposes` and `informed_by` edges to the epics and to artifacts outside the set (`initiative:adoption`, `epic:session-confinement`, `epic:dashboard-decisions`, `epic:onboarding-from-url`). I found no cycle in the declared edges. I traced 14 acceptance reads of state another item in the set produces, and 9 of them have an edge. The 5 without one are the findings above. The 5 findings are 3 reverse reads that make an effective cycle and 2 shape defects on the TUI side.

**What I could not establish:**
- Unease, not a finding: story:session-attach-command lands the command that writes into a session's pty one wave before story:session-attach-refuses-sessions adds the guard and the relay's peer-cgroup refusal. The edge makes that window inherent. If it matters, merge the guard rule into story:session-attach-command.
- Out of my lane (parallel safety): story:session-list-attachable edits the `task sessions` Taskfile lines that story:sessions-systemd-kind also edits. That story is outside the set, is a draft with no edges, and has no edge to this one. Its own Ambiguities says "deliver it first or merge".
- Out of my lane (acceptance): story:quick-start-docs-test acceptance 4 refers to "a documented variable" that puts the branch binary in the one-liner's place. story:install-one-liner defines no such variable.
- Out of my lane (parallel safety): story:session-attach-watch and story:session-attach-refuses-sessions both depend on story:session-attach-command and both change the relay's connection-accept path.
- The validator's two reports are not mine: five stories closed on an assertion, and story:sweep-legacy-waiters has no scope. Neither touches this set.

```findings
- file: .engineering/planning/story/install-one-liner.md
  line: 32
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7 reads website/docs/quick-start.md, which story:quick-start-page creates, while story:quick-start-page depends_on story:install-one-liner; the install story cannot be checked until the page that must come after it exists, so drop acceptance 7 here and leave the three-way text comparison to story:quick-start-page"
- file: .engineering/planning/story/demo-mode.md
  line: 35
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 8 requires website/docs/quick-start.md to show the demo's real output, but that page is created by story:quick-start-page, which depends_on story:demo-mode; the same reverse read, so move the sentence to story:quick-start-page and keep only the generated CLI reference here"
- file: .engineering/planning/story/quick-start-page.md
  line: 31
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 4 makes its printed-lines check the comparison that story:quick-start-docs-test performs, and that story depends_on story:quick-start-page, so each waits on the other; the page should state how it checks its own printed lines and docs-test should own the CI comparison"
- file: .engineering/planning/story/tui-open-session.md
  line: 33
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 reads the inside-session refusal that story:session-attach-refuses-sessions creates, and no depends_on edge, direct or through story:session-attach-watch or story:session-attach-command, leads to it; add depends_on story:session-attach-refuses-sessions"
- file: .engineering/planning/story/tui-decisions-answer.md
  line: 32
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it and story:tui-open-session both claim to keep sessions out of conductor tui (a guard rule here, a client self-refusal there) with no edge between them, so the rule has two owners; assign it to one story and have the other depend on it"
```