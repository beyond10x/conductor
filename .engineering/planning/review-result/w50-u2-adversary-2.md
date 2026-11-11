---
format: aep.planning-md/3
id: review-result:w50-u2-adversary-2
kind: review-result
status: active
title: Wave 50 U2 box-runtime-starts, adversary pass 2
relations:
- reviews: story:box-runtime-starts
revision: 1
---
Adversary pass 2 on `story:box-runtime-starts`, commit b41cfe1d4 on base 4d39c3c5c.

Verdict: NEEDS-CHANGE. Four findings, all introduced by the correction.

Cases added in `crates/conductor/tests/adv_box_mounts_pass2.rs` (run alone first: 4 passed, 2 failed):

- red `setup_apply_beside_a_store_at_the_default_path_writes_a_file_validate_passes_or_says_why`
- red `a_box_handed_a_store_without_a_directory_is_refused_or_writes_that_store`
- green `the_commands_before_the_config_file_leave_apply_a_file_validate_passes_twice`
- green `a_store_directory_that_holds_only_the_daemons_directory_is_refused`
- green `a_records_directory_that_holds_the_file_the_config_link_names_is_refused`
- green `every_config_file_a_page_shows_with_a_box_is_one_conductord_admits`

Suite after the cases (placement, adv_box_mounts, adv_box_mounts_pass2, setup, config, five docs
binaries, isolated_home, no_organization): `170 tests run: 168 passed, 2 failed, 1 skipped`.

Findings:

- F1 (warning, `setup.rs:528`): with a database at the old default path and no config file,
  `setup --apply` writes the old store beside `placement: systemd`, ends `ready` with exit 0, and
  `config validate` and conductord refuse that file as `placement.writable-root-shared`.
- F2 (note, `placement.rs:722`): a sqlite store that is not absolute passes `config validate` and
  yields a box with no `store` root and no refusal.
- F3 (note, `tests/adv_box_mounts.rs:316`): the changed assertion of the config-link case passes
  on a request that is refused.
- F4 (note, `placement.rs:1085`): no case holds the bound of 16 files to read.

Attacked and not broken: a store through a symlinked directory (substrate refuses a root reached
through a link); `sqlite:~/x.sqlite`; a postgres store (no store root, the rest unchanged); nested
or equal state, records and cache; cache or state under the daemon's directory; hidden entries,
links and nested records in the entry list; `..` above the root and an empty path; a second
`--apply` and a dry run; `config validate` against conductord for a written `daemon.store`; every
whole config with a box on a docs page is admitted.

```findings
- file: crates/conductor/src/setup.rs
  line: 528
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"with a database at the old default path and no config file, setup --apply writes the old store beside placement systemd and ends ready, and config validate and conductord refuse that file as placement.writable-root-shared"'
- file: crates/conductor/src/placement.rs
  line: 722
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: '"a sqlite store written without a directory passes config validate and yields a box with no store root and no refusal, so the runtime would open another database in its state directory; no page shows such a path"'
- file: crates/conductor/tests/adv_box_mounts.rs
  line: 316
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"the changed assertion of the config-link case passes on a request that is refused as placement.writable-root-shared, so it holds a mount conductord never sends"'
- file: crates/conductor/src/placement.rs
  line: 1085
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"no case holds the bound of 16 files to read, so dropping that row of the bounds leaves the suite green"'
```
