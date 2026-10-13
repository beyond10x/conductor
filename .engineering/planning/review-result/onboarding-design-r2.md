---
format: aep.planning-md/3
id: review-result:onboarding-design-r2
kind: review-result
status: active
title: Design critic, round 2, onboarding
relations:
- reviews: story:sandbox-spec
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
- reviews: story:spec-first-wording
revision: 1
---
needs-revision

Findings:
- story:sandbox-spec — its `depends_on story:spec-first-wording` edge has no reason beside it. The Notes name only `story:onboarding-run` and `story:snapshot-duration` as its predecessors. `story:spec-first-wording` shares no file with sandbox-spec: it scopes `.agents/*.md` and `tests/profiles.rs`, and sandbox-spec scopes `spec/`, the model and `cli.yaml`. The Notes should say the edge is the operator's v0.8.0 release gate, or the edge should go. — .engineering/planning/story/sandbox-spec.md:14,78-80

**Round-1 findings**
- **onboarding-design-r1#1 (setup duplicates doctor): no longer holds.** `setup-command.md:48` now says setup prints "doctor's rows, computed by doctor's own functions" and ends on "doctor's verdict function over both sets of rows". Only the four repository rows are new. The functions it names exist in `doctor.rs` (`examine` at line 314, `merge_pins` at 62, `verdict` at 511). Doctor rows the sandbox adds later, such as provider availability, therefore fall inside setup's verdict.
- **onboarding-design-r1#2 (Notes sandbox clause and cycle): no longer holds.** The Notes at `setup-command.md:63` now name only `doctor-identity` and `setup-guide-for-agents` as predecessors. I walked every `depends_on` and `blocks` edge in the store (112 edges over 46 nodes, including `session-envelope-seam`, `gitlab-guide` and `version-flag`) and found no cycle. The order is `setup-guide → doctor-identity → setup-command → snapshot-duration → onboarding-run → sandbox-spec → session-envelope-seam`, and the Notes agree with it.
- **onboarding-design-r1#3 (sandbox-spec edges without a reason): part still holds.**
  - The `onboarding-run` and `snapshot-duration` edges now have a reason, at `sandbox-spec.md:78-80`: the operator's gate that setup is released as v0.8.0 before sandbox work, plus the shared model, `cli.yaml`, `cli.rs` and `reference/cli.md`. That part is fixed, and the operator's decision settles the question of keeping the `onboarding-run` gate.
  - The edge added since, to `spec-first-wording`, has no reason. That is the finding above.
- **onboarding-design-r1#4 (snapshot-duration → setup-command edge without a reason): no longer holds.** `snapshot-duration.md:53` says both stories regenerate `crates/conductor-model` and `reference/cli.md` and edit `cli.rs`, and that `onboarding-run` follows on the same files. This is a shared-file ordering edge with its reason written beside it.

**Judged again**
- **Setup vs doctor split:** it is now sound. Doctor owns the machine, program, pin and forge-identity rows and the verdict. Setup adds repository rows and an `--apply` for the safe fixes. It never logs in, installs or migrates. SETUP.md's last step stays `conductor doctor`.
- **Cycles:** none. `doctor-identity` is now listed under the epic, which closes my round-1 scope unease.
- **Chain:** the five onboarding-side stories are a queue, but each edge records a shared file or a read of another item's output. Per the rubric that is a trade-off, not a finding. `snapshot-duration` has no logical tie to onboarding, so the shared-file edge is the only reason it waits. Splitting the generated model would be the alternative.
- **Acceptance reads:** I traced 6 reads to a producer in the set, and an edge records every one. Examples are `setup-command` reading `doctor-identity`'s rows and `SETUP.md`, and `onboarding-run` reading `conductor setup` output. Doctor's result rows in `config.yaml` that `sandbox-spec` adds follow `doctor-identity` and `setup-command` through the `onboarding-run` edge.

**What I read:** 9 artifacts in full (the epic, the 4 stories, `snapshot-duration`, `spec-first-wording`, `sandbox-spec` and `review-result:onboarding-design-r1`). I ran `aep plan artifact relations`, `graph`, `validate` (valid) and `waves`, plus `show` on each id. I also read `doctor.rs` signatures and the story files for line numbers.

**What I could not establish (outside my lane, not counted in the verdict):**
- `onboarding-run` installs conductor from a release archive, and no item in the set cuts a release that contains `conductor setup` before the runs. (outside the set)
- SETUP.md keeps the manual "write config / `conductor init` / `conductor trust`" steps and gains `conductor setup --apply` as an extra step. The `Check:` lines make the duplication harmless, but `setup-command` does not say whether the manual steps stay. (acceptance)
- `sandbox-spec`'s Notes say "wave 17 / wave 18", while `aep plan artifact waves` places `setup-command` in wave 17 and `sandbox-spec` in wave 21. I read the Notes' numbers as the release train's, not the tool's. (naming)

```findings
- file: .engineering/planning/story/sandbox-spec.md
  line: 14
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its depends_on edge to story:spec-first-wording has no reason beside it and the Notes name only onboarding-run and snapshot-duration; the two share no file (.agents and tests/profiles.rs against spec/, the model and cli.yaml), so the Notes should say the edge is the operator's v0.8.0 release gate, or the edge should go"
```
