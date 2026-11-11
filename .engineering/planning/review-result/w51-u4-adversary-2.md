---
format: aep.planning-md/3
id: review-result:w51-u4-adversary-2
kind: review-result
status: active
title: Wave 51 U4 box-reaches-forge-and-daemon, adversary pass 2
relations:
- reviews: story:box-reaches-forge-and-daemon
revision: 1
---
```
unit: story:box-reaches-forge-and-daemon U4 wave 51 pass 2, worktree w51-u4 at dba5aca5c plus three uncommitted test cases
verdict: CONFIRMED (doctor and conductord still disagree when conductord runs on its own --state-dir; red both ways)
cases: executed 1844→1847, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (part 6)
needs-coordinator: none
```

The shared check works, but doctor gives it a different path from the one conductord writes. The 1844 is the implementor's gate count from `impl-report-3.md`.

**1. Diff stat**
```
 crates/conductor/tests/placement.rs | 131 ++++++++++++++++++++++++++++++++++++
 1 file changed, 131 insertions(+)
```
Only a test file changed. Nothing is committed.

**2. Cases added** (end of `crates/conductor/tests/placement.rs`, all three red now)

- **`adv2_box_token_doctor_names_the_refusal_of_a_daemon_on_its_own_directory` (:5479).**
  - Setup: conductord's directory is `<case>/daemon`, as `daemon serve --state-dir` sets it, and its `slots` is a file.
  - conductord refuses the box with `placement.api-token-unsealed`.
  - Doctor prints the box as served. Red, run alone:
    ```
    panicked at crates/conductor/tests/placement.rs:5507:5:
    conductord refuses the box; doctor says: exit Some(1)
    ...
    box alpha: systemd, 4 writable roots served by the substrate daemon
    not ready: git (missing), gh (missing), ... b10x-gates (missing)
    ```
- **`adv2_box_token_doctor_refuses_no_box_for_a_slot_file_its_daemon_does_not_write` (:5518).** The reverse.
  - conductord on `<case>/daemon` writes its slot and serves the box.
  - Doctor refuses the box because of `~/.b10x/conductor/daemon/slots`, which conductord never writes. Red, run alone:
    ```
    panicked at crates/conductor/tests/placement.rs:5538:5:
    conductord serves the box; doctor says: exit Some(1)
    ...
    box alpha: placement.api-token-unsealed: ... create <case>/home/.b10x/conductor/daemon/slots: <case>/home/.b10x/conductor/daemon/slots is not a directory
    not ready: ..., box alpha (placement.api-token-unsealed)
    ```
- **`adv2_box_token_a_slot_directory_without_its_write_bit_is_restricted_and_written` (:5550).**
  - Setup: the slot directory belongs to this user, has mode 0500, and has no file in it.
  - `seal_api_token` would make it 0700 and then create the file. At e59189a23 the write did just that (read, not run).
  - The new check refuses first. Red, run alone:
    ```
    panicked at crates/conductor/tests/placement.rs:5572:5:
    assertion `left == right` failed: [... result: Refused, detail: Some("... placement.api-token-unsealed: ... create the slot file <case>/daemon/slots/api_token_a: <case>/daemon/slots is not writable: EACCES: Permission denied") }]
      left: Refused
     right: Succeeded
    ```

**3. Suite run** (after the cases existed)

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100:
```
Summary [ 242.799s] 1847 tests run: 1843 passed (4 slow), 4 failed, 12 skipped
FAIL conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger   (known)
FAIL conductor-cli::placement adv2_box_token_a_slot_directory_without_its_write_bit_is_restricted_and_written
FAIL conductor-cli::placement adv2_box_token_doctor_names_the_refusal_of_a_daemon_on_its_own_directory
FAIL conductor-cli::placement adv2_box_token_doctor_refuses_no_box_for_a_slot_file_its_daemon_does_not_write
```

**4. Findings** (they cover dba5aca5c plus my three cases)

| # | file:line | finding | verdict / origin | what reaches it |
|---|---|---|---|---|
| 1 | `crates/conductor/src/config.rs:5502` | Doctor's dry run checks the slot under `HOME/.b10x/conductor/daemon` (`box_requests` sets `Host.daemon` there). conductord writes under its own directory (`self.dir`, `reconcile.rs:1266`). The check function is shared, but its input path is not, so the two drift in both directions (:5507, :5538). The unit's own test (:4098) builds the broken state in both directories, which hides this. Doctor takes no `--state-dir` (`cli.rs:545`). | CONFIRMED / introduced | `conductor daemon install --state-dir <dir>` (`daemon/install.rs:55`), documented in `website/docs/guides/upgrade-and-rollback.md:17`. Possible fix: doctor learns conductord's directory (a flag, or the installed unit), or prints which path it checked. |
| 2 | `crates/conductor/src/placement.rs:823` | `slot_unwritable` is stricter than the write it guards. A slot directory of this user's without its write or search bit is refused, though the write makes it 0700 first and would succeed (:5572). A running box hits this at its next restart, since restarts reseal. The decision of correction 1 was that an existing box is not refused for the slot. The dangling-symlink refusal is the same kind, and the implementor declared it. | INFEASIBLE / introduced | Nothing found: no code or guide makes the slot directory non-writable. Possible fix: skip the write/search-bit check for a directory that is this user's, because the write restricts it anyway. |
| 3 | `crates/conductor/tests/placement.rs:4287` | No static guard over spawn sites. The only test covers `collect::run` and one direct start (`dashboard.rs:1623`, `claude agents --json`). The 15 other hand-placed `env_remove` calls, and any new `Command::new`, are unguarded. Mutant, not run: drop `.env_remove` at `decide/model.rs:138`; no test starts that path with the slot variable set. | CONFIRMED / introduced | Today the variable only names a closed descriptor, and nothing readable goes with it. It matters if a later change stops closing the descriptor. |

**5. Attacked, could not break**
- **One call site:** `sealed_token` is called only at `instance.rs:264`, once per `instance serve` process. No re-exec exists (no `CommandExt::exec` or fork in `src`). Nothing else reads `SUBSTRATE_SECRET_SLOTS`, so a reused descriptor 4 (the listener bound right after) is never read or closed again.
- **Closing:** the descriptor is closed whether the read succeeds or fails half-way, and the read error is kept. `close` errors are ignored, which is harmless (EBADF). The process has no thread or child before the close. `+4` and ` 4` parse to 4; numbers 2 and below and above the descriptor limit are refused. With two `api_token_*` pairs, only the first is read (substrate names one).
- **Spawn-site audit:** every `Command::new` the runtime can reach goes through `collect::run` or carries the removal. This includes tmux in `systemd.rs:306` and `watch.rs:506`, whose server inherits the client's environment, which no longer has the variable.
  - `build.rs` runs only under `conductor build run` and `broker serve`. No code in `src` starts `broker serve`.
  - `attach.rs`, `tui/`, `init.rs` (via `setup`) and `upgrade.rs` run only as commands a person types. `daemon/` uses `upgrade.rs` only inside conductord.
- **Mutants:** `slot_unwritable` always `Ok` is caught by :5423 and :4098. Removing the close is caught by :4287 (fd 4 has no close-on-exec in the test's harness). Removing the `env_remove` in `collect::run` is caught by :4287 (`--version`). `declared` always true is caught by :3883.
- **Write safety:** `write_slot_file` is unchanged from correction 1 (no no-follow, no exclusive create), so it is not weaker. A symlink at the slot path to this user's file passes the check. The write truncates the target, writes the token there and sets it to 0600. Only this user can plant that symlink, since the directory must be theirs and 0700.
- **Check vs write in other states:** these all give the same answer from check and write: a read-only file of this user's, a read-only file system (EROFS), another user's directory or file, a symlinked daemon directory of this user's, an ancestor that is not a directory, a dangling ancestor, and root.
- **Doctor output:** the dry run prints the path and nothing of the token, because the check never opens the file. A box on a remote daemon gets no slot and is not asked. The refusal reads `placement.api-token-unsealed: the daemon's token for the runtime is not in its slot …`. Where several refusals apply, both sides still refuse; I found no divergence other than finding 1.
- **Page:** the sentence at `the-host.md:67` matches the code.
- **Model-token slot (not a finding):** its slot at fd 3 stays open in every child, as on the base. Its name is now gone from the children's environment too, which the implementor disclosed; nothing in the tree reads it.

**6. Paths written outside the worktree**
- `<scratch>/w51/u4/adv2/` (the directory)
- `<scratch>/w51/u4/adv2/cases.log`
- `<scratch>/w51/u4/adv2/suite.log`

The case directories `placement/a2r`, `a2s` and `a2m` were also created under the build's `CARGO_TARGET_TMPDIR`, which the ledger manages.

**7. Findings block**
```findings
- file: crates/conductor/src/config.rs
  line: 5502
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'doctor''s slot check reads the default daemon directory while conductord on --state-dir writes its own, so doctor passes a box conductord refuses and refuses one it serves (red at tests/placement.rs:5507 and :5538)'
- file: crates/conductor/src/placement.rs
  line: 823
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'slot_unwritable refuses a slot directory of this user''s without its write bit although seal_api_token would make it 0700 and write the file, so a box served after correction 1 is refused at its next restart (red at tests/placement.rs:5572)'
- file: crates/conductor/tests/placement.rs
  line: 4287
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'no static guard holds the hand-made SUBSTRATE_SECRET_SLOTS removals, so dropping one of the 15 direct-site env_remove calls or adding a new Command::new leaves the suite green'
```
