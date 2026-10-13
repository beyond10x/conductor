# Wave 18

Coordinator: conductor-dev, `aep:implementing` 0.22.8, wave mode. Approved up front (rules item 99;
the operator, 2026-10-13: "make the setup released before sandbox", "next release must include the
updated dashboard"). One managed tree, `conductor-wave-17` (reused), branch `wave/18`; implementors
write without a local build and CI gates the round. Target: v0.8.0. One pull request.

## Units

| unit | stories, in order | after | stage |
|---|---|---|---|
| A1 | setup-guide-for-agents, doctor-identity, setup-command | | dispatched |
| B | spec-first-wording | | dispatched |
| A2 | conductor-upgrade | A1 | waiting |
| C | dashboard-spa | A2 (`crates/conductor/Cargo.toml`) | waiting |
| A3 | snapshot-member-failures, fetch-without-prompts, snapshot-duration | A2 | waiting |
| D | dashboard-overview, dashboard-repo-view, dashboard-health | A3, C | waiting |
| E | onboarding-run: the AGENTS.md release step now; the two recorded runs last, on a build of the branch | D | waiting |

Every edge is a shared file named in the story bodies. `story:onboarding-run`'s recorded runs need a
built binary and real sessions, so they run after the code units, and any gap they find is fixed
in this wave before the release.

Design reference for the dashboard: `docs/design/dashboard-mockup.html`.

Commits the approval covers: one commit per unit on `wave/18`, CI fixes, the closing store commit,
the fast-forward of `main` once CI is green, and the v0.8.0 release commit and tag.

## Outcome

Pending.
