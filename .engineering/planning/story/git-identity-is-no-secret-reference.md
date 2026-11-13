---
format: aep.planning-md/3
id: story:git-identity-is-no-secret-reference
kind: story
status: draft
title: An instance's git identity name is not listed as a secret reference
tags:
- from-finding
revision: 1
---
## Why

An instance's `git_identity.name` written as `secret:<x>` is listed by
`GET /v1/instances/{name}/secrets` as a key that names a secret, although the name is a plain
line and never resolved as a secret. Confirmed by reading at `v1.7.0` (`951330f4f4`); no case
was run.

- The parse takes it as text: `git_identity` is read at `crates/conductor/src/config.rs:4352-4363`,
  each member through `git_identity_problem` (`config.rs:4412-4433`). `git_identity_problem`
  refuses only an empty or over-200-byte value, a line break, a control character, `<` or `>`,
  white space at either end, and for the e-mail anything but one `@` (`config.rs:501-545`);
  `secret:x` passes as a name.
- The pod gets it as a plain value: `GIT_AUTHOR_NAME` and its kin are set to the name with
  `"value"` (`crates/conductor/src/daemon/pods.rs:416-424`). The rendered file drops
  `git_identity` (`crates/conductor/src/config/pod.rs:143`), so the secrets the pod's Secret
  gets (`pods.rs:328-331`) do not include `x`.
- The route counts it as a reference: `instance_secrets` reads the config file's text
  (`crates/conductor/src/daemon/api.rs:1301`) and hands it to `instance_view`
  (`api.rs:1322`), which lists every key `keyed` finds (`crates/conductor/src/secret.rs:939-977`).
  `keyed` collects every string under the instance's entry (`secret.rs:855-879`) that
  `secret_name` takes (`secret.rs:754-758`, `config.rs:2297-2304`), whatever its key
  (`secret.rs:773-802`). The row is `key: git_identity.name`, `secret: x`, and the route opens
  the secrets source to say whether `x` is set (`secret.rs:953-972`).
- `GET /v1/secrets` builds its rows with `referenced(text)` (`secret.rs:1000`, `:1017`), which is
  `keyed` by name (`secret.rs:843-848`), so it would name the instance under `x` too; which text
  its caller hands it: not read.
- `git_identity.email` cannot be both: `secret_name` takes no `@` (`config.rs:2298-2303`) and
  the e-mail needs one (`config.rs:533-543`).

## Acceptance

- With `git_identity.name: secret:<x>`, neither `GET /v1/instances/{name}/secrets` nor
  `GET /v1/secrets` lists `<x>` for the instance; a case writes it and reads both routes.
- A key that does name a secret (`forge_token`) is still listed in the same case.
