---
format: aep.planning-md/3
id: story:session-envelope-seam
kind: story
status: draft
title: A session starts only in the envelope its role requests
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-envelope-model
- depends_on: story:session-box-spike
- depends_on: story:spawn
- depends_on: story:instance-session-names
- depends_on: story:doctor-minimum-versions
- depends_on: story:profiles-forge-neutral
- depends_on: story:version-flag
- depends_on: story:sandbox-spec
- depends_on: story:gitlab-guide
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/collect/sessions.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/confine/places.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/confine.rs
- confidence: cited
  path: crates/conductor/tests/docs_forge.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: website/docs/guides/start-conductor.md
- confidence: cited
  path: website/docs/security/confinement.md
revision: 27
---
## Why

Sessions start in three places with no common path: `conductor spawn` launches a controller with
`claude --bg` itself (`crates/conductor/src/spawn.rs:170-192`), and `Taskfile.yml`
`conductor:start`, `conductor:restart` and `dev:start` call `claude --bg` and `claude stop`
directly. Nothing applies or measures the config's `confinement`: the sessions collector records
every `envelope_*` field absent (`collect/sessions.rs:327-335`). The operator, 2026-10-13: "we
need to prioritise the sandbox".

## Acceptance

- A `confine` module (`crates/conductor/src/confine/mod.rs`) with a `Provider` trait whose methods
  are the whole session surface: `available` (probe), `start` (from a request of absolute paths and
  limits, returns the started session's identity), `prompt` (delivers the first prompt or charter),
  `stop`, `resume` and `measure`. The `none` provider implements them with today's commands
  (`claude --bg`, `claude stop`, `claude --resume`) and measures nothing; `conductor spawn` and
  `session start` use only this trait.
- A place resolver (`crates/conductor/src/confine/places.rs`) turns each of a role's `writable`
  place words (`checkout`, `trees`, `build-cache`, `scratch`, `harness-state`, `records`) into
  existing absolute paths before the request reaches a provider; a test in
  `crates/conductor/tests/confine.rs` reads the paths of each word for a controller and for
  conductor.
- `session start --role` with a fake provider, one test per declared refusal
  (`crates/conductor/tests/confine.rs`): `strict` and provider unavailable exits non-zero naming
  the reason and starts nothing; `strict` and a measurement short of the request stops the session
  and exits non-zero; an unknown role, a session already running under the role's name, and a launch
  that fails each exit non-zero with their error; under `report` with the provider unavailable the
  session starts and its measurement says unboxed with the reason.
- The measurement of a started session is written to `<state>/sessions/<name>.envelope.json`, and
  the sessions collector fills that session's `envelope_*` fields from it; a test runs `session
  start` with the fake provider, then a snapshot, and reads them in `conductor snapshot sessions`.
- `conductor spawn` starts a controller through the trait: a test in
  `crates/conductor/tests/spawn.rs` with the fake provider sees one `start` and one `prompt` per
  spawn, with the controller role's request, and the `none` provider keeps today's argv.
- `conductor doctor` prints one provider row per provider after the program rows and before the
  verdict line, which stays last; a test with two fake providers, one available and one not, in
  `crates/conductor/tests/doctor.rs`.
- `Taskfile.yml` `conductor:start`, `conductor:restart` and `dev:start` call `conductor session
  start --role …` (and the restart stops through the provider) instead of `claude --bg`/`claude
  stop`; a test in `crates/conductor/tests/taskfile.rs` reads the three tasks.
- `.agents/conductor.md` no longer contains `claude --bg` and names `conductor spawn` and `conductor
  session start` as the only start forms; a test in `crates/conductor/tests/profiles.rs` holds both.
  The diff merges under `approval-record:sandbox-priority`.
- The hand-written pages `website/docs/security/confinement.md`, `website/docs/guides/start-conductor.md`
  and the `confinement.mode`/`provider` rows of `docs/config.md` describe the start path; the
  phrase "nothing starts a session in a box yet" is gone (a test in
  `crates/conductor/tests/docs_forge.rs` reads it).

## Notes

- Decided: the measurement is a file per session in the instance's state; controllers start only
  through `conductor spawn`.
- After `story:sandbox-spec` (every type and refusal above is declared there) and
  `story:gitlab-guide` (wave 17's last story: `doctor.rs`, `tests/doctor.rs`,
  `tests/watch.rs`, `.agents/conductor.md`, `tests/profiles.rs` and `tests/docs_forge.rs` are
  shared with wave 17).
- `story:session-limits-systemd`, `story:session-writes-bwrap` and
  `story:session-forge-credentials` implement this trait in `confine/`, after it.

## Scope

`crates/conductor/src/confine/mod.rs` (new), `crates/conductor/src/confine/places.rs` (new),
`crates/conductor/src/lib.rs`, `crates/conductor/src/cli.rs`, `crates/conductor/src/session.rs`,
`crates/conductor/src/spawn.rs`, `crates/conductor/src/doctor.rs`,
`crates/conductor/src/collect/sessions.rs`, `Taskfile.yml`, `.agents/conductor.md`,
`website/docs/security/confinement.md`, `website/docs/guides/start-conductor.md`, `docs/config.md`,
`crates/conductor/tests/confine.rs` (new), `crates/conductor/tests/spawn.rs`,
`crates/conductor/tests/doctor.rs`, `crates/conductor/tests/taskfile.rs`,
`crates/conductor/tests/watch.rs`, `crates/conductor/tests/profiles.rs`,
`crates/conductor/tests/docs_forge.rs`.
