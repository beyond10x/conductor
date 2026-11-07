---
format: aep.planning-md/3
id: story:trust-one-repository
kind: story
status: draft
title: conductor trust can trust one repository's checkout
summary: spawn refuses per checkout but trust only trusts every checkout
revision: 2
---
## Why

Reported 2026-11-07 from a running instance: `conductor spawn` refused a repository with
`refused: UntrustedCheckout: repository "<group>/<repo>": ... not marked trusted in
~/.claude.json; run conductor trust`, and `conductor trust` can only trust everything. To start
one controller every checkout under the root had to be trusted, production ones included.

Verified in the code: `conductor trust` takes `--instance` only and is described as "Mark every
git checkout under the instance's checkouts root, and its records directory, as trusted"
(`crates/conductor/cli.yaml:351`); spawn refuses per checkout (`trusted`,
`crates/conductor/src/spawn.rs:767`).

## Acceptance

- `conductor trust --repository <name>` marks only that repository's checkout (and the records
  directory) trusted, and prints the one path; a name with no checkout is refused naming it.
- The refusal of `spawn` names that command for the repository it refused.
- `conductor trust` without `--repository` behaves as today.
