# conductor-dev hand-over, 2026-10-12 (third)

Session 3092ca26 ran from 2026-10-12 11:47Z. Read this page, the instance's `rules.md`, the
hand-over committed last in the records' `docs/handoff/`, and design § 12.

## State

- **Released v0.2.0** (tag = `36ed43f`, PR 18, GitHub Release with the wave 04 notes).
- **Wave 05 merged and installed** (`docs/waves/wave-05.md`, PR 20, `main` `3b12b150`):
  `workspace-snapshot-end-to-end`, `agent-needs-user-setting-source`, `class-h-handover`, and a
  fix so that `conductor.example.yaml` validates. The installed binary is `3b12b150`, still named
  v0.2.0; the next release carries wave 05.
- Conductor was restarted on its own request (context 301k, N-20261012-05, session 633cf05e), and
  after the wave 05 install because `.agents/conductor.md` changed (hand-over records `05eb30c`).
  That restart ran once before the product checkout was fast-forwarded, so session 4aad94ef read
  the old profile; it was restarted again at once on the same hand-over: session 355bf50a.
- Disk: `/` 21–25G through the session; conductor grants a slot per wave (DEC-20261012-05, -06)
  with a start floor of 24G and a 21G floor for each step.

## Next, in this order

1. Release v0.3.0 (minor: a new config refusal) after the next wave, or alone if none comes in a
   day. AGENTS.md § Releases; every body through `b10x-gates scan-text` (rules item 97).
2. `story:agents-link-user-scope` (filed this session): `task agents:link` serves no session
   started with `--setting-sources project,local`.
3. The plan's next waves: `spawn` with `watch-unsent-report`, then the multi-instance chain
   (`aep plan artifact waves --kind story --status draft`).

## Open with the operator

Unchanged: whether the conductor role's `read_only_tools` names `PushNotification` and
`AskUserQuestion`. Recommended: `PushNotification` only.

## Lessons

- The store now refuses `implemented` without a recorded `test_result`: record the gate with
  `aep plan artifact evidence <id> --kind test_result --source "<gate, commit, counts>"`.
- A rule over a role must hold for the built-in defaults too: `config show` writes them out and
  three round-trip tests read them back.
- Fast-forward `~/beyond10x/conductor` to `origin/main` before `task conductor:restart`: the
  role's `profile` is read from that checkout, not from the tree that merged. A worktree-isolated
  session cannot run git there; leave the tree first.
- Run `cargo clean` on the tree's target right after the gate; the 3G it holds is what takes `/`
  under conductor's floor.
