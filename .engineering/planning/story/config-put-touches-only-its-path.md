---
format: aep.planning-md/3
id: story:config-put-touches-only-its-path
kind: story
status: active
title: config put edits only the named path and keeps the file's style
summary: A put rewrites the whole config file in block style; it should change only the path's text, comments and style kept
tags:
- from-issue
refs:
- provider: github
  reference: beyond10x/conductor#263
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/config/edit.rs
- confidence: cited
  path: crates/conductor/src/config/instance.rs
- confidence: cited
  path: crates/conductor/src/config/put.rs
- confidence: cited
  path: crates/conductor/tests/config_globals.rs
- confidence: cited
  path: crates/conductor/tests/instance_routes.rs
- confidence: cited
  path: website/docs/concepts/config.md
- confidence: cited
  path: website/docs/guides/configure-an-instance.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:31:02Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-20T21:31:02Z", actor: "human:unknown", revision: 6}
---
## Why

Issue #263 (seen on 1.19.2; unchanged on main at 1.22.1). `conductor config put --level instance
--instance <name> --path controllers.max_working --value 5` set the value but rewrote the whole
config file in another style: a file written in flow style (`controllers: {max_working: 4,
max_subagents: 2}`) came back in block style, every instance and the globals, not only the one
path. The file is owned by another repository (the host's infra checkout), so the rewrite showed
as a large unexplained diff there.

Read in the code on main:

- `put::put_config` (`crates/conductor/src/config/put.rs:360`) parses the file into a
  `serde_yaml::Value` (`:396`), finds the level's mapping (`scope`, `:112`), sets the path in the
  value tree (`set`, `:189`), and writes the whole tree back with `serde_yaml::to_string`
  (`:445`). Style, comments, quoting and blank lines are lost there. The module doc says so
  (`put.rs:8`).
- The instance calls (`config/instance.rs:129` `add_instance`, `:156` `put_instance`, `:183`
  `remove_instance`, all through `change`, `:237`) write the same way (`instance.rs:296`).
- `put_config` is the one writer for `conductor config put` (`config.rs:7933`), `PUT /v1/config`
  and the instance API's settings writes (`daemon/api.rs:1850`), and the tuner (`tune.rs:722`).
- Prior art on main: `config migrate` already edits the file line by line and keeps comments
  (`with_prefixes`, `crates/conductor/src/config.rs:7637-7694`). It edits the lines, reads the
  edited text back, and returns `None` unless the result equals the expected value tree. This
  story follows that pattern.

The shapes conductor's own files use (`conductor.example.yaml`, the test fixtures):

- block mappings at two-space indentation;
- block sequences of block mappings whose first key sits on the dash line (`  - name: personal`);
- one-line flow mappings and sequences, as a value (`exclude: [archive]`, `source: {repository:
  policies, ...}`) and as a whole sequence item (`- {role: director, harness: claude, model:
  opus}`, which the role level targets);
- plain and quoted one-line scalars, and a block scalar (`standing_rules: |`);
- full-line and trailing comments, and blank lines between instances.

## What changes

`config put` edits the file's text at the named path only. A small locator in safe Rust lives in a
new module, `crates/conductor/src/config/edit.rs`, which `put.rs` and `instance.rs` call. It finds
the span of the level's mapping the way `scope` finds it: the root's `globals`, the `instances`
item named `name`, or that item's `roles` item named `role`. Then it finds the span of the path's
keys under that mapping.

1. The value at an existing path is replaced in place, in the style it stands in: a flow
   mapping's entry stays in the flow mapping, and a block entry stays block. Only the value's own
   text changes, nested lines included. A trailing comment on its line is kept.
2. A key the file does not hold yet is inserted into the nearest existing mapping on the path, in
   that mapping's style and indentation:
   - flow: `, key: value` before the closing brace;
   - block: a new line at the mapping's indentation after its last entry, before the blank and
     comment lines that follow it.

   Missing intermediate mappings are written in the style of their parent. A level mapping the
   file lacks (`globals`) is inserted the same way at the root.
3. A removal (`--value` absent, as today) deletes the entry's own text only: one block line with
   its nested lines, or one flow entry with its separating comma. A block mapping left with no
   entry is written `{}` on its key's line. Removing a key the file does not hold changes no text.
   For that, `set` stops creating empty mappings along the path for a removal (today it does,
   `put.rs:209-211`), so the tree and the text agree.
4. How a value is written:
   - a scalar is written as serde_yaml renders it alone, trimmed (the `scalar` helper,
     `config.rs:8256`, which `put.rs` reaches as `super::scalar`);
   - inside a flow collection, a string whose rendering holds `,`, `[`, `]`, `{`, `}` or `#` is
     written double-quoted with JSON's escaping;
   - a mapping or sequence value is written as nested block lines under a block key, and as one
     line of JSON inside a flow collection;
   - a value whose text holds a line break falls back (see 6).
5. Comments, key order, quoting and blank lines elsewhere are kept byte for byte.
6. Safety net, as in `with_prefixes`: the edited text is parsed back and compared, as a value
   tree, with the tree `set` produced (the tree today's put writes). The put falls back to today's
   whole-file rewrite when the trees differ, or when no text edit can be made. That is the case
   for a document marker `---`; an anchor, alias, merge key or tag on the path; an explicit `?`
   key; a flow collection over more than one line; or a multi-line plain or quoted scalar at the
   path.
   - The fallback is named in one line, `config put: <yaml path>: <why>; the file is written
     whole`. `put_config` writes it on standard error (the CLI's stderr; the daemon's log for `PUT
     /v1/config` and the tuner) and adds it as one line of the commit body.
   - Nothing is added to `ConfigRevision`.
   - The text, edited or rewritten, is still validated by `parse` before it is written.
7. An unchanged text writes and commits nothing. That happens when the put's value is the one the
   file already holds, in the same style. The revision is still recorded, with an empty `commit`.
   `conductor config put` (`config.rs:7933-7945`) then prints `unchanged: the file already holds
   that value` instead of `not committed: ... is in no git checkout`. Without this, the case would
   reach `git commit` with nothing to commit (`commit`, `put.rs:266-283`), which fails.
8. The instance calls (`add_instance`, `put_instance`, `remove_instance`) replace, add or remove
   only the text of that one instance's list item:
   - a replaced or added item is serde_yaml's block rendering of the mapping, indented to the
     list's items; a block scalar such as `standing_rules` keeps its indentation relative to its
     key;
   - a removed item's lines go, from its dash line through its last nested line;
   - comments inside a replaced item go with it; the lines around it are kept;
   - an `instances` key the file lacks is inserted at the root.

   The same read-back check and fallback apply.
9. Docs: update the module docs of `put.rs` (`:8-9`, "written whole ... not kept") and
   `instance.rs` (`:1-3`). `website/docs/concepts/config.md:86-88` and
   `website/docs/guides/configure-an-instance.md:126-127` say the comments are lost; they now
   describe the edit and the fallback.

No specification change: no noun, field, command, error or outcome is added (AGENTS.md rule 1).
The ESS scenarios (`spec/scenarios/config-put-*.yaml`) are command timelines with events and
views and cannot express a file's bytes, so the flow-style and fallback cases are Rust tests.
`ConfigRevision` gets no fallback field: that would be a spec change and a regeneration. No
generated file changes (`website/docs/reference/**`, `crates/conductor-model`).

## Acceptance

Each test fails first on main. The fixtures carry comments, a flow mapping and a flow role item,
so main's whole-file rewrite changes more than the asserted lines. Files and diffs are compared
as text, line by line.

- In `crates/conductor/tests/config_globals.rs`, beside
  `put_validates_writes_commits_and_records_the_change` (`:369`), through `conductor config put`:
  - A v2 file with `controllers: {max_working: 4, max_subagents: 2}` and comments, put with
    `--level instance --path controllers.max_working --value 5`. Exactly one line differs from
    the old text; it reads `controllers: {max_working: 5, max_subagents: 2}`; `config show` reads
    5.
  - Block style with a trailing comment on the line: one line changes and the comment is kept. A
    new key adds one line at the mapping's indentation. A new key under a missing intermediate
    mapping adds its lines in the parent's style. A removal removes one entry's line or lines
    only.
  - A flow mapping: a new key is appended before `}`; a removal drops the entry and its comma.
  - `--level role --role director` on `- {role: director, harness: claude, model: opus}` with
    `--path model --value sonnet`: the item stays one flow line, now with `model: sonnet`.
  - `--level global` on a file with no `globals`: the section is added and every other line is
    kept.
  - The fallback: an anchor and alias on the path (`controllers: &caps {...}` and `<<: *caps`).
    The put writes the whole file, prints exactly one stderr line naming the YAML path, the commit
    body holds that line, and `config validate` passes.
  - The same value put twice: the second put leaves the file and `HEAD` as they were, exits 0,
    and prints `unchanged`.
- In `crates/conductor/tests/instance_routes.rs`, beside
  `a_person_creates_reads_replaces_and_removes_one_instance` (`:419`), on a two-instance file with
  comments and a flow role item in each:
  - `PUT /v1/instances/{name}` changes only that instance's item's lines;
  - `POST /v1/instances` appends one item and keeps every old line;
  - `DELETE /v1/instances/{name}` removes that item's lines only.
- Unit tests in `config/edit.rs` for the locator: replace, insert and remove in block and flow;
  the trailing comment; the `- name:` first key; the blank line between instances; a block scalar
  that is not on the path; and each fallback shape of (6).
- The existing put and instance tests stay green: `config_globals.rs`, `instance_api.rs`,
  `instance_routes.rs`, `dashboard_config.rs`, `tuner.rs`, `rev3_standing_rules.rs`,
  `sec_standing_rules.rs`.
- `task check` green.

## Constraints

- Rust only; no new crate. `crates/conductor/src/lib.rs:9` is `#![forbid(unsafe_code)]`, so
  `unsafe-libyaml` (in the lockfile as serde_yaml's dependency) cannot be called: its API is
  `unsafe fn`. Cargo.lock holds no other YAML parser with positions. The locator is safe Rust over
  the text, bounded to the shapes above; everything else falls back by (6).
- Tests isolate HOME (AGENTS.md rule 5) and use `example-org` names.

## Scope

- `crates/conductor/src/config/edit.rs` (new): the locator and the edits.
- `crates/conductor/src/config/put.rs`: `put_config` writes through the edit; `set` on removal;
  the fallback line; the unchanged case; the module doc.
- `crates/conductor/src/config/instance.rs`: `change` writes through the edit; the module doc.
- `crates/conductor/src/config.rs`: only the module list (`:169-173`, the `mod edit;` line) and
  the `put` handler's `unchanged` message (`:7933-7945`).
- `crates/conductor/tests/config_globals.rs` and `crates/conductor/tests/instance_routes.rs`.
- `website/docs/concepts/config.md` and `website/docs/guides/configure-an-instance.md`.

## Boundaries

- `config migrate` stays as it is: its `with_prefixes` (`config.rs:7637`), and its whole-file
  conversion of a `/1` file (`spec/domains/config.yaml:1307`). Moving `with_prefixes` onto the
  new locator is a follow-up.
- No change to the spec, the generated model or the reference pages; `spec/` is out of scope.
- Shares no file with `story:supervisor-cap-through-config-owner` (wave 73).

Follow-ups from the wave 73 review, not fixed in this story (findings 1, 2, 3, 5 and 7 are):

- **Follow-up: an emptied `instances: []` blocks later in-place adds.** Removing the last
  instance writes `instances: []` on the key's line (`config/edit.rs:131`); the next add then
  finds a flow list and refuses (`config/edit.rs:97`, "instances is not a block list"), so every
  later add writes the file whole.
- **Follow-up: CRLF and mixed line endings.** The locator splits on `\n` and trims `\r`
  (`config/edit.rs:192`, `:225`), but inserted and moved lines are written with `\n` alone, so an
  edit of a CRLF file mixes endings in it.
- **Follow-up: the daemon API and the tuner cannot tell a no-op.** `PUT /v1/config`
  (`daemon/api.rs:1850`) and the tuner (`tune.rs:722`) call `put_config`, which drops
  `Written::unchanged`; a no-op answers like a change, with an empty `commit`.
- **Follow-up: `.nan` forces the whole rewrite.** The read-back compares value trees
  (`config/edit.rs:156`), and NaN equals nothing, so a file holding `.nan` anywhere is written
  whole on every put; the no-op check (`config/put.rs:510`, `config/instance.rs:310`) misses for
  the same reason.
