# conductor-dev hand-over, 2026-10-12

The session that ran from 2026-10-07 to 2026-10-12 hands over to a fresh one. Read this page, the
instance's `rules.md` (records directory, `conductor config show`), the hand-over committed last in
the records' `docs/handoff/`, and design § 12.

## State

- **main** carries wave 03 (session names per instance, reports as SendMessage calls, tests isolated
  from HOME) and release `v0.1.0`. The installed binary is main as of wave 03.
- **The plan** (`aep plan artifact waves --kind story --status draft`): next is wave 04 under the
  operator's standing word for waves 03–11. The verb's first waves: `guard-all-tools` with
  `watch-tests-shared-isolation`; then `spawn` with `watch-unsent-report`; then the multi-instance
  chain (`instance-scoped-tasks`, `host-grants`) and the confinement epic `session-confinement`
  (`session-envelope-model`, `session-envelope-seam`, `session-limits-systemd`,
  `session-writes-bwrap`, `instance-usage-metrics`, `quota-support-report`).
- **Unassessed and urgent:** `story:workspace-from-config` has no typed scope yet. The instance's
  workspace was repaired by hand after the records move (relative member paths, a member for the
  records repository through a symlink in the instance directory); that story replaces the repair.
  Scope it and put it in wave 04.
- **Blocked:** the substrate stories (`substrate-client`, `substrate-gate-provider`,
  `substrate-tool-server`) wait on `upstream-blocker:substrate-usage-without-io`; the substrate
  repository is building that contract change. `upstream-blocker:substrate-gates-in-trees` is
  cleared by substrate 0.7.12.
- **Released in another repository** on the operator's word: loom 0.13.0, the plugin layer and a
  read-only Slack handler. Loom's controller owns its follow-ups (the YAML-only plugin and reference
  enrichment ideas). The live Slack run waits on the operator's own setup steps.

## Operations the operator relies on while travelling

User units, all transient (a reboot ends them; restart them with the same commands from the
records' rules, item 93):
- `conductor-host-stats`: edits one stats message in the operator's notification channel every 30 s.
- `claude-remote-control`: his Remote Control gateway (`conductor-gateway`), cwd this checkout.
- `ais-promote-claude.timer`: `ais promote claude` hourly.

If one is down, restart it and say so in one line.

## Lessons this session paid for

- In another repository, write no conductor name, path or decision id (rules item 95); use scratch
  paths without the word.
- Ask conductor for a build slot before every build; a full gate asks for its real size (a loom
  `task check` grew a tree to 14G).
- `conductor snapshot start-snapshot` takes over 10 minutes; give it 20.
- Never set `CARGO_TARGET_DIR`, even to an empty string.
- Read a commit id back before reporting it.
