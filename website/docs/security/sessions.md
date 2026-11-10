---
title: Sessions
sidebar_position: 3
description: How a session starts; the claude --bg launch, settings, setting sources, profile, refusals, and which processes run in a box.
lede: Every session starts as a background Claude Code session through conductor session start or conductor spawn; its settings decide whether the guard runs.
source: crates/conductor/src/session.rs, spawn.rs, confine/mod.rs, agent/, placement.rs, systemd.rs, daemon/reconcile.rs, daemon/host_roles.rs, config.rs; spec/domains/session.yaml
---

## Who starts a session

| session | command | start directory |
|---|---|---|
| a controller | `conductor spawn --repository <repo>`, which `conductor run` calls | the checkout `<checkouts.root>/<repo>` |
| the maintainer | `conductor session start --role maintainer` | the working directory |
| a conductor session, by hand | `conductor session start --role conductor` | the instance's `records` |
| a host role, `supervisor` or `teacher` | conductord runs `conductor --config <file> --state-dir <daemon dir>/host session start --role <role>` for each role `globals.roles` lists | the config file's directory |

`conductor agent start --instance <i> --role <r>` takes the same path through the `claude-code`
kind. `session start` is its alias for this release. The runtime the daemon starts, `conductor
instance serve`, serves the instance API and runs the loop, which starts the controllers.

## The launch

Every session is started by the `none` provider as one argv, never through a shell:

```console
claude --bg -n <session name> \
  --append-system-prompt-file <profile> \
  --model <model> \
  --permission-mode bypassPermissions \
  --setting-sources user,project,local \
  --settings <state>/sessions/<session name>.settings.json \
  "<first prompt>"
```

A controller's first prompt is its charter, `<records>/charters/<repo>.md`. `spawn` prints the
session id. With a config file the session gets `CONDUCTOR_INSTANCE` and the file's absolute path
in `CONDUCTOR_CONFIG`. `TOOL_SOCKET` is its socket to the instance's tool broker, registered with
the role's `grant` before the start.

A stop is `claude stop <id>`, up to 30 s for the session list to drop it, then `claude rm <id>`. A
resume is `claude --bg --resume <id>` with the same arguments. `conductor controller
pause-controller`, `stop-controller` and `resume-controller` use them.

## Setting sources

The launch writes `--setting-sources project,local`. When it asks for `bypassPermissions`, the
`none` provider adds `user`: Claude Code takes the bypass-permissions acceptance a background
session needs only from user or managed settings, not from `--settings` or local settings.
Without it, `claude --bg` refuses the mode or starts the session in manual mode.

So every session also loads `~/.claude/settings.json` (env, plugins, hooks) and
`~/.claude/agents/`. The `--settings` file wins on every key it sets, and the guard's hook runs
beside the user's. The start directory's project settings and hooks load too
([limits](/docs/security/limits#each-repositorys-own-hooks-run)). To keep the operator's
configuration out, run sessions as an OS user whose user settings hold only the acceptance.

## The settings file

| role | starts with, when the role names no `settings` |
|---|---|
| `conductor` | the binary's `conductor-settings.json`: the guard with `--pid "$PPID"`, and the lease claim |
| `controller` | the binary's `controller-settings.json`: the guard |
| `maintainer`, `supervisor`, `teacher`, a custom role | an empty settings object: no guard |

A role's own `settings` file takes their place. Make it wire the same hook
([the guard](/docs/security/the-guard)). Under a config file a session starts with a copy pinned
to its instance, `<state>/sessions/<session name>.settings.json`:
`"env": {"CONDUCTOR_INSTANCE": "<name>"}`, and `CONDUCTOR_INSTANCE=<name>` before `conductor` in each hook command. The harness
does not pass the starter's environment to the session, so only the pin selects the instance its
hooks act for. A file that pins another instance, or is no JSON, is refused (`InstanceNotPinned`).
`conductor config validate` names it by its `settings` path.

When the instance's token pool leases the session a token, the copy also names `conductor token
reveal --token <id>` as its `apiKeyHelper`
([token pool](/docs/security/operators-and-credentials#the-token-pool)). No session writes a
settings file ([file tools](/docs/security/the-guard#file-tools-edit-write-notebookedit)).

## Profile or agent

`--append-system-prompt-file` names the role's `profile`. A role with neither `profile` nor
`agent` starts from the profile the binary carries, written to
`<state>/sessions/<session name>.profile.md`. The binary carries `maintainer`, `supervisor`,
`teacher` and `controller` (`conductor role show --role <role>`). A role with an `agent` and no
`profile` starts with `--agent <name>`. The `conductor` role has no carried profile and starts
with `--agent conductor`.
`conductor config validate` refuses a `controller` role with an `agent` and no `profile`. No file
goes under `~/.claude/agents/`.

## Bypass permissions

With `--permission-mode bypassPermissions` no tool call asks first, and the guard is the only
check.
Claude Code refuses the mode in a background session until the OS user has accepted its
disclaimer: run `claude --dangerously-skip-permissions` once interactively, or set
`"skipDangerousModePermissionPrompt": true` in `~/.claude/settings.json`. Both write that key.

## Refusals

`session start` exits 1 with `session start: refused: <refusal>: role <role>: <detail>`:

| refusal | when |
|---|---|
| `UnknownRole` | a role `session start` does not start, one the file does not name, or one not on `claude` |
| `InstanceNotPinned` | the role's own settings file is missing, no JSON, or pins another instance |
| `SessionRunning` | a live session holds the session name |
| `ProviderUnavailable` | the instance asks for `strict` and the provider is not available |
| `LaunchFailed` | the start or the first prompt failed |
| `MeasurementShort` | under `strict`, the box measured short of the request; the session is stopped |

`conductor spawn` refuses, in this order and recording nothing: `NoCheckout` (no git checkout at
depth 1 or 2 under the root, a path outside it, or an `exclude`d one), `RecordsCheckout` (the
checkout is, holds or lies in the records), `NoControllerRole`, `NotClaude`, `NoProfile`,
`NoSettings`, `UntrustedCheckout` (`~/.claude.json` does not mark it trusted, which `conductor
trust` does), `NoCharter` (missing, empty, not UTF-8, or starting with `-`), `SessionRunning`,
`SessionsUnread`. Then it records the controller, refused as `AlreadyRunning` while the
repository's controller is `Running` or `Paused`, and launches. A failed launch is
`LaunchFailed`.

`conductor session start --role maintainer` refuses while a live session of its name runs
(`SessionRunning`), when its harness is not `claude` (`UnknownRole`), and when its own settings
file pins another instance (`InstanceNotPinned`). `conductor watch` reports `conductors: <n> live sessions named <name>:
<ids>` when two sessions hold the conductor's name.

## Boxes, slices and limits

Conductor builds no sandbox. It asks substrate for a box. Today only runtimes run in one:

| process | runs as | bounded by |
|---|---|---|
| an instance runtime whose `placement` names an adapter | a substrate PTY session in the instance's substrate workspace | the adapter's confinement facts |
| an instance runtime whose `placement` names none, such as `local` | the user unit `conductor_<instance>-runtime.service` (`systemd-run --user`), restarted 30 s after a failure | the instance's `limits` on its slice: `MemoryMax`, `CPUWeight`, `TasksMax` |
| every agent session | `claude --bg` | nothing: its record says it runs unboxed |

`placement` is `{daemon, adapter}`, or a bare string read as `{daemon: local, adapter: <string>}`.
The adapters are `systemd`, `linux`, `oci` and `pod`. A local daemon refuses `oci` and `pod` at
validate. `systemd` is `linux` inside a delegated systemd scope, adding `limits` and
`whole-tree-kill`. Before anything starts, the facts are checked against `substrate-daemon`'s
`/v1/machine`. A shortfall is refused with substrate's name (`confinement.family-unserved`,
`confinement.fact-unserved`), and nothing falls back. The runtime writes its state directory,
its records and the directory of a sqlite store through host roots the operator declares,
`substrate-daemon --writable-root state-<instance>=<state>`, `records-<instance>=<records>` and
`store=<directory>`, one root where one holds the others. It reads the config file, the
directories of `conductor` and the harness, the checkouts root, and each forge's CLI and login,
and nothing else of the host. The box gets one secret slot, the model token
([secrets](/docs/security/operators-and-credentials#secrets)). For a daemon on another host, see
the [remote daemon guide](/docs/guides/remote-daemon).

An instance's slice is `conductor_<session_prefix>.slice`, else `conductor.slice`. A prefix
character other than a letter or digit is written `\xNN` (`-` is `\x2d`), so no slice nests.

Agent sessions are unboxed: conductor's own `systemd` session provider is gone, and substrate
0.20.0 starts no agent session in a box. A `conductor.config/2` file has no `confinement` key, so
every session starts through `none`. A `conductor.config/1` file that asks for `strict` with the
`systemd` provider refuses every start (`ProviderUnavailable`).

A role's `limits`, `writable`, `network` and `forge_token_env` travel on each session's request,
each limit at or under the instance's. Under `none` nothing enforces or measures them. The record
in `<state>/sessions/<session name>.envelope.json`, read into the `envelope_*` columns of
`conductor snapshot sessions`, says the session runs unboxed.
