---
format: aep.planning-md/3
id: review-result:w50-u1-adversary-2
kind: review-result
status: active
title: Wave 50 U1 deliver-reaches-bg-session, adversary pass 2
relations:
- reviews: story:deliver-reaches-bg-session
revision: 1
---
Adversary pass 2 on `story:deliver-reaches-bg-session`, commit 2cc42f66c on base 4d39c3c5c.

Verdict: NEEDS-CHANGE. Seven findings, all introduced by the correction.

Cases added in `crates/conductor/tests/adv2_deliver_attach.rs` (real tmux on sockets under each
case's own directory, a stand-in client), all red on their first run:

- `a_live_background_session_with_a_name_a_launcher_socket_holds_is_reached`
- `a_delivery_the_attach_client_read_before_it_ended_is_not_answered_refused`
- `a_delivery_nobody_read_on_a_terminal_that_reports_no_input_is_not_answered_delivered`
- `a_delivery_ended_by_a_signal_while_it_waits_leaves_no_client`

Suite after the cases (deliver, deliver_attach, adv_deliver_attach, adv2_deliver_attach, run,
isolated_home, no_organization): `84 tests run: 80 passed, 4 failed, 0 skipped`.

Findings:

- G1 (warning, `session.rs:502`): the delivery socket name is up to 15 bytes longer than the
  launcher's, so a live session named with 93 or more characters is refused on every delivery.
- G2 (warning, stand-in only, `session.rs:585`): a client that read the text and ended is refused
  `SessionNotReachable`, which the loop offers again every tick, so the text is typed again.
- G3 (warning, `run/handlers.rs:922`): no bound on offering a kept item again.
- G4 (warning, read not run, `session.rs:1653`): the 60 s deliverer bound is below the attach
  route's own worst case; a timeout marks the item refused by a controller that stays Running.
- G5 (note, stand-in only, `session.rs:602`): a terminal that reports no readable input counts as read.
- G6 (note, `session.rs:542`): a delivery ended by SIGTERM leaves its pane and client.
- G7 (note, `session.rs:575`): no case reaches two of the refusals.

Attacked and not broken: a dash or dot in the name; the count per process; a launcher is never
ended; a dead launcher socket lists nothing as live; a pane that dies after Enter is refused;
multi-line text arrives whole; the renamed case gained assertions; a fresh model generation equals
the committed one byte for byte; the help texts match the code. Not attacked: the real harness, a
boxed session, the in-process agent API under threads.

```findings
- file: crates/conductor/src/session.rs
  line: 502
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: '"the delivery socket name adds up to 15 bytes to the session name, so a live session named with 93 or more characters, which a launcher socket holds, is refused File name too long on every delivery"'
- file: crates/conductor/src/session.rs
  line: 585
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: '"a client that read the text and ended is refused SessionNotReachable, which the loop reads for a live session as nothing typed and offers again every tick; shown only with a stand-in client"'
- file: crates/conductor/src/run/handlers.rs
  line: 922
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: '"a kept item whose refusal leaves the controller Running is offered again on every tick with no bound, each attempt up to 30 s and one journal line"'
- file: crates/conductor/src/session.rs
  line: 1653
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: '"the 60 s deliverer bound is below the attach route''s own worst case, and a timeout is no SessionNotReachable, so the item is marked refused_by a controller that stays Running and nobody delivers it; read, not run"'
- file: crates/conductor/src/session.rs
  line: 602
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: '"read_from takes a terminal that reports no readable input as read, so a client that never reads on a terminal where carriage return ends no line is answered delivered; stand-in only"'
- file: crates/conductor/src/session.rs
  line: 542
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"a delivery ended by SIGTERM while it waits leaves its pane and claude attach client until some later background delivery in the same state directory ends them"'
- file: crates/conductor/src/session.rs
  line: 575
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"no case reaches the refusals drew nothing within 20 s and its terminal closed before it was seen to read the text at line 622"'
```
