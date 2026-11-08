---
format: aep.planning-md/3
id: story:token-pool
kind: story
status: implemented
title: an engineer joins an instance and contributes their Claude token to its pool; claude-code agents lease from the pool
summary: conductor join and leave; the pool in the secrets source with owner, seat and state per token; a lease per agent start, resume on another token after a limit stop; the owner on every agent record; the controller for Remote Control on a named owner's token
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:secrets-source
- depends_on: story:agent-kinds
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T00:00:44Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-08T00:00:50Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-08T19:46:04Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-08: "some engineer of us is basically joining ... an instance ... and by doing so, they would basically hand out their token to the pool of Claude tokens. And the agents of kind Claude Code, they would make use of these tokens"; and on the Claude app as a UI: "yes, kind of option A". Design §5.3, §13 rows 10 and 11, §17.

## Acceptance

- `conductor join <daemon url> --instance <i>` logs the caller in through Identity, records a human actor as an operator of the instance with the grants `operators[].role` gives, and writes the caller's Claude token into the instance's token pool as a secret with owner, seat and state (`ContributeToken`); `conductor leave` withdraws it (`WithdrawToken`) and every agent on it is resumed on another token.
- The `claude-code` kind leases one token per agent start (`LeaseToken`: the owner's own token for an agent that owner asked for, otherwise the token with the most headroom); on a usage-limit stop it marks the token `limited until <time>` (`MarkTokenLimited`) and resumes the agent on another; with a pool of one token the behaviour is today's `auto-resume`, and `confine_systemd` and `session_attach` pass unchanged.
- Every agent record names the token's owner; `GET /v1/instances/{name}/tokens` and the dashboard list owners, seats and states, never values; no log line carries a value (a test greps the daemon's and the runtime's logs after a start and a resume).
- `roles.controller.token_owner: <operator>` starts the controller for Remote Control on that owner's token, so it appears in that owner's Claude app; without the key the lease rule above applies.
