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

## Deviations

- Implementors wrote without a local build (a build hold was in force), so CI was the first
  compile: eight gate runs. The failures were a lint, struct literals and command lists missing
  what parallel units added, a class test's fixture missing a new observation kind, a fixture
  exemption for two new views, and a floor test reading over its own floor. `task check` now runs
  `cargo test --no-fail-fast`, and the controller profile asks for a compiled, linted integration
  head before the push.
- `conductor upgrade` takes `--release`, not `--version`, which `ess-cli/1` reserves.
- `story:onboarding-run`'s two recorded runs run on the v0.8.0 archive after the release; any gap
  they find goes to v0.8.1.

## Outcome

`task check`, documentation and the Gates job green in CI on `660ce7fb7`
(https://github.com/beyond10x/conductor/actions/runs/38403188681); thirteen stories moved to
`implemented` on that record. Released as v0.8.0.
