---
format: aep.planning-md/3
id: review-result:sandbox-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, sandbox
relations:
- reviews: story:sandbox-spec
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
needs-revision

- story:session-envelope-seam — `SpawnRefusal` gains "provider unavailable" and "measurement short" in `story:sandbox-spec`, but the only spawn test is "sees one `start` and one `prompt` per spawn", so spawn can declare both refusals and never produce them. This is the same defect as round-1 #2, one command over. The body needs one spawn test per new refusal — .engineering/planning/story/session-envelope-seam.md:92 (`spec/domains/dispatch.yaml:83-97` lists the current variants)
- story:session-envelope-seam — "(and the restart stops through the provider)" names no command. `story:sandbox-spec` declares only `session start` (`.engineering/planning/story/sandbox-spec.md:52`, `cli.yaml` has no `session stop`), so the Taskfile has nothing to call, and a test that "reads the three tasks" cannot tell a provider stop from a bare `claude stop` — .engineering/planning/story/session-envelope-seam.md:98
- story:session-envelope-seam — "describe the start path" is not observable. The only checked text is the removal of "nothing starts a session in a box yet" (`docs/config.md:136`), and nothing says what the three pages must contain afterwards, such as naming `conductor session start` — .engineering/planning/story/session-envelope-seam.md:104

**Round-1 findings**

| # | Finding | Status | Evidence |
|---|---|---|---|
| 1 | `.agents/conductor.md` change had no check | Closed | `tests/profiles.rs` now holds that `claude --bg` is absent and the two start forms are named. The current file has `claude --bg` at lines 172 and 221, so the test fails before the work and passes after (seam :104). |
| 2 | Only three of five `session start` refusals tested | Closed | Seam :83 lists one test each for provider unavailable, measurement short, unknown role, session running and launch failed, plus the `report` case. A remaining gap on the spawn refusals is finding 1 above. |
| 3 | Live run did not say what limits to compare against | Closed | limits-systemd :50 now requires `memory.max`, `cpu.weight` and `pids.max` equal to the role's configured `limits`, and the slice to the instance's. |
| 4 | Home bind and `~` writability | Closed | writes-bwrap :47 sets `CLAUDE_CONFIG_DIR` to the harness-state directory and tests for no `--bind` of `~`. The live run tries `touch ~/outside-probe` and a `python3` write to `/home/<user>/outside-probe-2`. |
| 5 | No test for `envelope_writable` or the missing-place refusal | Closed | The measurement test is named, and the strict-mode `bwrap`-missing and place-missing refusals get one test each. The `report`-mode partial box is also tested. |
| 6 | "say what the box enforces" was a vote | Closed | A `docs_forge.rs` test reads the two removed sentences. Both exist now: `README.md:42` and `docs/design/conductor.md:284` ("Bash is story `bash-sandbox`"). |
| 7 | Live run with a token passes whatever the box hides | Closed | forge-credentials :43 adds a no-token case: `gh api rate_limit` and `glab api user` fail to authenticate. |
| 8 | Bus path invisible under `--ro-bind / /` | Closed | writes-bwrap :47 puts `--tmpfs /run/user/<uid>` in the ordered list, tested, and forge-credentials relies on that mask. |
| 9 | Doctor condition not observable | Closed | The condition is now an empty `forge_token_env`, with a test in `tests/doctor.rs`. |

`story:sandbox-spec` (new) gets no finding. Each declared item is a nameable spec entry, and the checks are `ess specify validate`, `task check` and the CLI tree test. The same goes for `story:session-limits-systemd`, `story:session-writes-bwrap` and `story:session-forge-credentials`: every bullet there is checkable or has a named test. Their live-run bullets are long but each clause is observable.

What I read: all 5 ids and `review-result:sandbox-acceptance-r1`, in full with `aep plan artifact show <id>` from `conductor-wave-17`, plus `README.md:42`, `docs/design/conductor.md:284`, `docs/config.md:136`, `Taskfile.yml`, `.agents/conductor.md`, `cli.yaml`, `spec/domains/dispatch.yaml` and `observation.yaml`, and the `approval-record:sandbox-priority` header.

What I could not establish:
- Whether `ess/24` is a real format. `spec/system.yaml` says `ess/22` now and `ess --version` is 0.59.0, but I did not find the format list.
- What baseline "refuses nothing new" in `story:sandbox-spec` compares against.
- Whether the `cli_tree` test checks a handler's answer or only the leaf's presence ("a handler that answers 'not implemented'").
- Out of my lane (unreported): `story:session-writes-bwrap` makes a missing place a strict-mode shortfall, while the seam's resolver returns only existing paths, so the two may overlap (design). `approval-record:sandbox-priority` is still `draft`, and the seam's merge depends on it (process).

```findings
- file: .engineering/planning/story/session-envelope-seam.md
  line: 92
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "SpawnRefusal gains provider unavailable and measurement short in story:sandbox-spec, but the only spawn test is 'sees one start and one prompt per spawn', so both refusals can be declared and never produced by spawn while the story passes; add one spawn test per new refusal"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 98
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'(and the restart stops through the provider)' names no command, story:sandbox-spec declares only `session start`, and a test that reads the three tasks cannot tell a provider stop from a bare `claude stop`; name the command conductor:restart calls to stop"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 104
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'describe the start path' is not observable; only the removal of 'nothing starts a session in a box yet' (docs/config.md:136) is tested, so name the text the three pages must contain afterwards, such as `conductor session start`"
```
