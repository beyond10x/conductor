# Wave 65 (2026-11-20: an instance declares read-only tool directories for its box)

The tenth wave of the goal that fixes the friction the agent box filed: the conductor side of #197.
It forks from `main` at release 1.19.2. The wave before is [wave 64](wave-64.md). The critic
panel's result: `review-result:w65-critic-panel`.

Width: one unit. `box-registry-egress` (#198) is held behind
`upstream-blocker:substrate-aperture-bound` (a GitHub instance uses all 4 egress apertures substrate
takes), and it shares the config specification, the generated model and doctor's box rows with
U1, so it follows after U1 even once the blocker moves. #220's director is
planned after this wave. Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | box-tool-roots | #197 | merged |

## Outcome

U1: an instance names host directories in `box.tools`; the box mounts each read-only and reads
it before the binary's and the harness's, so a directory holding them takes their place. A
missing one is left out and doctor names it on the `box <name> tools:` row; one that holds the
config file or conductord's directory is refused (`placement.tool-directory-shared`). `box.env`
goes into the box's start, and `config validate` refuses `HOME`, `PATH`, `CLAUDE_CONFIG_DIR`,
`XDG_STATE_HOME` and `CONDUCTOR_*` at `instances[i].box.env.<name>`. `box.egress` is reserved
for `box-registry-egress` (#198), which stays held upstream.

In the box: the specification, the HTTP binding, a fresh generation, `cargo fmt --check`,
workspace clippy, the docs check and the touched suites; the placement tests that bind a Unix
socket are proven by CI. Since the box restart, sccache cannot start in the box, so these builds
ran without it.
