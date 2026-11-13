---
format: aep.planning-md/3
id: review-result:work-start-security-round-1
kind: review-result
status: active
title: The work call's start, security review round 1
relations:
- reviews: story:control-plane-work-handler
revision: 1
---
needs-change

Recorder's note: the reviewer's report on commit `b7e73d7cda` "Start the controller at the work
call, with a default charter and the brief", recorded after release 1.7.0. The sections up to
"Not run" are the reviewer's; "What followed" is the recorder's, read at `v1.7.0` (`951330f4f4`).

## What was reviewed

The commit "Start the controller at the work call, with a default charter and the brief": the
work call starts a controller session, writes a default charter where none is, and puts the
caller's brief, reference, start ref and issue into the session's first prompt. Read by a
security reviewer on a model other than the implementor's session, in a worktree of its own,
with new cases in `crates/conductor/tests/sec_work_start.rs`.

Who calls: a control plane holding the daemon's control token; a holder of a runtime's own
token on the pod network; an agent session inside the instance; a human operator.

## Suites, as printed

- The reviewer's cases alone: `11 tests run: 5 passed, 6 failed, 0 skipped`.
- With `sec_work_route` and `work_start`: `26 tests run: 20 passed, 6 failed, 1 skipped`; the
  six are the reviewer's; `sec_work_route` 7 of 7, `work_start` 8 of 8.

## Claims

- A, the prompt file: not held. The brief's own block holds for a brief with the fence, longer
  runs, a run at the end without a newline, tildes, CRLF, a lone CR, U+2028, U+2029, an
  indented fence and the introducing line as its first line. `issue` and `start_ref` reach the
  prompt outside the block holding U+2028, U+0085, `##` and backticks, through the route and
  through `dispatch send-dispatch`.
- B, what starts: not held. A brief the route takes can be one no session can start: the
  prompt is one argument of the launch, a brief with a run of more than about 31,000 backticks
  exceeds the kernel's limit for one argument, and a NUL cannot be passed at all. The call
  answers `201` with a failed start and every cycle tries again, each time recording one more
  stopped controller. A refused request starts nothing; a repository no `include` entry covers
  starts nothing and gets no charter. Of the request, `repository` reaches the session name,
  the working directory, the charter's file name and the broker registration; the other
  members reach the last argument only; nothing reaches the environment. The environment of
  the session holds the runtime's own API token.
- C, the limit: held by reading; no concurrent case was run. Every start in the runtime takes
  one lock, calls are serialised by the write lock, a `409` holds a repository to one open
  unit, and the cycle starts at most one controller per repository.
- D, the default charter: not held for a symlink at the `charters` directory or at the file
  written beside the charter; a symlink at the charter's own path is replaced, not followed.
  No writer of such a symlink was shown.
- E, the row's `start` member: held. It is the declared value only; the `start-refused` line
  held neither the brief nor a token.
- F, the cycle's retry: held. Only conductor's own session or a human can create a dispatch
  with a reference through the command, and either could already start a controller.
- G, the earlier findings: held; `sec_work_route` 7 of 7.

## Findings

1. Blocker, there before this commit, made worse by it: a session the runtime starts inherits
   the runtime's API token. With it a session can call the work call, which since this commit
   starts a controller at once in any repository the instance names.
2. Warning, introduced: a brief with a NUL, or with a run of backticks above about 31,000
   bytes, is accepted and can never be passed as the prompt; each cycle records another stopped
   controller, without bound.
3. Warning, introduced: `issue` and `start_ref` reach the prompt outside the brief's block
   holding line separators, C1 controls, hashes and backticks.
4. Note, introduced, not shown reachable: the default charter is written through a symlinked
   `charters` directory or a symlinked file beside the charter.
5. Note, by decision: a unit's start passes over an operator's Inactive mark and the
   `merge: human` review cap.

## Not run

A concurrent case for the limit; a real harness (whether it passes its environment on to the
tools of a session); a pod on a cluster; clippy and the full package (the coordinator's gate).

## What followed

Release 1.7.0 (`git log v1.6.0..v1.7.0`) holds the fix in commit `45ede59cd2` "Harden the work
call's launch environment and its inputs". It adds `crates/conductor/tests/sec_work_start.rs`
with the reviewer's cases, and its message says their assertions are unchanged. Read at
`951330f4f4`; no case was run for this record.

- Finding 1: the variable is closed, the token is not. `instance::command`, which builds every
  process the runtime starts, removes `CONDUCTOR_API_TOKEN`
  (`crates/conductor/src/instance.rs:63-68`); case
  `sec_b_the_session_environment_holds_no_runtime_token` (`sec_work_start.rs:723`). An instance
  pod's `secret` volume lists the keys it mounts, the runtime's token not among them
  (`crates/conductor/src/daemon/pods.rs:486-494`, `:515`); case
  `the_secret_volume_lists_every_key_of_the_secret_but_the_runtimes_token`
  (`crates/conductor/tests/sec_instance_pods.rs:1718`). Open: a session runs as the user of
  its runtime, which holds the token; `story:session-cannot-read-runtime-token`.
- Finding 2: closed. A brief with a NUL or a backtick run longer than 255
  (`BRIEF_BACKTICKS_MOST`) is refused as `InvalidBrief`
  (`crates/conductor/src/dispatch/behaviour.rs:96`, `:103-114`), at the route
  (`crates/conductor/src/dashboard/work.rs:150`) and in `dispatch send-dispatch`
  (`crates/conductor/src/dispatch.rs:413`). The cycle leaves a unit to the operator after three
  starts that failed at the launch (`crates/conductor/src/run/dispatch.rs:173-208`). That stop
  covers the start of a unit only; `story:failed-start-stop-covers-every-start`.
- Finding 3: closed. `issue` and `start_ref` refuse C0 and C1 controls, U+2028, U+2029,
  bidirectional controls and the backtick (`behaviour.rs:88-91`, `:120-123`, `:141-143`), at the
  route (`work.rs:161`) and in `dispatch send-dispatch` (`dispatch.rs:404`). A `#` stays
  admitted in `issue` (`behaviour.rs:139`); the comment at `behaviour.rs:84-86` says the prompt
  writes each member on its own line after a fixed label (the prompt's code: not read).
- Finding 4: closed. The default charter is written only inside the records: `charters` must be
  a real directory, the file beside the charter is created anew, and a symbolic link there
  refuses the start (`crates/conductor/src/spawn.rs:760-828`).
- Finding 5: not changed; the message of `45ede59cd2` says so, by decision.

```findings
[
{"file":"crates/conductor/src/instance.rs","line":61,"category":"boundary","severity":"blocker","verdict":"CONFIRMED","origin":"pre-existing","message":"instance::command, which builds every process the runtime starts, removes the slot and parent variables only (lines 64-65), so a session inherits CONDUCTOR_API_TOKEN and can call the work call, which since this commit starts a controller at once in any repository the instance names"},
{"file":"crates/conductor/src/dashboard/work.rs","line":135,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the route takes any brief of 1 to 65536 bytes, a NUL or a backtick run above about 31000 included; the brief goes into the prompt at spawn.rs:186 and the prompt becomes the last argument of the launch at confine/mod.rs:361 (the unboxed provider; the boxed path not read), which cannot carry it, and each cycle records another stopped controller without bound"},
{"file":"crates/conductor/src/spawn.rs","line":787,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"start_ref (line 787) and issue (line 790) are written into the controller's first prompt outside the brief's fenced block, holding line separators, C1 controls, hashes and backticks"},
{"file":"crates/conductor/src/spawn.rs","line":761,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"default_charter writes the default charter through run::write_file (lines 764, 766; run.rs:1467), which writes a file beside the charter with fs::write and renames it over the charter: a symlink at the file beside or at the charters directory is written through, one at the charter's own path is replaced; no writer of such a symlink was shown"},
{"file":"crates/conductor/src/run/dispatch.rs","line":164,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"by decision: the cycle starts a unit for every Sent dispatch with a reference (line 126) whatever the repository's activity, and the merge: human review cap (line 189) is checked only in the loop over Active repositories; the work call's start, run::start_work at run.rs:1023, checks neither"}
]
```
