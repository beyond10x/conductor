---
format: aep.planning-md/3
id: review-result:w74-critic-panel
kind: review-result
status: active
title: Critic panel, wave 74 (guard-denial-names-its-config, spawn-pins-binary-and-config)
summary: 'guard-denial: scope PASS-WITH-CHANGES, design PASS-WITH-CHANGES, acceptance PASS-WITH-CHANGES; spawn-pins: scope PASS-WITH-CHANGES, design PASS-WITH-CHANGES, acceptance PASS-WITH-CHANGES; parallel-safety PASS; all edits taken in, no second round'
relations:
- reviews: story:guard-denial-names-its-config
- reviews: story:spawn-pins-binary-and-config
prose_only: a critic panel's per-lens verdicts in prose; every required change is taken into the story bodies
revision: 1
---
PASS-WITH-CHANGES (every required edit taken into the story bodies, guard-denial-names-its-config revision 7, spawn-pins-binary-and-config revision 21; no lens FAILed, so no second round)

Four lenses over `story:guard-denial-names-its-config` revision 5 and
`story:spawn-pins-binary-and-config` revision 18, read against the wave branch's base (release
1.22.2).

## Scope: PASS-WITH-CHANGES

guard-denial-names-its-config:

- The cited site is right: the fall-through denial in `Session::bash` (`guard/rules.rs:1530`,
  the return at `:1589-1597`).
- `crates/conductor/src/config.rs` (inferred) is removed. `gates_policy` is already a field of
  `Instance`, and the guard reads `Instance` through `Places::of` (`rules.rs:214`), so the change
  is one field on `Places` (`:134-187`).
- Missing files added:
  - `tests/doctor.rs`. Its assertion at `:1427-1428` requires the GitLab guard line to end
    "writes go through the bot route", which the change must break.
  - `website/docs/security/the-guard.md`, the Forge calls section.
- Named to stay green: `tests/guard_hook.rs:780-786`, which asserts "writes go through the bot
  route" for `gh`. The body keeps those words in the `gh` cases.

spawn-pins-binary-and-config:

- The scoper's line numbers were stale and are corrected: `pinned` `:1763`, `write_pinned`
  `:1722`, `session_settings` `:1747`, `pin_problem` `:1625`, `runs_conductor` `:1822`, the unit
  test `:8440`, `spawn::pinned_settings` `:1276` with callers `:229` (spawn) and `:657` (resume),
  `session.rs` `:1675` and `:1806-1823`.
- Added, because they assert the copy's hook text: `tests/instance_isolation.rs`, `tests/run.rs`
  and `tests/host_roles.rs`.
- Added: `crates/conductor/http.yaml`, for a re-pinned `source_digest` only.
- Every inferred path was read and is now cited.
- `website/docs/security/sessions.md:37-38` and the `spawn.rs` header (`:28-31`) claim that the
  session gets `CONDUCTOR_CONFIG`; through `claude --bg` it does not. Both are corrected.

## Design: PASS-WITH-CHANGES

guard-denial-names-its-config:

- How the guard knows the config without a store: the hook already parses the instance's config
  file on every call (`config::active()` through `places_of`, `guard.rs:249`; the file is
  `CONDUCTOR_CONFIG` or the default, `guard.rs:613`). `Places` already carries `forge_route`, the
  instance's name and the config file's path. Adding whether `gates_policy` is set needs no
  store, no extra file and no stat; the allowed path is untouched (rule 6). The `conductor.lease`
  copy plays no part: it decides only conductor-role writes the rule table allows.
- The fix the story proposed is wrong for the case that prompted it. A `glab` write on `gates`
  has no bot route at all, because the bot route writes to GitHub only (doctor's
  `BOT_ROUTE_HOSTS`), so "a policy for the bot route" would not have helped. The body now gives
  each forge and route its own fix: `gh` on `gates` with or without a policy, `glab` on `gates`
  (only `forge.route: direct`), and no config file.
- The instance is named by name and config file path. `Places` holds no `instances[<i>]` index.
  Naming the file also shows on the denial line when a hook read the wrong config, which is what
  the second story fixes.
- Already done on main: the direct route's denial of another repository's write names
  `forge.route direct` (`rules.rs:1975`, held by `tests/guard_rules.rs:6467-6539`). That line of
  the acceptance becomes "stays green". Doctor (`route_lines_of`) already reports the route
  problem at setup time, but the denial a session sees still misleads, so the story is not moot.
- No spec change: the reason is a string of the existing `GuardDecision`.

spawn-pins-binary-and-config:

- Sound, with five corrections taken in:
  1. `session_settings` returns a role file that already pins the instance unchanged, so that
     file would get no binary or config pin. Under a config file the copy is now always written.
     `pin_problem` and `config validate` keep reading the instance only.
  2. `current_exe` on Linux names a replaced binary `<path> (deleted)`. The runtime runs the old
     inode until conductord restarts it, so the suffix is stripped. A file name other than
     `conductor` keeps the bare word, so `runs_conductor` and `foreign_pin` still read the copy.
     A path with whitespace is quoted.
  3. The config path comes from `config::config_file()` for spawn, resume and session start.
     Today they differ, and session start joins a relative path to the host role's directory.
  4. The change is in the shared `config::pinned`, so `session start` (the director, host roles)
     gets the same pin. The director has the same defect.
  5. The process env `controller_launch` sets does not reach a `claude --bg` session. The settings
     `env` does: the reported workaround worked through it.
- Boxes and other placements: `spawn` refuses outside the instance's box (`OutsideBox`), so the
  spawner's `current_exe` is the box's own path. conductord starts host roles from its own
  `current_exe`. Kubernetes runs one image. No placement pins a host path into a boxed session.
- Named out of scope: a session's own Bash `conductor` calls still resolve on PATH, but now read
  the instance's config.
- Spec: no noun, field or error is added. Three spec comments that state the pin's content are
  corrected, which may re-pin `http.yaml`'s `source_digest`.

## Acceptance: PASS-WITH-CHANGES

- guard-denial-names-its-config:
  - Four cases in `tests/guard_rules.rs` through `forge_layout` (`:6372`) with an optional
    `gates_policy`, plus the no-config-file case. Each fails on main, since the reason names no
    instance, file or policy.
  - The `tests/doctor.rs:1427-1428` change, and a no-line-break check.
- spawn-pins-binary-and-config:
  - The PATH-shadow test is made concrete: a stub `conductor` first on PATH writes a marker. The
    copy's hook is run with `sh -c` and the copy's `env`, on a `gh pr merge` payload. It must
    exit 2 with no marker, and `guard guard-decisions --config` must list the denial. On main the
    stub answers 0.
  - Added: an own-pinned role file gets the copy; a resume rewrites the copy; `session start`'s
    copy; unit tests for `pinned` (idempotent, no `hooks` key, `env.CONDUCTOR_CONFIG` replaced)
    and for the binary helper (` (deleted)`, a space, another file name).
- No test of either story needs a Unix socket (spawn names the broker socket path and binds
  none), so none is CI-only.

## Parallel safety: PASS

- No shared file:
  - guard-denial-names-its-config: `crates/conductor/src/guard/rules.rs`,
    `tests/guard_rules.rs`, `tests/doctor.rs`, `website/docs/security/the-guard.md`;
  - spawn-pins-binary-and-config: `crates/conductor/src/{config,spawn,session}.rs`,
    `tests/{spawn,confine,instance_isolation,run,host_roles}.rs`,
    `spec/domains/{config,session,dispatch}.yaml`, `crates/conductor/http.yaml`,
    `website/docs/security/sessions.md`.
- The guard story left `config.rs` out of its scope, so only the spawn story edits it (`pinned`,
  `write_pinned`, `session_settings` and `mod tests`), and no merge conflict is possible there.
- `aep artifact waves` prints no collision between the two.
- Only the spawn story touches `spec/`, so a `task regen` or `task docs-generate` of one does not
  reach the other's files.
- The guard reads its config through `CONDUCTOR_CONFIG`, which the spawn story sets. The guard
  story's tests build `Places` directly and do not depend on it.
