---
format: aep.planning-md/3
id: story:build-shared-target-opt-out
kind: story
status: active
title: A repository can opt out of the shared target directory
summary: One repository's gate and release broke on the 0.25.0 shared target
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T13:09:22Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T13:09:22Z", actor: "human:timo", revision: 4}
---
## Why

Host supervisor, 2026-11-08: since 0.25.0 (`story:build-shared-target-dir`) `build run` points a
cargo-based command's `CARGO_TARGET_DIR` at `<cache>/target/<key>`, outside the tree. One
repository's gate broke on three counts: a task runs `wasm-bindgen
target/wasm32-unknown-unknown/release/<crate>.wasm` by its in-tree path ("No such file"); 9 tests
assume the target directory lies inside the workspace (Unix-socket paths, a `StripPrefixError`);
and its parallel trees rebuild in turn in the one shared directory. Its release waited on it (K5).
The supervisor set `CARGO_TARGET_DIR=<tree>/target` for that repository as a stopgap; `build run`
already keeps a value the caller sets (`shared_target`, `crates/conductor/src/build.rs`), so that
part of the ask holds today. What is missing is a way to say it once per repository in the config.

## Acceptance

1. `repositories[]` (`conductor.config.RepositoryRule`, `spec/domains/config.yaml`) gains
   `shared_target: false`; a matching repository's cargo-based runs set `CARGO_TARGET_DIR` to
   `<tree>/target` and print one line saying so. Default (absent or true) keeps the shared target.
   Test: one repository opted out, one not.
2. A rule that writes only `shared_target` is valid (the rule's "at least one of" invariant names
   it). `conductor config validate` test.
3. `docs/config.md` and `AGENTS.md` ("Worktrees and builds") name the key.
4. A `CARGO_TARGET_DIR` the caller sets is still kept: the existing test stays green.

## Out of Scope

Making the repository's tests independent of the target directory's location: that is its
controller's work.

## Ambiguities

None.
