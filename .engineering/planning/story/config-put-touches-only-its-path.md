---
format: aep.planning-md/3
id: story:config-put-touches-only-its-path
kind: story
status: draft
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
  path: crates/conductor/src/config/instance.rs
- confidence: cited
  path: crates/conductor/src/config/put.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/scenarios
revision: 2
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
  `serde_yaml::Value`, sets the path in the value tree (`set`), and writes the whole tree back with
  `serde_yaml::to_string` (`put.rs:445`). Style, comments, quoting and blank lines are lost there.
- The instance calls (`config/instance.rs:156` `put_instance`, `add_instance`, `remove_instance`,
  through `change`) write the same way (`instance.rs:296`).

## What changes

`config put` edits the file's text at the named path only:

1. The value at an existing path is replaced in place, in the style it stands in (a flow
   mapping's entry stays in the flow mapping; a block entry stays block); every other byte of the
   file is unchanged.
2. A key the file does not hold yet is inserted into the nearest existing mapping on the path, in
   that mapping's style and indentation (flow: `, key: value` before the closing brace; block: a
   new line at the mapping's indentation after its last entry); missing intermediate mappings are
   written in the style of their parent.
3. A removal (`--value` absent, as today) deletes the entry's own text only (one block line with
   its nested lines, or one flow entry with its separating comma).
4. Comments, key order, quoting and blank lines elsewhere are kept byte for byte.
5. The edited text is still parsed and validated (`parse`) before it is written; a result that
   does not parse to the same value tree as the old put would have produced is a defect the test
   names (the edit is checked against today's tree-based result: equal as values).
6. When no text edit can be made safely (an anchor, alias or merge key on the path, a multi-line
   flow collection, a tag), the put falls back to today's whole-file rewrite and its result says so
   on standard error in one line naming the YAML path; the revision records the same.
7. The instance calls (`put_instance`, `add_instance`, `remove_instance`) replace, add or remove
   only the text of that one instance's list item, by the same rules.

The specification records the behaviour: a scenario for a flow-style file whose put changes one
line, and one for the fallback. No new noun is expected; if the specification needs a field (such
as the fallback named on the revision), it is modelled first.

## Acceptance

- A test that fails first on main: a v2 config in flow style with comments; `config put --level
  instance --path controllers.max_working --value 5`; the file's diff is exactly one line, and
  `config show` reads 5.
- Block style: the same, one line changed; a new key adds one line; a removal removes one entry.
- `config instance put` on a two-instance file changes only that instance's item.
- The fallback case writes the whole file, prints one line naming the path, and still validates.
- `task check` green (spec, conformance, regeneration, clippy, tests, docs).

## Constraints

- Rust only; no new crate from a registry (the box has none). `unsafe-libyaml` is already in the
  lockfile (it gives event marks with byte indexes) if a parser with positions is needed; else a
  small locator over the text bounded to the shapes conductor's own files use.
- Tests isolate HOME (AGENTS.md rule 5) and use `example-org` names.
