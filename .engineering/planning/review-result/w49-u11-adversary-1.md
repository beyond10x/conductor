---
format: aep.planning-md/3
id: review-result:w49-u11-adversary-1
kind: review-result
status: active
title: Wave 49 U11 deliver-reaches-bg-session, adversary pass 1
relations:
- reviews: story:deliver-reaches-bg-session
revision: 4
---
unit: U11 story:deliver-reaches-bg-session, commit 87a3ef0d1 on base 3a27c6469, worktree ~/.local/state/worktree/trees/b10x/conductor/w49-u11
verdict: NEEDS-CHANGE
cases: executed 1651→1655, red 3
origin: introduced 4 / pre-existing 0 / undecided 2
wrote-outside-worktree: 7 paths
needs-coordinator: whether F2 (a kept answer is never delivered to the controller that stays Running) holds the unit or goes to the backlog

## 1. git --no-pager diff --stat

```
 crates/conductor/tests/run.rs | 29 +++++++++++++++++++++++++++++
 1 file changed, 29 insertions(+)
?? crates/conductor/tests/adv_deliver_attach.rs   (untracked, new test file)
```

Test files only. Nothing committed.

## 2. Cases added

Run alone first (`red1.log`, 4 cases: 1 passed, 3 failed), before any suite run.

| case | asserts | now |
|---|---|---|
| `tests/adv_deliver_attach.rs::one_delivery_reaches_the_attach_client_whole_and_leaves_nothing` | real tmux, stand-in `claude attach`: text with quotes, `$(id)`, backticks, `Enter`, `C-c`, `#{pane_id}`, a line starting with `-n --help` arrives byte for byte; no client and no socket left | green |
| `tests/adv_deliver_attach.rs::two_deliveries_at_once_to_one_background_session_both_arrive_and_leave_nothing` | two `session deliver` to one live background session at once, 4 rounds: none refused, none lost, no `claude attach` left running | red |
| `tests/adv_deliver_attach.rs::a_delivery_the_attach_client_never_read_is_not_answered_delivered` | attach client prints one line, takes input 3 s later: a delivery the session never read does not exit 0 | red |
| `tests/run.rs::adv_a_decision_refused_once_for_a_live_session_reaches_it_when_it_takes_text_again` | a `[DECISION]` refused once for a live session is delivered to that Running controller on a later tick, once the session takes text | red |

Red output, verbatim, from the run of these cases alone:

```
thread 'two_deliveries_at_once_to_one_background_session_both_arrive_and_leave_nothing' panicked at crates/conductor/tests/adv_deliver_attach.rs:223:5:
answered delivered but never read by the session: []
`claude attach` clients still running after every delivery ended: ["1787989"]
refused although the session was live: [
    "[REPORT cortex -] round 0 from the loop: conductor: session deliver: refused: SessionNotReachable: session ex-cortex: its launcher socket <state>/sessions/ex-cortex.tmux is absent; `claude attach cc062faf` in a pane of its own did not take it: `tmux -S ex-cortex.tmux -f /dev/null new-session -d -s ex-cortex -x 160 -y 50 -- claude attach cc062faf` exited 1: duplicate session: ex-cortex; nothing was recorded, and it is not tried again",
    (rounds 1 and 2: the same)
    "[REPORT cortex -] round 3 from the loop: ... `tmux -S ex-cortex.tmux load-buffer -b conductor <state>/sessions/ex-cortex.paste` exited 1: error connecting to ex-cortex.tmux (No such file or directory); ...",
    "[REPORT cortex -] round 3 from the operator: ... `tmux -S ex-cortex.tmux capture-pane -p -t =ex-cortex:` exited 1: error connecting to ex-cortex.tmux (No such file or directory); ...",
]
received:
[REPORT cortex -] round 0 from the operator
[REPORT cortex -] round 1 from the operator
[REPORT cortex -] round 2 from the operator
```

(`<state>` stands for the case's directory under the shared target's `tmp/adv-deliver-attach/at-once/`; the full paths are in `adv/red1.log`.)

```
thread 'a_delivery_the_attach_client_never_read_is_not_answered_delivered' panicked at crates/conductor/tests/adv_deliver_attach.rs:243:5:
answered delivered, yet the session read ""
exit Some(0)
stdout:
01a1c3b4-ef61-736e-8f6f-bc523c7e3b50
```

```
thread 'adv_a_decision_refused_once_for_a_live_session_reaches_it_when_it_takes_text_again' panicked at crates/conductor/tests/run.rs:1928:5:
assertion `left == right` failed: the Running controller of the live session gets its answer: [three ticks, each "actions": []]
  left: []
 right: [("ex-alpha", "[DECISION R-1] A one migration, then one layout (<decision id>)")]
```

After that run I renamed the unused field `scratch` to `_scratch` in `adv_deliver_attach.rs` (a compiler warning, no behaviour); the panic lines moved by three (223→226, 243→246).

## 3. Suite run, after the cases existed

Command: `conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast` (log `adv/suite.log`).

```
    Starting 1655 tests across 141 binaries (7 tests skipped)
     Summary [ 124.457s] 1655 tests run: 1650 passed (2 slow), 5 failed, 7 skipped
        FAIL conductor-cli::adv_deliver_attach a_delivery_the_attach_client_never_read_is_not_answered_delivered
        FAIL conductor-cli::build_queue_convoy waiting_runs_take_no_lock_while_the_ledger_does_not_change
        FAIL conductor-cli::adv_deliver_attach two_deliveries_at_once_to_one_background_session_both_arrive_and_leave_nothing
        FAIL conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
        FAIL conductor-cli::run adv_a_decision_refused_once_for_a_live_session_reaches_it_when_it_takes_text_again
error: test run failed
exit 100
```

- My three cases are red again. In the suite run the concurrency case left two `claude attach` clients running, not one.
- `build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger`: the failure the brief names as failing on the base.
- `build_queue_convoy::waiting_runs_take_no_lock_while_the_ledger_does_not_change` (`tests/build_queue_convoy.rs:297`): failed once here, was not in the implementor's gate failures, and I did not rerun it alone. The unit touches nothing it reads. Not classified.
- `guard_hook::hooks_running_at_once_each_answer_and_each_denial_is_recorded`: passed alone (`adv/guard_hook.log`, 1 run, 0.138 s) and passed in the suite run. One pass alone does not prove it stable; it is consistent with the 2 s store lock timeout under load, and the unit touches no guard or store code.

`<before>` 1651 is the implementor's gate summary (`u11/gate.log`: "1651 tests run").

## 4. Findings

Covers commit 87a3ef0d1.

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| F1 | `crates/conductor/src/session.rs:471` (`type_through_attach`) | Two deliveries to one live background session at once: in 4 rounds, 5 of 8 were refused `SessionNotReachable` (round 3: both, so that text reached nobody), and one or two `claude attach` clients were left running with their socket unlinked. Both callers use the socket path `<name>.tmux` and the tmux session name `<name>`; the second removes the first's socket (`fs::remove_file` at :474) or gets `duplicate session`, and each `kill-server` ends whichever server is on the path. | Two processes delivering to one session: the `conductor run` loop's `ShellDeliverer` and an operator's or conductor-dev's `session deliver`, or the agent API (`src/agent/claude_code.rs:211`). The window is each delivery's whole life, at least 1.5 s. The loop itself delivers one at a time. With F2, a loop delivery refused this way is never retried. | NEEDS-CHANGE | introduced |
| F2 | `crates/conductor/src/run/handlers.rs:895` (`send`) with `:990` (`made`) | A kept delivery (`[DECISION]`, `[NEED]` or `[RESOURCE]` answer) refused once for a live session is marked `refused_by` its controller, and `made` never tries that controller again. The controller now stays Running, so no next controller starts, and three later ticks delivered nothing although the session took text again. On the base the controller was recorded stopped and the item went to the next start. The unit's own case asserts "the same controller is not asked twice" and stops there. | Any refusal for a live session: F1's race, `claude attach` drawing nothing in 20 s, a launcher that answers and does not take the text (`reach`'s "left alone" branch), a `tmux` that fails once. The reporter's `[DECISION-REQUEST]` case is this path. The item is delivered only after something else restarts the controller. | NEEDS-CHANGE | introduced |
| F3 | `crates/conductor/src/session.rs:503` (`type_once_drawn`) | The pane counts as ready once it shows any character; 0.5 s later the text is pasted, and 0.5 s after Enter the server is killed. A client that draws a first line and reads input 3 s later never reads the text; the command exits 0 and records `delivered`. | Nothing found that I could show: it needs a `claude attach` that draws before it takes input for more than about 1 s, and I may not run the real harness. The implementor's probe (`u11/probe2.sh`) passed once on an idle session. The stand-in built the state. | INFEASIBLE | introduced |
| F4 | `crates/conductor/src/session.rs:435` and `crates/conductor/src/run/handlers.rs:903` | No case names two branches: the id that starts with `-` or holds whitespace (`names its live session by no id to attach to`), and `send`'s `Err(unread)` (the list cannot be read, controller stays Running). `grep` for "was not read" and "no id to attach" in `tests/` finds nothing. Dropping either branch leaves the suite green by reading; I did not build the mutant. | The first: a list entry with no `id` and no `sessionId` yields the id `with no id`. The second: a `claude` that is missing or exits non-zero while the loop runs. | CONFIRMED | introduced |
| F5 | `spec/domains/session.yaml:424` and `:549` | `SessionNotReachable` and the `unreachable` outcome still say "no tmux server answers on the session's launcher socket"; the refusal now also covers the session list and the attach route. The commit message says this was left out. AGENTS.md rule 1 is spec first. | Every reader of the specification and the generated help. | CONFIRMED | undecided |
| F6 | `crates/conductor/tests/build_queue_convoy.rs:297` | Failed once in my suite run; not rerun, not in the implementor's gate. | Host load suspected, not shown. | CONFIRMED | undecided |

F5 is `undecided` because the text predates the unit but became wrong through it; I leave that call to the coordinator. F6 is `undecided` because I did not run it on the base.

Fix named, not applied, for F1: one delivery at a time per session (a lock file beside the socket), or a socket and tmux session name of the delivery's own (`<name>.deliver.<pid>.tmux`), which also keeps the watch (`src/watch.rs:489`) and `session attach` (`src/attach.rs:183`) from reading a delivery pane as a launcher. For F2: a refusal that leaves the controller Running should not set `refused_by`, or should clear it after some ticks.

## 5. Attacked, not broken

- Text with quotes, `$()`, backticks, tmux key names, format strings, a leading dash, several lines: arrives whole through a real tmux.
- Session id with shell characters (`x;touch touched;$(touch touched)`): tmux runs `claude attach <id>` as argv, no shell (`adv/idprobe.sh`).
- Session id with a leading dash or whitespace, or no id: refused before tmux is called (read, not run).
- Error paths of one delivery alone: `kill-server` and socket removal run whether typing succeeded or not; a failed `new-session` leaves no server.
- A dead controller staying Running forever: when the list is readable and the session has no pid, the next refused delivery records it stopped, and the watch's `session gone` line restarts through `restart_with`, which stops the held controller. With the list unreadable, `spawn` refuses `SessionsUnread` anyway.
- The two older `run.rs` cases: the added `case.live(alpha, FIRST, false)` is needed for their `Stopped` assertion under the new rule; no assertion was removed.
- Not attacked: a boxed session, a `claude agents --json` that is slow (bound 120 s inside a loop tick) or lists two live sessions of one name (the first wins).

## 6. Paths written outside the worktree

- ~/.cache/scratch/<session>/w49/u11/adv/red1.log
- ~/.cache/scratch/<session>/w49/u11/adv/guard_hook.log
- ~/.cache/scratch/<session>/w49/u11/adv/suite.log
- ~/.cache/scratch/<session>/w49/u11/adv/run_case.rs
- ~/.cache/scratch/<session>/w49/u11/adv/insert.py
- ~/.cache/scratch/<session>/w49/u11/adv/idprobe.sh and the directory adv/idprobe/ beside it
- ~/.cache/scratch/<session>/w49/u11/adv-report-1.md

Failed cases keep their directories under the shared target: `~/.cache/b10x/conductor/beyond10x/target/conductor-362621f27d1a30dd/tmp/adv-deliver-attach/{at-once,slow}` and `tmp/run/adv-unreachable-live-again` (the `run.rs` case's own, by its fixture). No `claude attach` stand-in or tmux server of a case is still running. Worktree lease `adv-w49-u11-p1` released.

## 7. findings

```findings
- file: crates/conductor/src/session.rs
  line: 471
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: two deliveries at once to one background session share one socket path and tmux session name, so one is refused or both are, and a claude attach client is left running on an unlinked socket
- file: crates/conductor/src/run/handlers.rs
  line: 895
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a kept delivery refused once for a live session is marked refused_by a controller that now stays Running, so it is never delivered until something else restarts that controller
- file: crates/conductor/src/session.rs
  line: 503
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the pane is typed into 0.5 s after its first character and killed 0.5 s after Enter, so a client that draws before it reads input loses the text while the delivery is recorded; shown only with a stand-in client
- file: crates/conductor/src/session.rs
  line: 435
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no case reaches the refusal of an id with a leading dash or whitespace, nor the unread-list branch of send at handlers.rs:903
- file: spec/domains/session.yaml
  line: 424
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: SessionNotReachable and the unreachable outcome still describe the tmux launcher only, while the refusal now also covers the session list and the attach route
- file: crates/conductor/tests/build_queue_convoy.rs
  line: 297
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: waiting_runs_take_no_lock_while_the_ledger_does_not_change failed once in the suite run and was neither rerun alone nor run on the base
```
