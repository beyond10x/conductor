---
format: aep.planning-md/3
id: story:retro-submit
kind: story
status: draft
title: The engineer sees exactly what leaves and confirms it before a Slack message or GitHub issue is sent
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:retro-spec
revision: 2
---
## Why

`docs/design/user-retro.md` (approved 2026-10-13): nothing leaves until a person types send, and only to a private repository.

## Acceptance

- the confirmation prints the exact bytes and the count of replacements by kind;
- without `send` typed (or `--yes`), no target is called (a fake `gh`/`glab` records no call);
- `conductor config validate` refuses `retro.targets.github.repository` when the forge reports it public (a fake forge answers visibility);
- with GitHub and Slack configured, the issue is created under the engineer's login and the Slack message holds only its link and title;
- the Slack post goes through the instance's existing reports route.

## Scope

`crates/conductor/src/retro/submit.rs` (new), config spec and `config.rs`, tests.
