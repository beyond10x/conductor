# Design: conductor on Agency's API

Status: proposed, 2026-10-26. Store record: `design:conductor-on-agency` (draft). Nothing here is
implemented; its epics are drafts. It does not change [`conductor.md`](conductor.md) or
[`conductor-run.md`](conductor-run.md) until it is approved; § 7 says what it would change.

The operator, 2026-10-26: "we do not nesseseraly want to replace conductor in that sense, but we
want to to be based on the same foundation". The foundation is the one the agent stack definition
draws ([`agent-stack/stack.yaml`](https://github.com/beyond10x/agent-stack/blob/main/stack.yaml)):

| edge | label in the stack definition |
|---|---|
| conductor builds-on agency | "an application on Agency's API" |
| agency builds-on loom-sdk | "runs agents through Loom" |
| takt builds-on loom-sdk, takt calls loom | "depends on loom-sdk", "opens a case on a protocol" |
| flow `agent-work` | operator → takt → loom → metaharness → harness → llm → provider |

So conductor stays the coordinator across repositories. The sessions it drives today (one Claude
Code controller per repository) become agents that Agency runs through Loom, as `LoomAgent`
resources of the `Command` variant, whose program is takt. takt delivers inside one repository;
conductor decides which repository works on what, in which order, and who answers.

## 1. Sources

| source | what it gives this design |
|---|---|
| Agency's design, [`agency/docs/design/agency.md`](https://github.com/beyond10x/agency/blob/main/docs/design/agency.md), and its seven draft epics (`epic:agency-spec` M0 to `epic:agency-deploy` M5, and `epic:conductor-on-agency`, Agency's side of this design) | the API: agents, work, teams, auth, tools; § 5.6 "How conductor uses the API" |
| the organization's accepted architecture decisions of 2026-10-22 and 2026-10-26 | control-plane converges into Agency and conductor becomes an application on it, keeping its repository, CLI, store and guard; Loom owns the agent spec `loom.agent/1`, Agency's agent definition holds one by value, and Agency writes `LoomAgent` resources when it deploys |
| loom [`docs/design/loom-agent-crd-deployment.md`](https://github.com/beyond10x/loom/blob/main/docs/design/loom-agent-crd-deployment.md) § 9a, loom 0.42.0 | the `Command` variant: `spec.image`, `spec.command`, `spec.workspace`, `spec.secretFiles`; the `spec.agent` document mounted at `/config/agent.yaml` |
| takt `design:continuous-orchestration` § 6, § 9, § 11 and `product-requirements:takt` § 6 | the case tree inside one repository; build step 8, "the root across repositories", is designed but not built, and R13 keeps every run addressable from outside |
| the autodev trial of 2026-10-26 (record kept outside version control; public trace: the draft pull request [beyond10x/takt#17](https://github.com/beyond10x/takt/pull/17)) | what a takt run inside a `LoomAgent` pod looks like, and its 23 gaps (§ 5) |

## 2. What conductor keeps

conductor keeps what no layer below it can know: the organization's direction across repositories.

| conductor keeps | today (design section) | on Agency |
|---|---|---|
| goals and which repositories serve them | `goal` group, § 4 compare | unchanged; Agency has no goal noun and should not get one (Agency design § 5.6: conductor's judgement is *what* to do next, Agency is *who runs it with what*) |
| the queue across repositories: which repository works on what next, and which waits for which | dispatch v1 in `conductor-run.md` § 3; `[NEED]` routing, § 3 and § 6 | conductor orders work items across the repositories' work inputs; a `[NEED]` becomes a work item on the other repository's input, ordered after its cause |
| waves across repositories: a change that spans repositories lands in an order (a library release before its consumer's pin) | § 5 class C "which repository waits for which" | unchanged; takt's waves stay inside one repository (§ 6 D1) |
| approvals and decision classes C, O and H, their records and reversals | § 5, `decision` group | unchanged; requests arrive from Agency (gap G5) instead of from a controller's message |
| reporting: the operator brief, `REPORT.md`, deviations only | § 4 report, `conductor-run.md` § 3 daily tick | unchanged; it reads Agency's run and work views besides the forge |
| sensing the forge: repositories, pull requests, CI, releases, planning blockers, specifications | § 4 collectors, § 8 CI and PR probes | unchanged: these facts live with their owners, not in Agency |
| repository activity | § 10 | unchanged |
| the host's disk, while agents run on a local kind cluster on the same host | § 9 Resources, the host ledger | one reservation per run (§ 6 D7) until Agency and Loom carry storage capacity (G8) |
| accounts | the usage-limit handler in `conductor-run.md` § 3 | moves: credentials and grants are Agency's (auth). conductor keeps only the class-H list of what a person must do (a login, a key) |
| conductor-dev | § 12 | unchanged |

## 3. What moves

A repository's controller session, its charter, its sub-agents and the guard around them are
replaced by an Agency-run agent whose delivery work is takt:

```text
conductor ── SubmitWorkItem / StartRun ──> Agency ── writes LoomAgent (Command) ──> loom-operator
   ^                                          |                                       |
   └──── run and work-item events (SSE) ──────┘                                pod: takt run
                                                                                 └ loom-sdk → metaharness → harness → llm
```

### 3.1 conductor's concepts on Agency's surfaces

| conductor concept | Agency surface | state of that surface |
|---|---|---|
| controller: one Claude Code session per repository, started by `conductor spawn` | **agents**: an `Agent` provisioned from an `AgentDefinition` whose `loom.agent/1` document describes a takt `Command` agent; one `Run` per dispatch | designed for in-process Loom sessions only: gaps G1, G2 |
| charter (`charters/<repo>.md`, prose) | the definition's `loom.agent/1` document: brief, models, harness session, budget, stop policy, and takt's run configuration in it (§ 6 D3) | G1 |
| dispatch: a brief, `sent → started → done/failed` | **work**: a `WorkItem` whose reference is the AEP artifact (an epic, a wave or a story) on a `WorkInput` of source `AepStore` for that repository, assigned to the repository's workforce; the `Run` that serves it | designed; settling from a takt outcome is gap G3, the repository workspace gap G4 |
| `[REPORT]` verbs | `Run` and `WorkItem` state on `/agency/events` | designed; a `Command` run's exit is gap G2 |
| `[NEED]` across repositories | a `WorkItem` on the other repository's input; the order stays conductor's | designed |
| mailbox, `session deliver`, `SendMessage` | **teams**: `Send` and `Report` (conductor's broker moves to Agency; the operator, 2026-10-22, Agency design § 8 Q4 A) | designed (M3), not built |
| `conductor.tool` `Team`, `Grant`, broker | **teams** `Team`; **tools** `ToolBinding` | designed (M3) |
| the PreToolUse guard; `bypassPermissions` | **tools** and **auth**: `Policy` → `EffectivePolicy` → metaharness decides each tool call; the pod is the boundary | designed (M3); for takt runs the boundary is the pod until Substrate confinement (takt `dependency-blocker:loom-child-confinement`, open) |
| `[DECISION-REQUEST]` from a controller | an approval from a run: metaharness `Ask` per tool call (designed), and takt's `ApprovalRequired` for a case (no surface) | G5 |
| conductor itself calling unattended | **auth**: an `Application` principal | Agency's M1 to M4 admit one local operator only: G6 |
| usage limits, account resume | **auth**: `Credential` (custody in Connectors for subscription tokens), `CredentialGrant`; `Run` usage | designed without rotation across credentials: G7 |
| build slots and disk reservations | none | G8 |
| session probes of the watch: context, silent, stalled, usage limit | `RunList`, `RunDetail`, events; takt's own stops (budget, `NoProgress`, `StepRefused`) | designed; the context hand-over has no counterpart because takt sessions are bounded steps, not long sessions |
| dashboard pages for runs and sessions | the cockpit (`epic:agency-cockpit`, M4) | designed; conductor's goal, decision and deviation pages stay |

### 3.2 Agency gaps

Each gap is a missing Agency surface conductor needs; the owner is a proposal for that repository
to accept or move.

| # | gap | proposed owner | needs below Agency |
|---|---|---|---|
| G1 | **Program agents.** An `AgentDefinition` assumes a metaharness kind and an in-process Loom session. A takt delivery is a `Command` variant: an image, an entrypoint, a workspace and takt's run configuration | agency, `epic:agency-spec` (the definition models the `Command` variant from `loom.agent/1`, not beside it) | loom: takt's configuration expressible in `loom.agent/1` (trial gap "Command variant takes opaque config files instead of a mandatory loom.agent/1 spec.agent", taken as in-document here, § 6 D3) |
| G2 | **Deployed runs.** `StartRun` drives an in-process `SessionExecutor` (Agency design § 5.4). conductor needs `StartRun` to write a `LoomAgent` resource and to see the run end, with its exit code | agency, `epic:agency-runs` | loom: a `Command` run runs to completion instead of as a restarting Deployment, and its status reports exit code and completion (two trial gaps) |
| G3 | **Settling from a program's outcome.** A `WorkItem` settles from a Loom `Proposal`. A takt run ends in an exit code (takt's registry: 0 to 8 built, 9 and 10 proposed), a case outcome, validator evidence bound to a commit, and a branch | agency, `epic:agency-runs` | takt: its outcome files stay a stable contract (PRD R13) |
| G4 | **The repository workspace.** No Agency noun says which repository, base branch and volume a run works in; Loom has `spec.workspace` | agency, `agency.work` (a `WorkInput` of source `AepStore` names the repository; the definition's deployment names the claim) | none |
| G5 | **Run-level approvals.** Agency's approval inbox carries metaharness `Ask` decisions per tool call. A takt case stops at `ApprovalRequired` (exit 4) and moves on `takt approve <case> <capability>`; Agency has no surface for that question or its answer | agency, `epic:agency-tools-policy` | takt: the pending approvals file of its root case (`design:continuous-orchestration` § 7) |
| G6 | **An application principal before the deployment milestone.** M1 to M4 admit one local operator by session cookie; conductor calls without a person | agency, `epic:agency-service` | none |
| G7 | **Credential headroom.** Several subscription credentials, a run stopped on a usage limit, resumed on one with headroom. Agency has grants and a workforce ceiling, no choice among credentials | agency, `agency.auth` | connectors: issue and refresh of subscription tokens (its OAuth2 work) |
| G8 | **Storage capacity.** A run's workspace and build output take disk (the trial's volume: 60Gi; its build output, 3.0 GiB, was cleaned by hand). `Workforce.capacity` counts runs and tokens only | agency, `Workforce.capacity` | loom: volume size and ephemeral-storage limits on the `LoomAgent` |

Not an Agency gap: forge writes from a run (push, pull request, merge). They belong to takt
(`story:fenced-forge-dispatch`, draft) and Connectors; Agency only grants the credential, with
custody in Connectors.

## 4. Migration path

Each step is done when its evidence exists; a step that has none is not done.

| # | step | owner | evidence of done |
|---|---|---|---|
| 0 | done, 2026-10-26: takt 0.12.0 ran as a `LoomAgent` `Command` resource on loom 0.42.0 in a local kind cluster | takt, loom | run 7 of the trial: 8 steps, validator records, a bot branch and the draft pull request [beyond10x/takt#17](https://github.com/beyond10x/takt/pull/17) (§ 5) |
| 1 | **first step.** A dispatch names its runner. conductor's specification gains a runner on the dispatch and the controller; `claude-code-session` is the only runner and today's behaviour | conductor, `epic:dispatch-runner` | `task check` green; new conformance scenarios for the runner field pass; every existing dispatch and spawn test passes unchanged; no command removed |
| 2 | a clean repeat of the trial: a released takt image on a released loom, no hand patch (no `kubectl patch` for seccomp, no loopback relay, no trial image), the `Command` run runs to completion and reports its exit code, one story ends with its gate green | loom, takt, llm, metaharness (their trial gaps) | a second trial record with 0 hand patches; a pull request whose checks are green |
| 3 | the `loom-agent` runner: conductor dispatches one story of one repository as a `LoomAgent` run on the local kind cluster, reads the resource's status for started and done or failed, reserves the run's disk in the host ledger; the pull request is opened by the bot route on the host (fenced) | conductor, `epic:loom-agent-runner` | the dispatch log's `sent`, `started` and `done` lines for that dispatch, written by `conductor run` from the resource's status; the bot pull request; one host-ledger reservation released at the run's end |
| 4 | two repositories and a wave: conductor runs one wave per repository (takt's `one-session-dispatches-the-waves`) in two repositories with one `[NEED]` between them, the second ordered after the first's release | conductor, `epic:cross-repository-waves` | two runs; the second started only after the first's dispatch was `done`; one `[NEED]` routed and recorded; `conductor records check` exit 0 |
| 5 | Agency M0 to M2, with G1 to G4 and G6 | agency, loom | Agency's own M2 acceptance (one unattended run from `SubmitWorkItem` to `SettleWorkItem`), run here with a takt `Command` definition |
| 6 | the `agency` runner: the dispatches of steps 3 and 4 go through `SubmitWorkItem` and `StartRun`; conductor reads `/agency/events`; its records keep the decisions and dispatches | conductor, `epic:agency-runner` | the step-4 scenario repeated through Agency; `RunDetail` shows both runs; each dispatch's state equals its work item's; `conductor records check` exit 0 |
| 7 | messages, approvals and credentials move: `Send` and `Report` replace the mailbox for Agency-run agents; a takt approval is answered by conductor through Agency (class C) or escalated (O, H); credentials are grants (Agency M3, G5, G7) | conductor, agency | one takt `ApprovalRequired` answered through Agency and recorded as a conductor decision; one run stopped on a usage limit resumed on another credential |
| 8 | the default flips: every Active repository is dispatched through the `agency` runner; `claude-code-session` stays a runner an instance may name | conductor | 7 consecutive days with 0 controller sessions spawned; every dispatch ends `done` or `failed` with a recorded outcome; accepted stories per day and tokens per accepted story reported against the step-3 baseline |

The first step is step 1. It needs nothing from another repository and changes no behaviour, so
it runs while step 2 runs in loom and takt. Step 3 waits for step 2: a runner built on a run that
never ends would read a pod that sleeps for ever (§ 5) as a run in progress.

## 5. What today's trial proved, and what it did not

Numbers from the trial record of 2026-10-26.

| proved | figure |
|---|---|
| takt runs as the program of a `LoomAgent` `Command` resource, in a pod, with its workspace on a volume | resource admitted (`SpecAdmitted`, variant `Command`), loom 0.42.0, takt 0.12.0 |
| a takt case runs model steps, tests and gates inside the pod and records validator evidence | run 7: 8 steps, 632,481 tokens; `tests.run` passed 3 times (117, 117 and 118 passed, 0 failed); validators recorded `test_result` pass ×3 and `gate_result` fail ×2 |
| the run's commits leave the pod and reach the forge only through the bot route | branch fetched from the pod by `git bundle`; 5 commits, author and committer `b10x-bot[bot]`; draft pull request [beyond10x/takt#17](https://github.com/beyond10x/takt/pull/17), not merged |
| a case resumes across runs | the case was resumed with `--case` from run 2 on |
| what is missing is listable | 23 gaps: 9 loom, 10 takt, 2 llm and harness, 1 metaharness, and 1 in the trial's own setup notes, fixed there |

| not proved | why |
|---|---|
| an accepted story | the `citations` gate failed twice: 24 `read from` citations of one file unresolved after the edits (1,132 citations in the repository, 1,108 resolved); the run stopped on its iteration budget (exit 3) |
| an unattended run | 6 of 7 runs stopped on setup: missing system packages (run 1), the toolchain off the gate's path (2), the model endpoint's session header (3), a harness version pin (4), a credential file's mode (5), no gates policy (6). Run 7 needed a seccomp and AppArmor `Unconfined` patch, a loopback relay for the session header, the fourth trial image, a story moved `draft → proposed` by hand, and a case resumed past `--iterations 1` |
| a run that ends | the `Command` pod runs as a Deployment; the wrapper ends in `sleep infinity`; no status carries the exit code |
| confinement | the pod ran unconfined; Substrate confinement is a later slice (loom design § 9a, takt's open `loom-child-confinement`) |
| forge writes from the run | fenced: the branch was pushed and the pull request opened from the host |
| more than one repository, or Agency | one repository; Agency has no service yet |
| a vendor harness under metaharness | the run used the native harness on a hosted model endpoint |
| cost or time per accepted story | no story was accepted; the record gives no wall-clock times. I don't know |

## 6. Decisions

Taken in this design, each reversible; none needs the operator. A choice that would change an
accepted architecture decision, a contract a consumer depends on, money, providers or credentials
is not taken here (§ 9).

| # | question | options | taken | why |
|---|---|---|---|---|
| D1 | Who orchestrates across repositories? | A. conductor; takt stays inside one repository. B. takt's build step 8, "the root across repositories"; conductor only reports. C. both | **A** | the stack definition makes conductor the cross-repository coordinator; takt's own default is one dispatcher per repository (`design:continuous-orchestration` O5) and its PRD keeps runs addressable from outside (R13), which is what conductor needs. takt's step 8 is takt's to re-cut; this design asks only that a run accept a base branch and a story from outside (the trial gap "Name the story to run") |
| D2 | How does conductor reach runs before Agency has an API? | A. wait for Agency M2. B. an interim `loom-agent` runner: conductor writes `LoomAgent` resources itself through the cluster's command line, a program `conductor doctor` checks, behind the step-1 seam. C. keep Claude Code controllers until Agency M4 | **B** | it earns runtime evidence (steps 3 and 4) before Agency's API is fixed; Loom runs without Agency, and the accepted decision that Agency writes `LoomAgent` resources does not reserve that write to Agency |
| D3 | Where does takt's run configuration live? | A. in the `loom.agent/1` document the pod reads at `/config/agent.yaml`. B. opaque configuration files beside the document | **A** | one agent schema, which Agency's definition holds by value (accepted 2026-10-26); B would give Agency a second document to hold and so reopen that decision. The trial gap "One config contract with loom: read /config/agent.yaml" is this option |
| D4 | What is one run? | A. one run per dispatch to a repository, running takt's waves until its frontier or budget ends. B. one run per story. C. one long-lived agent per repository taking work items | **A** | it is today's dispatch-per-controller, takt dispatches the waves itself (`story:one-session-dispatches-the-waves`, implemented), and a run that ends is what G2 asks of Loom |
| D5 | Who answers a run's approval? | A. conductor, by its classes: C it answers, O and H it escalates. B. the operator, always, in the cockpit | **A** | the decision classes of `conductor.md` § 5 do not change with the runner |
| D6 | How do a run's forge writes reach the forge? | A. fenced: the run leaves a branch, the bot route on the host pushes and opens the pull request. B. the bot's key as a Secret in the cluster. C. a Connectors-held credential through the effect port | **A** until C exists | A is how the trial ran; C is takt's `fenced-forge-dispatch` and Connectors' custody; B is allowed for the local kind cluster (the operator, 2026-10-26) and kept as a fallback if A blocks step 4 |
| D7 | Who reserves disk for a run? | A. conductor's host ledger, one reservation per run, while the cluster is on the host. B. Agency's workforce capacity. C. pod limits only | **A** now, **B** and **C** with G8 | the host ledger already holds every build on the host; a local cluster shares the host's disk |
| D8 | What happens to Claude Code controllers during the migration? | A. kept as the `claude-code-session` runner, per repository, until that repository passes step 8. B. cut over at once | **A** | no command or contract breaks; a repository falls back by naming the runner |

Decided by the operator, recorded and not reopened:

| question | the operator's answer |
|---|---|
| is conductor replaced? | no: "we do not nesseseraly want to replace conductor in that sense, but we want to to be based on the same foundation" (2026-10-26) |
| which model route the agents use | not a blocker: "it goes through meta-harness, and meta-harness can equally control codex or Claude code, so we are not really stuck on that" (2026-10-26); subscription tokens are issued and refreshed by Connectors (2026-10-25) |
| does conductor's broker move to Agency? | yes, Agency design § 8 Q4 A (2026-10-22) |
| does takt link to Agency? | no, Q7 B (2026-10-22). This design keeps that: Agency deploys takt as an agent's program; takt never calls Agency |
| keys in the local kind cluster | the model key and the bot's key as Secrets there (2026-10-26) |

## 7. Against the current design

### Why the current multi-repository design pauses

The operator paused the instance on 2026-10-21 until its new design. The reasons are measured in
[`conductor-run.md`](conductor-run.md) § 1 and [`roles-and-surfaces.md`](roles-and-surfaces.md) § 1:
a model session relayed what a program could (236 of 268 messages in ten hours), ran at a median
context of 432k tokens, and the rules governing it were 98 KB of prose. `conductor run` took the
loop out of the model. What it left is the repository controller: a long-lived Claude Code session
per repository, in `bypassPermissions`, fenced by a command-line heuristic (§ 7), handed over at a
context threshold, and done when it says so. takt replaces exactly that job with bounded steps, a
case on disk and validator evidence bound to a commit; Loom and Agency give it a run, a boundary
and an owner of credentials.

### What survives, what pauses

| `conductor.md` section | on Agency |
|---|---|
| § 1 what conductor is; facts live with their owner | survives; "a session" is already retired by `conductor-run.md` |
| § 2 roles | operator, conductor (now the daemon) and conductor-dev survive; **repository controller** and **worker** become an Agency-run agent and takt's roles |
| § 3 topology | the star survives as conductor ↔ Agency; SendMessage and the mailbox to sessions pause for Agency-run agents (Send and Report replace them) |
| § 4 the loop, collectors | survive; dispatch writes work items instead of spawning |
| § 5 decision classes, authority | survive unchanged |
| § 6 messages | the first-line grammar survives as the body of `Send` and `Report`; `[HANDOVER]` and `[RESOURCE]` have no use for Agency-run agents |
| § 7 the guard | pauses for Agency-run agents (metaharness and the pod decide); stays for `claude-code-session` |
| § 8 the watch | CI, pull-request and disk probes survive; session probes pause for Agency-run agents |
| § 9 config, resources | survive; the config gains a runner per instance or repository; the host ledger reserves per run (D7) |
| § 10 the store | survives; it records runs by their Agency or `LoomAgent` id |
| § 11 the CLI, profiles | the CLI survives with no command removed; `repo-controller.md` and charters pause for Agency-run agents |
| § 12 conductor-dev | survives |
| `conductor-run.md` | survives: it is the loop that calls Agency |

Epics this design would pause, proposed and not moved here: `epic:session-confinement`,
`epic:session-attach`, `epic:agent-tools`, `epic:role-names` and `epic:harness-routes` (their
subject moves to the pod, metaharness, Agency's tools and Loom's agent document);
`epic:usage-continuity` (G7). The rest stay.

## 8. Draft epics

| epic | outcome | steps |
|---|---|---|
| `epic:dispatch-runner` | a dispatch and a controller name their runner; `claude-code-session` is the only one and today's behaviour | 1 |
| `epic:loom-agent-runner` | conductor dispatches to a takt `LoomAgent` run on a local kind cluster and records its life from the resource's status | 3 |
| `epic:cross-repository-waves` | conductor orders runs across two or more repositories by their `[NEED]`s and releases | 4 |
| `epic:agency-runner` | conductor dispatches through Agency's API and reads its events; messages, approvals and credentials go through Agency | 6, 7, 8 |

Agency's side is its own `epic:conductor-on-agency`; the gaps of § 3.2 go to Agency's and Loom's
stores, not this one.

## 9. Not in this design

- Running agents anywhere but a local kind cluster on the operator's host: a hosted cluster brings
  spending and credentials leaving the host, which are the operator's to decide.
- Removing or renaming any conductor command, or retiring the `claude-code-session` runner.
- Agency's own design, its milestones and the order of its epics.
- takt's cross-repository step 8 (D1 asks takt to re-cut it; the cut is takt's).
- Substrate confinement of the run (loom and takt).
- Publishing images to a registry.
