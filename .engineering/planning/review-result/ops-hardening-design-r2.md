---
format: aep.planning-md/3
id: review-result:ops-hardening-design-r2
kind: review-result
status: active
title: Design critic, round 2, epic operating-loop-hardening
relations:
- reviews: epic:operating-loop-hardening
revision: 1
---
approve

The twelve stories hold together as a set. The only edges among them are four `depends_on` lines, and each has a stated reason:
- `story:operator-decision-guarded` before `story:single-conductor-lease`: both regenerate `crates/conductor-model` and edit `crates/conductor/src/cli.rs`.
- `story:single-conductor-lease` before `story:lease-guard`: the guard reads the lease that `conductor session claim` writes, so the acceptance in `tests/guard_hook.rs` needs it.
- `story:guard-store-migrate` before `story:lease-guard`: both change the guard module.
- `story:restart-current-profile` before `story:agents-link-user-scope`: both edit `Taskfile.yml` and `crates/conductor/tests/taskfile.rs`.

The longest chain is three items (`story:operator-decision-guarded`, then `story:single-conductor-lease`, then `story:lease-guard`). That leaves eight stories with no ordering. There is no cycle and no serialisation.

The lease split is sound. `story:single-conductor-lease` is demonstrable alone, since the claim refuses a live holder and takes over a stale one. `story:lease-guard` is demonstrable against a fixture lease. `story:watch-two-conductors` reads only the `claude agents --json` listing, so it needs no edge to the other two. I did not find a half-abstraction in any pair.

**What I read:** 12 stories and the epic, with `aep plan artifact show` or `cat`. I also ran `aep plan artifact relations`, `aep plan artifact graph` (I filtered it to every edge touching the 13 ids, so edges to artifacts outside the set would have shown) and `aep plan artifact validate`. Validate reported "valid", with only the five unrelated stories closed on an assertion. I walked 16 edges inside the set. The 17th is `story:instance-scoped-tasks` to `story:restart-current-profile`, which is outside the set and runs into it, so it adds no cycle. I traced about six acceptance reads to a producer: the lease read by the guard, the Taskfile shared by two stories, the `public-text-check` scan over files siblings edit, and the dashboard and doctor tests. Every one that crosses stories has an edge.

**What I could not establish:**
- Unease, not a finding: `story:single-conductor-lease` leaves open how `task conductor:restart` (edited by `story:restart-current-profile`) interacts with a claim while the old pid is still exiting. The new session could be refused as a second conductor. The note at `single-conductor-lease.md:66` says the question is not settled.
- Out of my lane (scope): the epic's "smaller frictions" bullet still lists "session tasks miss prefixed sessions", but that item is now owned by `story:instance-scoped-tasks`.

```findings
[]
```
