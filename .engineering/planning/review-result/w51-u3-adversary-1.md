---
format: aep.planning-md/3
id: review-result:w51-u3-adversary-1
kind: review-result
status: active
title: Wave 51 U3 doctor-checks-guard-forge-writes, adversary pass 1
relations:
- reviews: story:doctor-checks-guard-forge-writes
revision: 1
---
```
unit: U3 story:doctor-checks-guard-forge-writes, branch impl/doctor-checks-guard-forge-writes at c4082e69c (clean tree plus one untracked test file)
verdict: NEEDS-CHANGE
cases: executed 1845→1850, red 4
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (scratch directory, listed in part 6)
needs-coordinator: none
```

Four of my five cases are red. The worst: doctor says `ready` while the hook denies a lead every forge write. Two fix lines name changes that can't be made or don't help, and setup ends `ready` where doctor doesn't (the decided point).

**1. What I touched** (`git --no-pager diff --stat` is empty: the only change is an untracked test file)
```
?? crates/conductor/tests/adv_w51_doctor_guard.rs
```
It is a test file. No implementation file was touched.

**2. Cases added** (all in `crates/conductor/tests/adv_w51_doctor_guard.rs`; red output is from `red-alone.log` and `red-alone-2.log`)
- `adv_doctor_is_ready_while_the_hook_denies_the_lead_of_a_second_checkout` (:201), **red**.
  - Setup: a `local` source on `gates` with a policy file that exists. Checkouts `alpha` (GitHub origin) and `beta` (GitLab origin).
  - The hook denies `glab mr create -R example-group/beta --fill` in `beta` with exit 2; that assertion passes.
  - Then doctor fails: `assertion left != right failed: doctor ends ready while the hook denies the lead of beta every forge write: exit Some(0)`, with `forge route alpha: gates, policy …`, then `guard controller: allowed`, then `ready`.
- `adv_the_policy_fix_names_a_path_the_config_file_accepts` (:243), **red**.
  - With `B10X_GATES_POLICY=gates-policy.json` (relative), I wrote the value from the fix line into the config. `config validate` then gives: `instances[0].gates_policy: "gates-policy.json" is not a path: write it absolute (/…) or under the home directory (~/…)`, exit 1.
  - Its first run failed on my own string split, which was a typo and not a finding. I fixed the split and reran the case alone; the output above is from that rerun.
- `adv_doctor_is_not_ready_on_a_gates_policy_that_names_no_file` (:280), **red**.
  - `gates_policy: <missing file>` and `gates_policy: <directory>` each give `forge route alpha: gates, policy <path>` and `ready`, exit 0.
- `adv_doctor_with_a_denied_role_changes_no_byte_of_state_or_records` (:312), **green**.
  - Every file of the case (store file, lease copy, records log) has the same bytes after doctor, with a denied role.
- `adv_setup_is_not_ready_where_doctor_is_not_ready_on_the_route` (:532), **red**: `setup ends "ready" where doctor ends "not ready: box zeta (placement.writable-root-missing), forge route zeta (no gates_policy)": exit Some(0)`.

**3. Suite run** (after the cases existed): `conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100.
```
Summary [ 312.746s] 1850 tests run: 1841 passed (4 slow), 9 failed, 12 skipped
```
- The 4 red cases are mine.
- 4 are known `guard_hook` load failures: `hooks_running_at_once_…`, `the_hook_denies_with_exit_2_…`, `with_a_config_file_a_session_in_records_…` and `an_unreadable_payload_…`. Where the log shows the reason, it is "the store did not take it within 2 s".
- 1 is the known `build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger`.

**4. Findings** (the tree is c4082e69c; the guard lines and route lines don't exist at base 61787f015, so every finding is `introduced`)
1. **Lead sampled at one checkout only.** `crates/conductor/src/doctor.rs:1822`: `lead_sample` asks only about the first checkout whose origin names a repository.
   - Measured: case :201.
   - What reaches it: the `local:` source over a directory of checkouts that the README and getting-started configs use. The route check only flags `gitlab` sources, so a GitLab-origin checkout under a `local` source on `gates` gets no check at all.
   - Fix: ask about every checkout, or at least one per origin host. NEEDS-CHANGE.
2. **Fix line names a relative path.** `doctor.rs:1649-1656`: the fix prints `B10X_GATES_POLICY` as it is written.
   - Measured: case :243.
   - What reaches it: any relative value in the variable.
   - Fix: join it with the working directory, as setup's `policy()` already does. NEEDS-CHANGE.
3. **A `gates_policy` that names no file counts as a policy.** `doctor.rs:1649`: only `is_none()` counts as no policy, so a missing file or a directory reads ready.
   - Setup's `policy()` already refuses such a path (`is_file`).
   - The unit's own tests assert `ready` on a `gates-policy.json` they never create (`tests/doctor.rs:141`, :1386).
   - Fix: treat a path that is not a readable file as no policy. NEEDS-CHANGE.
4. **Setup's verdict leaves out the route** (`crates/conductor/src/setup.rs:835`), measured by case :532. This is the decided point. NEEDS-CHANGE.
5. **Note: the guard line on `gates` for a GitHub origin can never say denied.** `doctor.rs:1742`: the sample there is `b10x-gates api …`. No guard rule reads `b10x-gates api` (`bot_route` matches only `b10x-gates gh`), so that line always reads `allowed`. On `gates` the guard check comes down to whether the origin's host is GitHub. CONFIRMED.
6. **Note: the docs case skips config blocks it can't parse.** `crates/conductor/tests/doctor.rs:1795` skips a block that does not parse without saying so; it only requires `read >= 6`. A page whose block gains a typo, or a placeholder, drops out of the route check and the case still passes. Today the only block it skips is the deliberately broken one in configure-an-instance.md. CONFIRMED.

**5. Attacked, not broken**
- Doctor and the hook agree on the sample itself: same `Active` construction, same places, and `CONDUCTOR_INSTANCE` reaches sessions.
- Origin parsing: ssh form, `ssh://`, port, `.git` suffix, case of the host.
- Doctor records nothing: byte comparison with a store file and a lease copy present.
- Step-number citations across the repository, and the config blocks of every page.
- Upgrade `said`: matching rows to the verdict, multi-instance names, the fallback when there is no verdict.
- Mutants that tests catch: route check always `direct`, `asked` always allow, verdict ignoring guard lines, `environment_policy` reading another variable.

**6. Paths written outside the worktree**
- `<scratch>/w51/u3/adv1/blocks.py`
- `<scratch>/w51/u3/adv1/blocks.txt`
- `<scratch>/w51/u3/adv1/red-alone.log`
- `<scratch>/w51/u3/adv1/red-alone-2.log`
- `<scratch>/w51/u3/adv1/suite.log`

Test fixtures are under the shared target's `tmp/adv_w51_doctor_guard*`; each case's `Drop` removes them when the case passes.

```findings
- file: crates/conductor/src/doctor.rs
  line: 1822
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'lead_sample asks only about the first checkout with an origin, so doctor ends ready while the hook denies the lead of a later checkout (a GitLab origin under a local source on gates) every merge request create'
- file: crates/conductor/src/doctor.rs
  line: 1653
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'with a relative B10X_GATES_POLICY the fix line names a relative gates_policy that config validate refuses as no path'
- file: crates/conductor/src/doctor.rs
  line: 1649
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'a gates_policy naming a missing file or a directory counts as a policy and doctor ends ready, while setup takes only a readable file'
- file: crates/conductor/src/setup.rs
  line: 835
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'setup --apply ends ready while doctor on the file it wrote ends not ready naming forge route zeta (no gates_policy)'
- file: crates/conductor/src/doctor.rs
  line: 1742
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'on gates the GitHub sample b10x-gates api matches no guard rule, so that role''s guard line can only read allowed'
- file: crates/conductor/tests/doctor.rs
  line: 1795
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the page config-block case skips blocks that do not parse without saying so, so a block that stops parsing leaves the route check and the case stays green'
```
