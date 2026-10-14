# Measured self-improvement

Status: approved by the operator on 2026-10-13, with four changes made in this revision (the
gateway, the K11 and per-task baselines, the evidence kept beside the records, stories for I4 and
I7). A retrospective of conductor over its first week (2026-10-06 to
2026-10-13), and a loop that keeps measuring and improving it. No story is delivered before the
operator approves this design.

Sources: two read-only analyses over the instance's records (decisions and dispatch logs, charters,
hand-overs), the forge history of the instance's repositories (PRs, CI runs, releases) and 320
session transcripts (50 conductor, 6 conductor-dev, 264 controller sessions), and a per-task analysis
of the same transcripts. Every number below names its instrument. The evidence (tables, data and the
scripts that computed them) is kept in the instance's state directory beside the records
(`<state>/conductor-dev/retro-2026-10-13/`), not in this repository, because it cites session ids.

**Scope.** Every session of the instance and every path a message takes: conductor, conductor-dev,
the controllers and their workers, and the operator's **gateway**: the Remote Control session the
operator reaches conductor through, and its message path (operator → gateway session →
cross-session message → conductor, and back). The gateway's sessions are measured like the others
(K3 corrections, K8 waiting, K9 session loss), and each relay through it is a message round trip
(K13 below), so a message held, dropped or delayed on that path is visible.

## 1. Vision

Conductor turns the operator's intent into merged and released work across many repositories while
the operator spends time only on what needs a person: direction (the north star and its goals),
class-O decisions, and the few steps only their hands can take. Every other stop is conductor's to
remove, and conductor-dev removes it by measuring where the loop loses time or needs a person, and
changing the product so it does not happen again. This is the north star's "the operator decides
direction, conductor runs the loop" made measurable.

## 2. KPIs

Each KPI: what it counts, the instrument that computes it, the baseline (2026-10-06..13), and a
target for the end of the next two weeks.

| KPI | Definition | Instrument | Baseline | Target |
|---|---|---|---|---|
| K1 operator touches per merged PR | operator-decided decisions + class-O escalations + class-H items, ÷ PRs merged | decision store + forge PRs | **0.245** (111 / 453); 0.21 without the 16 withdrawn ledger items | ≤ 0.10 |
| K2 unnecessary escalations | share of O and H items withdrawn, moot or resolved without the operator | decision store (`Withdrawn`, reasons) | **O: 33 %** (6/18); ledger H: 76 % (16/21) | ≤ 10 % |
| K3 operator corrections | human turns that correct conductor or conductor-dev, per day | session transcripts (turn classifier) | **38** in 7 days; peak day 92 human turns, 71 to conductor-dev | ≤ 2 per day |
| K4 intent → merged | dispatch sent → done, median / p90 | dispatch log | **0.75 h / 4.17 h** | keep median, p90 ≤ 3 h |
| K5 merged → released | PR merged → release tag, median / p90; merged and never released | forge + snapshot releases | **0.79 h / 21.97 h**; 44 PRs unreleased | p90 ≤ 4 h; unreleased only by decision |
| K6 rework | failed CI runs per merged PR; PRs closed unmerged; patch releases within 24 h of a release | forge runs and releases | **0.18** (0.46 on 10-13); 1.5 %; **20 %** (conductor 4 of 10) | ≤ 0.10; ≤ 2 %; ≤ 10 % |
| K7 idle controller time | gaps > 15 min in a controller session holding an open dispatch | transcripts + dispatch log | **137.6 session-h** in 132 gaps (46.9 h after it reported blocked) | −75 % |
| K8 waiting | time sessions spend stopped on CI, disk, a slot or the operator | transcripts (`sleep`, background waits) + host ledger | **134.6 h** on background jobs; 8.9 h in sleep polls | −50 % |
| K9 session loss | usage-limit errors, sessions that never resumed, conductor restarts per day | transcripts + hand-overs | **148** limit errors, 14 lost sessions; 50 conductor sessions in 7 days (24 restarts) | 0 lost; ≤ 2 restarts per day |
| K10 guard friction | guard denials per day, and tokens spent recovering | guard decisions + transcripts | **236** denials, ~135k tokens | −75 % |
| K11 output tokens per merged PR | assistant output tokens per merged PR, taking the final usage record of each message id | transcripts (usage fields) + forge | **416,740** per controller-delivered PR (137 of 376 merged PRs mapped, 36 %) | −30 % |
| K13 relay round trip | operator message through the gateway → conductor answer; any message held or undelivered | gateway and conductor transcripts | to measure (`story:kpi-transcripts`) | set after first measurement |
| K12 defects shipped | fix releases per feature release; issues labelled bug | forge | **0.45** patch per feature release | ≤ 0.2 |

**Per-task KPIs.** A task is one merged PR, with the dispatch that produced it joined to it (the
latest dispatch to its repository before the merge, until the dispatch records its PR). They track the
organisation's long-running goal of deterministic execution through the loom SDK and AEP with the
fewest model calls. Baselines over 128 controller-delivered merged PRs, 2026-10-07..13, median / p90:

| KPI | baseline | target |
|---|---|---|
| T1 tokens per task | output 153,638 / 389,917; cache read 38.8M / 116.4M; cache write 869k / 2.53M; uncached input 542 / 1,180. Fully loaded (all 320 sessions ÷ 376 merged PRs): output 195,836, cache read 52.2M | −30 % each release cycle until flat |
| T2 time per task | dispatch → merged 1.48 h / 50.0 h; dispatch → released 10.58 h / 50.1 h (90 and 85 PRs) | p90 dispatch → released ≤ 8 h |
| T3 cost per task, list-price equivalent | $16.97 / $39.59; fully loaded $21.22 per merged PR; window total $7,977 (cache read 49 %, cache write 32 %, output 18 %) | −30 % |
| T4 LLM calls per task | 265 / 590; fully loaded 294 | −50 % (`epic:deterministic-steps`) |

Per dispatch (every controller session, merged or not, 247 tasks): median 220 calls, 1.1 h, $11
(p90 797, 5.4 h, $54). Prices are the published list prices per model as of 2026-10-13, kept with
the evidence; actual spend is by subscription. Two cautions the instrument fixes: usage must be read
from the final record of each message id (sub-agents write a streaming snapshot first, which
undercounts output eightfold), and PRs must be mapped to sessions by repository and time or by the
dispatch, since reported merges covered only 36 % of merged PRs.

Message round trips per decision were measured too: a decision request is answered in a median of
0.4 min, so decisions are not where time goes.

## 3. Friction, ranked by measured cost

| # | Friction | Cost | Evidence (kept beside the records) | Cause |
|---|---|---|---|---|
| F1 | controllers idle with an open dispatch until a message wakes them | 137.6 session-h | 132 gaps, median 34 min | a controller ends its turn with nothing armed to wake it (no wait registered, no watch) |
| F2 | waiting on CI and other background jobs | 134.6 h + 8.9 h polling | 758 sleep calls, 62 blocked sleep loops | no shared "tell me when this run concludes" — every session polls |
| F3 | usage limits | 88.6 session-h, 14 lost sessions, 6 resumes by hand | 60 parallel controllers hit one limit twice on 10-07 | dispatch ignores the shared budget; no resume on reset |
| F4 | operator corrections and re-asks | 38 corrections, 92 turns on one day | 71 of them to conductor-dev on 10-13 | conductor-dev stopping for disk, slots or approval; process rules held in prose |
| F5 | guard denials | 236 denials, ~135k tokens | reaching other repositories, the records, settings | rules the sessions do not know before they act; denials without the allowed form |
| F6 | release lag and churn | p90 22 h merged → released; 20 % patch within 24 h | conductor 4 of 10 feature releases patched within 2.5 h | releases not cut by the loop; gates compile too late (fixed by rules item 112) |
| F7 | false operator items | 21 ledger H items in one evening, 16 withdrawn | 10-13 18:29–19:32Z | protocol accounting faults (fixed in v0.10.2–0.10.3) |
| F8 | conductor restarts | 24 restarts, 37 hand-overs in 7 days | context fills at ~300k | large start reading; long sessions |
| F9 | tooling releases that break the loop | a gates release crashed every bot API call one hour after a fix release | 10-13 19:4xZ | no smoke test of a tool before machine-wide install |
| F10 | status polling by another session | ~12 full status reads in 70 min | 10-13 18:2x–19:3xZ | no subscription to state changes |

## 4. Improvements, ranked by expected KPI effect

| # | Improvement | Moves | Expected |
|---|---|---|---|
| I1 | **Registered waits**: every session that stops arms a wait (`conductor wait` on a PR's checks, a dispatch, a grant, a message) that wakes it; the watch fires it (`story:watch-waits`, `story:watch-pr-checks`) | K7, K8 | −70 % idle, polling gone |
| I2 | **Budget-aware dispatch**: conductor dispatches against the shared usage budget, staggers starts, and resumes sessions when a limit resets (`story:session-limits-systemd` for the host side) | K9, K7 | 0 lost sessions |
| I3 | **Release by the loop**: a merged PR set with no release decision is released by its controller within the wave; the board shows merged-unreleased age | K5 | p90 merged→released ≤ 4 h |
| I4 | **Guard rules shown before the act**: the guard's rules for a role are in its start reading as a short table, and every denial names the allowed command (partly done in v0.9.0) | K10, K3 | −75 % denials |
| I5 | **Tool smoke test before install**: conductor-dev installs a new tool version only after a smoke run of the commands sessions use (`gates api GET`, `aep validate`, `ess validate`), and keeps the previous to roll back | K6, K9 | no crashing tool on the machine |
| I6 | **Smaller start reading**: start reading split into what every session needs and what it reads on demand (`story:conductor-start-reading`, `story:controller-start-reading`) | K9, K11 | restarts ≤ 2 per day |
| I7 | **Subscriptions instead of polls**: `conductor status --follow` and a state-change feed other sessions read | K8 | polls gone |
| I8 | **The measurement itself** (§ 5): KPIs computed by the binary every day and shown on the board and dashboard | all | the loop below can run |

## 5. The loop

1. **Measure.** `conductor kpi` computes K1–K12 from the store, the records, the forge snapshot and
   the transcripts (the transcript measures through a reader with the same privacy rules as the
   retrospective: counts and ids, no message text). It runs daily with the snapshot and writes a
   `KpiObservation` per KPI per day. The board and the dashboard show each KPI with its target and
   its trend.
2. **Detect.** A KPI past its target for two days, or a friction signature (a guard denial repeated
   by three sessions, an operator correction of the same kind twice, a tool error repeated) becomes
   a friction finding, with its evidence ids.
3. **Propose.** conductor-dev turns each finding into a draft story citing the finding and the KPI it
   should move, with the expected effect; findings already in the store are not filed twice.
4. **Deliver.** conductor-dev delivers the stories in its waves, as today, under the same gates.
5. **Verify.** A story is `implemented` on its tests; a week later the KPI it named is read again and
   the story's body records whether it moved. One that did not move is reopened as a finding.

**What the loop changes alone, and what stays with the operator.** Retros, the loop's and the
users', are about conductor's process: their fixes go into conductor's own instructions and profiles,
never into the operator's global instruction files.

| alone (conductor-dev, reported as taken) | the operator's |
|---|---|
| product code, tests, specs, docs, profiles and adapters of the product | any change to `rules.md` that loosens a rule, and every class-O decision |
| KPI instruments, thresholds of detection, draft stories | the KPI targets and the vision (§ 1–2) |
| tool upgrades after a passing smoke test, and their rollback | a new tool, or a tool's removal |
| withdrawing operator items the product raised by its own fault | withdrawing an item a person raised |
| conductor's own instructions within its grant: the instance's `rules.md`, the conductor, controller and worker profiles, adapters and start prompts | the operator's global instruction files (their user-level agent instructions and settings): out of scope for the loop and for every retro |

## 6. Epic and stories

`epic:measured-self-improvement`, with draft stories:
1. `kpi-spec` — the KPI and friction-finding nouns, specified first.
2. `kpi-records` — K1, K2, K4, K5 from the store, records and forge snapshot.
3. `kpi-transcripts` — K3, K7, K8, K9, K10, K11, K13 from transcripts (gateway included), counts
   only, final usage record per message id, PRs mapped to sessions by repository and time.
3a. `kpi-per-task` — T1–T4 per merged PR with its dispatch joined.
4. `kpi-board` — the KPI table on the board and the dashboard, with targets and trend.
5. `friction-findings` — detection of friction signatures into findings with evidence.
6. `findings-to-stories` — conductor-dev's profile turns findings into draft stories, deduplicated.
7. `kpi-verify` — the week-later check written into the story.
8. `tool-smoke-install` — I5.
9. `release-by-the-loop` — I3.
10. `guard-rules-shown` — I4: each role's guard rules in its start reading, every denial naming the
    allowed form.
11. `status-subscription` — I7: `conductor status --follow` and a state-change feed instead of polls.
I1, I2 and I6 are carried by stories already in the store (`watch-waits`, `watch-pr-checks`,
`session-limits-systemd`, `conductor-start-reading`, `controller-start-reading`); the steps that need
no model call are `epic:deterministic-steps`.

## Left out

- Ranking or scoring people. Per-person data exists only in user retros (`docs/design/user-retro.md`):
  a report is attributed to its author for follow-up and for counting distinct engineers, never
  ranked or compared.
- Changing the operator's rules, targets or vision without them.
- Automatic merges or releases that skip a gate.
- Billing: T3 is a list-price equivalent computed from tokens, not what the organisation is charged
  (subscription); no invoices or accounts are read.
- Measuring repositories outside the instance.
