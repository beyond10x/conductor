---
format: aep.planning-md/3
id: review-result:w49-guard-admits-own-pipelines-adversary-2
kind: review-result
status: active
title: Wave 49 U5 adversary, pass 2
relations:
- reviews: story:guard-admits-own-pipelines
revision: 1
---
unit: story:guard-admits-own-pipelines, w49-u5 committed tree at 9f49c60c8 plus uncommitted test cases
verdict: CONFIRMED (5 ways a pipeline write with no -R still reaches another repository, and 1 over-denial of ordinary own-repository lines)
cases: executed 1618→1625, red 6
origin: introduced 1 / pre-existing 0 / undecided 5
wrote-outside-worktree: 12 paths, all under the unit's scratch adv2/
needs-coordinator: a base-commit run to settle the 5 undecided origins; adv_store::adv_racing_handles_lose_no_proposal failed once in the run without the cases and passed in the run with them

Cases added after line 6990 of crates/conductor/tests/guard_rules.rs: helper adversary2_misses
and seven adversary2_* cases; run alone 7 run, 1 passed, 6 failed.
- adversary2_the_steps_the_cases_rely_on_are_admitted_alone (green)
- adversary2_a_stack_change_that_does_not_move_the_shell_is_followed (red): popd +1, popd -n, pushd -n
- adversary2_a_cd_to_the_previous_directory_or_a_stack_entry_is_followed (red): cd ~-, cd ~1
- adversary2_a_cd_that_fails_leaves_glab_where_it_was (red): cd to a missing directory from a nested clone
- adversary2_a_two_operand_cd_is_followed (red): zsh cd old new
- adversary2_a_remote_the_config_parser_skips_is_read (red): a key on the section header's line; remote.origin.glab-resolved
- adversary2_an_own_bare_retry_piped_or_repeated_is_admitted (red): `| tail -5`, `2>&1 | head -20`, `glab ci retry 101 && glab ci retry 102` denied

Checked with bash 5, zsh and glab 1.115 in scratch: every stack and tilde line ends in the
other clone; glab calls projects/example-group%2Fother for both config states.

Suite with the cases: 1625 run, 1618 passed, 7 failed (the 6 plus build_run). Without: 1618 run,
1616 passed, 2 failed (build_run and adv_store::adv_racing_handles_lose_no_proposal, which passed
in the other run).

Base line 1835 admitted every pipeline write without -R, so findings 1-5 would be red on the
base too; they contradict the acceptance "the same commands naming another repository stay
denied" and website/docs/security/the-guard.md.

Could not break: bash -c and eval bodies, command substitution, { ; }, if/fi, declare -x,
typeset, readonly, local, nohup, time, glab() as a function, [Include] and [include ""], .git
files and linked trees.

```findings
- file: crates/conductor/src/guard/rules.rs
  line: 1799
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "glab_place pops on every popd and moves on pushd -n, so `pushd ~/clones/other && popd +1 && glab ci retry 123` is admitted and retries another project's pipeline"
- file: crates/conductor/src/guard/rules.rs
  line: 1828
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "cd_path anchors every ~ word and word_path joins ~- and ~1 as a literal name, so `cd ~/clones/other; cd <checkout>; cd ~-; glab ci retry 123` is admitted"
- file: crates/conductor/src/guard/rules.rs
  line: 1852
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "a cd to a missing directory is read as its nearest checkout while the failed cd leaves glab in a nested clone of another project, and the retry is admitted"
- file: crates/conductor/src/guard/rules.rs
  line: 1785
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "only the first cd operand is read, so zsh's `cd <checkout> <other clone> && glab ci retry 123` lands in the other clone and is admitted"
- file: crates/conductor/src/guard/rules.rs
  line: 3541
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "config_remotes skips a key on its section header's line and implicit_denial ignores remote.*.glab-resolved, and glab 1.115 retries example-group/other in both states the guard admits"
- file: crates/conductor/src/guard/rules.rs
  line: 1813
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "an own bare retry piped to tail or head, or repeated as `glab ci retry 101 && glab ci retry 102`, is denied though the doc row admits another glab call on the line"
```
