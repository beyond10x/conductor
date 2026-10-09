---
format: aep.planning-md/3
id: review-result:adv-w03-u1-pass-2
kind: review-result
status: active
title: Adversary, W03 U1 session names, pass 2
relations:
- reviews: story:instance-session-names
revision: 1
---
unit: W03 U1 `story:instance-session-names`, pass 2. Uncommitted tree `~/.local/state/worktree/trees/b10x/conductor/cw03-u1` (branch `w03/u1-session-names`, base `wave/03` 3914d0e18), correction round 1 applied
verdict: NEEDS-CHANGE
cases: executed 586→590, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 paths under `~/.cache/conductor-dev-analysis/w03/scratch/u1-adv2` (part 6)
needs-coordinator: yes. Finding 1: acceptance says an instance without a prefix "keeps today's guard rules" and also that conductor messages "only sessions of its own instance". Both cannot hold once a prefixed instance sits beside one without a prefix.

**1. Diff stat**

`git diff --stat` against HEAD mixes the implementor's lines with mine, so here are my additions only:

| file | mine |
|---|---|
| `crates/conductor/tests/guard_rules.rs` | +147 lines from :3820: helpers `adv2_*` and 3 tests |
| `crates/conductor/tests/config.rs` | +76 lines from :2018: helper `adv2_session_names` and 1 test |

Both are test files; I changed no implementation file. `rustfmt --check` passes on both files and `cargo clippy -p conductor-cli --all-targets -D warnings` exits 0.

**2. Cases added.** Each was run alone (HOME isolated) before the suite, and all 4 are red now.

| # | case | asserts | red output (verbatim) |
|---|---|---|---|
| 1 | `guard_rules.rs:3956` `adv_w03_u1_p2_the_bare_default_conductor_may_not_message_an_isolated_instance` | The conductor of the default `alpha` (no prefix) is denied `b-ess` and `b-conductor`. `beta` has prefix `b` and its own conductor, so it is isolated. | `verdict: Allow, reason: "conductor's SendMessage is not restricted"` |
| 2 | `guard_rules.rs:3900` `adv_w03_u1_p2_a_ref_to_the_bare_instance_s_b_tools_is_denied_beside_beta_s_own` | Two live sessions are named `b-tools`: beta's controller of `tools`, and alpha's controller of its repo `b-tools`. `b-tools [aaaa1111]` (beta's) is allowed, which passed. `b-tools [bbbb2222]` (alpha's) must be denied. | `target: "b-tools [bbbb2222]", verdict: Allow, reason: "…: a session of instance beta"` |
| 3 | `guard_rules.rs:3929` `adv_w03_u1_p2_a_listed_bare_controller_without_a_pid_is_still_alpha_s` | The list is readable and shows alpha's `b-tools` in `alpha-root/b-tools` as `blocked` with no pid. Beta's conductor must be denied. | `target: "b-tools", verdict: Allow, reason: "SendMessage to \"b-tools\": a session of instance beta"` |
| 4 | `config.rs:2051` `adv_w03_u1_p2_a_served_controller_finds_its_conductor_s_name_in_config_show` | Run with `CONDUCTOR_INSTANCE=gamma`, where `gamma` is served (no conductor role) and alpha has prefix `a`: `config show --format json` must print `a-conductor`, the only name the guard allows. | `config show prints the session names [] and default String("gamma")` |

The red runs for cases 1–3 are logged in `red-guard.log` and `red-guard-2.log`; case 4 is in `red-config.log`.

My first version of cases 2 and 3 put UUID-shaped `sessionId` values in the list. That made the existing hygiene test `no_id_recorded_from_a_live_session_is_left_in_the_guard_s_files` go red. I removed those fields, which the guard does not read, and re-ran the cases alone (`red-guard-2.log`); they are red for the same reasons.

**3. Suite run** (after the cases existed)
`env -u CONDUCTOR_CONFIG -u CONDUCTOR_INSTANCE HOME=<scratch>/home CARGO_HOME=~/.cargo RUSTUP_HOME=~/.rustup CARGO_BUILD_JOBS=8 cargo test -p conductor-cli --no-fail-fast`
- Result: `EXIT=101`, 586 passed, 4 failed (exactly cases 1–4), 3 ignored, `error: 2 targets failed`.
- The before count of 586 is the implementor's correction-round run (`scratch/u1/fix1-full-isolated.log`: 586 passed, 0 failed).

**4. Findings** (they cover the uncommitted tree above)

| # | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| 1 | `crates/conductor/src/guard/rules.rs:896` | NEEDS-CHANGE | introduced | Case 1: a conductor without a prefix may still message an isolated instance's sessions. This contradicts decision 3 and the acceptance line "conductor's only to sessions of its own instance". The implementor's own assertion `guard_rules.rs:3571` (`b-ess` → "not restricted") asserts the opposite of case 1, so one of the two must go. | The migration the acceptance prescribes ("every instance but one gets a prefix") leaves exactly one conductor without a prefix beside isolated ones. One possible fix: when the file has other instances, deny a recipient another isolated instance owns (`session_instance`); a one-instance file stays unrestricted. |
| 2 | `crates/conductor/src/guard/rules.rs:1004` | CONFIRMED | introduced | Case 2: the guard takes the cwd of the first live session with that name and ignores the ` [<ref>]` that picks the recipient. The verdict depends on the list order, and a ref to the other instance's session is allowed. | Decision 1's own case (alpha's repo `b-tools`) plus a beta repo `tools`. Both controllers are then named `b-tools`, and `config validate` cannot refuse that because it knows no repository names. I did not check how the harness maps a ref to an entry. |
| 3 | `crates/conductor/src/guard/rules.rs:1688` | CONFIRMED | introduced | Case 3: `live_sessions` drops entries without a pid, so cwd becomes `None` and the name alone places `b-tools` in beta. The guard is fail-closed when the list cannot be read but fail-open when the list is readable and lacks the recipient's live entry. | `.agents/conductor.md:125` documents rate-limited controllers that `claude agents --json` shows `blocked` with no pid. Whether SendMessage delivers to such a session: I don't know. |
| 4 | `.agents/repo-controller.md:52` | NEEDS-CHANGE | introduced | Case 4: the profile says the conductor's name is printed by `conductor config show`, and for a served instance "its session name is the one to use". For a served instance, `config show` prints no conductor name and prints `default: gamma`. A controller following the profile literally sends to `g-conductor`, which is denied. The denial text names `a-conductor`, so the controller can recover after one denial. | Any controller of a served instance, started with `CONDUCTOR_INSTANCE=<name>` as `.agents/conductor.md:157` says. The `default:` rewrite in `show` predates this unit. Fix: `config show` prints the serving conductor's name, or the profile names another source for it. |

**5. Attacked, could not break**
- `session_instance` placement held in every layout I probed:
  - instances sharing one checkouts root, and roots nested in each other;
  - a controller working in a managed tree (`<trees>/<repo>/<tree>`), including a repo that has no checkout under the root;
  - grouped checkouts `<group>/<repo>` (the derived name `<p>-<group>/<repo>` and rule 1 agree);
  - with prefixes required not to overlap, no name matched two instances.
- Served/isolated messaging:
  - a served controller cannot reach an isolated instance's conductor;
  - a prefixed default conductor is denied the isolated instance's sessions;
  - the socket form is checked against name and cwd.
- Instances without a prefix: no rule denies anything that the base rules allowed.
- Watch and dashboard: they place sessions with the same function as the collector, and I found no instance whose own session they drop. Before this unit the watch already never covered a served instance's checkouts root.
- Profile and Taskfile wording against the binary:
  - `config show` prints `session_prefix: null`, and `roles[].session_name` as `conductor` / `conductor-dev` / `null`, in both text and JSON;
  - the `NAME` jq paths in the Taskfile match those keys;
  - `-n <controller session name>` matches `start-controller --session-name`.
  - `attach` and `sessions` in the Taskfile still use the bare names, but the story assigns those to `instance-scoped-tasks`.

**6. Paths written outside the worktree** (all under `~/.cache/conductor-dev-analysis/w03/scratch/u1-adv2/`)
- `home/` (empty test HOME)
- `probe/conductor.yaml`
- `red-guard.log`, `red-guard-2.log`, `red-config.log`, `suite.log`, `clippy.log`

I started no sessions and left no processes running. I took no worktree lease (the `worktree` CLI shows no lease command), made no commit and did not write the planning store.

**7. Findings block**

```findings
[
  {"file": "crates/conductor/src/guard/rules.rs", "line": 896, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the default conductor without a prefix may still message an isolated instance's sessions and conductor, against decision 3 and the acceptance line 'conductor's only to sessions of its own instance', while guard_rules.rs:3571 asserts the opposite"},
  {"file": "crates/conductor/src/guard/rules.rs", "line": 1004, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "a prefixed conductor's recipient check takes the cwd of the first live session with the name and ignores the [ref], so a ref to the bare instance's same-named controller is allowed"},
  {"file": "crates/conductor/src/guard/rules.rs", "line": 1688, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "a listed recipient without a pid loses its cwd and is placed by name alone, so the guard fails open on a readable list where it fails closed on an unreadable one"},
  {"file": ".agents/repo-controller.md", "line": 52, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a served instance's controller is told to read its conductor's session name from conductor config show, which prints no conductor name for that instance and prints itself as default"}
]
```
