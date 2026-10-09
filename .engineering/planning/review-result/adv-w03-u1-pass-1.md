---
format: aep.planning-md/3
id: review-result:adv-w03-u1-pass-1
kind: review-result
status: active
title: Adversary, W03 U1 session names, pass 1
relations:
- reviews: story:instance-session-names
revision: 1
---
unit: W03 U1 `story:instance-session-names`, uncommitted working tree `w03/u1-session-names` (base `wave/03` 3914d0e18) at `~/.local/state/worktree/trees/b10x/conductor/cw03-u1`
verdict: NEEDS-CHANGE
cases: executed 578→582, red 4
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 10 paths under `~/.cache/conductor-dev-analysis/w03/scratch/u1-adv` (part 6)
needs-coordinator: yes. The operator's live config cannot take this unit as built, and one conductor running several instances (`.agents/conductor.md` § More than one instance) contradicts the guard rule. Findings 6 and 7.

**1. Diff stat (my additions only; the rest of the tree diff is the implementor's)**

| file | change |
|---|---|
| `crates/conductor/tests/adv_w03_u1_watch.rs` | new, 1 test |
| `crates/conductor/tests/config.rs` | +24 lines, 1 test |
| `crates/conductor/tests/guard_rules.rs` | +44 lines, 1 test |
| `crates/conductor/tests/instance_sources.rs` | +45 lines, 1 test |

All four are test files. I changed no implementation file. `rustfmt --check` is clean on all four.

**2. Cases added. Each was run alone first, with HOME isolated; all four are red now.**

| # | case | asserts | red output (verbatim excerpt) |
|---|---|---|---|
| 1 | `config.rs:2005` `adv_w03_u1_a_prefix_the_bare_instance_s_role_names_carry_is_refused` | `config validate` refuses `session_prefix: conductor` beside an instance with no prefix. That instance's `conductor-dev` carries the prefix `conductor`. | `left: Some(0) right: Some(1)` … `valid, 2 instance(s): alpha, beta` |
| 2 | `instance_sources.rs:1477` `adv_w03_u1_the_bare_instance_s_own_controller_is_not_redacted_by_another_prefix` | The unprefixed instance's own controller `b-tools`, in `~/src/b-tools`, is still placed when another instance has prefix `b`. | `left: [{"cwd":"","name":Null,"repository":Null,"session_ref":""…}]` `right: [{"cwd":"~/src/b-tools","name":"b-tools","repository":"b-tools"…}]` |
| 3 | `guard_rules.rs:3584` `adv_w03_u1_the_recipient_the_controller_profile_names_is_allowed_under_a_prefix` | The recipient that `.agents/repo-controller.md` names ("Messages go only to \`conductor\`") is allowed for a controller of an instance with a prefix. | `verdict: Deny, reason: "SendMessage to \"conductor\": a controller messages only a-conductor"` |
| 4 | `adv_w03_u1_watch.rs:42` `adv_w03_u1_another_instance_s_conductor_dev_is_not_labelled_conductor` | When `b-conductor-dev` works in alpha's `<root>/conductor`, alpha's watch does not print `context: conductor`. | `beta's conductor-dev hands alpha's conductor over: ["context: conductor 352k tokens (session c3c3c3c3)"]` |

Logs: `red-config.log`, `red-sources.log`, `red-guard.log`, `red-watch.log` in the scratch directory.

**3. Suite run** (after the cases existed). Command: `env -u CONDUCTOR_CONFIG -u CONDUCTOR_INSTANCE HOME=<scratch>/home CARGO_HOME=~/.cargo RUSTUP_HOME=~/.rustup CARGO_BUILD_JOBS=8 cargo test -p conductor-cli --no-fail-fast`
- Result: `EXIT=101`, 578 passed, 4 failed (exactly the 4 cases above), 3 ignored, `error: 4 targets failed`.
- Before count: the same run with `-- --skip adv_w03_u1` (my 4 cases deselected) gave `EXIT=0`, 578 executed.

**4. Findings**

| # | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| 1 | `crates/conductor/src/collect/sessions.rs:307` | NEEDS-CHANGE | introduced | Case 2: the unprefixed instance's own controller is redacted. The guard has the same gap: beta's conductor may message alpha's `b-tools`, and if alpha has a repository `b-conductor`, its controller gets beta's conductor name. | Spec: one unprefixed instance may sit beside prefixed ones. That is the mixed state the operator lands in when migrating two unprefixed instances. Reached by any repository of the unprefixed instance named `<other prefix>-…`. |
| 2 | `crates/conductor/src/watch.rs:862` | NEEDS-CHANGE | introduced | Case 4: another instance's conductor-dev is labelled `conductor`, and the profile tells conductor to hand itself over on that line. On base, the same session (named `conductor-dev`) read `context: conductor-dev`. The watch places sessions by directory only; the collector now places by name first. | `dev:start` has no `cd`, so every instance's conductor-dev runs in the one conductor checkout. That checkout is under the root of whichever instance owns the conductor repository. |
| 3 | `.agents/repo-controller.md:48` | NEEDS-CHANGE | introduced | Case 3: a controller that follows its profile has every message denied under a prefix. | Every controller of an instance with a `session_prefix`. This file is outside the story's scope list, but the unit's guard change caused the drift. |
| 4 | `crates/conductor/src/config.rs:1112` | INFEASIBLE | introduced | Case 1: `distinct_prefixes` compares prefixes only, not prefixes against the fixed names the unprefixed instance keeps. | Nothing found that sets `session_prefix: conductor`. I built the case; it is a smaller finding. |
| 5 | `crates/conductor/tests/watch.rs:595` | CONFIRMED | introduced | The existing test `conductor_dev_in_conductor_s_directory_is_labelled_by_its_role_name` goes red when HOME holds a valid config with `session_prefix: a`: `left: ["context: conductor 352k…"]`. The in-process watch tests read the real home's config through `config::active()`. | Any developer whose live config has a prefix, which this unit pushes the operator towards. Log: `probe-watch-home.log`. |
| 6 | `crates/conductor/src/config.rs:1112` | NEEDS-CHANGE | introduced | The brief states the operator's live config has two instances without a prefix, and this unit refuses that. Every non-guard command then stops, and the guard falls back to the built-in instance's places (`cli.rs:88`). Acceptance says "installing this changes nothing for a running instance". | I did not read the operator's live config. The two-instance state is taken from the brief. Needs a decision: migrate the config together with the install, or make the refusal a warning first. |
| 7 | `.agents/conductor.md:151` | NEEDS-CHANGE | introduced | § More than one instance tells one conductor to start and message another instance's controllers (`CONDUCTOR_INSTANCE=<name>`). Under the new rule (the implementor's own test `conductor_messages_only_its_own_instance_s_sessions`), conductor is denied `b-*`. The other instance's controllers may message only `b-conductor`, which does not exist. | The common rules describe one conductor on this host while the operator's config names two instances. That is this setup. Needs a decision: one conductor per instance, or a guard rule for grouped instances. |

Covers the uncommitted tree above, on base `wave/03` 3914d0e18.

**5. Attacked and could not break**
- Controller rules: `a-conductor-dev`, `a-other`, a socket of `b-conductor`, and the unprefixed `conductor` are all denied under prefix `a`.
- Prefix overlap: `a` beside `a-b` is refused, and `ab-x` does not carry `a`. With every instance prefixed, no name carries two prefixes.
- Conductor's `recipient` must match `to`; a socket to another instance's pid is denied.
- A written role `session_name` that differs from the derived one, a controller `session_name`, an empty prefix, a prefix with a leading or trailing `-`, and non-ASCII characters are all refused.
- An explicit `session_prefix: null` is read as absent (`Reader::get`).
- Dashboard controller count: another prefix's session in this instance's checkout is not counted.
- Taskfile `conductor:start`, `restart` and `dev:start`: `NAME` is shell-quoted. `config show` prints only the active instance, so `.instances[0]` is correct.
- `start-controller --session-name`: only the derived name is accepted.

**6. Paths written outside the worktree** (all under `~/.cache/conductor-dev-analysis/w03/scratch/u1-adv/`)
- `home/` (empty test HOME)
- `probe-home/.b10x/conductor/conductor.yaml`
- `red-config.log`, `red-sources.log`, `red-guard.log`, `red-watch.log`
- `probe-watch-home.log`, `suite.log`, `suite-deselected.log`

I started no sessions and left no processes running. I did not acquire a worktree session lease. I did not commit or write the planning store.

**7. Findings block**

```findings
[
  {"file": "crates/conductor/src/collect/sessions.rs", "line": 307, "category": "boundary", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "beside a prefixed instance, the instance without a prefix has its own controller redacted when its repository name starts with the other prefix and '-', and the guard lets the other conductor message it"},
  {"file": "crates/conductor/src/watch.rs", "line": 862, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "another instance's conductor-dev working in the shared conductor checkout is labelled `context: conductor`, which tells this instance's conductor to hand itself over"},
  {"file": ".agents/repo-controller.md", "line": 48, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the controller profile still sends every message to bare `conductor`, which the guard now denies for a controller of a prefixed instance"},
  {"file": "crates/conductor/src/config.rs", "line": 1112, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "config validate accepts session_prefix `conductor` beside an instance without a prefix, whose fixed name `conductor-dev` then carries that prefix"},
  {"file": "crates/conductor/tests/watch.rs", "line": 595, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the in-process watch pass tests read the real home's config through config::active(), and go red once that config has a session_prefix"},
  {"file": "crates/conductor/src/config.rs", "line": 1112, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "refusing two instances without a prefix stops every non-guard command on the operator's live config, against the acceptance line that installing changes nothing for a running instance"},
  {"file": ".agents/conductor.md", "line": 151, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the profile's section on running several instances from one conductor contradicts the guard, which denies that conductor's messages to the other instance's controllers and theirs back to it"}
]
```
