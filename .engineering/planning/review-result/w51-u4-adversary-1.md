---
format: aep.planning-md/3
id: review-result:w51-u4-adversary-1
kind: review-result
status: active
title: Wave 51 U4 box-reaches-forge-and-daemon, adversary pass 1
relations:
- reviews: story:box-reaches-forge-and-daemon
revision: 1
---
```
unit: story:box-reaches-forge-and-daemon, U4 wave 51, worktree w51-u4 at e59189a23 plus uncommitted test additions
verdict: INFEASIBLE (one red case; the state it builds is one I could not show anybody reaches)
cases: executed 1837→1840, red 1
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 6 paths (listed in part 6)
needs-coordinator: none
```

I could not break the token path itself. The one red case is a doctor gap in a state that is hard to reach.

**1. Diff stat**
```
 crates/conductor/tests/placement.rs | 120 ++++++++++++++++++++++++++++++++++++
 1 file changed, 120 insertions(+)
```
Test file only. Nothing is committed.

**2. Cases added** (end of `crates/conductor/tests/placement.rs`)
- `adv_box_token_doctor_names_a_box_whose_declared_slot_conductord_cannot_seal` (:5120). **Red now.** The daemon declares `api_token_alpha` and `~/.b10x/conductor/daemon/slots` is a regular file. conductord would refuse this box as `placement.api-token-unsealed` (`reconcile.rs:1274-1277`). Doctor says it is fine. Red output of the case run alone:
  ```
  panicked at crates/conductor/tests/placement.rs:5143:5:
  conductord would refuse the box; doctor says: exit Some(1)
  ...
  box alpha: systemd, 4 writable roots served by the substrate daemon
  not ready: git (missing), gh (missing), ... b10x-gates (missing)
  ```
  The verdict names no box. The exit 1 comes only from tools missing in the test's isolated PATH. The first run failed on a socket path that was too long (setup error, not a finding); I shortened the case directory name and reran.
- `adv_box_token_a_restart_in_the_box_reseals_the_new_token` (:5038). **Green.** A restart inside the box mints a new token, rewrites the slot file with it and names the slot again. The unit had no case for this.
- `adv_box_token_a_wider_slot_file_and_directory_are_made_owner_private` (:5084). **Green.** A slot file left at 0644 and a directory at 0755 become 0600 and 0700. The unit's suite only checked the mode of a newly created file, so dropping either `set_permissions` call would have stayed green. This case catches that.

**3. Suite run** (after the cases existed)
`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100:
```
Summary [ 286.207s] 1840 tests run: 1837 passed (5 slow), 3 failed, 12 skipped
FAIL conductor-cli::placement adv_box_token_doctor_names_a_box_whose_declared_slot_conductord_cannot_seal
FAIL conductor-cli::adv_store adv_racing_handles_lose_no_proposal      ("database is locked" under load; the unit touches no store code)
FAIL conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger   (known)
```
The 1837 before-number is the count the implementor reported for its gate (1837 run).

**4. Findings** (they cover e59189a23 plus my test additions)

| # | file:line | finding | verdict / origin | what reaches it |
|---|---|---|---|---|
| 1 | `crates/conductor/src/doctor.rs:1551` | Doctor's own pre-substrate refusals do not include the new `placement.api-token-unsealed`. A box conductord refuses passes doctor's dry run. Measured at test :5143. | INFEASIBLE / introduced | Nothing found. Substrate reads a slot file only if it belongs to its own user and is mode 0600, so conductord and substrate run as one user. That user can always write the file unless the path is broken, as my case builds it. |
| 2 | `crates/conductor/src/daemon/reconcile.rs:1597` | `keep()` writes `runtimes.json`, which holds every instance's token, at the umask mode (0644). The daemon directory is created at the umask mode too. So the 0600 slot file guards a token that is also readable elsewhere. | CONFIRMED / pre-existing | I read the base (not run): the same `keep()` at line 1562 and `create_dir_all` in `Daemon::open`. The token only opens `api.sock`, which is mode 0600, so another user holding it cannot connect. Low impact. |
| 3 | `crates/conductor/src/run/dispatch.rs:75` | Substrate places the sealed slot at fd 4 without close-on-exec. Every process the boxed runtime spawns inherits fd 4 and `SUBSTRATE_SECRET_SLOTS`, so it can read the token. | CONFIRMED / introduced | `run_step` starts every loop step. A runtime started as a unit already passes `CONDUCTOR_API_TOKEN` to its children on the base. This matters once sessions start inside the box. |

**5. Attacked, could not break**
- **Slot format:** `SUBSTRATE_SECRET_SLOTS` matches substrate's `secret_slot_environment` at the pinned SDK revision (`name=fd`, joined by commas, sorted). The value arrives as a sealed memfd, and opening `/proc/self/fd/N` reads it from the start.
- **Instance names:** a name with `/`, `..`, a space, `=` or a comma maps to `_`. The result fits substrate's slot-name rule and its 64-byte cap, with no path traversal.
- **Shared slots:** two names that differ only in case or punctuation share one slot. Each start rewrites the file first, and substrate reads it at that start, so neither box gets the other's token.
- **Token handling:**
  - Order is env, then slot, then mint. The box's environment is built by conductord with the token variable filtered out, so nothing inside the box can override it.
  - A token longer than the 4096-byte read buffer, or a trailing newline, is handled (it is trimmed).
  - Descriptors 0/1/2 or a negative number cannot reach `sealed_token`, because substrate sets the variable.
  - A slot name that is a prefix of another is not mistaken for it.
- **Timing:** start and restart both seal the token before substrate is asked (case :5038). If the slot is declared when conductord asks but undeclared at the start, substrate refuses the start and the next pass starts without the slot, so the reconciler settles. A daemon that does not answer counts as undeclared, and the open then fails as unreachable, as on the base.
- **Mutants:** `declares` always true or always false is caught by the unit's cases. The mode bits are covered now by :5084.
- **Leaks:** neither the token nor any value appears in a refusal detail, doctor's note, the `HostSlot` or `BoxRequest` debug output, or the session start body.
- **Doctor's note:** it is not printed for a remote box. The flag is ready to paste, and substrate checks the file at its own startup, as the pages say.
- **Pages:** SETUP step 9, getting-started section 5, the host, run isolated, operators and credentials and README step 5 match the code, and their links resolve.

**6. Paths written outside the worktree**
- <scratch>/w51/u4/adv1/ (the directory)
- <scratch>/w51/u4/adv1/tests.diff
- <scratch>/w51/u4/adv1/cases.rs
- <scratch>/w51/u4/adv1/run1.log
- <scratch>/w51/u4/adv1/run2.log
- <scratch>/w51/u4/adv1/suite.log

**7. Findings block**
```findings
- file: crates/conductor/src/doctor.rs
  line: 1551
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'doctor''s dry run omits conductord''s new placement.api-token-unsealed refusal, so a box whose declared slot file cannot be written shows as served (red at tests/placement.rs:5143), in a state no operator was shown to reach'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 1597
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'runtimes.json, which holds every instance''s token, is written at the umask mode into a daemon directory created at the umask mode, so the slot file''s 0600 does not keep the token owner-private'
- file: crates/conductor/src/run/dispatch.rs
  line: 75
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the sealed API-token descriptor 4 and SUBSTRATE_SECRET_SLOTS are inherited by every process the boxed runtime spawns, as CONDUCTOR_API_TOKEN already is for a runtime started as a unit'
```
