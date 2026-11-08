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
| 1 | spec-v1 | the whole v1 system modelled in `spec/`, the model regenerated; every other story reads its domain here | proposed |
| 2 | daemon-reconcile, store-shared, terminal-log-name | the daemon that reconciles instances; the one shared database; the terminal log's name | planned |
| 3 | instance-api, config-globals, instance-upgrade | the instance runtime with its API; the config levels and `PutConfig`; upgrade with rollback | planned |
| 4 | federated-views, agent-kinds, secrets-source, host-roles-by-daemon | the global API's federated routes; the agent trait with `claude-code`; secrets as slots; the host roles under the daemon | planned |
| 5 | tui-readonly, tuner, daemon-remote, placement-substrate-local, token-pool | the TUI; the tuning grant; the daemon on a remote box; a substrate box per instance and agent; the token pool | planned |
| 6 | placement-substrate-remote | a remote substrate daemon over HTTPS; attach as a stream | planned |
| cutover | — | the live instance moves to `v1`'s binary and config once; `v1` merges into `main` after one day; release 1.0.0 | planned |

Stories of the plan: 18. Active stories outside the plan (the v0 line's backlog, delivered on
`main` by its own waves) are not candidates for a wave of this plan.

## Backlog

Drafts filed from findings during this plan's waves, each in `draft` until the operator accepts it:

- none yet
