---
format: aep.planning-md/3
id: story:nested-repository-session-names
kind: story
status: active
title: A repository nested as group/repo gets a session name the broker accepts
summary: session_name writes / as -, as the workspace does
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T04:18:05Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T04:18:05Z", actor: "human:timo", revision: 3}
---
## Why

Observed 2026-11-07 with 0.19.1 plus the session-start fixes: `conductor spawn --repository
<group>/<repo>` refused every repository nested one group deep: `register session
<prefix>-<group>/<repo> with the broker: "<prefix>-<group>/<repo>" is not a session name`.
`config::session_name` wrote the repository as it is, while the broker takes a session name only
as one path word (`registry::well_formed`), and the workspace already writes `<group>-<repo>`.

## Acceptance

- The session name of `<group>/<repo>` is `<prefix>-<group>-<repo>`, or `<group>-<repo>` without
  a prefix; a repository at the top level keeps its name.
