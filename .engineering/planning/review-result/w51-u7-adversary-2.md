---
format: aep.planning-md/3
id: review-result:w51-u7-adversary-2
kind: review-result
status: active
title: Wave 51 U7 container-start-without-prompt, adversary pass 2
relations:
- reviews: story:container-start-without-prompt
revision: 1
---
```
unit:                   U7 (wave 51), story:container-start-without-prompt, adversary pass 2 (last), head 960040541 plus my uncommitted test file
verdict:                NEEDS-CHANGE
cases:                  executed 1976→1979, red 2 (both mine; package run also has 3 failures on the brief's known list)
origin:                 introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: yes, scratch only (part 8)
needs-coordinator:      yes: commit the test file; decide findings 1 and 2 (records commit), 3 (workflow case) and 4 (infeasible note)
```

The correction holds on the five points you named. Each mutant turns its named case red. It breaks in one place: the records commit that the correction added (`init::commit_lying`). Two red cases show it.

**1. What I touched**
- Only `crates/conductor/tests/adv_w51_container_start.rs` changed: 232 lines added, uncommitted. That is three `adv2_` cases, two helpers and a paragraph in the module doc. No `adv_` case changed.
- It passes `rustfmt --check` and the gates scan (`1 file(s) carry no private rule`). It contains no `/home/` path.
- I applied five mutants to `token.rs`, `daemon/api.rs`, `daemon/mod.rs` and `init.rs` for one run, then restored them from copies. `sha256sum -c` says all 4 are OK, and `git diff --stat` shows only the test file.

**2. Cases added** (outputs from `suite2.log`)
- `adv2_a_records_repository_with_a_commit_is_left_alone_while_its_head_has_none`: **red**
  - The records repository has its commit. The case runs `git checkout --orphan draft`, edits `rules.md`, then starts the daemon again.
  - Output: `a start committed into a records repository that has a commit: 2 commits, branch draft at "06af8ef6…"` and `committed: README.md, NORTHSTAR.md, rules.md, .engineering/workspace.yaml, docs/handoff/2026-11-11.md`
- `adv2_a_file_git_ignores_in_the_hand_over_directory_does_not_keep_the_records_uncommitted`: **red**
  - The first start has no git identity. Then a backup file `<handoff>.md~` is placed beside the hand-over, with `*~` in `.git/info/exclude`. The second start has an identity.
  - Output: `a start with a git identity left the skeleton uncommitted: exit Some(128) fatal: ambiguous argument 'HEAD'`
  - The daemon printed: `` … is not committed; the next start commits it: `git add` failed: The following paths are ignored by one of your .gitignore files: (hint: Disable this message …) ``
- `adv2_the_check_workflow_runs_the_container_job_on_every_pull_request`: **green**
  - It holds the real `check.yml` to these rules: `on.pull_request` with no filter, a push to `main`, no `needs:`, the check job's `runs-on`, and `timeout-minutes` above 0.
  - It also refuses six workflow mutants it builds in memory, so no file is edited.

**3. Each run's Summary line**
- Mutant run (all five mutants at once, named cases only), `mutants.log`: `Summary [ 8.567s] 13 tests run: 3 passed, 10 failed, 57 skipped`
- First package run, `suite.log`: `Summary [ 293.573s] 1979 tests run: 1959 passed (5 slow), 20 failed, 13 skipped`. **This run is void.** `cp -p` put back the old mtimes, so cargo did not rebuild and the run tested the mutated binary. I touched the four files (bytes unchanged, checked) and ran again.
- Package run, `suite2.log`: `Summary [ 438.307s] 1979 tests run: 1974 passed (7 slow), 5 failed, 13 skipped`
  - Failures: my 2 red cases, plus 3 on the known list: `adv_store::adv_racing_handles_lose_no_proposal` (database is locked), `build_run::a_second_signal…`, and `public_text::planning_and_docs_carry_no_instance_id`.
  - `isolated_home` passes.

**4. Mutants and the case that catches each** (from `mutants.log`)

| Mutant | Red case and output |
|---|---|
| Reveal refusal removed | `adv_reveal_model_token_refuses…`: `the model token of a boxed instance was printed: exit Some(0)` |
| `own_host` accepts any host | `dashboard::every_answer_carries_the_content_security_policy` (`GET / for attacker.example left: 200 right: 403`), `dashboard::only_get_of_the_app_and_its_data_is_answered`, and `container_start::a_loopback_address_answers_the_host_it_is_bound_to_and_no_other` (`Host example.org:…: Some((200, …))`) |
| Seeding moved back before the listeners | `adv_healthz_answers_while_the_unattended_start_seeds` (`no answer within 3s`), and `container_start::while_the_unattended_start_seeds…` (`/healthz while the start seeds: None`) |
| `unattended` dropped from the fall-back | `token_pool::without_unattended_an_empty_pool_leaves_the_launch_on_the_harness_login`: settings carry `apiKeyHelper` |
| `commit_lying` runs where a commit exists | `container_start::an_unattended_start_seeds_each_instance_and_a_second_start_overwrites_nothing`: `a second commit: … committed: README.md, …` |

**5. Findings**
1. **The records commit is made wherever HEAD has no commit, not only where the repository has none.** `crates/conductor/src/init.rs:283-284`. warning, NEEDS-CHANGE, introduced (the correction).
   - `commit_lying` asks only whether `rev-parse --verify --quiet HEAD` fails.
   - A records repository with commits whose HEAD is on a branch with none yet (after `git checkout --orphan`) gets a commit with conductor's init message. The operator's uncommitted edits go into it.
   - That breaks the coordinator's rule "a repository that has a commit is left alone, whatever its work tree holds". It also breaks the `config.yaml` comment and the guide page ("a skeleton whose repository has no commit yet").
   - Proof: case 1 red.
   - Fix: test for any commit at all (for example, `git rev-list -n 1 --all` prints nothing).
2. **One file git ignores in `docs/handoff/` keeps the skeleton uncommitted at every start.** `crates/conductor/src/init.rs:233-240` (`lying`) with `:300-301`. note, CONFIRMED, introduced (the correction).
   - `lying` names every regular file of `docs/handoff/` in `git add`. `git add` refuses an ignored path with exit 1, so the whole commit fails.
   - Each start prints "the next start commits it", which never happens. `/readyz` stays 503 for the whole process.
   - The reason printed is git's first and last line, which leaves out the path.
   - Who reaches it: an editor backup or swap file in the hand-over directory, plus a global or repository exclude for it.
   - Proof: case 2 red.
3. **The workflow case does not hold "the job runs".** `crates/conductor/tests/container_start.rs:1373-1420`. note, CONFIRMED, introduced.
   - It reads the job's steps, `if:`, `continue-on-error:`, `shell:` and `defaults:`. It does not read the workflow's `on:` or the job's `needs:`, `runs-on:` or `timeout-minutes:`.
   - My decision: this leaves the acceptance unheld.
     - A `needs:` on a job with `if: false` skips the container job, and the workflow run is still green.
     - Dropping `pull_request` means pull requests never run the job.
     - A runner label that no runner has leaves the job queued and never run.
   - My green case 3 now turns red on each of those six mutants. Fix: adopt case 3, or fold its checks into the unit's case.
4. **Fail-open when `local_addr()` fails.** `crates/conductor/src/daemon/api.rs:528-533`. note, INFEASIBLE, introduced (round 1; the correction rewrote the line and kept the fallback).
   - When `local_addr()` fails, the listener is taken as `Loopback(127.0.0.1:0)`. For a listener bound to `0.0.0.0` that means `/v1/` is served without a bearer, to `Host` 127.0.0.1 or localhost with no port or `:0`.
   - Infeasible: getsockname on a bound socket does not fail in practice.
   - Fix if wanted: give `serve` the address `daemon_serve` bound.

**6. Attacked and could not break**
- **Reveal:**
  - The refusal comes before any read of the secrets source, so neither the value nor its length is in the line or on the exit path.
  - Every placement string that names an adapter is refused. That covers `systemd`, `linux` and `<adapter>@https://…` (a remote daemon), written as a string or a map.
  - A `/2` file has no `globals.placement`, and a `/1` file maps a confinement provider to `systemd` or `none`.
  - The instance name must match exactly, so a prefix or other letter case gets `UnknownInstance`.
  - Other placement strings are taken as no box everywhere (`boxed`), consistently.
  - The settings file and the helper command name only the instance. No other printer was added.
- **`own_host` and `same_origin`:** they match exactly, ignoring case, against six strings each. That refuses:
  - other spellings: `127.1`, hex, `[::ffff:…]`, a zone id
  - malformed hosts and ports: a trailing dot, leading zeros in the port, an empty port, `h:p:p`
  - Origins: `https://` and `null`.
  - `0.0.0.0`, `::` and `::ffff:127.0.0.1` are not loopback, so they are `Bearers`. `own_host` never sees an unspecified address.
  - Writes still need the page token beside the origin (`api.rs:1099`).
- **Seeding order:**
  - `/readyz` checks the seeding flags first and needs a publication for 200. So it cannot answer 200 in the window before the flag, nor while seeding.
  - Bodies are fixed strings, so `/metrics` and `/healthz` carry no step text.
  - During seeding, `/v1/` reads wait 10 s and then get 503 with the seeding reason. An upgrade gets 503 (no `QUEUE` until `upgrade::listen`), and instance routes get 503 (unpublished). Role starts are queued and run only after `Daemon::open`, which comes after the seeding.
  - No runtime starts before the seeding ends. After a failed seeding runtimes do start (the coordinator decision). Spawn still refuses an untrusted checkout (`spawn.rs:161`).
- **`commit_lying`, the other cases:**
  - Files the operator already staged are not committed (`commit -- paths`; checked with git in scratch).
  - A missing `.git` is left alone, and a `.git` file of a born worktree or submodule is skipped.
  - A symlinked skeleton file is added as a link, not its content. A symlinked `docs/` is refused by git.
  - No work-tree byte is written.
  - Hooks and signing run as in base `init` (parity).
  - The commit contacts no remote, so no remote URL or credential can reach the log. The only things git adds to the line are the user@hostname of "unable to auto-detect email" and the records path, which the line names anyway.
- **Fall-back gate:** `daemon_config()` reads the file `set_active` located. The runtime gets `CONDUCTOR_CONFIG=<file>` from the daemon (`reconcile.rs:784`), so the daemon and the runtime read the same key. With no active file it is `None`, which means no fall-back (the safe side).
- **Pages and spec comments:**
  - The `/readyz` reason texts quoted on the guide page equal `metrics::SEEDING` and `SEEDING_FAILED`.
  - The 10 s wait equals `STORE_BOUND`.
  - The loopback and `systemd-run` sentences are there.
  - No generated file embeds the changed comments; `conductor-model` holds only `unattended — Boolean`. The CLI reference comes from `cli.rs`, which did not change.

**7. Not done**
I ran no container, applied no workflow mutant to the file, and wrote no case for the local_addr fall-back.

**8. Paths written outside the worktree**
- `<scratch>/w51/u7/adv2/`, holding `gitprobe.sh`, `probe/` (git scratch repositories), `orig/` (the four originals and `sums`), `mutants.log`, `suite.log` (void) and `suite2.log`.
- The red cases keep their directories under the build target: `target/conductor-362621f27d1a30dd/tmp/csadv/records-orphan` and `…/csadv/records-ignored`.

```findings
- file: crates/conductor/src/init.rs
  line: 283
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'commit_lying commits wherever HEAD does not resolve, so a records repository that has commits but whose HEAD is on an orphan branch gets the skeleton and the operator''s uncommitted edits committed under conductor''s init message, against the rule that a repository with a commit is left alone'
- file: crates/conductor/src/init.rs
  line: 233
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'lying names every file of docs/handoff in git add, and git add refuses an ignored path, so one ignored file there (an editor backup) fails the records commit at every start while each start says the next one commits it'
- file: crates/conductor/tests/container_start.rs
  line: 1373
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the workflow job case reads neither on nor needs, runs-on or timeout-minutes, so a job that needs a skipped job, runs on no pull request or on a runner no one has never runs and the case stays green; adv2_the_check_workflow_runs_the_container_job_on_every_pull_request holds them'
- file: crates/conductor/src/daemon/api.rs
  line: 532
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'when local_addr fails the listener is taken as loopback on 127.0.0.1:0, so a listener bound to a non-loopback address would serve /v1/ without a bearer; getsockname on a bound socket does not fail in practice'
```
