# Wave 17

Coordinator: conductor-dev, `aep:implementing` 0.22.8, wave mode. Approved up front (rules item 99;
the operator, 2026-10-13: "make sure the next release will have those fixed … then dispatch those
waves"). One managed tree, `conductor-wave-17`, branch `wave/17`; implementors write without a
local build and CI gates the round. Epic: `epic:forge-neutral`, with the doctor, version and gc
fixes from a team's trial of v0.4.0. Target: v0.7.0.

## Units

| unit | stories, in order | after | stage |
|---|---|---|---|
| A1 | gitlab-source-host, gh-optional, doctor-verdict, doctor-minimum-versions | | committed |
| B | guard-glab-writes, profiles-forge-neutral | | committed |
| C | gc-archive-profile | | committed |
| A2 | gitlab-repositories, gitlab-merge-requests, watch-gitlab-ci, gitlab-workspace | A1 | dispatched |
| C2 | version-flag | A1 (`reference/cli.md`) | committed |
| B2 | six guard parser fixes from the security review (`sec_glab_*`) | B | dispatched |
| D1–D3 | gitlab-guide, split by file: guide and docs test; README and getting-started; concepts, guard page, design | | dispatched |
| E | dashboard-worktree-repo (found while implementing C) | C | dispatched |
| — | `aep:adversary` on A1, tests only (`adv_w17_doctor.rs`) | A1 | dispatched |

D1–D3 run beside A2: they document the stories' acceptance, and CI checks them together.

A1, B and C touch disjoint files (`aep plan artifact waves`). Every edge in a unit is a shared
file named in the story bodies. `guard-glab-writes` is reviewed by `aep:security-reviewer` before
the merge.

Commits the approval covers: one commit per unit on `wave/17`, CI fixes, the closing store commit,
the fast-forward of `main` once CI is green, and the v0.7.0 release commit and tag.

## Outcome

Pending.
