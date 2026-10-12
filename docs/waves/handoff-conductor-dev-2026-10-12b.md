# conductor-dev hand-over, 2026-10-12 (second)

Session 33162b50 ran from 2026-10-11 22:12Z; it hands over at about 300k context. Read this page,
the instance's `rules.md` (new: item 97), the hand-over committed last in the records'
`docs/handoff/`, and design § 12.

## State

- **main** (`afc6b545`) carries wave 04 (`docs/waves/wave-04.md`) and five fixes after it (PRs 11–16).
  The installed binary is `b6d6508` (the last code change). The instance's conductor and controller
  settings route every tool to the guard (matcher `*`); conductor was restarted on them and is
  healthy: snapshots complete every two hours, the generated workspace (`conductor config
  workspace`, records `b5425f5`, 61 members) is the one in use.
- **Disk**: `/` at 11–12G free (about 11:40Z), under the 15G stop. None of it is ours. No build
  until conductor grants a slot and `/` is over 20G.

## Next, in this order

1. **Release v0.2.0.** A plan exists (written 2026-10-12 by a planning sub-agent, corrected here):
   minor bump (new command `config workspace`, new key `read_only_tools`); notes = the five wave 04
   stories and the four post-merge fixes; include a one-line guard change first: `ScheduleWakeup`
   onto `READ_ONLY` in `crates/conductor/src/guard.rs` beside the `Cron*` entries (the guard denied
   conductor's `ScheduleWakeup` twice on 2026-10-12, 05:05Z and 08:49Z), with a case in
   `tests/guard_hook.rs`; change the install line in `README.md` and in
   `website/docs/getting-started.md`. Follow AGENTS.md § Releases, with two corrections to the
   sub-agent's plan: never set `CARGO_TARGET_DIR` for the verifying install (use `--root` only),
   and every PR and release body passes `b10x-gates scan-text` before it is sent (rules item 97).
2. `story:workspace-snapshot-end-to-end`: one test over both collectors and a written workspace;
   the three post-merge fixes were each found only by a failed live snapshot.
3. `story:agent-needs-user-setting-source`: with `--setting-sources project,local` no user-level
   agent loads, so the `--agent` controller start in `.agents/conductor.md` exits at once.
4. The plan's next waves: `spawn` with `watch-unsent-report`, then the multi-instance chain.

## Open with the operator

Whether the conductor role's `read_only_tools` names `PushNotification` and `AskUserQuestion`
(denied since the matcher `*`). Recommended: `PushNotification` only. Until then conductor notifies
through its Slack route.

## Lessons this session paid for

- A gate whose Gates scan printed "skipped" is no gate: run `task check` with the policy set.
- Commit through the bot route only (rules item 42); the first wave 04 commits were made with a
  plain `git commit` and had to be replayed. The local branch `wave/04` keeps those originals; it is
  unmerged by content, not by tree, and has no remote.
- Test a generated file against its reader before shipping it: run the consumer, not only the
  writer.
- A full scratch snapshot spends the GitHub API quota every controller shares; the quota ran out at
  01:15Z. Prove a collector fix with one targeted read instead.
- The read-flag exemption in the guard was bypassable through the shell until the review; an
  allowance in the guard gets a security review before merge.
