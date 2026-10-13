---
format: aep.planning-md/3
id: review-result:sandbox-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, sandbox
relations:
- reviews: epic:session-confinement
- reviews: story:sandbox-spec
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
needs-revision

Round 1's finding no longer holds. Two new findings from this round remain.

**Round 1 (`review-result:sandbox-scope-r1#1`) is fixed.** The epic now claims forge credentials in both places the finding asked for:
- The Outcome has a bullet: "forge credentials limited to read-only tokens the instance names, so a forge write goes only through the bot route (story `session-forge-credentials`)" (`.engineering/planning/epic/session-confinement.md:19-20`).
- The first-part Acceptance has a check naming the story and stating that its live run waits on `decision-blocker:sandbox-forge-tokens` (lines 44-46).

`story:session-forge-credentials` is now claimed by the epic's own text. That is the correction.

Findings:

```
epic:session-confinement — the done-condition "The epic is done when the first part's six checks pass" now sits under seven first-part checks (the forge check was added), so the epic can be marked done with one check unmet, possibly the forge one round 1 asked for; the number must read seven or the sentence must name which check is excluded — .engineering/planning/epic/session-confinement.md:59
story:sandbox-spec — the acceptance moves `spec/system.yaml` to `ess/24` ("the newest format, which `spec/system.yaml` moves to") although no sandbox declaration needs it and no sentence of the epic asks for a format migration; the story should validate on the current `ess/22` or name the sandbox declaration that needs `ess/24` — .engineering/planning/story/sandbox-spec.md:50
```

**Promises, traced.** I extracted 13 first-part promises: 6 from the Outcome and 7 from the Acceptance. I traced 10 to the set and 3 to stories outside this round.

| Promise | Claimed by |
|---|---|
| Every session boxed: conductor, conductor-dev, controllers, workers | `session-envelope-seam` and `session-limits-systemd` |
| CPU, memory and process limits | `session-limits-systemd` |
| Writes limited to the session's own places | `session-writes-bwrap` |
| Forge credentials read-only | `session-forge-credentials` |
| `strict` / `report` | `session-envelope-seam` |
| Acceptance 1, 2 | `session-limits-systemd` |
| Acceptance 3 | `session-writes-bwrap` |
| Acceptance 4 | `session-forge-credentials` |
| Acceptance 6 | `session-envelope-seam` |
| Usage attributed (Outcome), dashboard CPU and memory (Acceptance 5) | `instance-usage-metrics`, outside this round |
| Doctor quota report (Acceptance 7) | `quota-support-report`, outside this round |

The "Disk is not limited" exclusion is respected: no item in the set limits disk. The three later-part stories are out of this round.

No other duplicate claims or silent narrowing found. `sandbox-spec` declares types and `session start` as a "not implemented" leaf, and the seam implements them. The writes story's docs statement ("does not bound forge or network side effects") is a sequencing matter that the forge story's last bullet updates. It is not a double claim.

**What I read:** 8 artifacts, plus the epic file with line numbers. They were the epic, the five stories, `review-result:sandbox-scope-r1`, `approval-record:sandbox-priority` and `decision-blocker:sandbox-forge-tokens`. I also skimmed the heads of `instance-usage-metrics` and `quota-support-report`. Commands: `aep plan artifact show` on each, `aep plan artifact graph`, `ess specify formats`, `ess specify validate --path spec` (valid on `ess/22`), `cat -n` on the epic and `sandbox-spec`, and `AGENTS.md` rule 1.

**What I could not establish:**
- Whether `ess/24` is wanted for a reason outside the store, such as the operator preferring the newest format. Nothing I could read says so, so I filed a warning rather than a blocker.
- Out of my lane (design): the epic says "tokens the instance names", while `forge_token_env` is declared per role (`roles[].forge_token_env`). That is a granularity choice for the design critic.
- Out of my lane (acceptance): `sandbox-spec` leaves `UNMAPPED:` markers "settled before the seam is implemented", which is not a checkable condition.
- Out of my lane (parallel-safety): the shared files with wave 17.

```findings
- file: .engineering/planning/epic/session-confinement.md
  line: 59
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the done-condition \"The epic is done when the first part's six checks pass\" now sits under seven first-part checks (the forge check was added), so the epic can be marked done with one check unmet, possibly the forge one round 1 asked for; the number must read seven or the sentence must name which check is excluded"
- file: .engineering/planning/story/sandbox-spec.md
  line: 50
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance moves spec/system.yaml to ess/24 (\"the newest format, which spec/system.yaml moves to\") although no sandbox declaration needs it and no sentence of the epic asks for a format migration; the story should validate on the current ess/22 or name the sandbox declaration that needs ess/24"
```
