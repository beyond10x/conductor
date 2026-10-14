---
format: aep.planning-md/3
id: story:health-command
kind: story
status: draft
title: conductor health measures sessions, delivery and the build queue
relations:
- decomposes: epic:controller-health
revision: 1
---
## Why

The probe is a Python script in a user-level skill (rules: the product runs no scripts), so only
conductor-dev can run it and nothing records its readings.

## Acceptance

- Spec first: a `HealthReport` record (window, per session: turns, input by kind, cache-read share,
  cold wakes and their tokens, output, failed tool calls, context at the last turn, rate-limit
  stops; per repository: first-parent commits on `main`, merged and red pull requests, `main` CI;
  host: queued, running, held, oldest and median wait, reservation over peak) in
  `spec/domains/observation.yaml`; `ess specify validate` exits 0.
- `conductor health [--hours N] [--format text|json]` builds it from `claude agents --json`, the
  session transcripts, git, the forge and the host ledger, stores it, and prints a table and the
  findings ranked by tokens at stake; it never prints an email, token or prompt text. Tests over
  fixture transcripts (a limit stop is an assistant entry with `isApiErrorMessage` and `error:
  rate_limit`; usage is `message.usage`, counted once per request id) hold every column.
- `conductor health --compare` prints each number against the previous stored report.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/health.rs` (new),
`crates/conductor/src/cli.rs`, `crates/conductor/tests/health.rs` (new), fixtures.
