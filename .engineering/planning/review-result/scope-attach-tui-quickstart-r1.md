---
format: aep.planning-md/3
id: review-result:scope-attach-tui-quickstart-r1
kind: review-result
status: active
title: Scope critic, round 1 (session-attach, tui-frontend, quick-start)
relations:
- reviews: epic:session-attach
- reviews: epic:tui-frontend
- reviews: story:init-defaults
- reviews: story:demo-mode
- reviews: story:quick-start-docs-test
revision: 1
---
needs-revision
epic:session-attach — the Why names `task conductor:attach` and the guide's `claude attach <id>` pointer as broken for systemd sessions, but no story retargets the task (Taskfile.yml:81-99) or the prefixed-instances note, and story:session-attach-command AC8 only requires the guide to "name the command"; session-attach-command is the natural owner — .engineering/planning/epic/session-attach.md:14
story:tui-open-session — attaching or watching from the TUI is not in the epic's Why or Outcome, which promise a client that "shows what the dashboard shows" and answers decisions, and the dashboard has no attach key; either the epic's Outcome claims it or the story goes — .engineering/planning/epic/tui-frontend.md:18
story:init-defaults — the five-minute path is install plus `conductor demo`, and demo-mode AC2 builds its own `demo` instance in "a config file of its own", so this story's "working one-instance setup … to reach a dashboard" duplicates the demo's instance-and-dashboard claim and changes `conductor init` for existing users beyond the epic's Outcome; demo-mode's `depends_on` it is unused — .engineering/planning/story/init-defaults.md:15
story:demo-mode — the epic promises the demo dispatch is "labelled as a demo in every place it shows", but AC3 requires the `demo` label only on the session or dispatch row and none on the activity-feed event, so the promise is narrowed without a record — .engineering/planning/story/demo-mode.md:30
story:quick-start-docs-test — the epic promises a CI job "before every release", but AC3 runs the page check on pull requests, weekly and on `release: published`, which fires after the tag is public, and nothing says the pre-publish run was dropped — .engineering/planning/story/quick-start-docs-test.md:29

What I read: 19 artifacts, with `aep plan artifact show` on the 3 epics and 16 stories, then `aep plan artifact graph` and `aep plan artifact list`, plus Taskfile.yml:78-102 and website/docs/guides/start-conductor.md:134-144. I wrote the promise list from each epic before reading its stories.
- epic:session-attach: I extracted 7 promises and traced 6. The gap is the `task conductor:attach` and guide retarget.
- epic:tui-frontend: 10 promises, all traced. One story reaches beyond the epic, tui-open-session.
- epic:quick-start: 11 promises, 9 traced whole. Two are narrowed: "labelled in every place" and "before every release".
- Totals: 28 promises, 24 traced whole, 4 flagged. Nothing in the graph already claims any of the five findings.

What I could not establish:
- Whether the operator wants attach from the TUI or `conductor init` writing a config. I judged only whether the parent's text claims them.
- Out of my lane: tui-open-session AC6 (client refusal of `conductor tui` inside a session) and tui-decisions-answer AC6 (guard denial of `conductor tui`) both cover "a session cannot run the TUI", which is for the design critic.
- Out of my lane: quick-start-docs-test AC7 cites an "AGENTS.md § Releases" that AGENTS.md does not appear to have, which is for the acceptance critic.

```findings
- file: .engineering/planning/epic/session-attach.md
  line: 14
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Why names task conductor:attach (Taskfile.yml:81-99) and the guide's claude attach pointer (start-conductor.md:139-141) as broken for systemd sessions, but no story retargets the task or the prefixed-instances note; session-attach-command AC8 only requires the guide to name the command"
- file: .engineering/planning/epic/tui-frontend.md
  line: 18
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:tui-open-session (attach or watch from the TUI with one key) is not in the epic's Why or Outcome, which promise a client that shows what the dashboard shows and answers decisions; the epic's Outcome must claim it or the story is reach beyond the parent"
- file: .engineering/planning/story/init-defaults.md
  line: 15
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the five-minute path is install plus conductor demo, and demo-mode AC2 creates its own demo instance and config, so init-defaults duplicates the demo's instance-and-dashboard claim and changes conductor init for existing users beyond the epic's Outcome; demo-mode's depends_on on it is unused"
- file: .engineering/planning/story/demo-mode.md
  line: 30
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic promises the demo dispatch is labelled as a demo in every place it shows, but AC3 requires the demo label only on the session or dispatch row and none on the activity-feed event"
- file: .engineering/planning/story/quick-start-docs-test.md
  line: 29
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic promises the CI job runs before every release, but AC3 runs it on pull requests, weekly and on release: published, which fires after the tag is public, and nothing records that the pre-publish run was dropped"
```