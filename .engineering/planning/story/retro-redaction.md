---
format: aep.planning-md/3
id: story:retro-redaction
kind: story
status: draft
title: The retro report is scrubbed of secrets, hosts, paths and names, with corpus and property tests
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:retro-spec
revision: 2
---
## Why

`docs/design/user-retro.md` (approved 2026-10-13): every secret, host, URL, IP, path, e-mail and deny-listed name is replaced.

## Acceptance

- over the seeded corpus `crates/conductor/tests/fixtures/retro/`, every seed of each kind is replaced and every other byte is unchanged;
- an absolute path under `~` keeps only its last component (a seeded `~/clients/<name>/x` loses `<name>`);
- a property test inserts random seeds at random positions and finds none after scrubbing;
- `b10x-gates scan-text` with the policy reports no hit in the scrubbed corpus;
- a free-text answer over 1,000 characters is refused.

## Scope

`crates/conductor/src/retro/redact.rs` (new), tests and fixtures.
