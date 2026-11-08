# v1 wave 5: the TUI, the tuner, the remote daemon, the token pool, the substrate box, the host roles

Wave 5 of the v1 plan ([`v1-plan.md`](v1-plan.md)). Six stories. `wave/v1-5` is cut from
`wave/v1-4` after its last merge; wave 4 merged into `v1` as pull request 112.

## Units

| unit | story | files (scope) | branch |
|---|---|---|---|
| N | `story:tuner` | a new `src/tune.rs`, `store/proposal.rs`, the daily tick in `run/dispatch.rs`, `spec/domains/improvement.yaml` | `impl/tuner` |
| P | `story:token-pool` | a new `src/token.rs`, `store/token.rs`, the token routes in `daemon/api.rs`, `spec/domains/token.yaml`, `config.yaml` | `impl/token-pool` |
| R | `story:daemon-remote` | `daemon/{install,units,mod,api}.rs`, a new `daemon/browser.rs`, `src/login.rs`, `src/remote.rs`, the `--daemon` global | `impl/daemon-remote` |
| U | `story:tui-readonly` | a new `src/tui/`, `main.rs`, `attach.rs`, the architecture page's TUI section | `impl/tui-readonly` |
| L | `story:placement-substrate-local` | a new `src/placement.rs`, `store/placement.rs`, `daemon/reconcile.rs`, `doctor.rs`, `build.rs`, `b10x.toml` | `impl/placement-substrate-local` |
| H | `story:host-roles-by-daemon` | a new `daemon/host_roles.rs`, `daemon/units.rs`, `confine/systemd.rs` (the host slice), `Taskfile.yml`, `init`, the host profiles | `impl/host-roles-by-daemon` |
| C | CI failures on `v1` | `Taskfile.yml`, `crates/conductor-docs/src/config.rs` | `fix/v1-ci` |

N started from `wave/v1-3`, P and R from `wave/v1-4`, the others from `wave/v1-5`. Merge order: P,
N, C, R, U, L, H. R merged P's branch first and keeps one login module.

## Decisions

The decisions that changed what a story's acceptance says:

- The tuner is deterministic and runs in the instance's daily tick, applying through
  `put_config` as the tuner agent; one change at a time, kept or reverted after seven days.
- The token reaches the harness only through the `apiKeyHelper` of its settings copy.
- `conductor login` and the routed commands take the global `--daemon` (or `CONDUCTOR_DAEMON`), no
  positional URL. The dashboard logs in through Identity with a short-lived HttpOnly cookie.
- Placement: `adapter: linux` opens a substrate box through `substrate-daemon` (substrate 0.16.1);
  `adapter: systemd` stays on conductor's own units, and `confine/systemd.rs` stays.
- The host roles start under the daemon on the systemd path in `conductor_host.slice`; the
  substrate placement plugs into `Units::start_host_role`.

## Outcome

| unit | result |
|---|---|
| N tuner | settings proposals after the daily tick; applied inside the grant, refused with a finding outside it |
| P token-pool | `join` and `leave`; leases at agent start; a usage limit moves the next start to another token |
| R daemon-remote | `daemon.loopback`, the system unit, `conductor login`, `--daemon`, the browser login, approvals by operator |
| U tui-readonly | `conductor tui` over the global API: instances, sessions, settings; attach on a local daemon |
| L placement-substrate-local | a `linux` instance's runtime runs in a substrate box; the placement map; the substrate pin |
| H host-roles-by-daemon | the daemon keeps the supervisor and the teacher running; `ask` reaches the supervisor |
| C CI fixes | the dashboard task's values are quoted; the config page shows the built-in defaults |

Deviations from the plan:

- Waiting on substrate (its 0.16.1 API): agents as PTY sessions inside the box (the SDK sends no
  writable roots), per-box secret values, renewing a box past 24 hours, and removing
  `confine/systemd.rs` (no adapter selection on the wire).
- The fix unit started from `v1` as GitHub built it; its commit was cherry-picked onto
  `wave/v1-5`, whose history differs from GitHub's after each data-API publication.
- Host-role actions are printed, not recorded as Reconciliations: that record names an instance.
