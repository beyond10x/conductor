# Wave 55 (2026-11-15: a boxed runtime's sessions run without `claude --bg`)

The operator approved this wave on 2026-11-15 (`approval-record:wave-2026-11-15c-conductor`): one
story, committed on an integration branch, without a release. One pull request from
`wave/2026-11-15c-conductor` into `main`, forked from `main` at `964f4a89` (release 1.12.0). The
wave before is [wave 54](wave-54.md).

## Units

| unit | story | waits for | stage |
|---|---|---|---|
| U1 | box-session-without-bg | nothing | on the wave branch |

## Outcome

- A substrate box refuses `socket(AF_UNIX)`, and `claude --bg` needs a Unix socket for its
  background service, so every session a boxed runtime started failed `LaunchFailed`. A new
  provider, `child`, starts the session as a plain child process: `claude -p --input-format
  stream-json --output-format stream-json --verbose --replay-user-messages --session-id <uuid>`
  with the arguments `--bg` gets, its standard input a FIFO it holds open itself.
- Conductor takes `child` in place of `none` wherever this process may not create a Unix
  socket, and when a `conductor.config/1` file writes `confinement.provider: child`; `claude
  --bg` stays as it was everywhere else.
- The session's entry and files are in `conductor-sessions/` of Claude Code's state directory
  (`CLAUDE_CONFIG_DIR`). Every reader of the Claude session list sees these sessions beside the
  ones `claude agents --json` prints. A delivery is one stream-json message on the FIFO; a stop
  is `SIGTERM` to the process group, `SIGKILL` after 30 s; a resume is `claude -p --resume <id>`.
- `conductor session attach` opens no such session (it has no terminal), and the tool broker's
  session socket stays a Unix socket, which a box refuses.
