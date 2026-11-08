# The v1 wave plan

The waves that deliver `epic:architecture-next` on the integration branch `v1`
(`docs/design/architecture-next.md` § 14). Each wave is one page beside this one
(`v1-wave-<k>.md`), its integration branch `wave/v1-<k>` is cut from `v1` and merged back into `v1`
when its gate is green. Nothing merges into `main` until the cutover.

Derived on 2026-11-08 from the active stories of the epic and their declared `depends_on` edges
(`aep plan artifact graph`). `aep plan artifact waves --kind story --status active` places a story
only once its typed scope entries exist; every story of the epic but `spec-v1` was unassessed when
this page was written, so the typed scope of a later wave's stories is written at that wave's
stage 1 and the verb's answer re-checks the wave then. The verb never adds a story to a wave; the
order below is the order the edges allow, and the design's § 14 order where the edges leave a choice.

| wave | stories | objective | status |
|---|---|---|---|
| 1 | spec-v1 | the whole v1 system modelled in `spec/`, the model regenerated; every other story reads its domain here | implemented |
| 2 | daemon-reconcile, store-shared, terminal-log-name | the daemon that reconciles instances; the one shared database; the terminal log's name | implemented |
| 3 | instance-api, config-globals, instance-upgrade | the instance runtime with its API; the config levels and `PutConfig`; upgrade with rollback | implemented |
| 4 | federated-views, agent-kinds, secrets-source | the global API's federated routes; the agent trait with `claude-code`; secrets as slots | implemented |
| 5 | tui-readonly, tuner, daemon-remote, placement-substrate-local, token-pool, host-roles-by-daemon | the TUI; the tuning grant; the daemon on a remote box; a substrate box per instance and agent; the token pool; the host roles under the daemon, after the substrate (design § 14 orders placement before host roles) | implemented |
| 6 | placement-substrate-remote | a remote substrate daemon over HTTPS; attach as a stream | implemented |
| cutover | — | the live instance moves to `v1`'s binary and config once; `v1` merges into `main` after one day; release 1.0.0 | planned |

Stories of the plan: 18. Active stories outside the plan (the v0 line's backlog, delivered on
`main` by its own waves) are not candidates for a wave of this plan.

## How a wave lands on `v1`

Every wave merges into `v1` through a pull request from its wave branch, and its closing commit
(the story's evidence and move, the wave page's outcome) is on the wave branch before that merge
(operator, 2026-11-08, option A). The bot route refuses a direct push to `v1` once `v1` holds a
merge commit GitHub made rather than the bot.

## How the coordinator decides

A decision with a safe default (one that can be undone) is taken by the coordinator, done, and
reported afterwards as a decision taken, with its reason. The coordinator stops for the operator
only at a wave's approval and before an action that cannot be undone (operator, 2026-11-08). A
question with a safe default once left the plan idle for three and a half hours.

The operator approved every remaining wave of this plan ahead of time (2026-11-08). Each wave
starts when the one before it closes, with no approval stop; the coordinator still stops before
an action that cannot be undone, such as the cutover onto `main`.

No unit and no wave runs the full gate locally (operator, 2026-11-08). A unit runs only the checks
its change touches: `ess specify validate`, `task regen` when the specification changed, the test
files it added or changed, the unit tests of the modules it changed, and `cargo fmt --check`. The
full gate runs once, at the end, in CI.

Every placement goes through substrate, `systemd` included (operator, 2026-11-08): conductor
confines nothing itself. What substrate cannot serve is filed as a substrate story; conductor
keeps no fallback of its own.

## Backlog

Drafts filed from findings during this plan's waves, each in `draft` until the operator accepts it:

- none yet
