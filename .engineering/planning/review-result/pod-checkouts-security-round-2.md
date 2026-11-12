---
format: aep.planning-md/3
id: review-result:pod-checkouts-security-round-2
kind: review-result
status: active
title: Fixes of round 1, the start snapshot and the instance secrets read, security review round 2
relations:
- reviews: story:pod-source-checkouts
- reviews: story:instance-secret-state
revision: 1
---
needs-change

unit: security review round 2 of d31d595a2, c04ad0190 and b1c043695 (`917848c1e..ac51f6e67`), plus my uncommitted test additions in `k8s-checkouts-fix`
verdict: needs-change
cases: executed 36→40, red 3 (36 is the same five test files without my four new cases, counted from the run below)
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 paths (listed below)
needs-coordinator: whether the seeding should keep trusting checkouts outside `include` in a pod (F2), and whether non-forge `kubernetes.env` secrets should be withheld (F3)

1. **Worktree changes:** `git diff --stat` shows `crates/conductor/tests/sec_pod_checkouts.rs | 298 +` (new cases appended; the round 1 cases are unchanged). One new untracked file, `crates/conductor/tests/sec_instance_secrets.rs`. No implementation file was touched.

2. **Cases added**, first run alone (`--test sec_pod_checkouts --test sec_instance_secrets`):
- `a_dot_git_step_in_another_case_or_with_a_trailing_dot_runs_no_code` (listings `platform/.GIT` and `platform/.git.`): green.
- `a_start_trusts_no_checkout_its_include_no_longer_covers`: **red**. "the start trusted a checkout no include entry covers: […/records, …/root/alpha, …/root/beta]". The runtime printed "trusted: …/root/beta".
- `a_stopped_snapshot_leaves_no_clone_running`: **red**. "the clone (process 525561) still ran after the snapshot's process group was stopped".
- `a_mapping_learns_nothing_of_a_secret_a_kubernetes_env_entry_names`: **red**. "the rows tell a mapping of the operator's env entry's secret: ["model_token set true modified_at Some(\"2026-11-12T14:45:39…Z\")"]".
- My round 1 `.git` case is now green.

3. **Suite run:** `zsh …/sec2/gate.sh` runs clippy (exit 0), then nextest on `sec_pod_checkouts`, `sec_instance_secrets`, `pod_checkouts`, `instance_secrets` and `isolated_home`. Result: `40 tests run: 37 passed, 3 failed, 0 skipped`, nextest exit 100. The three failures are the three red cases above.

**Claims**
- **A. Holds.** `names_no_checkout` ignores case (`checkouts.rs`). `bring`, the listing, `include`/`exclude`, `found` and `is_checkout` all refuse or skip such a step.
  - `.git.`: on Linux a directory of that name is no git directory (green case).
  - A `.git` step in the group part: the group appears only in the URL and API path, never on disk.
  - Group `.`: `under` keeps the path relative.
  - A path that differs only in case is a separate directory.
  - git refuses `.git` as any path component of a checked-out tree, so a repository cannot carry `<sibling>/.git` or a `.git` link.
  - Symbolic links inside a checkout are read by no scan, since every scan stops at depth 2 or at a checkout.
  - The pod no longer writes a nested `.git`. So the git-presence helpers (guard, doctor's `git_dirs_under`, trust's walk, spawn) only meet one left by a pre-fix binary, which was never released.
- **B. Does not hold** (red case).
  - The seeding's walk trusts every directory directly under the root that holds `.git` (`trust.rs:352`, at every start for paths not yet trusted).
  - A checkout cloned under an earlier `include`, or one whose source host changed, is trusted again at each start of a pod whose home is fresh.
  - spawn refuses it (outside `include`), so trust takes effect only if a session starts there by another route.
  - A clone's temporary directory holds `.git` only 3 levels down, and is not trusted.
- **C. Holds.**
  - Only `daemon.kubernetes.env` (the operator) reaches the pod's environment; the manifest gets nothing else from a mapping (`pods.rs:325-360`).
  - `0`: no clone begins at the start; the passes clone them.
  - Negative or not a number: one line is printed, and the 20-minute default applies.
  - `u64::MAX`: no bound at all, 8 clones at a time, no panic.
  - Note: the bound stops only *beginning* clones; a clone already running can last another 20 minutes (`checkouts.rs:528`).
- **D. Does not hold** on the bound.
  - The start snapshot is limited to 3600 s (`instance.rs:443`), and the API thread and probes keep answering during it.
  - The loop's start can be delayed by about 40 minutes of start clones plus up to 1 hour of snapshot.
  - No new code path and no write outside the state directory beyond what the cycle's snapshot already does.
  - But `collect::run` gives each child its own process group (`collect/mod.rs:654`). Stopping the snapshot leaves its `git clone` running with no timer (red case), and it writes into a temporary directory the next pass removes. The cycle's step (`STEP_BOUND`) has the same gap.
- **E. Partly holds.**
  - No value, length or digest appears in the answer.
  - Names must follow the `[a-z0-9][a-z0-9._-]*` segment rule, so `dir.join(name)` stays inside the source directory, and a caller cannot place a link there.
  - Admission happens before the route (`api.rs:1027` vs `1125`); `/v1/secrets` still refuses the control token (`api.rs:2072`).
  - Not holding: a mapping that names a non-forge `daemon.kubernetes.env` secret, or any other name in the source, gets `set` and `modified_at` for it (red case). `withheld` covers only the control token, the store password and forge entries (`secret.rs:885`). Configs with a `globals` secret show its rotation time to the caller.
  - The `500` body names the daemon's config path, and the `SourceUnusable` detail names the source directory (`api.rs:1261`).
- **F. Holds as stated.** `GLAB_SEND_TELEMETRY=false` is set only in `answer` (`repositories.rs:1777`). doctor, the watch, spawn, sessions and the agents' own `glab` calls in the pod still report. `glab config --help` (1.115.0) says reports go to "your GitLab instance", the host that already holds the token, so this is not a token leak. Setting it in the pod's environment would cover every call.

**Findings**
- **F1** (warning, CONFIRMED, introduced): `collect/mod.rs:654`. Stopping a snapshot leaves its clones running. The mechanism predates this work; clones and the start snapshot are what reach it.
- **F2** (note, CONFIRMED, introduced): `trust.rs:352`. The seeding re-trusts checkouts no `include` covers.
- **F3** (note, CONFIRMED, introduced): `secret.rs:885`. A mapping can learn whether a non-withheld secret is set, and when it changed.
- **F4** (note, not run, introduced): `api.rs:1261`. Error bodies name internal paths.
- **F5** (note, not run, introduced): `repositories.rs:1777`. Telemetry is turned off only for the collectors' `glab` calls.

**Not checked:** a real forge over the network; whether a pod's home is on the volume claim (I found no `HOME` in `pods.rs`); the keychain and remote sources under the new route.

**Written outside the worktree:**
- `<the review scratch directory>/sec_pod_checkouts.committed.rs`
- `<the review scratch directory>/append.rs`
- `<the review scratch directory>/run-own.log`
- `<the review scratch directory>/gate.sh`
- `<the review scratch directory>/gate.log`
- The red cases' directories, kept because they failed: `<the shared build target>/tmp/sec_pod_checkouts/{start-trust,orphan-clone}` and `…/tmp/sec-instance-secrets/env-entry`

```findings
[
{"file":"crates/conductor/src/collect/mod.rs","line":654,"category":"concurrency","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"each collector command leads its own process group, so stopping the start snapshot or a cycle step at its bound leaves a git clone running with no timer, writing into a temporary directory the next pass removes"},
{"file":"crates/conductor/src/trust.rs","line":352,"category":"integrity","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a pod start with a fresh home trusts again every git checkout directly under the root, including one cloned under an earlier include that no entry covers now"},
{"file":"crates/conductor/src/secret.rs","line":885,"category":"integrity","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a mapping that names a non-forge kubernetes env secret or any other non-withheld name in the source learns through the instance secrets read whether it is set and when its file changed"},
{"file":"crates/conductor/src/daemon/api.rs","line":1261,"category":"judgement","severity":"note","origin":"introduced","message":"the error bodies of the instance secrets read name the daemon's config file path and the secrets source directory to a control token caller"},
{"file":"crates/conductor/src/collect/repositories.rs","line":1777,"category":"judgement","severity":"note","origin":"introduced","message":"glab telemetry is turned off only for collector calls, while doctor, spawn, the watch, sessions and agents in the pod still report usage to the forge host"}
]
```
