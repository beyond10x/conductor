---
format: aep.planning-md/3
id: story:host-session-profiles
kind: story
status: active
title: Conductor ships the host supervisor and teacher profiles, and init installs them
summary: the supervisor and teacher agent files live on one host only
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T12:43:07Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T12:43:07Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07, relayed by the host supervisor: "the supervisor should now instruct the
conductor dev to basically include the supervisor as an agent profile as well", and to the
teacher: "you should also become such profile included in conductor repo". Today the host
supervisor's and the teacher's agent files live only under `~/.claude/agents/` on one host, in no
repository; a change to them has no review and a fresh host has neither.

## Acceptance

- The repository carries two more generic profiles, `conductor-supervisor` and
  `conductor-teacher`, beside `conductor-dev.md` and `repo-controller.md`: one file each in
  `.agents/` and one adapter each in `.claude/agents/`. Their content comes from the host's
  current agent files with every instance-, organization-, person- and host-specific line taken
  out (AGENTS.md rules 4 and 9); what is true for one host goes in that host's own file or in an
  instance's `rules.md`.
- Each file states its start command, the `claude --bg --agent <name> -n <name>` line.
- `conductor init` installs both adapters under `~/.claude/agents/` when missing, and never
  overwrites a file that differs from the shipped one: it prints one line per file, installed,
  kept or differs. A test over a temporary home reads all three cases.
- `crates/conductor/tests/no_organization.rs` (or the profiles test) scans the two new files.

## Notes

- Split from `story:host-bootstrap`, whose remainder (config template, persistent units, the usage
  watcher's drop-in, the disk keeper) stays draft.
- Once this ships, changes to these profiles arrive as pull requests to this repository.

## Source of the content (added 2026-11-07)

The host supervisor, 2026-11-07, relaying the operator ("persistent in the sense that it's part of
conductor and not of some outside rules"): the shipped supervisor profile is the host's current
supervisor agent file, and the teacher profile the current teacher file, as they stand when this
story is built, so the rules learned on 2026-11-07 ship with conductor. Organization-, person-
and host-specific lines are still taken out (AGENTS.md rules 4 and 9).

## Start tasks (added 2026-11-07)

- Taskfile tasks `supervisor:start` and `teacher:start` beside `maintainer:start`, each running the start command its profile writes; refuse while a session of that name runs. Source: the host supervisor, 2026-11-07, relaying the operator: "both of them should now probably update the agent profiles and then respawn ... make sure nothing is lost for both of them".

## Hand-off directories and profile sync (added 2026-11-07)

- The supervisor's and the teacher's hand-offs live in `<state>/supervisor/handoff/` and `<state>/teacher/handoff/`, as conductor-dev's live in `<state>/conductor-dev/handoff/`; each profile names its directory and reads its newest file at start. Source: the host supervisor, 2026-11-07, relaying the operator: "all company/repo/project related rules must remain outside of the conductor repo itself".
- `.agents/conductor-supervisor.md` and `.agents/conductor-teacher.md` are synced from the host's current agent files in the pull request after 0.23.0, holding no organization, instance or repository name; a test greps them for the instance's names from a fixture config.

- `task profiles` and `conductor init` also install `.claude/agents/conductor-supervisor.md` and `conductor-teacher.md` into `~/.claude/agents/`: a file that is absent or is one of conductor's adapters is written; any other file is left and named, as `agents:link` does. Found 2026-11-07: after a merge, the installed teacher adapter kept an old `model:` until it was copied by hand.
