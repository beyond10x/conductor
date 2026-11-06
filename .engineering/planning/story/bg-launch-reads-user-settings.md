---
format: aep.planning-md/3
id: story:bg-launch-reads-user-settings
kind: story
status: implemented
title: A background launch in bypassPermissions reads user settings, where Claude Code keeps the disclaimer acceptance
summary: The none provider's claude --bg launches pass --setting-sources user,project,local
refs:
- provider: github
  reference: beyond10x/conductor#83
relations:
- informed_by: story:sessions-without-global-config
- informed_by: story:agent-needs-user-setting-source
- informed_by: story:spawn
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T15:42:44Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T15:42:45Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T16:14:12Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported in https://github.com/beyond10x/conductor/issues/83 (Conductor 0.19.0, macOS, provider
`none`, Claude Code 2.1.318): `conductor spawn` answers `LaunchFailed: claude --bg exited 1: --bg
with bypassPermissions requires accepting the disclaimer first. Acceptance is kept in your user
settings, which this launch does not read. Launch again with user settings on.`

The `none` provider starts every session (`spawn`, `session start`, the resume behind
`resume-controller`) as `claude --bg … --permission-mode bypassPermissions --setting-sources
project,local --settings <settings>`. Probed with Claude Code 2.1.319, 2026-11-06, a scratch
directory, `--model haiku`, prompt "reply with ok", each started session stopped and removed by
its id:

| launch (all `--bg --permission-mode bypassPermissions`) | result |
|---|---|
| `--setting-sources project,local` | exit 1, the error above |
| plus `--settings` holding `skipDangerousModePermissionPrompt: true` | exit 1, the error above |
| `--dangerously-skip-permissions` in place of the mode | exit 1, the error above |
| the key in the start directory's `.claude/settings.json` | exit 1, the error above |
| the key in `.claude/settings.local.json` | starts, then "Bypass permissions was requested at launch and ignored": manual mode |
| `--setting-sources user,project,local`, the key in user settings | starts, "bypass permissions on" |

Read from the 2.1.319 bundle: the background worker accepts bypass only when the key is in user
or managed (policy) settings, or the legacy `bypassPermissionsModeAccepted` is in `~/.claude.json`.
No environment variable and no `--settings`, project or local form counts. A foreground session,
which the `systemd` provider runs, takes the key from `--settings`.

## Decision and trade-off

A `--bg` launch whose arguments ask for `bypassPermissions` reads user settings too:
`--setting-sources user,project,local`. Only the `none` provider adds `user`, and only to the
arguments of `claude --bg`; the launch itself (`Launch.args`) and the `systemd` provider keep
`project,local`.

This narrows `story:sessions-without-global-config` and `story:agent-needs-user-setting-source`
for the `none` provider: its sessions load the user's `~/.claude/settings.json` (env, plugins,
hooks, attribution) and find user-level agents in `~/.claude/agents/`. The role's `--settings`
file still wins over user settings on every key it sets, the guard hook still runs, and
`conductor config validate` still refuses a `controller` role with an `agent` and no `profile`.
An operator who wants no user configuration in a session runs the `systemd` provider, or gives
the role's OS user a user settings file that holds only the acceptance.

## Acceptance

- Through the `none` provider, `conductor spawn`, `conductor session start` and the controller
  resume run `claude --bg … --setting-sources user,project,local …`; tests `tests/spawn.rs`
  (`spawn_starts_the_controller_start_form_in_the_checkout_and_records_it_running`,
  `the_none_provider_resumes_with_the_start_form`), `tests/taskfile.rs` (`SETTING_SOURCES`),
  `tests/confine.rs` (`a_bg_launch_in_bypass_permissions_reads_user_settings`).
- A launch through a provider other than `none` keeps `--setting-sources project,local`; tests
  `tests/confine_systemd.rs` (the harness command of the unit), `tests/spawn.rs`
  (`spawn_starts_through_the_provider_with_one_start_and_one_prompt`).
- The specification's controller start form (`conductor.dispatch.SpawnController`), the README,
  `docs/config.md` and the site's start and confinement pages say which setting sources each
  provider passes and where the disclaimer acceptance must be.
