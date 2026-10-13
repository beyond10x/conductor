---
format: aep.planning-md/3
id: review-result:sandbox-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, sandbox
relations:
- reviews: epic:session-confinement
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
needs-revision

The four stories claim the epic's first-part promises in the set. `story:session-forge-credentials` is not unrequested work, but it is a gap the epic does not name. The epic's Outcome and Acceptance never mention forge access, network or credentials, so the epic can finish with the story unimplemented.

**Promises extracted from the epic**

I took the epic's body (`.engineering/planning/epic/session-confinement.md`) first, before the stories.

| Promise | Claimed by |
|---|---|
| Every session is boxed, including conductor, conductor-dev, controllers and workers (lines 13-16) | `session-envelope-seam` (`session start --role`, Taskfile `conductor:start` and `dev:start`, `spawn` through the same function) |
| "CPU, memory and process limits per instance and per session" (line 17) | `session-limits-systemd` |
| "writes limited to the session's own places" (line 18) | `session-writes-bwrap` |
| "usage attributed per instance and per session" (line 19) | `instance-usage-metrics`, which is outside this round |
| "a session that cannot be boxed is not started (`strict`), or started and recorded as unboxed (`report`)" (lines 20-21) | `session-envelope-seam`; `session-writes-bwrap` adds the strict refusal when bwrap or a place is missing |
| Acceptance bullets 1 and 2: controller cgroup with limits, conductor boxed, worker in its controller's cgroup (lines 35-39) | `session-limits-systemd` live run |
| Acceptance bullet 3: Bash write outside fails, inside succeeds (lines 40-41) | `session-writes-bwrap` live run |
| Acceptance bullet 5: strict with no provider refuses (lines 43-44) | `session-envelope-seam` |
| Acceptance bullets 4 and 6 (dashboard CPU and memory; doctor on quotas) | outside this round (`instance-usage-metrics`, `quota-support-report`) |

No two stories claim the same outcome. The doctor probes differ: the seam lists providers, systemd probes delegated controllers, and bwrap probes `bwrap --ro-bind / / true`.

**Forge credentials: a gap the epic should name**

- **Traceable to the ADR.** `architecture-decision-record:external-confinement` lists "network" among the dimensions conductor confines (Decision paragraph).
- **Not traceable to the epic.** The epic's four Outcome bullets and six first-part checks do not cover it. `story:session-writes-bwrap` already defers its forge and network residue to `story:session-forge-credentials`, so the epic would be marked done with that residue unbounded.
- **The operator's request covers it.** `approval-record:sandbox-priority` reads "seam, systemd limits, bwrap writes, forge credentials" as the epic's next work.
- **Fix.** The epic should add an Outcome bullet, for example "forge writes limited to read-only credentials", and a first-part acceptance check naming `session-forge-credentials`. The check should state that its live run waits on `decision-blocker:sandbox-forge-tokens`, so the wait is stated rather than silent. The story body needs no change for this.

**What you read:** 9 artifacts, plus the epic file with line numbers. I ran `aep plan artifact show` on the epic, the four stories, the ADR, `decision-blocker:sandbox-forge-tokens` and `approval-record:sandbox-priority`. I also ran `aep plan artifact graph` filtered to session, confinement, forge and credential. I extracted 9 promises from the epic; 7 trace to an item and 2 sit with out-of-round stories. The 7 traced are the five Outcome promises plus acceptance bullets 3 and 5 as separate checks. Acceptance bullets 1 and 2 are folded into the limits-systemd row.

**What I could not establish**
- Whether `session-envelope-seam`'s Taskfile and `session start --role conductor-dev` path carries a live conductor-dev run. The epic's acceptance only checks "conductor's own session", so I did not count it as a gap.
- Out of my lane (design): `session-forge-credentials` depends on `story:gitlab-workspace` in `epic:forge-neutral`, whose dependency reason the body does not state.
- Out of my lane (acceptance/design): `session-writes-bwrap` binds `~` writable "limited by the live run's finding". That is looser than the epic's "writes limited to the session's own places", and the story states it openly.

```findings
- file: .engineering/planning/epic/session-confinement.md
  line: 18
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome ('writes limited to the session's own places') and the six first-part checks name no forge-credential or network bound, so story:session-forge-credentials, which the ADR's 'network' dimension and approval-record:sandbox-priority support, is claimed by no promise; the epic should add an Outcome bullet and a first-part acceptance check naming it, and note that its live run waits on decision-blocker:sandbox-forge-tokens"
```
