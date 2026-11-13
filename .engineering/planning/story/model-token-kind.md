---
format: aep.planning-md/3
id: story:model-token-kind
kind: story
status: active
title: An instance says which kind its model token is, and a subscription token reaches the harness as its variable
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T01:11:52Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T01:11:52Z", actor: "human:timo", revision: 3}
---
## Why

Read in the code at v1.6.0: an instance's `model_token` reaches the harness one way only. The
session's settings copy names `apiKeyHelper` (`crates/conductor/src/token.rs`, `apply_helper`),
and the helper prints the secret's value, which the harness sends as an API key.

The harness's documentation names a second kind of credential, the long-lived token of a
subscription login (`claude setup-token`), and documents it for one route only: the variable
`CLAUDE_CODE_OAUTH_TOKEN` in the harness process's environment. The same documentation says an
`apiKeyHelper` outranks that variable. So an instance whose `model_token` holds such a token has
no documented route today: the value is handed over as an API key.

## Acceptance

- The instance mapping says which kind its `model_token` is (the story names the member by the
  config domain's vocabulary; the two kinds are an API key and a subscription token), declared
  in `spec/`. The member is optional and its absence means an API key.
- For a subscription token: the harness process of every session the instance starts has
  `CLAUDE_CODE_OAUTH_TOKEN` set to the secret's value in its environment, and the session's
  settings copy holds no `apiKeyHelper`. The story states how the variable reaches a harness
  that runs in a terminal multiplexer session (which process exports it, and that it is not
  written into a unit file, a command line or the settings copy), and a test with a harness
  double reads the variable from the double's environment and the settings copy from disk.
- For an API key, and for a mapping without the member: the settings copy names `apiKeyHelper`
  as before and the variable is not set. A test that passed before the change holds this.
- The value appears in no log line, no command line, no answer of the API and no file other
  than the instance's Secret and what the secrets source already writes: a test with a marker
  value reads the runtime's log, the terminal log, the session's settings copy, the process
  list entry of the harness double and `GET /v1/instances/{name}` for the marker.
- An instance that runs as a pod is one of the cases: the pod's rendering carries the value as
  it carries `model_token` today (a key of the instance's Secret), and the kind adds no second
  copy.
- A kind the specification does not declare is refused with a declared error at
  `POST /v1/instances` and `PUT /v1/instances/{name}`.
- `website/docs/guides/run-in-kubernetes.md` names the member, says which kind takes which
  route, and says what an expired subscription token needs: a new value in the Secret and
  `POST /v1/instances/{name}/restart`, since nothing refreshes it.

## Notes

- No test uses a real token of either kind; the harness is a double. The first run with a real
  subscription token is the operator's, and the story's report says it was not shown.
- Not in this story: a pool of tokens, a second token per instance, and what the API reports
  when a token meets a usage limit.
- A placement that opens a box hands over the model token through a secret slot
  (`spec/domains/config.yaml`): the story says whether the kind applies there or is refused as
  not supported.
