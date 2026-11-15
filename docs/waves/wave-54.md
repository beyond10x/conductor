# Wave 54 (2026-11-15: a boxed runtime and its controllers do forge work)

The operator approved this wave on 2026-11-15 (`approval-record:wave-2026-11-15b-conductor`): one
story, committed on an integration branch, without a release. One pull request from
`wave/2026-11-15b-conductor` into `main`, forked from `main` at `9edbdcee` (release 1.11.0). The
wave before is [wave 53](wave-53.md).

## Units

| unit | story | waits for | stage |
|---|---|---|---|
| U1 | box-forge-work | nothing | on the wave branch |

## Outcome

- A box reaches each forge host of its instance's sources through an egress aperture of its own,
  `forge_<host>` (the host with `_` for every byte but a letter or a digit), which the operator
  declares as `substrate-daemon --egress-aperture forge_<host>=<host>:443/tcp`. The start names
  the dial-in aperture first, then `conductor`, then the forge apertures, each only where the
  daemon declares it. More than four apertures are refused at validate,
  `placement.apertures-exceeded`. `decision-blocker:substrate-sdk-single-aperture` is cleared.
- The checkouts root and the managed trees root are writable roots, `checkouts-<instance>` and
  `trees-<instance>`, in place of the read-only checkouts root; conductord makes the trees root.
- The box reads git's global configuration and the `worktree` CLI's, and keeps Claude Code's
  state (`CLAUDE_CONFIG_DIR`) and the `worktree` registry (`XDG_STATE_HOME`) in its state root.
  conductord marks the checkouts and the records trusted there before it opens the box, and
  `conductor trust` marks them too.
- `conductor doctor` prints `box <instance> forge`, `box <instance> git` and
  `box <instance> trust`, and names each flag or setting that is missing.
