---
format: aep.planning-md/3
id: approval-record:wave-2026-11-15b-conductor
kind: approval-record
status: approved
title: 'Conductor wave 2026-11-15b: a boxed runtime and its controllers do forge work'
relations:
- decides: story:box-forge-work
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-15T09:47:14Z", actor: "human:agent", revision: 2, executor: "human:timo"}
---
# Conductor wave 2026-11-15b: a boxed runtime and its controllers do forge work

## Approval

The operator approved this wave on 2026-11-15, in the session that planned it, choosing to keep
the instance's runtime in its substrate box and make the box able to do forge work: commit and
push the change on an integration branch with a planning item for every change, without a
release.

## Units

| unit | artifact | note |
|---|---|---|
| U1 | `story:box-forge-work` | one egress aperture per forge host, named for the host and kept where the daemon declares it, refused above four at validate; the checkouts and managed trees roots writable; git's and the worktree CLI's configuration read; Claude Code's state and the worktree registry in the state root, with the trust of the checkouts marked there; doctor rows for the forge, git and trust |

## Terms

- **Gate:** the pull request's checks; nothing merges red and no check is overridden.
- **Release:** none in this wave; the release decision is separate.
- **Evidence:** `tests/placement.rs` and `tests/trust.rs`; the local run recorded on the story.
