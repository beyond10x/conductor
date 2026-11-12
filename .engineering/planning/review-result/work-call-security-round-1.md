---
format: aep.planning-md/3
id: review-result:work-call-security-round-1
kind: review-result
status: active
title: The work call over HTTP, security review round 1
relations:
- reviews: story:control-plane-work-handler
revision: 3
---
needs-change

Recorder's note: the reviewer's report as returned on 2026-11-12, recorded by the coordinator; scratch paths are replaced. The two confirmed findings and the three introduced notes go back to the implementor; the pre-existing note becomes a backlog draft.

unit: security review of 709e7847e "Take a unit of work over HTTP and answer its state" (feat/work-call, k8s-forge-img); findings cover that commit plus my one new test file
verdict: needs-change (2 CONFIRMED warnings, introduced)
cases: executed 26→29, red 2 (26 is the same run with my file's 3 cases left out; I made no separate run without it)
origin: introduced 4 / pre-existing 1 / undecided 0
wrote-outside-worktree: 4 paths (part 6)
needs-coordinator: none

1. Worktree diff (`git status --short`; untracked, so `git diff --stat` is empty): `?? crates/conductor/tests/sec_work_route.rs`. That is the only change, and it is a test file. No implementation file touched, nothing under `.engineering/`.

2. Cases in `crates/conductor/tests/sec_work_route.rs` (helpers copied from `tests/work_route.rs`), first run alone:
   - `a_report_of_one_repository_is_not_the_last_report_of_another_repositorys_unit` is **red**. Alpha's session writes `[REPORT alpha <beta's id>] progress …` and one tick runs. `GET /v1/work/WI-B` then shows that line as beta's `last_report`. Verbatim: `{"dispatch_id":"<the id of beta's dispatch>","last_report":"[REPORT alpha <the id of beta's dispatch>] progress text from alpha's session",...,"repository":"beta",...}`
   - `a_session_does_not_attach_its_message_to_another_repositorys_dispatch` is **red**. Running as session `eo-alpha`, `message handle-message --message-id <its alpha report> --dispatch-id <beta's id>` succeeds, and beta's row shows `"last_report":"[REPORT alpha <the id of alpha's dispatch>] progress on alpha's export"`.
   - Run alone: `3 tests run: 1 passed, 2 failed` (`child` is the tick helper and passes).

3. Suite: `conductor build run --reserve 8G -- cargo clippy -p conductor-cli --all-targets -- -D warnings` exited 0. Nextest on `sec_work_route`, `work_route`, `instance_routes` and `isolated_home`: `29 tests run: 27 passed, 2 failed, 0 skipped`, exit 100. Only my 2 cases are red.

4. Findings:
   - **Claim E, not held** (CONFIRMED, introduced, warning). In `crates/conductor/src/run/mailbox.rs:252`, `report()` accepts an explicit dispatch id without checking that it belongs to the repository the `[REPORT]` line names. `mark_handled` then stores that id on the message, and `work_state` (`dispatch.rs`, the `last_report` filter) shows it. **Reached by:** any session that writes a report line. **Fix:** refuse a dispatch whose repository differs from `<repo>` before returning it as `reported`. Moving another repository's dispatch state this way was already possible at base 9f8c2968a; that part is pre-existing.
   - **Claim E, not held** (CONFIRMED, introduced, warning). `HandleMessage` sets `dispatch_id: input.dispatch_id` with no guard: the dispatch need not exist, belong to the message's repository, or be what a report message names (`spec/domains/dispatch.yaml:1735`, `message.rs:403`). **Reached by:** a session running the CLI. **Fix:** the `handled` outcome refuses a `dispatch_id` that is not a dispatch of the repository the report names.
   - **Claim C, judgement** (not run, introduced, note). `start_ref` and `issue` are accepted unchecked and unbounded apart from the 1 MiB body. A leading `-`, a newline, `..` or `@{` is stored in the store and in the dispatch log; `start_ref` reaches no command yet. **Reached by:** nothing in this commit. Whatever passes `start_ref` to git later must check it first.
   - **Claim D, judgement** (not run, introduced exposure of pre-existing code, note). In `status.rs:1514`, `cell()` handles only `\n`, `\r`, `|` and `\`. An ESC sequence or U+2028 in a brief now reaches `conductor status` and the board from a remote caller.
   - **Claim G, judgement** (not run, introduced, note). `dashboard/work.rs:664`: `failed()` returns the whole error chain in the 500 body (store and dispatch-log paths). The daemon passes it to the control plane unchanged.
   - **Claim E, log cost** (pre-existing pattern, note). In `dispatch.rs`, `dispatch_lines` reads every month file from `sent_at`'s month on, whole, into memory. It takes no lock and has no cap, on every `GET`. Over HTTP `sent_at` is always now; `--sent-at` on the command line can set any date. The cost is one full read of the dispatch log since that month.

5. Reviewed and held:
   - **A.** Admission is shared on both sides. The control token is admitted for `work` only (`api.rs:1799`, `admit_write` at `:1599`). Only `if-none-match`, `content-type` and `accept` are forwarded (`api.rs:2496`), so the caller's own `X-Conductor-*` headers are dropped. Both parsers split only on CRLF, so a bare `\n` or `\r` cannot inject a header. A holder of the runtime's token can now be recorded as `control-plane`. Before this commit it could already be recorded as any operator, and `ControlPlane` passes no guard that `Human` does not, so it gains no power, only that label.
   - **B.** The reference check is ASCII-only, at most 64 characters, the same function on both sides, run on the raw undecoded route (`behaviour.rs:37`, `api.rs:1217`).
   - **C.** Both sides refuse a chunked body and one without `Content-Length` with 411, and one over the bound with 413 before reading. The daemon re-frames with its own length. Unknown members are refused. `repository` is only looked up as a set member.
   - **F.** One cross-process lock covers the check and the send (`dispatch.rs:358`). A reference held by a dispatch that ended answers 200 and cannot be sent again, as the message says. The 409 names ids only.
   - **H.** `mark_handled` compares paths lexically, so neither `..` nor a symbolic link marks a message outside the mailbox.
   - **G.** No token or header value appears in any body or log line.

6. Written outside the worktree, all in `<the review scratch directory>/`: `case.log`, `suite.sh`, `suite.log`, and the directory itself. The shared target was built through `conductor build run`.

```findings
[
{"file":"crates/conductor/src/run/mailbox.rs","line":252,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"a REPORT line naming one repository and another repository's dispatch id becomes that dispatch's last_report in the work row"},
{"file":"spec/domains/dispatch.yaml","line":1735,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"HandleMessage sets dispatch_id from input unchecked so a session attaches its own report to any dispatch and the work row shows it"},
{"file":"crates/conductor/src/dispatch.rs","line":195,"category":"judgement","severity":"note","origin":"introduced","message":"start_ref and issue are taken over HTTP unchecked and unbounded except by the body size; a later consumer passing start_ref to git must refuse a leading dash and other ref syntax"},
{"file":"crates/conductor/src/status.rs","line":1514,"category":"judgement","severity":"note","origin":"introduced","message":"a brief from the control plane reaches the status table with control characters and Unicode line separators unescaped"},
{"file":"crates/conductor/src/dashboard/work.rs","line":664,"category":"judgement","severity":"note","origin":"introduced","message":"a 500 answer carries the whole error chain including host paths of the store and the dispatch log to the caller"},
{"file":"crates/conductor/src/dispatch.rs","line":296,"category":"judgement","severity":"note","origin":"pre-existing","message":"each GET reads every dispatch log month file from the month of sent_at on into memory with no cap or lock"}
]
```
