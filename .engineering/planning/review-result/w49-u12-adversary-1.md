---
format: aep.planning-md/3
id: review-result:w49-u12-adversary-1
kind: review-result
status: active
title: Wave 49 U12 box-runtime-starts, adversary pass 1
relations:
- reviews: story:box-runtime-starts
revision: 4
---
unit: U12 story:box-runtime-starts, commit ebb9c53f4 on base 3a27c6469, worktree ~/.local/state/worktree/trees/b10x/conductor/w49-u12
verdict: NEEDS-CHANGE
cases: executed 36→43, red 7
origin: introduced 6, pre-existing 1, undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: the live run must decide F3 (does the loop fail or lose its exports without a writable cache); F2 is a design call (where the store lives in the documented layout)

## 1. git --no-pager diff --stat

Empty: the only change is one untracked test file.

```
?? crates/conductor/tests/adv_box_mounts.rs
```

No implementation file touched.

## 2. Cases added

File: `~/.local/state/worktree/trees/b10x/conductor/w49-u12/crates/conductor/tests/adv_box_mounts.rs`.
Each case parses a config as the docs write it (`config::parse`), calls `placement::request` with
the env conductord hands on (`HOME`, `PATH` finding nothing, `daemon.store` as written), and
checks the request against a port of substrate-wire's lexical rules at the pinned revision
5466930 (`is_absolute_canonical`, `is_path_beneath`, the overlap checks of
`validate_read_only_roots`, `validate_read_only_files`, `validate_writable_roots`) or against
what this unit's docs promise. All seven are red now.

Order: the brief allowed one build request, so the cases did not run alone first. They were all
written before anything ran, and the one run is both the first execution of each case and the
suite run of part 3.

| # | case | asserts | red output (paths shortened to `<case>`) |
|---|---|---|---|
| F1 | `the_box_of_the_configure_an_instance_guide_is_admitted_by_substrate` | the config of `guides/configure-an-instance.md` § 1 gives a request substrate admits | `left: Some("exec.writable-root-mount-overlap: records-personal at <case>/<home>/example-org/records and <case>/<home>/example-org")`, `right: None` (adv_box_mounts.rs:202) |
| F2a | `the_getting_started_box_cannot_write_the_config_file_it_is_said_to_read` | in the getting-started layout the config file is under no writable root | `<case>/<home>/.b10x/conductor/conductor.yaml is under a writable root: [HostRoot { name: "store", path: "<case>/<home>/.b10x/conductor" }]` (:220) |
| F2b | `the_getting_started_box_cannot_write_the_daemons_own_directory` | same layout: `daemon/runtimes.json`, `daemon/binaries.json`, `daemon/binaries/<v>/conductor` are under no writable root | `<case>/<home>/.b10x/conductor/daemon/runtimes.json is under a writable root: [HostRoot { name: "store", ... }]` (:246) |
| F3 | `the_box_writes_the_cache_directory_the_loop_exports_into` | `<cache>/exports` is under a writable root | `<case>/<home>/.cache/b10x/conductor/home/exports is under no writable root: [HostRoot { name: "store", ... }]` (:269) |
| F4 | `a_records_path_written_with_a_trailing_slash_is_a_mount_substrate_admits` | `records: /…/records/` gives a canonical mount | `left: Some("exec.writable-root-mount-invalid: records-alpha at \"<case>/own/records/\"")`, `right: None` (:293) |
| F5 | `a_config_file_that_is_a_link_in_a_writable_root_is_read_in_the_box` | the file a config link names is in some mount | `<case>/<home>/dotfiles/conductor.yaml is the file the link in the box names and no mount holds it: files [], roots [...] [...]` (:326) |
| F6 | `two_instances_of_one_file_name_two_state_roots` | instances `a.b` and `a_b` get different root names | `left: "state-a_b"`, `right: "state-a_b"` (:359) |

## 3. The suite run

Command, from the worktree (one request, the only one made):

```
conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test placement --test adv_box_mounts --no-fail-fast
```

Exit status 100. Summary, verbatim:

```
     Summary [   1.039s] 43 tests run: 36 passed, 7 failed, 0 skipped
        FAIL [   0.007s] ( 9/43) conductor-cli::adv_box_mounts the_getting_started_box_cannot_write_the_daemons_own_directory
        FAIL [   0.007s] (10/43) conductor-cli::adv_box_mounts the_box_writes_the_cache_directory_the_loop_exports_into
        FAIL [   0.007s] (11/43) conductor-cli::adv_box_mounts a_records_path_written_with_a_trailing_slash_is_a_mount_substrate_admits
        FAIL [   0.007s] (12/43) conductor-cli::adv_box_mounts the_box_of_the_configure_an_instance_guide_is_admitted_by_substrate
        FAIL [   0.013s] (13/43) conductor-cli::adv_box_mounts two_instances_of_one_file_name_two_state_roots
        FAIL [   0.013s] (14/43) conductor-cli::adv_box_mounts a_config_file_that_is_a_link_in_a_writable_root_is_read_in_the_box
        FAIL [   0.013s] (15/43) conductor-cli::adv_box_mounts the_getting_started_box_cannot_write_the_config_file_it_is_said_to_read
error: test run failed
```

All 36 cases of `placement` pass. `36` is that binary's count in this same run; the seven added
cases are the only failures. Full log: `~/.cache/scratch/<session>/w49/u12/adv/run1.log`.

## 4. Findings

Covers commit ebb9c53f4. Origins were decided by reading the base with `git show 3a27c6469:…`,
not by running there.

| # | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F1 | crates/conductor/src/placement.rs:771 | NEEDS-CHANGE | introduced | A read-only root that contains a writable root is pushed beside it (`admit` keeps writable entries and adds the outer read-only root). Substrate's `validate_writable_roots` refuses that as `exec.writable-root-mount-overlap`. At base the only writable root was `<state>/sessions`, so there was no nest. | The config printed in `website/docs/guides/configure-an-instance.md:31` (`checkouts.root: ~/example-org`, `records: ~/example-org/records`, `placement: linux`) and in `guides/gitlab.md`. That box can never open. |
| F2 | crates/conductor/src/placement.rs:747 | NEEDS-CHANGE | introduced | In the getting-started layout the one writable root is `store=~/.b10x/conductor`. It holds the config file (dropped as "seen", so not read-only), and conductord's own directory `~/.b10x/conductor/daemon` (`daemon/mod.rs:44`): `runtimes.json` with every runtime's API token, which `box_request` strips from the box env on purpose, `binaries.json`, and the upgraded binaries. The boxed runtime can rewrite its own `placement` and limits, which conductord applies at its next pass. `security/sessions.md:145` says "and nothing else of the host"; `getting-started.md:206` says "read-only, the config file". The unit's own test asserts the config is not a read-only file there. | getting-started steps 3 to 6 exactly as written; `conductor daemon serve --install` uses the default daemon directory. |
| F3 | crates/conductor/src/placement.rs:779 | CONFIRMED | introduced | The `mounts` doc says the list is everything the runtime and the loop write. The instance's `cache` is not in it. The loop writes `<cache>/exports` each pass (`collect/blockers.rs:151`) and the decider runs in `<cache>/decide` (`decide/model.rs:122`). The assertion is also red when read against the base source; I call it introduced because this unit is what makes the loop run in a box at all, and the claim in the doc comment is new. | Every instance from a config file; the default cache is `~/.cache/b10x/conductor/<name>`, outside every root. Whether the write fails or lands on a throwaway filesystem needs the live run. |
| F4 | crates/conductor/src/placement.rs:835 | CONFIRMED | introduced | `config::parse` keeps an absolute path as written (`config.rs:2292`), `mounts` passes `instance.records` through unchanged, and substrate refuses a mount ending in `/` (`exec.writable-root-mount-invalid`). `at_or_under` also misses a child of a root written with a trailing slash, so such a pair would nest. Records were not mounted at base. | An operator who writes `records: /srv/x/records/`; `config validate` accepts it. |
| F5 | crates/conductor/src/placement.rs:856 | INFEASIBLE | introduced | A config file that is a symlink inside a writable root is dropped as already seen, and its target is mounted nowhere, so the link dangles in the box. Programs get the fallback at :875 (`target != found`), the config does not. | Nothing found: no doc tells anyone to link the config file. I built the state. |
| F6 | crates/conductor/src/placement.rs:570 | CONFIRMED | pre-existing | `a.b` and `a_b` are both valid instance names (`config.rs:3298`) and both map to `state-a_b` / `records-a_b`. substrate-daemon refuses a name declared twice, and the request is checked by name only, so the second instance's box would get the first's directory at its own path. Base had the same mapping for `sessions-<instance>`. | Two instances in one file whose names differ only in `.` versus `_`; `config validate` accepts them. |

Residue without a case:

- crates/conductor/src/placement.rs:812, CONFIRMED, pre-existing, note: read-only roots (binary, harness, checkouts) are passed unresolved, and substrate refuses a root reached through a symlink (`exec.read-only-root-not-a-directory`). Reached by a checkouts root that is a symlink. Not in the table or the block because I did not run it.

## 5. Attacked and not broken

- `/a/b` against `/a/bc`: `at_or_under` checks the component boundary.
- Equal state and records paths: one root, the first name.
- Writable root inside another in any order: one root, the outer one, no nest.
- Postgres store, relative sqlite path: no store root, nothing else changes.
- Store URL forms: `store_dir` parses exactly as `store::Location::parse` does.
- HOME: the service env never carries one, `box_request` always adds the daemon's, and conductord refuses to start without an absolute HOME.
- Dangling harness link: not found on PATH, so no mount; a link in a mounted directory gets its target at its own path.
- Absent forge login files: skipped by `is_file`.
- Bounds: at most 3 read-only roots, files deduplicated by mount, at most 3 writable roots.
- Docs: README, SETUP and the five docs pages name the same `--writable-root <name>=<path>` the refusal in `roots_unserved` prints; no `sessions-<instance>` is left anywhere.
- Checkouts root, binary directory and HOME as a whole are never writable in the documented layouts. `sqlite:~/x.sqlite` would make HOME writable; that is the operator's own choice of store path.

## 6. Paths written outside the worktree

- ~/.cache/scratch/<session>/w49/u12/adv/run1.log
- ~/.cache/scratch/<session>/w49/u12/adv/run1.exit

Plus this report, ~/.cache/scratch/<session>/w49/u12/adv-report-1.md, as instructed. The test run left case directories under the shared target's `tmp/adv_box_mounts/`.

## 7. Findings block

```findings
- file: crates/conductor/src/placement.rs
  line: 771
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a read-only root that contains a writable root is emitted beside it, so the box of the configure-an-instance guide's own config (records under the checkouts root) is refused by substrate as exec.writable-root-mount-overlap
- file: crates/conductor/src/placement.rs
  line: 747
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: in the getting-started layout the single writable root store=~/.b10x/conductor makes the config file and conductord's own directory (runtime tokens, binaries.json, upgraded binaries) writable in the box, against the docs' read-only promise
- file: crates/conductor/src/placement.rs
  line: 779
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the instance's cache directory, which the loop writes exports and the decider's directory into, is in no mount although the list claims everything the loop writes
- file: crates/conductor/src/placement.rs
  line: 835
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a records or state path written with a trailing slash reaches the request unchanged and substrate refuses it as exec.writable-root-mount-invalid
- file: crates/conductor/src/placement.rs
  line: 856
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a config file that is a symlink inside a writable root is dropped and its target is mounted nowhere, so it dangles in the box; no documented workflow links the config file
- file: crates/conductor/src/placement.rs
  line: 570
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: instance names a.b and a_b map to the same writable root names, so two valid instances share one declared root
```
