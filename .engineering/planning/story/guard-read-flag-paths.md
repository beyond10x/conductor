---
format: aep.planning-md/3
id: story:guard-read-flag-paths
kind: story
status: implemented
title: A settings or profile path given as a read-only flag value does not make a write elsewhere a write to it
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-11T23:59:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-11T23:59:50Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-11T23:59:50Z", actor: "human:timo", revision: 5, decided_on: {"asserted":{"test_result":1}}}
---
## Why

Conductor's session 00135485 starts a controller with one Bash call that appends the charter
section and then runs `claude --bg … --append-system-prompt-file <profile> --settings <controller
settings>`. The guard denied both such calls (transcript
`~/.claude/projects/-home-timo--b10x-conductor-beyond10x-records/00135485-368b-4f4a-ae54-388c504a74cf.jsonl`,
lines 1600, 2286, 3072): "the command names a .claude/ settings file or a role's settings file and
holds a write form", and "the command names a role's profile file and holds a write form". Neither
call wrote either file: the paths were read-only flag values, the write form (`>>`, `cat >`) wrote
`charters/`. `Places::writes_settings` and `Places::writes_profile`
(`crates/conductor/src/guard/rules.rs:489`, `:511`) test "names the file" and "holds a write form"
independently. Conductor works around it by splitting the call (its hand-over 2026-10-11-0226,
"First" item 4).

## Acceptance

- A Bash command whose only mention of a role's settings or profile file is the value of
  `--settings`, `--append-system-prompt-file` or `--system-prompt-file`, and whose write forms all
  name other targets (`shell::written`), is not denied by those two rules. Test: the controller
  start shape from the transcript, with `>> <records>/charters/x.md` in front, is allowed by these
  rules.
- The same command with `> <profile>` or `sed -i … <settings>` anywhere is still denied. Test each.
- A command that names the file outside such a flag and holds any write form is still denied, as
  today.
- The allow path does not get slower (`tests/guard_latency.rs`).

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_hook.rs`.
