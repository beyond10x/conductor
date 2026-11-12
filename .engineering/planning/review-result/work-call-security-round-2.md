---
format: aep.planning-md/3
id: review-result:work-call-security-round-2
kind: review-result
status: active
title: 'The work call over HTTP, security review round 2: the fix of round 1'
relations:
- reviews: story:control-plane-work-handler
revision: 1
---
needs-change

Recorder's note: the reviewer's report as returned on 2026-11-12, recorded by the coordinator; scratch paths are replaced. The two introduced notes go back to the implementor; the pre-existing warning becomes a backlog draft and its case is kept, ignored, under that draft's name.

unit: round 2 security review of the work call fix, working tree of `feat/work-call` against `709e7847e` (not committed; head is still `709e7847e`)
verdict: CONFIRMED (3 failing cases: 1 warning, 2 notes; none blocks)
cases: executed 12→17, red 3 (12 is the same run with my 5 cases left out; there was no separate run without them)
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 6 paths (part 6)
needs-coordinator: whether a report line must be tied to the session that sent it (finding 1)

1. `git diff --stat` shows only the implementor's files, all already modified when I started. My only change is in the untracked `crates/conductor/tests/sec_work_route.rs`: I added 5 cases after line 428. Lines 1 to 428 are byte-identical to the copy I took before editing. No implementation file and nothing under `.engineering/` was touched.

2. Cases, first run on their own (`cases.log`, exit 100, `5 tests run: 2 passed, 3 failed`):
   - `a_report_naming_a_lookalike_repository_moves_no_dispatch_of_another`: green. Lines tried: `Beta`, `BETA -`, prefix and suffix names (`beta` naming `beta-2`'s dispatch and the reverse, `bet`), and `./beta`, `beta/`, `example-org/beta`. A line with two ids is already refused by `message send`.
   - `a_refused_handle_message_leaves_the_message_as_it_was`: green. Exit 1 with `MessageDispatchMismatch`, and the message stays `Received` with no dispatch.
   - `a_session_of_one_repository_does_not_report_for_another_repositorys_unit`: **red** (`:588`). Alpha's session sends `[REPORT beta <the id of beta's dispatch>] done …` and beta's row ends up `"state":"Done"` with `"last_report":"[REPORT beta <the id of beta's dispatch>] done evidence from alpha's session"`.
   - `a_start_ref_is_taken_only_where_git_takes_it_as_a_ref_name`: **red** (`:662`). `start_ref takes what git check-ref-format refuses: ["@", ".hidden", "foo/.bar", "foo.lock/bar"]`.
   - `no_bidirectional_control_of_a_brief_reaches_status_or_a_view`: **red** (`:703`). U+202E, U+202C, U+2066 and U+2069 are printed by `dispatch dispatches` (text and markdown) and by `status --full`.

3. Suite (`suite.sh`): `cargo clippy -p conductor-cli --all-targets -- -D warnings` exit 0. `cargo nextest run -p conductor-cli --no-fail-fast --test sec_work_route --test work_route`: `17 tests run: 14 passed, 3 failed, 0 skipped`, exit 100. The only red cases are the 3 above.

4. Claims:
   - **A held.** Both round 1 cases pass now. Their panic lines moved 384→386 and 420→424, which is what reformatting two `format!` calls produces. I kept no byte copy from round 1, so formatting-only is inferred from that shift.
   - **B held for lines that name another repository.** The rule compares names exactly (`mailbox.rs:266`).
     - Not held for a line that names B consistently. Every session writes into the one mailbox `mailbox/conductor`. `message send` is open to every session (`cli.rs:950` `MailboxWrites`) and takes any `--from`.
     - So a session of repository A can write `[REPORT B <B's id>] done` and move B's dispatch. That was already possible at `9f8c2968a`, so it is pre-existing and not this fix's to close.
     - The store does record the sending session as the actor, so a later check could tie the line's `<repo>` to that session.
   - **C held.** The outcome is declared in `spec/domains/dispatch.yaml:1766` and generated at `behaviour.rs:2273`, after `need-unrouted`.
     - `repository_named` reads the first line the same way the mailbox does.
     - The guard denies controllers `message handle-message` (`cli.rs:944` `Writes`).
     - A first line with extra words is refused at `message send`, so it never becomes a stored message.
   - **D.** Leading `-`, ranges, reflog expressions, `^ ~ :` and `..` are all refused. The ordinary set (`main`, `release/1.5.0`, `v1.5.0`, a 40-digit id, `feature/x.y`) is taken. The route and `send-dispatch` call the same `refused_member`.
     - The spec's "git admits" claim does not hold for path parts starting with `.`, parts ending in `.lock`, and `@` (`behaviour.rs:261`).
     - `start_ref` also takes shell characters (`$ ; | \` ' "`), as git does.
     - `issue` may still hold a URL of any scheme, spaces, quotes, C1 controls, U+2028/2029 and bidi characters. It shows in `dispatch dispatches` (`dispatch.rs:877`, which flattens all of those except bidi), and raw in the JSON views, the store and the dispatch log line (`records.rs:349`). It is not shown in the work row, `status` or the dashboard.
   - **E held for C0, DEL, C1, U+2028 and U+2029** (`status.rs:466`, `:521`; `cli.rs:2501`). Bidi characters pass in both places. They passed at `709e7847e` too; the work call is what lets a remote caller's brief reach them.
   - **F held.** The `500` body carries the error name, the already-validated reference and a fixed detail (`dashboard/work.rs:178`). The log line holds the request line and the error chain. The chain's contexts name commands and ids only: no token, no brief.
   - **G (for the backlog draft).** `dispatch report-*` is `Writes` (`cli.rs:934-942`), so the guard denies it to controllers. Conductor's session, the loop and the operator can run it. No HTTP route reaches it. Two ways around: a controller can run a script file the guard does not read, and it can write the consistent `[REPORT]` line above.

5. Checked and found nothing wrong: the mismatch check, which compares the dispatch's repository after the lookup; the message file versus the stored first line (the `handle_message` check catches a difference); a refused command, which writes nothing; regeneration of the model; the daemon passing the `500` through unchanged.

6. Written outside the worktree, all in `<the review scratch directory>/`: the directory itself, `sec_work_route.rs.before`, `cases.sh`, `cases.log`, `suite.sh`, `suite.log`.

```findings
[
{"file":"crates/conductor/src/run/mailbox.rs","line":266,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"a session of one repository writing a REPORT line that names another repository and its dispatch consistently moves that dispatch and sets its last_report, since nothing ties the line's repository to the sending session"},
{"file":"crates/conductor/src/dispatch/behaviour.rs","line":261,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"valid_start_ref takes values git check-ref-format refuses (a path step starting with a dot, a step ending in .lock, a lone @) although the spec says it admits what git admits"},
{"file":"crates/conductor/src/status.rs","line":524,"category":"integrity","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"bidirectional embedding, override and isolate characters of a brief from the work call still reach status and the text and markdown views"}
]
```
