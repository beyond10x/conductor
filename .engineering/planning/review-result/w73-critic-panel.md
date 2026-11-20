---
format: aep.planning-md/3
id: review-result:w73-critic-panel
kind: review-result
status: active
title: Critic panel, wave 73 (config-put-touches-only-its-path, supervisor-cap-through-config-owner)
summary: 'config-put: scope PASS-WITH-CHANGES, design PASS-WITH-CHANGES, acceptance PASS-WITH-CHANGES; supervisor-cap: scope PASS-WITH-CHANGES, design PASS-WITH-CHANGES, acceptance PASS-WITH-CHANGES; parallel-safety PASS; all edits taken in, no second round'
relations:
- reviews: story:config-put-touches-only-its-path
- reviews: story:supervisor-cap-through-config-owner
revision: 1
---
PASS-WITH-CHANGES (every required edit taken into revision 3 of both stories' bodies and revision 4 of their scopes; no lens FAILed, so no second round)

Four lenses over `story:config-put-touches-only-its-path` revision 2 and `story:supervisor-cap-through-config-owner` revision 2, read against the wave branch's base (release 1.22.1).

## Scope: PASS-WITH-CHANGES

config-put-touches-only-its-path:

- Kept as one unit: one writer (`put_config`) and one sibling writer (`instance::change`) share one new edit module.
- Citations checked: `put.rs:360` and `:445`, `instance.rs:156` and `:296` are correct. Added `scope` `:112`, `set` `:189`, `add_instance` `:129`, `remove_instance` `:183` and `change` `:237`, and the other callers (`config.rs:7933`, `daemon/api.rs:1850`, `tune.rs:722`).
- The wrong test file was named. `tests/config.rs` holds no `config put` case; the put cases are in `tests/config_globals.rs` (`:369`, `:497`, `:613`), and the instance calls are tested through `PUT/POST/DELETE /v1/instances` in `tests/instance_routes.rs` (`:419`). There is no `conductor config instance put` command; the acceptance now uses the routes.
- Missing files added: `website/docs/concepts/config.md:86-88` and `website/docs/guides/configure-an-instance.md:126-127`, which say a put loses the comments; the module docs of `put.rs:8-9` and `instance.rs:1-3`; and `config.rs`, limited to the `mod edit;` line and the `put` handler's message for an unchanged file.
- Removed from scope: `spec/domains/config.yaml` and `spec/scenarios` (see Design). Generated pages (`website/docs/reference/**`) carry no text about the write and stay unchanged.

supervisor-cap-through-config-owner:

- One unit. The row citation `supervisor.md:238` is correct, and the section heading is at `:231`.
- Wrong test file. `tests/host_roles.rs` tests the daemon starting host roles. The profile-text cases, with the supervisor's line cap of 463, are in `tests/profiles.rs` (`:154-174`). The scope swaps `host_roles.rs` for `profiles.rs`.
- No mirror of the profile under `website/` or in any generated file.

## Design: PASS-WITH-CHANGES

config-put-touches-only-its-path:

- The draft's route through `unsafe-libyaml` cannot be built: `crates/conductor/src/lib.rs:9` is `#![forbid(unsafe_code)]`, and every `unsafe-libyaml` entry point is `unsafe fn`. Cargo.lock holds no other YAML parser with positions (only `serde_yaml`, `unsafe-libyaml`, `toml_edit`). The body now names a safe-Rust locator in a new `config/edit.rs`, bounded to the shapes conductor's files use.
- Those shapes are taken from `conductor.example.yaml`: block mappings; `- name:` items with the first key on the dash line; one-line flow values; one-line flow sequence items such as `- {role: director, ...}`, which the role level edits and the draft did not mention; block scalars (`standing_rules`); trailing comments; blank lines between instances.
- Prior art named: `with_prefixes` (`config.rs:7637-7694`) already edits lines and reads the text back against the expected tree, returning `None` when they differ. The safety net in (6) follows it. A read-back that differs from `set`'s tree now falls back; the draft wrongly called it "a defect the test names".
- Fallback rule corrected. "The revision records the same" needs a `ConfigRevision` field, which means a spec change and a regeneration. The fallback is now named in one stderr line from `put_config` and one line of the commit body, with no new field. The fallback shapes are named: `---`, anchors, aliases, merge keys, tags, explicit `?` keys, multi-line flow collections, multi-line scalars at the path, and values with a line break.
- New edge cases closed:
  - A put of the value the file already holds now leaves the text unchanged. That would reach `git commit` with nothing to commit (`put.rs:266-283`) and fail, so it now writes and commits nothing and prints `unchanged`.
  - A removal no longer creates empty mappings along the path (`put.rs:209-211`), so tree and text agree.
  - A block mapping emptied by a removal is written `{}`.
  - A string with flow indicators inside a flow collection is double-quoted.
  - Collection values are written as nested block lines in block context and as one line of JSON in flow context.
  - Missing `globals` or `instances` are inserted at the root.
- Spec (AGENTS.md rule 1): no noun, field, command, error or outcome is added, so no spec change is needed. ESS scenarios are command, event and view timelines and cannot express a file's bytes, so the "scenario for a flow-style file" becomes a Rust test.
- Instance calls: a replaced or added item is serde's block rendering, indented to the list's items. Comments inside a replaced item go with it; everything around it is kept.

supervisor-cap-through-config-owner:

- Sound. "the host's file" is now "the host's own file", the profile's existing term for `host/supervisor.md` (`:29`, `:82`), so no owner is named in the product (rule 9). The row stays one line, and the profile stays at 462 of its 463 lines.

## Acceptance: PASS-WITH-CHANGES

- config-put-touches-only-its-path:
  - Each case is made to fail first on main by requiring fixtures with comments, a flow mapping and a flow role item. On a file already in serde's own block form, main's rewrite would reproduce the file and the "one line changed" test would pass on main.
  - Cases added: the role-level flow item; a global put on a file without `globals`; a flow insert and removal; a missing intermediate mapping; the fallback (one stderr line, the commit body, `config validate`); a same-value put (file and `HEAD` unchanged, `unchanged` printed); the instance routes (put, add, remove each touch one item's lines); and unit tests of the locator for each shape and fallback.
  - The existing put and instance tests are named to stay green.
- supervisor-cap-through-config-owner: the case reads the profile through `profiles.rs`'s whitespace-joined `profile()` and asserts the exact appended words. It fails first on main. `roles.rs` already holds `role show` to the file, so no binary test is needed.

## Parallel safety: PASS

- The scopes are disjoint by path component:
  - config-put: `crates/conductor/src/config/{put,instance,edit}.rs`, `crates/conductor/src/config.rs`, `tests/config_globals.rs`, `tests/instance_routes.rs`, and two hand-written website pages;
  - supervisor-cap: `crates/conductor/roles/supervisor.md` and `tests/profiles.rs`.
- Neither touches `spec/`, `crates/conductor-model` or a generated page, so `task docs-generate` and `task regen` produce no shared file.
