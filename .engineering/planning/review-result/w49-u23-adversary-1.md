---
format: aep.planning-md/3
id: review-result:w49-u23-adversary-1
kind: review-result
status: active
title: Wave 49 U23 documented-commands-parse, adversary pass 1
relations:
- reviews: story:documented-commands-parse
revision: 3
---
unit: U23 story:documented-commands-parse, commit 1df336584 on base 3290f2211, worktree ~/.local/state/worktree/trees/b10x/conductor/w49-u23
verdict: NEEDS-CHANGE
cases: executed 9→14, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 paths
needs-coordinator: whether finding 1 holds the unit or the acceptance statement is narrowed to commands that take --input-json

## 1. Diff stat

```
 crates/conductor/tests/docs_start_conductor.rs | 181 +++++++++++++++++++++++++
 1 file changed, 181 insertions(+)
```

Test file only. The cases are appended as `mod adversary`; no existing line is changed.

## 2. Cases added

All in `crates/conductor/tests/docs_start_conductor.rs`, all red now. Red output is from the run of
`-E 'test(adversary::)'` alone, which ran before the suite (`adv/cases.log`, exit 100).

**adv_a_a_missing_flag_of_a_command_without_input_json_is_found** (:479). A console block with
`conductor session start`, `conductor daemon start-role`, `conductor spawn`; asserts a finding on each line.

```
assertion `left == right` failed: each line lacks the flag its handler requires; handled 0, findings []; the binary answers [
    "conductor session start -> Some(1): conductor: session start: --role is required",
    "conductor daemon start-role -> Some(1): conductor: daemon start-role: --role is required: supervisor or teacher",
    "conductor spawn -> Some(1): conductor: spawn: --repository is required",
]
  left: []
 right: ["page.md:2", "page.md:3", "page.md:4"]
```

**adv_b_a_state_dir_on_the_line_does_not_hide_the_missing_reason** (:513). Three `stop-controller`
lines without `--reason`: plain, `--state-dir <dir>` after the subcommand, `--state-dir <dir>` before it.

```
assertion `left == right` failed: each line stops a controller without --reason; handled 2, findings [
    "page.md:2: ... the handler asks for `--reason`: conductor: controller stop-controller: --reason is required",
    "page.md:3: ... --state-dir <dir>`: the handler asks for `--reason`: conductor: controller stop-controller: --reason is required",
]
  left: ["page.md:2", "page.md:3"]
 right: ["page.md:2", "page.md:3", "page.md:4"]
```

**adv_c_the_floor_of_each_page_is_what_the_page_holds** (:535). Holds both real guides, compares with
the floors of `PAGES`, and holds each guide again with its `stop-controller` lines removed (the
mutant of an extractor that does not read them).

```
the floors do not hold the pages:
website/docs/guides/start-conductor.md: holds 29 commands, 3 handled; the floor is 20 and 3
website/docs/guides/first-task-existing-repo.md: holds 26 commands, 5 handled; the floor is 8 and 3
website/docs/guides/first-task-existing-repo.md: with no stop-controller line read (25 commands, 4 handled) the floor 8 and 3 still holds
```

**adv_d_a_conductor_command_behind_an_assignment_or_a_program_is_read** (:578).
`id=$(conductor controller controllers --nope)`, `watch -n 5 conductor session list --nope`, and a
fenced block inside a blockquote with `--nope`; asserts a finding on each.

```
assertion `left == right` failed: each line gives a flag its command does not take; commands 0, findings []
  left: []
 right: ["page.md:2", "page.md:3", "page.md:7"]
```

**adv_e_a_command_continued_with_a_backslash_is_one_command** (:600).

```
assertion `left == right` failed: the commands read from a block with continued lines
  left: [(2, "controller|stop-controller|--controller-id|<id>|\\"), (4, "controller|pause-controller|--controller-id|<id>|\\")]
 right: [(2, "controller|stop-controller|--controller-id|<id>|--reason|<why>"), (4, "controller|pause-controller|--controller-id|<id>|--nope")]
```

## 3. Suite run

One build request, after the cases existed:
`conductor build run --reserve 8G -- sh <scratch>/adv/run.sh`, whose second command is
`cargo nextest run -p conductor-cli --test docs_start_conductor --test docs_quickstart --no-fail-fast`.

```
    Starting 14 tests across 2 binaries
        PASS docs_quickstart (7 cases)
        PASS conductor-cli::docs_start_conductor the_checks_of_this_test_find_what_they_look_for
        PASS conductor-cli::docs_start_conductor every_conductor_line_of_the_guides_runs_as_written
        FAIL conductor-cli::docs_start_conductor adversary::adv_a_a_missing_flag_of_a_command_without_input_json_is_found
        FAIL conductor-cli::docs_start_conductor adversary::adv_b_a_state_dir_on_the_line_does_not_hide_the_missing_reason
        FAIL conductor-cli::docs_start_conductor adversary::adv_c_the_floor_of_each_page_is_what_the_page_holds
        FAIL conductor-cli::docs_start_conductor adversary::adv_d_a_conductor_command_behind_an_assignment_or_a_program_is_read
        FAIL conductor-cli::docs_start_conductor adversary::adv_e_a_command_continued_with_a_backslash_is_one_command
     Summary [   2.558s] 14 tests run: 9 passed, 5 failed, 0 skipped
error: test run failed
exit 100
```

`executed 9` is the nine cases of this run that are not mine. Full output: `adv/suite.log`.

## 4. Findings (commit 1df336584 plus the uncommitted cases)

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| 1 | `crates/conductor/tests/docs_start_conductor.rs:256` | adv_a: `handler_decides` asks the handler only of commands that take `--input-json`. `session start`, `daemon start-role`, `spawn` without their flag exit 1 with `--role is required` / `--repository is required`, and the test reports nothing (handled 0) | `start-conductor.md` itself documents these: `spawn --repository` (:64), `session start --role` (:103, :108), `daemon start-role --role` (:109), also `message send` (:69) and `instance upgrade` (:117). 26 of its 29 commands are parsed only; dropping the flag from any of them is the reported defect again and stays green. The acceptance says the test "fails on a missing required flag" | NEEDS-CHANGE | introduced |
| 2 | `crates/conductor/tests/docs_start_conductor.rs:39` | adv_c: floors 20/3 and 8/3 against 29/3 and 26/5 held. 9 and 18 commands can go unread green; `first-task-existing-repo.md` with its `stop-controller` line unread still passes (25/4). Also read, not run: no case fails when a page is removed from `PAGES` | any later change of `command_texts` or `conductor_commands`; the line this unit fixed in the second guide is not pinned by anything | CONFIRMED | introduced |
| 3 | `crates/conductor/tests/docs_start_conductor.rs:234` | adv_b: `leaf` stops at the first word that is no subcommand, so `conductor --state-dir <dir> controller stop-controller --controller-id <id>` resolves to the root and its handler is never asked. `--state-dir` after the subcommand is found | nothing found in the two pages of `PAGES`. `website/docs/security/sessions.md:16` writes `conductor --config <file> --state-dir <dir> session start`, a page the test does not read | INFEASIBLE | introduced |
| 4 | `crates/conductor/tests/docs_start_conductor.rs:162` | adv_d: a `conductor` command after `name=$(`, after another program (`watch`, and so `sudo`, `exec`), or in a fenced block inside a blockquote is not read at all (commands 0), so an unknown flag there is silent | nothing found in the two pages of `PAGES`. `website/docs/getting-started.md:180` writes `state=$(conductor config show ...)`, a page the test does not read | INFEASIBLE | introduced |
| 5 | `crates/conductor/tests/docs_start_conductor.rs:119` | adv_e: a fenced command continued with `\` is read as its first line plus the word `\`; the continuation's flags are not read. It fails loud (clap refuses `\`), so a correct continued command is a false finding, not a hidden one | nothing found: no page under `website/docs` continues a `conductor` line with a backslash | INFEASIBLE | introduced |

Fix named, not applied. 1: ask the handler of every leaf whose line gives no flag the handler names,
or keep a declared table of handler-required flags for leaves without `--input-json`; a full line of
`session start`, `spawn` or `daemon start-role` must not be run (it acts), a line stripped of its
flags can be (each bails first: `session.rs:1125`, `spawn.rs:136`, `daemon/mod.rs:254`). Bare prose
mentions (`conductor spawn` at first-task :76 and :169, `session deliver` at start :71) need the
same pass the group-alone case has. 2: pin the floors to the counts, or assert the named lines are
among those read. 3: skip leading global flags and their values in `leaf`.

## 5. Attacked and not broken

- Extractor returning nothing: the guides case fails on the floor (0 < 20), the planted case on its literal list.
- Renamed subcommand, unknown flag, wrong value type (`--controller-id not-an-id`): found by clap, covered by the planted case.
- `<placeholder>` hiding a parse error: found no form in the two guides where the sample masks one; `--state-dir <dir>` after the subcommand does not mask the handler (adv_b line 3).
- Prompted console lines, `&&`, env prefix, inline spans over two lines, table cells, indented fences: read (port of the extractor over both guides lists 29 and 26 commands, matching adv_c).
- Missing handler flag of a command that takes `--input-json`: found; `goal.rs:52`, `controller.rs:258` check flags before the store opens.
- Writes outside the isolated home: not measured. Read only: every run gets `HOME` and `--state-dir` under the case's `target/tmp` directory.

## 6. Paths written outside the worktree

- ~/.cache/scratch/<session>/w49/u23/adv/append.rs
- ~/.cache/scratch/<session>/w49/u23/adv/port.py
- ~/.cache/scratch/<session>/w49/u23/adv/run.sh
- ~/.cache/scratch/<session>/w49/u23/adv/request.log
- ~/.cache/scratch/<session>/w49/u23/adv/cases.log
- ~/.cache/scratch/<session>/w49/u23/adv/suite.log
- ~/.cache/scratch/<session>/w49/u23/adv/done
- ~/.cache/scratch/<session>/w49/u23/adv-report-1.md

Inside the worktree, ignored: `target/tmp/docs_start_conductor/adv_a` to `adv_d`, kept because the cases failed.

```findings
- file: crates/conductor/tests/docs_start_conductor.rs
  line: 256
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a missing handler-required flag is found only for commands that take --input-json, so 26 of the guide's 29 commands, session start, daemon start-role and spawn among them, pass without the flag their handler refuses
- file: crates/conductor/tests/docs_start_conductor.rs
  line: 39
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the floors 20/3 and 8/3 sit below the 29/3 and 26/5 held, so an extractor that stops reading up to 9 and 18 commands, the fixed stop-controller line of the second guide included, stays green
- file: crates/conductor/tests/docs_start_conductor.rs
  line: 234
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a global flag before the subcommand resolves the line to the root command and its handler is never asked; neither page of PAGES writes that form
- file: crates/conductor/tests/docs_start_conductor.rs
  line: 162
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a conductor command after name=$(, after another program or in a blockquoted fence is not read; neither page of PAGES writes one
- file: crates/conductor/tests/docs_start_conductor.rs
  line: 119
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a fenced command continued with a backslash is read as its first line plus the word backslash, a loud false finding; no page continues a conductor line that way
```
