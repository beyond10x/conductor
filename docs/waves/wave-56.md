# Wave 56 (2026-11-18: usage limits without ais, pool token kinds, box harness updates)

The first wave of the goal that fixes the friction the agent box filed. One pull request from
`wave/2026-11-18-conductor` into `main`, forked from `main` at `f72b358a` (release 1.14.4). The
wave before is [wave 55](wave-55.md).

Width: six units, at most four at once (`controllers.max_subagents`); the host pool and limit-aware reassignment
(#206) wait for U1 and U2, which change the pool's kind and the limit record they build on.

## Units

| unit | story | issue | waits for | stage |
|---|---|---|---|---|
| U1 | pool-token-kind-files-source | #200 | nothing | on the wave branch |
| U2 | limit-resume-at-reset | #205 | nothing | on the wave branch |
| U3 | doctor-usage-probe-notifier | #205 | nothing | on the wave branch |
| U4 | box-harness-versions-dir | #192 | nothing | on the wave branch |
| U5 | box-source-checkouts | #196 | nothing | on the wave branch |
| U6 | build-run-through-slot-gate | #199 | nothing | on the wave branch |

## Decisions taken in this wave (class C)

- One token pool per host, shared by its instances (#206), in the next wave.
- Under the `files` secrets source, `conductor join` writes the token's file (mode 0600) and
  `leave` removes it, as #206 asks; no other secret is written there.
- A boxed runtime will lease pool tokens from conductord, so a token added later needs no box
  restart (next wave).
- A limited session whose process is gone is resumed from its transcript at its reset.
- A second limit after a resume backs off: 5 minutes, doubling, at most an hour.
- `model_token` stays the fallback when the pool is empty.
- A missing notifier gets a doctor row; it is not made configurable.
- `conductor join --kind` takes the variant names (`SubscriptionToken`, `ApiKey`), as every other
  enum flag does.
- A `conductor snapshot` a controller runs inside its box counts as in the box, and clones the
  missing checkouts as the runtime does.
- Under `SLOT_GATE`, `build run` records the run in a new lane, `Gate`, that waits for disk only.

## Found while building

- The sandbox the units built in refuses Unix sockets, so the suites that bind one (`placement`,
  `setup`, `api_parity`, the session-start cases of `token_pool`) ran only in CI.
- A session's `CLAUDE_CONFIG_DIR` reaches the integration tests' trust check; the tests should clear
  it as they clear `CONDUCTOR_*` (a story in the next wave).

## Outcome

To be written when the wave merges.
