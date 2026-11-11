---
format: aep.planning-md/3
id: review-result:w51-u1-adversary-2
kind: review-result
status: active
title: Wave 51 U1 need-repository-from-tag, adversary pass 2
relations:
- reviews: story:need-repository-from-tag
revision: 1
---
```
unit: U1 wave 51, story:need-repository-from-tag, head c697a81aa plus my uncommitted test additions in <trees>/w51-u1
verdict: CONFIRMED
cases: executed 1834→1837, red 0
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: decision 1 says a NEED for `conductor` puts nothing in `pending.json`, but when the instance has a checkout named `conductor` the code relays the NEED to it. Which one wins (finding F4)?
```

I found nothing that goes red against the correction. I added three cases. They pass on head, and each one fails under a code change (a mutant) that the current suite lets through.

## 1. Diff stat

```
 crates/conductor/tests/run.rs | 75 +++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 75 insertions(+)
```

The only path is a test file. I ran no git write and no `worktree` lease command.

## 2. Cases added (all green on head; each catches a mutant the suite misses)

All three are in `crates/conductor/tests/run.rs`, after pass 1's `adv_a_mailbox_need_for_conductor_itself_is_not_refused_to_its_sender`:

- **`adv2_a_refused_report_or_handover_still_reads_refused`:** a refused `[HANDOVER alpha] context 210k` (no open dispatch) and a refused `[REPORT alpha <id>] unblocked` both start `[REPORT alpha -] refused: `.
- **`adv2_a_need_for_conductor_with_a_dot_is_the_supervisor_s_and_punctuation_is_named`:** `conductor. Activate…` gets no answer and nothing is kept. `-- export the store id` is answered `not relayed: the need names "--", `.
- **`adv2_a_need_for_a_checkout_named_conductor_is_kept_for_it`:** with a checkout `conductor` and no controller, the NEED for `conductor` is kept under `conductor`, and the sender gets no delivery.

Run of the three alone:

```
Summary [   2.887s] 3 tests run: 3 passed, 59 skipped
```

## 3. Package run (after the cases existed)

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100:

```
    Starting 1837 tests across 151 binaries (12 tests skipped)
     Summary [ 198.665s] 1837 tests run: 1836 passed (4 slow), 1 failed, 12 skipped
        FAIL [  31.428s] ( 639/1837) conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
```

The one failure is the host-load test that brief-common names. It is not mine.

## 4. Findings (cover c697a81aa)

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| F1 | `crates/conductor/src/run/mailbox.rs:181` | CONFIRMED | introduced | note |
| F2 | `crates/conductor/src/run/mailbox.rs:410` | CONFIRMED | introduced | note |
| F3 | `crates/conductor/src/run/mailbox.rs:413` | CONFIRMED | introduced | note |
| F4 | `crates/conductor/src/run/mailbox.rs:404` | CONFIRMED | pre-existing | note |

**F1. Nothing checks that other refusals still say "refused".**
- **The mutant:** `error.is::<NotRelayed>()` replaced by `true`, so every refused REPORT, HANDOVER, DECISION-REQUEST and RESOURCE reads `not relayed:`.
- **Why it survives:** the refused-REPORT tests (`run.rs` near 2653 and 2697) check `DispatchStateConflict`, the message and the form, never the word after `-]`. A grep of the tests finds no assertion on `] refused:`.
- **Pinned by:** case 1. On the base the word was a fixed literal, so there was nothing to mutate.

**F2. Two changes to the `conductor` check survive.**
- **First mutant:** `name == RECIPIENT` replaced by `run == RECIPIENT`. It survives because the test only uses `conductor:`, where `run` and `name` are the same.
- **Second mutant:** the RECIPIENT check moved above the `known` checks. It survives because no test has a checkout named `conductor`, so the implementor's claim "a real `conductor` repository still takes the NEED" is not pinned.
- **Pinned by:** cases 2 and 3.

**F3. A field of punctuation only can be printed as `""` and nothing catches it.**
- **The mutant:** `read = if name.is_empty() { field } else { name }` replaced by `name`.
- **Why it survives:** no test sends such a field.
- **Pinned by:** case 2. `message send` accepts `[NEED alpha N-1] -- export …`, so a controller can send it.

**F4. A checkout named `conductor` gets the NEED, against decision 1 as worded.**
- **What happens:** a NEED meant for the supervisor (`controller.md:85`, worktree workspace not activated) is delivered to that repository's controller, or kept in `pending.json` for its next start. Case 3 measures this; the code is `mailbox.rs:404-412`.
- **The supervisor still gets it:** the file stays in the mailbox.
- **What reaches it:** an instance whose checkouts root holds a checkout named exactly `conductor`. I did not show that any instance has one.
- **Origin:** the same as on the base, which kept every NEED under its first word.
- **Your call:** case 3 pins the implementor's choice. If decision 1 should win instead, invert that case.

## 5. Attacked and could not break

- **Answered once:** `mailbox.handled` names each file after `act`. A not-relayed NEED gets one answer and one journal line, and the `conductor` NEED (`Ok(false)`) gets neither, on later ticks too. The file stays, as on the base.
- **Endings and groups:** `conductor:`, `conductor.`, `conductor/` and `conductor-` are the supervisor's. `group/conductor`, `conductors` and `-conductor` get not relayed. That is correct, since none of them is the recipient.
- **Different case:** `Conductor:` is answered "not relayed … stays in conductor's mailbox, where the supervisor reads it". That is true, and decision 1 names lowercase `conductor` only.
- **Truth of the not-relayed line:** checked for a prose word, a sibling with no checkout, `--`, `...` and non-ASCII. It names the field as written and is one line, quoted with `{:?}`.
- **Empty field:** the answer says `refused: the need names no repository`, as on the base. `message send` cannot produce it, because `message.rs:157` needs at least two words of rest.
- **Other kinds:** only `need()` produces `NotRelayed`, with no context wrapped around it. REPORT, HANDOVER, DECISION-REQUEST and RESOURCE get the same wording and recipients as on the base.
- **Texts against code:** these match the code: the module doc table row (`mailbox.rs:17`), the doc comments at `:153-158` and `:346-352`, `conductor-run.md:36` and `:96-99`, `decisions-and-messages.md:91` and `controller.md:68` (120 lines, at its cap). `supervisor.md:156` and `:435` still hold, because the file stays in the mailbox.

## 6. Paths written outside the worktree

- `<scratch>/w51/u1/adv2/cases-alone.log`
- `<scratch>/w51/u1/adv2/package.log`

The two builds went into the shared target through `conductor build run`.

## 7. Findings block

```findings
- file: crates/conductor/src/run/mailbox.rs
  line: 181
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'no test asserts that a refused REPORT or HANDOVER answer starts "refused:", so replacing the NotRelayed check with true survives the suite; adv2_a_refused_report_or_handover_still_reads_refused pins it'
- file: crates/conductor/src/run/mailbox.rs
  line: 410
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'comparing run instead of name to RECIPIENT, or moving the RECIPIENT check above the known checks, survives because the suite only sends "conductor:" and has no checkout named conductor; two adv2 cases pin both'
- file: crates/conductor/src/run/mailbox.rs
  line: 413
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'printing the trimmed name instead of the field for a punctuation-only field (an empty quoted name) survives the suite; adv2_a_need_for_conductor_with_a_dot_is_the_supervisor_s_and_punctuation_is_named pins it'
- file: crates/conductor/src/run/mailbox.rs
  line: 404
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'with a checkout named conductor, a NEED for conductor is relayed or kept in pending.json for that repository instead of being left only for the supervisor as decision 1 words it'
```
