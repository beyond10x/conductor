---
format: aep.planning-md/3
id: story:spawn-pins-binary-and-config
kind: story
status: active
title: A spawned session's hooks run the spawning binary against the instance's config
scope:
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/confine.rs
- confidence: cited
  path: crates/conductor/tests/host_roles.rs
- confidence: cited
  path: crates/conductor/tests/instance_isolation.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/domains/session.yaml
- confidence: cited
  path: website/docs/security/sessions.md
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:00Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:01Z", actor: "human:timo", revision: 6}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from the first conductor 1.0.1
run of a trial instance. 1.0.1 was installed beside an older `conductor` that comes first on the
login PATH; conductord.service and conductor_<instance>-runtime.service carry PATH and
CONDUCTOR_CONFIG for 1.0.1. After `conductor spawn` (`claude --bg`), `/proc/<pid>/environ` of the
lead showed the login PATH and no CONDUCTOR_CONFIG. `<state>/sessions/<name>.settings.json` held
only `env.CONDUCTOR_INSTANCE` and the hook `CONDUCTOR_INSTANCE=<i> conductor guard
record-guard-decision --from-pre-tool-use || exit 2`, with conductor by bare name. The hook and
`conductor message send` ran the older binary against the default config, and `conductor guard
guard-decisions` of the instance stayed empty.

Workaround used: a role `settings` file with env.PATH and env.CONDUCTOR_CONFIG. That it worked
shows that the harness applies a settings file's `env` to the session and its hooks, while the
environment of the process that starts `claude --bg` does not reach them.

## Today (main, 1.22.2)

- `config::pinned` (`crates/conductor/src/config.rs:1768`) sets only `env.CONDUCTOR_INSTANCE` and
  prefixes each hook command that runs `conductor` (`runs_conductor`, `:1822`) with
  `CONDUCTOR_INSTANCE=<i> `, keeping the bare `conductor`. `config::write_pinned` (`:1722`) writes
  it; `config::session_settings` (`:1747`) returns the role's own file unchanged when it already
  pins the instance (`pin_problem`, `:1625`), so an operator's pinned file gets no copy at all.
- `spawn::controller_launch` (`crates/conductor/src/spawn.rs:489-498`) puts `CONDUCTOR_INSTANCE`
  and `CONDUCTOR_CONFIG` in the launch's process environment only, which `claude --bg` does not
  pass on (the pin's own reason, `config.rs:1618-1623`). The module header (`spawn.rs:28-31`) and
  `website/docs/security/sessions.md:37-38` claim the session gets `CONDUCTOR_CONFIG`; through the
  `none` provider it does not.
- The same pin serves `session start` (`crates/conductor/src/session.rs:1675` `write_pinned`,
  `:1817-1823`, env at `:1806-1815`), for the director and the host roles.
- `spawn::pinned_settings` (`spawn.rs:1276`) is called at spawn (`:229`) and at resume (`:657`),
  so a resume writes the copy again.

## Design

- The pin carries three things: the instance, the config file and the binary. `config::pinned`
  takes them together (a small struct, e.g. `Pin { instance, config: &Path, binary: Option<&Path> }`)
  and, besides today's `env.CONDUCTOR_INSTANCE` and prefix:
  - sets `env.CONDUCTOR_CONFIG` to the config file, replacing any value the input held;
  - in each hook command that runs `conductor`, after stripping the `CONDUCTOR_INSTANCE=` prefixes
    as today, replaces the first word of the rest when it is `conductor` or ends in `/conductor`
    with the binary's path. A command whose rest starts with another word keeps it (only the
    prefix is added, as today).
  - Adds no other key (`story:pinned-settings-keep-missing-hooks`: no `"hooks": null`).
- The config file is `config::config_file()` (`config.rs:1611`): the absolute file `set_active`
  located, the same for spawn, resume and session start. `controller_launch` and `session start`
  take their `CONDUCTOR_CONFIG` process env from it too (today `environment.cwd.join(file)` in
  spawn and `cwd.join(&file)` in session start, where for a host role `cwd` is the host's
  directory, not the process's).
- The binary is `std::env::current_exe()`, with two corrections, in one function with unit tests:
  - a trailing ` (deleted)` is removed: on Linux `current_exe` reads `/proc/self/exe`, which names
    the file so once an install replaced it, and the runtime keeps running the old inode until
    conductord restarts it. The path then names the new binary at the same place.
  - a path whose file name is not `conductor` keeps the bare `conductor` (`runs_conductor` and
    `foreign_pin` read the copy again at resume and in `config validate`). A path holding
    whitespace or a shell metacharacter is single-quoted; `runs_conductor` trims quotes.
  - `current_exe` resolves symlinks: an install through a symlink pins its target. Accepted: the
    copy is rewritten at every spawn and resume.
- Under a config file a session always starts with the copy: `session_settings` writes the copy
  whenever `pin_refusal` finds nothing, including for a role file that already pins the instance
  (the role's file is never changed). Without this, the acceptance holds only for roles that name
  no settings. `pin_problem`, `pin_refusal`, `unpinned`, `foreign_pin` and `config validate`'s
  notes keep reading the instance only, so their messages do not change.
- The change is in the shared function, so `session start` gets the same pin (the director's
  `conductor guard` and `conductor session claim` hooks run the binary that started it). That is
  the same defect for the director; leaving it out would need a second code path.
- Without a config file nothing changes: the built-in instance writes no pin, as today.
- Placements: `spawn` refuses outside the instance's box (`OutsideBox`, `spawn.rs:40-42`), and the
  runtime in the box spawns, so `current_exe` is the box's own path, which the session in the
  same box runs. Host roles are started by conductord through `conductor session start` from its
  own `current_exe` (`session.rs:2109`). Kubernetes runs the runtime and the sessions in one
  image. No placement mixes a host binary path with a boxed session.
- Out of scope, named for a follow-up: a session's own `conductor` calls from Bash (`conductor
  message send`, `conductor report`) still resolve `conductor` on the session's PATH. With
  `env.CONDUCTOR_CONFIG` in the copy they now at least read the instance's config. Pinning PATH
  would replace the whole PATH in the settings and is not done here.

No spec noun, field or error is added (AGENTS.md rule 1). The spec's comments that state what the
pin writes are corrected: `spec/domains/config.yaml:271-283`, `spec/domains/session.yaml:63-76`,
`spec/domains/dispatch.yaml:124-128`. If `ess specify http validate --path spec --binding
crates/conductor/http.yaml` then names a new `source_digest`, re-pin it with `task regen`, and run
`task docs-generate`.

## Acceptance

- `tests/spawn.rs`, new, failing on main: with the PATH stand-in harness (`:228-249`) and a
  second executable `conductor` stub first on a PATH that writes a marker file and exits 0, a
  spawn under a config file writes a copy whose `env.CONDUCTOR_CONFIG` is the config file and
  whose PreToolUse hook's program word is the binary under test (`CARGO_BIN_EXE_conductor`,
  canonicalized). Running that hook command with `sh -c`, the copy's `env` applied, HOME a
  temporary directory, `CONDUCTOR_CONFIG` and `CONDUCTOR_INSTANCE` otherwise removed, the stub's
  PATH, and a Bash payload of `gh pr merge 3` from the checkout on standard input: exit 2, no
  marker, and `conductor guard guard-decisions --config <file>` lists the denial. On main the stub
  runs and the hook exits 0.
- `tests/spawn.rs`: a role whose own settings already pin the instance (the `pinned()` fixture,
  `:422`) starts with the copy, not the role's file; the role's file is unchanged. The assertions
  that expect the role's own file as `--settings` (`:461-462`, `:1625`) change accordingly, and
  `spawn_starts_unpinned_settings_with_a_pinned_copy` (`:890`, the `starts_with("CONDUCTOR_INSTANCE
  =example-org conductor ")` at `:929`) expects the binary's path.
- `tests/spawn.rs`, beside the resume cases (`:1246-1371`): a resume (`controller resume-controller`) rewrites the copy with the
  resuming binary and the config file.
- `tests/confine.rs`: `session start` of the director writes the copy with the binary's path and
  `env.CONDUCTOR_CONFIG`; the assertions on the bare hook (`:566`, `:716`, `:722`) and on
  conductor-dev's own pinned file as `--settings` (`:787-788`, the comment at `:111` and `:497-498`)
  change accordingly.
- Unit tests in `config.rs`'s `mod tests` (`:8433`): `pinned` with a binary path replaces
  `conductor` and a `/old/conductor` word, keeps a hook whose rest starts with another program,
  sets `env.CONDUCTOR_CONFIG` over an existing value, adds no `hooks` key (the #85 test at
  `:8440` updated to the new signature), and is idempotent on its own output; the binary helper
  strips ` (deleted)`, quotes a path with a space, and keeps `conductor` for a file of another name.
- Stay green, changed only where they assert the copy's hook text: `tests/instance_isolation.rs`
  (`:385`), `tests/run.rs` (`:459`), `tests/host_roles.rs` (`:198`); the fixtures that only write a
  role file (`tests/work_start.rs:182`, `tests/sec_work_start.rs:162`, `tests/run_handover.rs:292`)
  are input and need no change.
- `website/docs/security/sessions.md` (`:37-38`, `:92-97`) and the module header of `spawn.rs`
  (`:28-36`) state the pin's three parts and that the process environment does not reach a
  `claude --bg` session.

None of these tests needs a Unix socket (spawn names the broker's socket path and binds none);
all run locally.

## Scope

- `crates/conductor/src/config.rs`: only `pinned` (`:1763-1817`), `write_pinned` (`:1722`),
  `session_settings` (`:1747`), a new binary-path helper beside them, and `mod tests` (`:8433`).
- `crates/conductor/src/spawn.rs`: module header, `controller_launch` (`:489-498`),
  `pinned_settings` (`:1276`).
- `crates/conductor/src/session.rs`: `write_pinned` (`:1675`) and the start's env and settings
  (`:1806-1823`).
- Tests: `tests/spawn.rs`, `tests/confine.rs`, and where they assert the copy:
  `tests/instance_isolation.rs`, `tests/run.rs`, `tests/host_roles.rs`.
- `spec/domains/{config,session,dispatch}.yaml` (comments), `crates/conductor/http.yaml` (only a
  re-pinned `source_digest`), `website/docs/security/sessions.md`.
