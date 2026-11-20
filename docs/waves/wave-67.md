# Wave 67 (2026-11-20: a rate-limited forge read waits; the box's records commit finds its bot key)

The twelfth wave of the goal that fixes the friction the agent box filed: #241 and #242. It forks
from wave 66's head. The wave before is [wave 66](wave-66.md). The critic panel's result:
`review-result:w67-critic-panel`.

Width: two units, disjoint in scope (collectors and their design note; the records commit, doctor
and the box guide). `collector-failure-stays-its-own`, which takes the half of the superseded
retry story that is not a retry, is drafted for a later wave. Rust builds run one at a time (one
lock, one build per box). Both units change the planning store, so the generated status page is
regenerated after the second merge rather than merged by hand.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | forge-rate-limit-retried | #241 | merged |
| U2 | box-records-commit-config-home | #242 | merged |

## Outcome

U1: a `glab` or `gh` read whose standard error names `HTTP 429`, `502` or `503` is retried
after 5, 15 and 45 seconds (each wait at most 60 s, 65 s in all), and every forge read of
the collector waits for the same instant; a wait that would end past the collector's deadline
is not taken, and a read still refused names the forge, the command and `rate limited (HTTP
429)`. git, aep and tar keep their one immediate retry. Each GitLab commit's paths are kept in
`<cache>/forge-diffs.json`, written to a temporary file and renamed into place, so a later snapshot
reads only the diffs it has not read. The waits go through one seam, so the tests take none.

U2: the records commit runs `b10x-gates` with the instance's `gates_policy` as `B10X_GATES_POLICY`
when one is set, and in a box with `XDG_CONFIG_HOME` set to the one `box.env` names, else
`<cache>/xdg-config`; the box's start environment and an unboxed instance's commit are
unchanged. Doctor names, for a boxed instance on `forge.route: gates`, each of the bot route's
two files missing under that directory and a `gates_policy` under no directory the box binds;
it looks at the paths only.

In the box: the specification, the HTTP binding, a fresh generation, `cargo fmt --check`,
workspace clippy, the docs check, the library tests and the collector, snapshot, records, doctor,
config, box_session, daemon_pods, no_organization, isolated_home, profiles and roles suites.
