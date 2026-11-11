---
format: aep.planning-md/3
id: review-result:w51-u7-adversary-1
kind: review-result
status: active
title: Wave 51 U7 container-start-without-prompt, adversary pass 1
relations:
- reviews: story:container-start-without-prompt
revision: 1
---
unit: U7 of wave 51, story:container-start-without-prompt. Findings cover worktree `w51-u7`, head ce52cfe6b, plus my one uncommitted test file.
verdict: NEEDS-CHANGE
cases: executed 1968→1972, red 4
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (part 6)
needs-coordinator: yes. Commit `crates/conductor/tests/adv_w51_container_start.rs`, and decide findings 5 to 7, which are design calls and have no red case.

**1. Diff**
`git --no-pager diff --stat` is empty: the only change is one untracked file, `crates/conductor/tests/adv_w51_container_start.rs` (484 lines). It is a test file, so the bound held. I changed no implementation, spec, doc or workflow file and applied no mutant. It passes `rustfmt --check` and the gates scan (`1 file(s) carry no private rule`).

**2. Cases added** (all red now; I ran them alone first, `red1.log`, Summary: `4 tests run: 0 passed, 4 failed, 0 skipped`)
- `adv_reveal_model_token_refuses_an_instance_whose_placement_opens_a_box`
  - It first checks that `placement: none` reveals the value (exit 0). It then asserts that `placement: systemd` refuses.
  - Red: `the model token of a boxed instance was printed: exit Some(0)` / `<the value>`
- `adv_a_loopback_address_other_than_127_0_0_1_answers_its_own_probes`
  - Config `address: 127.0.0.2`, then `GET /healthz` sent to 127.0.0.2 with its own `Host`.
  - Red: `Ok((403, "{\"error\":\"this listener answers only its own host\"}\n"))`
- `adv_a_records_commit_that_failed_at_the_first_start_is_made_at_the_next`
  - First start with an empty git identity, so the commit fails. The second start has an identity and should make the commit.
  - Red: `fatal: ambiguous argument 'HEAD': unknown revision` and `left: "" right: "1"`. The second start printed nothing about the uncommitted records.
- `adv_healthz_answers_while_the_unattended_start_seeds`
  - A stand-in `worktree inspect` takes 8 s. `/healthz` is sent while the start seeds.
  - The case first checks that `/healthz` answers 200 once the daemon serves, so the request itself is valid.
  - Red: `Err("no answer within 3s: Resource temporarily unavailable (os error 11)")`

**3. Package run** (after the cases existed)
`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100, `suite1.log`:
`Summary [ 325.705s] 1972 tests run: 1966 passed (4 slow), 6 failed, 13 skipped`
- 4 failures are my cases.
- `build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger` is on the known-load list.
- `public_text::planning_and_docs_carry_no_instance_id` is settled.
- `isolated_home` passes with my file in place.

**4. Findings** (all at ce52cfe6b)

1. **Model-token reveal has no placement check.** `crates/conductor/src/token.rs:1369`, NEEDS-CHANGE, warning, introduced.
   - Measured: case 1. `token reveal --model-token` prints the model token of an instance at a placement that opens a box. The CLI help (`cli.rs:1248`, "an instance whose placement opens no box") and `spec/domains/token.yaml` `RevealToken` say it serves only placements with no box.
   - What reaches it: any process of the user that has the config file. On a host with a boxed instance and an unboxed one, the unboxed agent can print the boxed instance's token with one command. A container is not affected, because all its instances are `none`. It is not reachable over HTTP: `http.yaml` has no reveal route.
   - Fix: refuse when `crate::placement::boxed(instance)` is true.
2. **A loopback address other than 127.0.0.1 makes the daemon refuse its own clients.** `crates/conductor/src/daemon/api.rs:689` (with `:529` and `own_host` at `:938`), CONFIRMED, warning, introduced.
   - Measured: case 2. `daemon.address` accepts any IP address (`config.rs:2692`). A loopback address such as `127.0.0.2` or, by the same code, `::1` is served as the loopback listener. That listener answers only the `Host` values `127.0.0.1` and `localhost`. So every request that names the bound address gets 403, `/healthz` included.
   - What reaches it: an operator who writes `address: "::1"` or another 127/8 address. The spec says "an IP address".
   - Fix: accept the bound address in `own_host`, or refuse loopback addresses other than `127.0.0.1` in the config.
3. **A failed records commit is never retried.** `crates/conductor/src/daemon/unattended.rs:58`, CONFIRMED, warning, introduced.
   - Measured: case 3. The records step skips whenever `NORTHSTAR.md` exists. So a skeleton whose commit failed once stays uncommitted at every later start, and nothing says so again. Doctor's records row also reads only `NORTHSTAR.md` (`setup.rs:539`).
   - What reaches it: a container started without the `GIT_*` variables (the image sets no git identity), then restarted with them while keeping its filesystem. `setup --apply` has the same check at base (`setup.rs:751`), but a person runs that and sees the error.
4. **Probes get no answer while the unattended start seeds.** `crates/conductor/src/daemon/mod.rs:176`, NEEDS-CHANGE, warning, introduced.
   - Measured: case 4. The listener is bound at `:166`, then `seed` runs, and the serve thread starts only at `:209`. While seeding, `/healthz` and `/readyz` get no answer. This breaks U5's rule in the same file: "the listeners take requests as soon as the config is read".
   - What reaches it: every unattended first start, probed by a container healthcheck or a cluster's liveness probe. Seeding takes up to 10 s for `inspect` plus 20 s for `activate`, per instance, plus git.
   - Fix: start the serve threads before `seed`. The `/v1/` paths already wait in `api.await_start()`.
5. **The empty-pool fallback is not limited to containers.** `crates/conductor/src/token.rs:766`, CONFIRMED, warning, introduced. This is a judgement finding with no case.
   - The fallback is gated on neither `daemon.unattended` nor the `files` source. At base, an empty pool left the launch unchanged (base `token.rs:720`), and base accepted `model_token` at `placement: none` (`config.rs:3557`), where it did nothing.
   - What reaches it: a workstation config with an unboxed instance that names `model_token`. Its agents now switch from the harness's own login to the API key without anyone asking. If the keychain holds no value for it, the agents fail to authenticate.
6. **The workflow job test misses some mutants.** `crates/conductor/tests/container_start.rs:1137`, CONFIRMED, note, introduced.
   - `the_check_workflow_s_container_job_runs_the_image_case` reads only the job's `permissions` and its `run:` steps. A job given `if: false` or `continue-on-error: true` stays green.
   - The test also passes, with a "skipped" line, when `task` is not on PATH (`:1126`).
7. **`systemd-run` on PATH counts as a user manager.** `crates/conductor/src/daemon/units.rs:120`, INFEASIBLE, note, introduced.
   - A container whose image holds `systemd-run` but runs no manager takes the unit path, and every start fails.
   - Infeasible because I could not show anyone reaches it: the shipped `Dockerfile` installs no systemd.

**5. Attacked, could not break**
- **Files source, writes:** `secret put`, `Secrets::put` (ReadOnly), pool contribute (ReadOnly) and `keep` (only the client's keychain) write nothing. The backend's capabilities hold no Write. `ess specify validate` reports valid.
- **Files source, slot names:** `..`, a leading dot and invalid names are refused by `StoredName::parse` (`secret.rs:409`) and read as missing.
- **Files source, values:** a file is read at each use, so a rotated secret is picked up. One trailing newline or CRLF is dropped. A FIFO or a huge file would block or load in full; only the person who mounts the directory can build that, so I wrote no case.
- **Non-loopback listener:**
  - Every `/v1/` path needs an Identity bearer. The daemon's own token is not accepted there.
  - `index.html` carries no token (`api.rs:961`).
  - The session cookie is `Secure`.
  - `::ffff:127.0.0.1` is treated as needing bearers, which is stricter. `localhost` is refused by the config.
- **Child runtimes:** after SIGKILL, `end_with` ends the runtime (the implementor's case shows it). `PARENT_VARIABLE` is removed from grandchildren. A dead child is reaped at the next `active()` call. The failed-start back-off applies (the implementor's case passes `Refused`).
- **Seeding overwrites nothing:** `init` refuses when any skeleton file exists (`init.rs:155`).
  - Trust is put back at every start, including an operator's explicit `false`. That is by design.
  - It trusts every git directory under the root (`trust.rs:352`). That is `conductor trust` at base.
- **Doctor:** `doctor.rs` is unchanged by the unit. `guard … not checked` follows U3's own condition, the same as on a host.
- **Workflow test:** a filter that matches no case, a dropped `--ignored`, or a replaced `task container-test` step each turn it red.

**6. Paths written outside the worktree**
- `<scratch>/w51/u7/adv1/` (the directory)
- `<scratch>/w51/u7/adv1/red1.log`
- `<scratch>/w51/u7/adv1/suite1.log`
- Case directories left by the red cases, which keep their directory when they fail: `~/.cache/b10x/conductor/beyond10x/target/conductor-362621f27d1a30dd/tmp/csadv/`

No container was run.

```findings
- file: crates/conductor/src/token.rs
  line: 1369
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'token reveal --model-token prints the model token of an instance whose placement opens a box, though its help and the spec limit it to placements with no box'
- file: crates/conductor/src/daemon/api.rs
  line: 689
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'a daemon.address that is loopback but not 127.0.0.1 (127.0.0.2, ::1) answers 403 to every request naming its own address, the probes included'
- file: crates/conductor/src/daemon/unattended.rs
  line: 58
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'a records skeleton whose commit failed at one unattended start is skipped at every later start because NORTHSTAR.md exists, so it is never committed and nothing says so again'
- file: crates/conductor/src/daemon/mod.rs
  line: 176
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the unattended seeding runs after the listener is bound and before it is served, so /healthz and /readyz get no answer while the start seeds, against U5''s rule that the listeners take requests as soon as the config is read'
- file: crates/conductor/src/token.rs
  line: 766
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the empty-pool fallback to model_token applies on every host and source, not only unattended or files, so a workstation instance at placement none that names model_token moves its agents from the harness login to the API key'
- file: crates/conductor/tests/container_start.rs
  line: 1137
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the workflow job test reads no if or continue-on-error key, so a job that never runs or never fails stays green, and the test passes silently where task is not on PATH'
- file: crates/conductor/src/daemon/units.rs
  line: 120
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'systemd-run on PATH counts as a user manager, so a container image holding systemd-run without a running manager would take the unit path and fail every start; the shipped image holds none'
```
