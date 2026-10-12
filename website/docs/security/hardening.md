---
title: Hardening checklist
sidebar_position: 6
description: A checklist for running a conductor instance on repositories you care about; what to set up on the host, on the forge and in the config, and how to check that the guard runs.
lede: The guard narrows what a session does; the host, the forge and the token decide what a mistake or a hostile prompt can reach.
source: crates/conductor/src/guard.rs; crates/conductor/src/config.rs; crates/conductor/src/doctor.rs; crates/conductor/src/trust.rs; Taskfile.yml; .claude/controller-settings.json, .claude/conductor-settings.json
---

Work through this list before you start sessions on an instance, and again after an upgrade.
Examples use `$CONDUCTOR_CONFIG` for the config file and `~/example-org/` for the instance's
directories.

## Host

- [ ] **Run sessions as a dedicated operating-system user.** Its home holds only the instance's
  checkouts, records, state and caches, and the credentials the sessions need. No personal SSH
  keys, cloud credentials or other projects. A Bash command can reach whatever this user can
  ([limits](/docs/security/limits)).
- [ ] **Keep `$TMPDIR` off the home directory.** A `$TMPDIR` that is the home directory, or holds
  it, is no scratch for the guard; one that points into a checkout or the records is never
  scratch either.
- [ ] **Run `conductor doctor`.** It reports each program conductor starts as present, with its
  version, or missing, and flags one older than a `b10x.toml` pin
  ([CLI reference](/docs/reference/cli#conductor-doctor)).

## Forge

- [ ] **Give sessions a scoped token.** Limit it to the instance's repositories, and leave out
  admin, secrets and organization rights. The guard does not check `git push` or direct API calls;
  the token's scope is what bounds them.
- [ ] **Protect `main` and release branches.** Require pull requests, at least one review, and
  the required status checks, and do not let the session's account bypass them.
- [ ] **Require review on `.claude/`.** Each repository's project settings and hooks run in its
  controller's session. A code-owners rule on `.claude/` keeps a change there from landing
  unreviewed.

## Config

- [ ] **Keep secrets out of the config file.** `conductor config validate` refuses a value that
  looks like a token (`ghp_`, `github_pat_`, `glpat-`, `xox<letter>-`) and never prints it; name where a
  secret lives instead.
- [ ] **Name the settings file of every guarded role.** The `controller` role's `settings` points
  at `.claude/controller-settings.json` or a file that wires the same hook; the `conductor` role's
  defaults to this checkout's `.claude/conductor-settings.json`. Give `conductor-dev` one too if
  it should be guarded ([config reference](/docs/reference/config#role)).
- [ ] **Give the controller role a `profile`.** `conductor config validate` refuses a controller
  role with an `agent` and no `profile`.
- [ ] **Keep `read_only_tools` minimal.** Name only tools that read: no MCP tool that writes,
  posts, sends or runs anything. Every tool a role lists is allowed to all of that role's
  sessions, wherever they run.
- [ ] **Run `conductor config validate`** after every edit of the config file.

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

  The last row names the session `probe`, the tool `Write` and the reason. An exit status of 0
  means the guard allowed the write: stop and find out why before you start sessions. Nothing is
  written to the target; the hook only decides.
- [ ] **Check a live session.** In a running controller, ask for a write outside its checkout;
  it should be refused with a `conductor guard: denied:` line, and the denial should appear in
  `conductor guard guard-decisions`.
- [ ] **Check that conductor runs once.** `claude agents` lists no live session under the conductor's
  name (`<session_prefix>-conductor`, or `conductor`) before you start or resume one.

## Repository and build

- [ ] **Set a Gates policy for `task check`.** The secret scan in `task check` runs only when
  `B10X_GATES_POLICY` names a policy, and is skipped otherwise.
- [ ] **Review what `task trust` trusts.** It marks the records directory and every git checkout
  directly under `checkouts.root` as a trusted Claude Code workspace in `~/.claude.json`. Their
  project settings and hooks then run without a prompt. Keep only repositories you have reviewed
  under that root.
- [ ] **Accept the bypass-permissions disclaimer deliberately.** `task conductor:start` refuses
  until it has been accepted once. Accept it as the session user, knowing that the guard is the
  only check from then on.
