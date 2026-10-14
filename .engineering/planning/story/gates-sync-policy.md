---
format: aep.planning-md/3
id: story:gates-sync-policy
kind: story
status: draft
title: conductor publishes the committed Gates policy to CI's secrets
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

CI reads the Gates policy from GitHub Actions secrets, and every class-C baseline move or enrolment
changes the policy file, so each needed the operator to refresh the secrets; nine policy commits
reached no CI between 2026-10-12 and 2026-10-14. The operator, 2026-10-14: "do whatever increases
autonomy, but without sacrificing security or having a risk of accidentally committing sensitive
data to the remote." A hand-written sync script did the job once; the product runs no scripts
(AGENTS.md rule 3).

## Acceptance

- Spec first: `conductor gates sync-policy [--apply] [--force]` publishes only the committed policy
  (`origin/<default>:policy.json` of the repository `gates.policy_repository` names, never a
  working-tree file) to the secrets `gates.secrets` lists: an organization secret with the policy
  minus the exceptions of the repositories that carry their own, a repository secret per such
  repository with only its exceptions, and an organization secret with the whole policy gzip-9 and
  base64. It refuses a policy that is not a Gates policy and any value of 48,000 bytes or more,
  publishing nothing; it prints `unchanged: <commit>` when that commit was last published (a marker
  in the state directory) unless `--force`.
- Without `--apply` it is a dry run: it prints the commit, each target and each value's size, and
  writes nothing; with `--apply` it sets the secrets and the marker and prints one line. Tests with
  a fake forge CLI on `PATH` hold the dry run, the refusal by size, `unchanged`, and an apply.
- The guard treats `gates sync-policy --apply` as class H (a forge write of a secret, design § 5)
  unless the operator approves an exemption; `--dry-run` is allowed to conductor. A guard case
  holds both. `.agents/conductor.md` says conductor runs the dry run on drift and hands the operator
  the one `--apply` command with its output.

## Scope

`spec/domains/config.yaml`, `spec/components.yaml`, `crates/conductor-model`,
`crates/conductor/cli.yaml`, `crates/conductor/src/cli.rs`, `crates/conductor/src/gates.rs`,
`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/gates_sync.rs` (new),
`crates/conductor/tests/guard_rules.rs`, `.agents/conductor.md`, `docs/config.md`.
