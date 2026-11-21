# Wave 74 (2026-11-21: a guard denial names its config; spawned hooks run the spawning binary)

The nineteenth wave of the goal that fixes the friction the agent box filed. It forks from `main`
at release 1.22.2, after a triage of the 38 active stories: 19 were already on `main` and moved to
implemented with evidence, one was archived as superseded. The critic panel's result:
`review-result:w74-critic-panel`.

Width: two units, `controllers.max_subagents`; they share no file.

## Units

| unit | stories | stage |
|---|---|---|
| U1 | guard-denial-names-its-config | merged |
| U2 | spawn-pins-binary-and-config | merged |

## Outcome

A forge write the guard denies under `forge.route: gates` names the instance, its config file, the
route, whether a `gates_policy` is set and the fix for that case: a `gh` write with a policy goes
through the bot route or `forge.route: direct`; without one, set `gates_policy` or the direct
route; a `glab` write is told the bot route writes to GitHub only and to set `forge.route: direct`;
with no config file, to write one. The guard reads nothing more than before: the policy state comes
from the instance it already reads, and only the denial branch writes the text. Doctor's guard line
carries the same words.

A spawned, resumed or started session's settings copy pins the instance, the config file
(`env.CONDUCTOR_CONFIG`, which a `claude --bg` session reads where the starter's environment does
not reach) and the binary that started it, by absolute path, in every hook; a role's own settings
file that already names the instance is copied too, never changed. A binary replaced on disk loses
Linux's ` (deleted)` suffix; a binary of another name keeps the bare word. The specification's three
comments on the pin are corrected; the model is unchanged.

In the box: the guard, doctor, confine and config suites, clippy; the spawn cases that launch a
session and the run suite need a host that may create a Unix socket, and CI proves them. Five
anonymous-forge review cases fail in the box on the wave's base as well (git sends a netrc login).
