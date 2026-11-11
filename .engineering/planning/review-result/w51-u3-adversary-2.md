---
format: aep.planning-md/3
id: review-result:w51-u3-adversary-2
kind: review-result
status: active
title: Wave 51 U3 doctor-checks-guard-forge-writes, adversary pass 2
relations:
- reviews: story:doctor-checks-guard-forge-writes
revision: 1
---
```
unit: U3 story:doctor-checks-guard-forge-writes, branch impl/doctor-checks-guard-forge-writes at 120d23210 plus one uncommitted test file
verdict: NEEDS-CHANGE
cases: executed 1857→1862, red 4
origin: introduced 6 / pre-existing 0 / undecided 0
write-outside-worktree: 5 paths (scratch, part 6) plus failed-case fixtures under the shared target's tmp/
needs-coordinator: finding 1 (which side changes: setup's route_for or doctor's checkout scope)
```

Four of my five cases are red. The worst: `setup --apply` writes the bot route for a GitHub repository, and its own verdict then refuses that route as soon as any non-GitHub clone sits in the same parent directory. That breaks the operator's "fresh install ends ready with no hand edit" rule.

**1. What I touched**
```
 crates/conductor/tests/adv_w51_doctor_guard.rs | 171 +++++++++++++++++++++++++
 1 file changed, 171 insertions(+)
```
It is a test file only. No implementation file was touched.

**2. Cases added** (`crates/conductor/tests/adv_w51_doctor_guard.rs`; the red output is from the run of the `adv2_` cases alone, `adv2/red-alone.log`)
- `adv2_the_bot_route_setup_writes_is_not_refused_for_a_clone_beside_it` (:616), **red**.
  - Setup: GitHub repository `zeta` and a GitLab clone `notes` in the same `src/`, with `B10X_GATES_POLICY` naming a readable file. Setup writes `gates_policy` (that precondition passes).
  - Doctor then ends `not ready: box zeta (placement.writable-root-missing), forge route zeta (no bot route for gitlab.example.com), guard controller (denied)`.
- `adv2_a_github_checkout_over_ssh_on_port_443_is_a_github_checkout` (:652), **red**: `left: "not ready: forge route alpha (no bot route for ssh.github.com), guard controller (denied)"`, `right: "ready"`.
- `adv2_setup_reads_a_github_origin_in_any_case` (:678), **red**. With origin `https://GitHub.com/example-org/zeta.git` and a policy, the plan prints `forge route zeta: direct, which --apply writes: the origin is on GitHub.com, and the bot route writes to github.com only`.
- `adv2_setup_names_a_refused_box_as_doctor_does` (:707), **red**: `left: "not ready: box zeta (placement.writable-root-shared), box (refused)"`, `right: "not ready: box zeta (placement.writable-root-shared)"`.
- `adv2_setup_before_the_substrate_daemon_ends_with_doctor_s_verdict` (:733), **green**. It guards the mutant "setup leaves doctor's box lines out of its verdict". No existing setup case catches that mutant: each one that expects an exact verdict serves the boxes first.
- Withdrawn: `adv2_a_gitlab_clone_beside_a_github_source_changes_no_verdict`. It ran red, then I removed it before the suite run. Its expectation contradicts the documented workspace model (`website/docs/concepts/config.md:124`: a `github` source's members are every checkout with an origin under the root), and `spawn` accepts any checkout under the root.

**3. Suite run** (after the cases): `conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100
```
Summary [ 249.041s] 1862 tests run: 1857 passed (4 slow), 5 failed, 12 skipped
```
The 5 failures are my 4 red cases and the known `build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger`. The 1857 before-count is the implementor's gate on this tree.

**4. Findings** (tree 120d23210)
1. **Setup writes a route its own verdict refuses.**
   - Where: `crates/conductor/src/setup.rs:265`. `route_for` looks only at the repository's origin. But the root it writes is the parent directory, and doctor reads every checkout under it (`doctor.rs:1730`) with no regard to source kind.
   - Measured: case :616.
   - What reaches it: SETUP.md step 6 (`root: <directory holding the repository>`) on any `~/src` that also holds a GitLab or other non-GitHub clone. Step 13 then sends the person to step 11, and the fix line there reverses the bot route setup just wrote.
   - Fix (coordinator's choice): either `route_for` writes `direct` when a checkout under the root is not on GitHub, or the per-checkout check is limited to `local` sources as the correction brief worded it. The second conflicts with `config.md:124`.
   - NEEDS-CHANGE, introduced.
2. **`ssh.github.com` is refused** (`doctor.rs:1620`).
   - GitHub's documented SSH-over-443 origin is treated as a host the bot route does not reach. That gives a false `not ready` and a `glab` sample.
   - Measured: case :652.
   - What reaches it: a checkout cloned with that URL. Rare.
   - CONFIRMED, introduced.
3. **A GitHub origin written in mixed case is read as GitLab** (`setup.rs:132`).
   - `origin_of` compares the host case-sensitively, so `route_for` derives `direct` with a self-contradicting reason. Doctor's `bot_route_writes_to` ignores case, so the two disagree.
   - The misreading was already at the base; the derived route is new.
   - Measured: case :678.
   - CONFIRMED, introduced.
4. **Setup names one box refusal twice.**
   - Where: the `box` row's `box (refused)` (`setup.rs:883`, via `failure`), next to doctor's `box <name> (placement.writable-root-shared)`.
   - Measured: case :707.
   - What reaches it: a database at the old default path (getting-started documents this case).
   - CONFIRMED, introduced.
5. **Docs drift: the guide loops at its step 1** (`website/docs/guides/first-task-existing-repo.md:79`).
   - It says "run `conductor setup` again until its last line is `ready`", but the substrate daemon is only started in its section 2. Setup now asks the substrate daemon, so on `placement: systemd` it cannot end `ready` before then. At the base it did.
   - Measured: case :733 (setup ends with doctor's `box zeta (…)` reason).
   - NEEDS-CHANGE, introduced.
6. **Docs drift: the quoted verdict is wrong.** `first-task-existing-repo.md:59` and `getting-started.md:138` say the verdict reads `not ready: box (refused)`. It now reads `not ready: box <name> (placement.writable-root-shared), box (refused)`.
   - Measured: case :707.
   - CONFIRMED, introduced.

**5. Attacked, not broken**
- Pass 1 fixtures:
  - Case 5 now writes its instance first.
  - Case 2 gained `session_prefix`.
  - No assertion was weakened, removed or made unreachable.
- Process count: origins are read from `.git/config` files, with no `git` process per checkout and the guard asked in process. There is no own bound, which is linear and harmless.
- A broken or missing origin is skipped: the hook reads the same nothing for it.
- `and <n> more` arithmetic is right, and reasons do not carry the repository name, so they collapse correctly.
- A `direct` route prints no `no bot route` line.
- Plan line matches the `--apply` line.
- `--apply` twice, set then unset: it finds the instance and rewrites nothing.
- Policy path checks: a path with a space is quoted; `~` and a relative value both match the broker.
- Origin variants:
  - two remotes: the `origin` remote wins;
  - no origin, or a local-path origin: `direct`.
- A readable file that is not a policy is taken as one. That is by decision; conductor never parses a policy.
- Setup with no substrate daemon is bounded by `SUBSTRATE_BOUND` (5 s); there is no hang.
- Mutants caught:
  - `route_for` always `direct`;
  - `readable_file` accepting a directory;
  - `lead_lines` asking only the first checkout.
- The `$ ` rule: no page has a `$ cat` config block (blocks scanned). It would skip one, but nothing reaches that.
- Step citations after the renumbering are consistent.

**6. Paths written outside the worktree**
- `<scratch>/w51/u3/adv2/blocks.py`
- `<scratch>/w51/u3/adv2/doctor.diff`
- `<scratch>/w51/u3/adv2/setup.diff`
- `<scratch>/w51/u3/adv2/red-alone.log`
- `<scratch>/w51/u3/adv2/suite.log`
- Fixtures of the red cases under the shared target's `tmp/adv_w51_doctor_guard*/adv2-*`. `Drop` keeps them when a case panics.

```findings
- file: crates/conductor/src/setup.rs
  line: 265
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'route_for writes the bot route from the one repository''s origin while doctor and setup''s own verdict refuse it for any non-GitHub clone under the parent directory setup writes as the checkouts root, so a fresh install that follows SETUP.md does not end ready'
- file: crates/conductor/src/doctor.rs
  line: 1620
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'a checkout whose origin is GitHub''s ssh.github.com on port 443 gives no bot route for ssh.github.com and a denied glab sample, so doctor is not ready on a GitHub repository'
- file: crates/conductor/src/setup.rs
  line: 132
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'origin_of compares the host case-sensitively, so a GitHub.com origin with a policy derives the direct route with the reason that the origin is not on github.com, while doctor reads the host in any case'
- file: crates/conductor/src/setup.rs
  line: 883
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'with a refused box setup''s verdict names box (refused) beside doctor''s box <name> (<refusal>), one refusal twice, where doctor''s verdict on the same file names it once'
- file: website/docs/guides/first-task-existing-repo.md
  line: 79
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the guide says to rerun conductor setup until it is ready before its section 2 starts the substrate daemon, and setup now cannot end ready before that on placement systemd'
- file: website/docs/getting-started.md
  line: 138
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'this page and first-task-existing-repo.md:59 quote the verdict as not ready box (refused), which setup now prints after doctor''s box <name> (placement.writable-root-shared)'
```
