---
format: aep.planning-md/3
id: review-result:w51-u1-adversary-1
kind: review-result
status: active
title: Wave 51 U1 need-repository-from-tag, adversary pass 1
relations:
- reviews: story:need-repository-from-tag
revision: 1
---
```
unit: U1 wave 51, story:need-repository-from-tag, head 44065353f (base 61787f015) plus my uncommitted test additions in <trees>/w51-u1
verdict: NEEDS-CHANGE
cases: executed 1833→1835, red 2
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 3 paths
needs-coordinator: the wave decision (refuse every unknown `<other-repo>`) against the shipped role texts, which have the supervisor answer such NEEDs; you decide which yields
```

`cases` before is the implementor's reported 1833. The package run also had one red that is not mine: `build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger`, the host-load failure brief-common names.

## 1. Diff stat

```
 crates/conductor/tests/mailbox.rs | 29 +++++++++++++++++++++++++++++
 crates/conductor/tests/run.rs     | 32 ++++++++++++++++++++++++++++++++
 2 files changed, 61 insertions(+)
```

Both are test files. No implementation file was touched, and I ran no git write.

## 2. Cases added (both red now, run alone before the suite)

**A. `crates/conductor/tests/run.rs:2335` `adv_a_mailbox_need_for_conductor_itself_is_not_refused_to_its_sender`**

- **Asserts:** with alpha running, `[NEED alpha N-20261017-01] conductor worktree workspace not activated: /srv/example-org` stays in the mailbox and alpha gets no `refused` delivery.
- **Result:** the mailbox-file assertion passed; the refusal assertion is red.

```
panicked at crates/conductor/tests/run.rs:2352:5:
assertion `left == right` failed: the sender is told its NEED for conductor is refused and not kept: {"actions":["relayed"],"lines_handled":0,...} ["conductor run: mailbox 0001.md: the need names \"conductor\", no repository of this instance (none has a running controller or a checkout under checkouts.root), so it is not kept"]
  left: [("ex-alpha", "[REPORT alpha -] refused: the need names \"conductor\", no repository of this instance (none has a running controller or a checkout under checkouts.root), so it is not kept\nYour message: [NEED alpha N-20261017-01] conductor worktree workspace not activated: /srv/example-org\nThe form: [NEED alpha <request-id>] <other-repo> <what>\n")]
 right: []
```

**B. `crates/conductor/tests/mailbox.rs:534` `adv_the_need_the_controller_role_teaches_for_an_unactivated_workspace_is_sent`**

- **Asserts:** the `[NEED conductor …]` line read out of `roles/controller.md` is accepted by `conductor message send` and left in the mailbox.
- **Result:** red.

```
panicked at crates/conductor/tests/mailbox.rs:545:5:
`message send` refuses the line the controller role teaches, "[NEED conductor] worktree workspace not activated: /srv/example-org": exit Some(1)
stderr:
conductor: message send: refused: Unparsed: from "ex-cortex" to "conductor": the first line "[NEED conductor] worktree workspace not activated: /srv/example-org" has none of the forms; the format is on standard output, and nothing was written
```

Run of the cases alone: `Summary [0.659s] 6 tests run: 4 passed, 2 failed, 73 skipped`. The 4 passes were probes, which I removed from the tree afterwards (part 5).

## 3. Suite run (after the cases existed)

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100:

```
    Starting 1835 tests across 151 binaries (12 tests skipped)
     Summary [ 189.748s] 1835 tests run: 1832 passed (3 slow), 3 failed, 12 skipped
        FAIL [  33.198s] ( 391/1835) conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
        FAIL [   0.009s] (1217/1835) conductor-cli::mailbox adv_the_need_the_controller_role_teaches_for_an_unactivated_workspace_is_sent
        FAIL [   0.290s] (1491/1835) conductor-cli::run adv_a_mailbox_need_for_conductor_itself_is_not_refused_to_its_sender
```

The implementor's reported failure `watch::a_following_watch_asleep_between_passes…` passed in this run.

## 4. Findings (cover head 44065353f)

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| F1 | `crates/conductor/src/run/mailbox.rs:372` | NEEDS-CHANGE | introduced | warning |
| F2 | `crates/conductor/roles/controller.md:85` | CONFIRMED | pre-existing | warning |
| F3 | `website/docs/concepts/decisions-and-messages.md:91` | CONFIRMED | introduced | note |

**F1. A NEED meant for the supervisor is now answered "refused … so it is not kept" to its sender.**
- **Measured:** case A.
- **What reaches it:** `controller.md:85` teaches a NEED for `conductor` and says to go on "until it is answered". `supervisor.md:156` has the supervisor answer every `[NEED]` in the mailbox, and `supervisor.md:435-436` names host-tool and sibling-checkout NEEDs as its own. In the form `message send` accepts, such a NEED carries `conductor`, a tool name or an uncloned sibling as `<other-repo>`.
- **Base:** `need()` kept any first word in `pending.json`; the loop told the sender nothing and the supervisor read the mailbox file.
- **Head:** the file still stays in the mailbox, so the supervisor still reads it. This is not a "reaches no one now" regression. What is new is the refusal, which tells the controller its request was dropped while the role texts say it will be answered.
- **Fix, either one:** leave NEEDs for the mailbox's own recipient and host needs unrefused (or word the refusal as "not relayed to a controller"), or change `controller.md:85` and `supervisor.md:435`.
- **One real loss, which the decision accepts:** on base a NEED for a sibling with no checkout was delivered if that sibling was later cloned and its controller started. On head it is not; the sender is told and must resend.

**F2. The one NEED line the controller role teaches literally is refused by `conductor message send`.**
- **Measured:** case B. The line has one tag field; `message.rs:228` requires two.
- **What reaches it:** any controller following `controller.md:85`. `tests/profiles.rs:421` pins the role text as a constant.
- **Origin basis:** not run on the base. `git diff 61787f015 HEAD` is empty for `message.rs` and `roles/`, and `message send` calls nothing the unit touched.

**F3. Three documents still state the old NEED behaviour unconditionally.**
- `website/docs/concepts/decisions-and-messages.md:91` and `docs/design/conductor-run.md:36` say a NEED is delivered or kept for the next start, with no refusal.
- The module doc at `mailbox.rs:6-9` says the sender of a refused message is always told or the answer kept, which the new silent path contradicts.
- No test compares these documents with the code.

**The three "not done" items:**
- **Host-tool or sibling NEED refused:** introduced; this is F1.
- **A refused NEED stays `Received`:** not introduced. On base a kept NEED also stays `Received` (`mark_handled` excludes `Need`, `mailbox.rs:533`), and any refused message returns before `mark_handled`.
- **Leading punctuation gives an empty run:** not a regression. Base kept it forever under the quoted word; head refuses it and names the field. Read at `mailbox.rs:361-375`, not run.

## 5. Attacked and could not break

Four probes were green on head. No committed case covers these lines, and I did not mutate to prove a mutant survives.

- **Grouped repository and paths:** `grp/delta:` and `grp/delta.` are kept under `grp/delta`. `grp/`, `../alpha`, `alpha/../beta`, `/etc`, `..` and `.git` are each refused and nothing is kept under those names.
- **Dot rule:** with checkouts `beta.` and `beta` both present, `beta.` and `beta.:` name `beta.` and `beta:` names `beta`. `gamma..`, `gamma_` and `gamma/` name `gamma`.
- **Running controller, checkout removed:** the NEED is still delivered.
- **Sender with a checkout and no running controller:** the refusal waits in `pending.json` and is delivered on its next start.

By reading only, not run:

- **Silent path of `answer_refused`:** on base such an answer sat in `pending.json` under a name no controller takes. A controller that was running got its refusal on base and still does. The journal carries the reason line (`mailbox.rs:88`); nothing says the sender went unanswered.
- **"Known" by a foreign directory:** any directory with `.git` under the checkouts root counts, which is the same test `spawn` applies.
- **Refusal text bound:** the field is printed without a bound at `mailbox.rs:372-376`, debug-escaped. The older `Your message:` line is equally unbounded, and the refusal goes to the controller of the tag repository as before.
- **Non-ASCII:** the run stops at the first non-ASCII character, so `bêta` is reported as `"b"`.

## 6. Paths written outside the worktree

- `<scratch>/w51/u1/adv1/cases-alone.log`
- `<scratch>/w51/u1/adv1/package.log`
- `<scratch>/w51/u1/adv1/run-rs-with-green-probes.patch` (case A plus the four green probes, for whoever wants them as pins)

The two builds went into the shared target through `conductor build run`. I ran no `worktree hook` lease command.

## 7. Findings block

```findings
- file: crates/conductor/src/run/mailbox.rs
  line: 372
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'a NEED whose other-repo is conductor, a host tool or an uncloned sibling is now answered "refused, not kept" to its sender, while controller.md:85 and supervisor.md:156 and :435 have the supervisor answer it from the mailbox'
- file: crates/conductor/roles/controller.md
  line: 85
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: 'the line the role teaches, "[NEED conductor] worktree workspace not activated: <root>", is refused Unparsed by conductor message send because the NEED form needs two tag fields'
- file: website/docs/concepts/decisions-and-messages.md
  line: 91
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'this row, docs/design/conductor-run.md:36 and the module doc at run/mailbox.rs:6-9 still say a NEED is always delivered or kept and a refused sender always told, which no longer holds for a repository the instance does not know'
```
