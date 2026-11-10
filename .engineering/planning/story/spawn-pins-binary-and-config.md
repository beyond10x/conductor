---
format: aep.planning-md/3
id: story:spawn-pins-binary-and-config
kind: story
status: active
title: A spawned session's hooks run the spawning binary against the instance's config
scope:
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/conductor/tests/confine.rs
- confidence: inferred
  path: crates/conductor/tests/spawn.rs
- confidence: inferred
  path: spec/domains/config.yaml
- confidence: inferred
  path: spec/domains/dispatch.yaml
- confidence: inferred
  path: spec/domains/session.yaml
- confidence: inferred
  path: website/docs/security/sessions.md
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:00Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:01Z", actor: "human:timo", revision: 6}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from the first conductor 1.0.1 run of a trial instance. 1.0.1 was installed beside an older `conductor` that comes first on the login PATH;
conductord.service and conductor_<instance>-runtime.service carry PATH and CONDUCTOR_CONFIG for 1.0.1.
After `conductor spawn` (`claude --bg`), `/proc/<pid>/environ` of the lead showed the login PATH
and no CONDUCTOR_CONFIG. `<state>/sessions/<name>.settings.json` held only
`env.CONDUCTOR_INSTANCE` and the hook `CONDUCTOR_INSTANCE=<i> conductor guard
record-guard-decision --from-pre-tool-use || exit 2`, with conductor by bare name. The hook and
`conductor message send` ran the older binary against the default config, and `conductor guard
guard-decisions` of the instance stayed empty. The header of `crates/conductor/src/spawn.rs` says
the session gets CONDUCTOR_INSTANCE and CONDUCTOR_CONFIG.

Workaround used: a role `settings` file with env.PATH and env.CONDUCTOR_CONFIG.

## Acceptance

- The pinned settings copy a spawn writes names the binary that spawned it by absolute path
  (current_exe) in every hook, and sets env.CONDUCTOR_CONFIG to the config file the instance runs
  from; a test with a second `conductor` first on PATH shows the hook running the spawning binary
  against that config.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` settings pinning (`config::pinned` and its callers) — inferred, the story names only `spawn.rs`; the tree puts the rewrite in `config.rs`
- **Files:** `crates/conductor/src/spawn.rs` — cited, named in the story; `pinned_settings` at :818, the config path computed at :456-465
- **Files:** `crates/conductor/src/config.rs` — inferred, `pinned` :1285, `write_pinned` :1239, `session_settings` :1264, unit test :6435
- **Files:** `crates/conductor/tests/spawn.rs` — inferred, :677 and :731 assert the bare-name hook; the PATH stand-in harness for the new test is at :228-241
- **Symbols:** `config::pinned`, `config::write_pinned`, `config::session_settings`, `spawn::pinned_settings`, `config::CONFIG_VARIABLE`, `std::env::current_exe` — inferred, reached by `git grep` from the story's `<session>.settings.json` and `CONDUCTOR_CONFIG`
- **Also likely:** `crates/conductor/src/session.rs` — inferred, calls `config::write_pinned` :1060 and `config::session_settings` :1185, so a signature change reaches it
- **Also likely:** `crates/conductor/tests/confine.rs` — inferred, :564, :714, :720 assert the bare-name hook for `session start`; breaks only if the shared function changes for every caller
- **Documents:** `spec/domains/dispatch.yaml` — inferred, :121 describes the pinned copy of `spawn`
- **Documents:** `spec/domains/session.yaml` — inferred, :63-65 state what the pin writes
- **Documents:** `spec/domains/config.yaml` — inferred, :236 states the hook form `CONDUCTOR_INSTANCE=<name> conductor guard …`
- **Documents:** `website/docs/security/sessions.md` — inferred, :69 states the pin's content
- **Confidence:** medium — the defect site is read from the tree (`config.rs:1285-1323` writes only `CONDUCTOR_INSTANCE` and keeps the bare `conductor`), but whether the fix stays spawn-only or changes the function `session start` shares is the implementor's choice
- **Would collide with:** any unit touching settings pinning in `crates/conductor/src/config.rs` (`pinned`, `write_pinned`, `session_settings`, `pin_problem`), the launch path of `spawn.rs` or `session.rs`, or the hook-text assertions in `tests/spawn.rs` and `tests/confine.rs` — inferred
- **Safety fact:** an absolute-path hook still counts as pinned: `runs_conductor` (`config.rs:1341`) accepts a word ending in `/conductor`, and `unpinned` and `foreign_pin` (`config.rs:1185-1222`) test only the `CONDUCTOR_INSTANCE=<name> ` prefix, so `pin_problem` and `config validate` do not start refusing the rewritten copy — step 2, unproven, inferred. It fails if the `current_exe` path holds whitespace or its file name is not `conductor`.
