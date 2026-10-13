---
format: aep.planning-md/3
id: review-result:onboarding-parallel-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2, onboarding
relations:
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
- reviews: story:spec-first-wording
revision: 1
---
approve

Round 1's seven findings no longer hold. I found nothing new, so the findings list is empty.

**Round 1, finding by finding**

1. **Guide and doctor-identity on `website/docs/getting-started.md`: fixed.**
   - `story:doctor-identity` now has `depends_on story:setup-guide-for-agents` (`doctor-identity.md:10`).
   - Both bodies name the other: `doctor-identity.md:58` and `setup-guide-for-agents.md:50`.
   - `waves` puts the guide in wave 1 and doctor-identity in wave 2. It still prints the collision, which is now ordered.
2. **snapshot-duration's body did not explain its new edge: fixed.** Notes at `snapshot-duration.md:53-55` name setup-command, the shared `crates/conductor-model` and `website/docs/reference/cli.md`, and `cli.rs`.
3. **snapshot-duration's scope omitted `cli.rs`: fixed.** `crates/conductor/src/cli.rs` is now in the scope (`snapshot-duration.md:15-16`).
4. **onboarding-run's scope was too narrow: fixed.**
   - The scope now lists `setup.rs`, `doctor.rs`, `cli.yaml`, `cli.rs`, `spec/domains/config.yaml`, `crates/conductor-model`, `reference/cli.md`, `tests/setup.rs` and `docs_setup.rs` (`onboarding-run.md:12-35`).
   - The new `depends_on story:snapshot-duration` edge (`:10`) orders it after snapshot-duration, and Notes (`:60-62`) say why.
   - Wave 5 holds onboarding-run alone.
5. **setup-command's Notes cited the wrong order against the sandbox wave: fixed.**
   - The sandbox sentence is gone. Notes (`setup-command.md:63-65`) now cite doctor-identity, the guide and snapshot-duration, and each matches an edge.
   - `doctor.rs` is now in the scope (`:21`).
6. **doctor-identity's scope listed the wrong docs: fixed.**
   - `reference/config.md` is gone. The scope now has `reference/cli.md`, `cli.yaml`, `cli.rs` and `website/docs/guides/troubleshoot.md`.
   - `troubleshoot.md` mentions doctor 11 times (`git grep -c -i doctor`).
   - No other story in the set lands on it.
7. **setup-command's scope omitted `docs_setup.rs`: fixed.** It is in the scope (`setup-command.md:26-27`), and its acceptance line 59 and Notes name it.

**Judged again**

- **Waves.** The chain is guide (1), doctor-identity (2), setup-command (3), snapshot-duration (4), onboarding-run (5). Every adjacent pair has a direct `depends_on` edge, so doctor-identity before snapshot-duration, and the guide before onboarding-run, are ordered transitively. Each shared surface is named in a body.
- **`story:spec-first-wording`.** It lands on `.agents/conductor-dev.md`, `.agents/conductor.md`, `.agents/repo-controller.md` and `crates/conductor/tests/profiles.rs`. No chain story touches any of them.
  - `waves` shows no collision between it and any set member.
  - It sits in wave 2 only because of `story:host-grants` and `story:session-envelope-seam`, which are outside this set.
  - No code reads the repo-controller text (`git grep` of `repo-controller.md` in `crates/` found none).
  - I treat it as independent of the chain.
- **Files that do not exist yet.** `SETUP.md`, `crates/conductor/src/setup.rs`, `crates/conductor/tests/setup.rs` and `crates/conductor/tests/docs_setup.rs` are not in the tree. The guide creates the first and last, and setup-command creates the middle two. Later stories edit them only behind an edge.

**What I read:** six stories in full, `review-result:onboarding-parallel-r1`, and `story:doctor-minimum-versions` (status implemented). I ran `aep plan artifact waves --kind story --status draft`, `aep plan artifact graph`, `aep plan artifact show`, `git grep`, `ls` and `git diff --stat`.

**Surface counts:** cited 6, inferred 0, unplaceable 0.

**Acceptance reads:** I traced six to a producer in the set, and four are recorded by a direct `depends_on` edge.
- Direct, four: doctor-identity reads the guide's page; setup-command reads doctor-identity's rows; setup-command reads the guide's `SETUP.md` and `docs_setup.rs`; onboarding-run reads `conductor setup`.
- Transitive only, two: onboarding-run reads the guide's `SETUP.md` and doctor-identity's rows through setup-command. The chain is linear, so this is not a gap.

**What I could not establish**
- I judged the working tree. `doctor-identity.md`, `snapshot-duration.md` and `sandbox-spec.md` have uncommitted edits (`git status`). If the store at review time differs, the verdict does not carry over.
- Out of my lane:
  - `story:sandbox-spec` depends on `onboarding-run`, `snapshot-duration` and `spec-first-wording`. It is outside the set, so I did not assess it.
  - Whether setup-command's `--apply`, which both reads and writes, fits the single `Access` value per leaf in `LEAVES` is a design question.
  - The earlier concern about `SETUP.md` falling outside the public-text check is now covered by `setup-guide-for-agents.md:46`.

```findings
[]
```
