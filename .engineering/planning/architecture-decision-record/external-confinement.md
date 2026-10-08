---
format: aep.planning-md/3
id: architecture-decision-record:external-confinement
kind: architecture-decision-record
status: accepted
title: Harness sessions run unconfined inside; conductor confines them from outside
relations:
- supersedes: story:bash-sandbox
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-08T22:08:31Z", actor: "human:timo", revision: 2}
---
## Decision

Harness sessions (conductor, conductor-dev, every controller and its workers) run unconfined
inside: full permissions (`bypassPermissions`), no harness sandbox (Claude Code's
`sandbox.enabled` stays off), Bash unrestricted. Conductor confines each session from outside, in a
process envelope it starts the harness in and controls: resources (CPU, memory, processes),
filesystem (what the session may write and read), network, lifetime. The envelope is measured at
the session's boundary, and a session whose envelope cannot be established is not started, or is
started only when the instance's config says unconfined is acceptable, and is then reported as
unconfined.

Operator, 2026-10-09: "i want to run the harnesses completey unconfined and then confine them
myself ... for full power + control" and "same way btw as metaharness is doiing it".

## The pattern this follows

metaharness (`~/beyond10x/metaharness`, read 2026-10-09):
- Claude Code starts fully permissive: `--dangerously-skip-permissions`, `--setting-sources`,
  `--settings`, `--strict-mcp-config` (`crates/metaharness-claude/src/launch.rs:143-146,1054-1064`).
- An embedder-supplied provider starts the child inside a sealed process envelope and returns
  measurements taken at the child boundary (`crates/metaharness/src/process.rs:99-121`, trait
  `ProcessEnvelope`); strict mode refuses a child whose envelope evidence does not match the request
  (`process.rs:182-193`).
- Optionally the harness's tools are metaharness's own MCP server (`mcp-serve --workspace <cwd>
  --writable --allow-program …`, `launch.rs:1143-1152`), which executes inside substrate; "The
  fences are the hooks and the confinement, not the approver" (`AGENTS.md:246`).
- The runner re-execs itself under `systemd-run --user --scope` so its own cgroup sits inside the
  delegated root substrate requires (`AGENTS.md:213-215`).

## Consequences

- The guard (PreToolUse hook) stays for messages and as an accident catcher; it is no longer the
  boundary for writes. Its Bash heuristics stop mattering for safety once the envelope enforces the
  filesystem.
- Story `bash-sandbox` (Claude Code's own sandbox) is not the route: archived.
- Conductor needs an envelope provider seam like metaharness's: a request (limits, writable and
  readable paths, programs, network), a provider that starts the harness in it, and measurements
  at the boundary. Providers: substrate (when the spike shows a Claude Code session runs under it),
  and a plain Linux provider (`systemd-run --user --scope` for limits, `bwrap` for the filesystem).
- Epic `session-confinement` carries the work; epic `multi-instance-host` keeps names, tasks and the
  host ledger.
