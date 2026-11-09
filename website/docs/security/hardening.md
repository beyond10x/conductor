---
title: Hardening checklist
sidebar_position: 7
description: A checklist for running a conductor deployment on repositories you care about; the host, the forge, the config, the daemon and its operators, and how to check that the guard runs.
lede: The guard narrows what a session does; the host, the forge, the token and the grants decide what a mistake or a hostile prompt can reach.
source: crates/conductor/src/guard.rs, crates/conductor/src/config.rs, crates/conductor/src/doctor.rs, crates/conductor/src/trust.rs, crates/conductor/src/daemon/api.rs, crates/conductor/src/secret.rs; crates/conductor/kinds/claude-code/; Taskfile.yml
---

Work through this list before you start sessions on an instance, and again after an upgrade.
Examples use `$CONDUCTOR_CONFIG` for the config file and `~/example-org/` for the instance's
directories.

## Host

- [ ] **Run sessions as a dedicated operating-system user.** Its home holds only the instance's
  checkouts, records, state and caches, and the credentials the sessions need: no personal SSH
  keys, cloud credentials or other projects. Agent sessions run unboxed, and a Bash command
  reaches whatever this user can ([limits](/docs/security/limits)).
- [ ] **Keep `$TMPDIR` off the home directory.** A `$TMPDIR` that is the home directory, or holds
  it, is no scratch for the guard. Neither is one inside a checkout or the records.
- [ ] **Place each runtime on an adapter** where the host's `substrate-daemon` serves one, so the
  instance's runtime opens in a substrate box
  ([sessions](/docs/security/sessions#boxes-slices-and-limits)). Otherwise set the instance's
  `limits`, which its slice carries.
- [ ] **Run `conductor doctor`.** It reports each program conductor starts, `substrate-daemon`
  among them, as present with its version or missing, and flags one older than a `b10x.toml` pin
  ([CLI reference](/docs/reference/cli#conductor-doctor)).

## Forge

- [ ] **Give sessions a scoped token.** Limit it to the instance's repositories, without admin,
  secrets or organization rights. The guard does not check `git push` or direct API calls. The
  token's scope bounds them.
- [ ] **Protect `main` and release branches.** Require pull requests, at least one review and the
  required status checks, and do not let the session's account bypass them.
- [ ] **Require review on `.claude/`.** Each repository's project settings and hooks run in its
  controller's session. A code-owners rule on `.claude/` keeps a change there from landing
  unreviewed.
- [ ] **Choose `forge.route` on purpose.** Under `gates`, the default, `gh` and `glab` only read
  and writes take the bot route. `direct` lets a session open, comment on and merge its own
  repository's pull or merge requests ([the guard](/docs/security/the-guard#forge-calls)).

## Config

- [ ] **Keep secrets out of the config file.** `conductor config validate` refuses a value that
  looks like a token (`ghp_`, `github_pat_`, `glpat-`, `xox<letter>-`) and never prints it. Write
  `secret:<name>` and put the value with `conductor secret put --name <name>`
  ([secrets](/docs/security/operators-and-credentials#secrets)).
- [ ] **Wire the guard for every role that should have it.** The `conductor` and `controller`
  roles start with the settings the binary carries, which wire it. A `settings` either names
  instead must wire the same hook. The maintainer and the host roles start with none: give their
  role a `settings` file if they should be guarded ([config reference](/docs/reference/config#role)).
- [ ] **Give the controller role a `profile` if it names an `agent`.** `conductor config validate`
  refuses a controller role with an `agent` and no `profile`.
- [ ] **Keep `read_only_tools` minimal.** Name only tools that read: no MCP tool that writes,
  posts, sends or runs anything. A listed tool is allowed to every session of the role, wherever
  it runs.
- [ ] **Run `conductor config validate`** after every edit of the config file.

## Daemon and operators

- [ ] **List every operator in `daemon.operators`** with only the grants they need. While the
  list is empty, the loopback user holds every grant
  ([grants](/docs/security/operators-and-credentials#operators-and-grants)).
- [ ] **Treat `127.0.0.1:<daemon.port>` like the operator's terminal.** Do not forward it, and do
  not run the daemon where another user reaches loopback: whoever reads its page writes as the
  operator.
- [ ] **Serve remote clients on `daemon.tls` only,** with an Identity service, and set
  `daemon.loopback: false` on a box with no local operator.

## Verify the guard

- [ ] **Provoke a denial and read it back.** Feed the hook a payload it must deny, from a
  controller's checkout, with `CONDUCTOR_CONFIG` set to the instance's config file:

  ```console
  cd ~/example-org/checkouts/<repo>
  echo '{"session_id":"probe","tool_name":"Write","cwd":"'"$PWD"'","tool_input":{"file_path":"/tmp/../etc/probe"}}' \
    | conductor guard record-guard-decision --from-pre-tool-use
  echo "exit $?"   # 2, with "conductor guard: denied: …" on standard error
  conductor guard guard-decisions --format jsonl | tail -n 1
  ```

  The last row names the session `probe`, the tool `Write` and the reason. Exit status 0 means
  the guard allowed the write: stop and find out why before you start sessions. The hook only
  decides. Nothing is written to the target.
- [ ] **Check a live session.** Ask a running controller for a write outside its checkout. It is
  refused with a `conductor guard: denied:` line, and the denial appears in `conductor guard
  guard-decisions`.
- [ ] **Check that conductor runs once.** `claude agents` lists no live session under the
  conductor's name (`<session_prefix>-conductor`, or `conductor`) before you start or resume one,
  and `conductor watch` prints no `conductors:` line.

## Repository and build

- [ ] **Set a Gates policy for `task check`.** Its secret scan runs only when `B10X_GATES_POLICY`
  names a policy, and is skipped otherwise.
- [ ] **Review what `conductor trust` trusts.** It marks the records directory and every git
  checkout directly under `checkouts.root` as trusted Claude Code workspaces in `~/.claude.json`.
  Their project settings and hooks then run without a prompt. Keep only repositories you have
  reviewed under that root.
- [ ] **Accept the bypass-permissions disclaimer knowingly.** `claude --bg` refuses
  `bypassPermissions` until the session user has accepted it once. From then on the guard is the
  only check. The acceptance lives in that user's `~/.claude/settings.json`, which every session
  loads, so keep that file to what an unattended session may run with.
