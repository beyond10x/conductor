---
format: aep.planning-md/3
id: story:bash-sandbox
kind: story
status: draft
title: Bash writes are bounded by the harness sandbox, not by parsing shell
revision: 1
---
# Bash writes are bounded by the harness sandbox, not by parsing shell

## What

`conductor guard` reads Bash commands with a hand-written shell lexer
(`crates/conductor/src/guard/shell.rs`). An adversary review of it found 6 more forms that pass it
(heredoc scripts, `bash -ce`, `env -C`, `GIT_DIR`, `$PWD`, and conductor writing its own files with
`sed -i` or `>`), and listed 15 it considers out of reach of any heuristic (`eval`, `python -c`,
`xargs`, piping into a shell, …). The operator: "seems like this becoming a real big feature".
Extending the lexer was stopped; those forms are accepted limits, pinned by tests that name this
story (`crates/conductor/tests/adv_guard.rs`).

The bound that does not depend on parsing is an operating-system one: Claude Code's Bash sandbox,
if its settings can restrict which paths a session's commands may write. Unverified: whether the
installed Claude Code (2.1.292) offers it on Linux, how it is configured, and whether it applies to
`--settings` files and to sub-agents. Find out from the harness's own documentation, then:
- controllers: Bash may write only the checkout, its managed trees, the build cache and scratch;
- conductor: Bash may write only its records and its scratch.

If the sandbox cannot express that, record why and close this story; the guard's file and message
rules plus the snapshot's dirty-path attribution (design § 5.7) remain the boundary.

## Scope

- `.claude/conductor-settings.json`
- `.claude/controller-settings.json`

## Acceptance

- A cited answer (documentation URL or `claude --help` output) on whether and how the sandbox
  restricts write paths.
- If it does: both settings files carry it, and a live `claude -p` probe in a scratch checkout
  shows `printf x > <checkouts root>/<other repository>/x` refused by the sandbox.
