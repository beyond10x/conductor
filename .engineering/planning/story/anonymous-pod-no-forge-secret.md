---
format: aep.planning-md/3
id: story:anonymous-pod-no-forge-secret
kind: story
status: active
title: An anonymous instance's pod carries no forge credential under any entry name
relations:
- depends_on: story:forge-access-anonymous
scope:
- confidence: cited
  path: crates/conductor/src/collect/checkouts.rs
- confidence: cited
  path: crates/conductor/src/collect/repositories.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/daemon/pods.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests
- confidence: cited
  path: website/docs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T13:49:08Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T13:49:09Z", actor: "human:timo", revision: 3}
---
## Why

`story:forge-access-anonymous` says the pod of an instance with `forge_access: anonymous` carries
no credential of any forge. Two independent reads of its commit found where that does not hold.

Shown by a failing case each:

- `crates/conductor/src/daemon/pods.rs`, `manifests`: the pod leaves out `daemon.kubernetes.env`
  entries by variable name only. A second entry under another name that names the same secret as
  `GH_TOKEN` is kept, so the daemon's forge token is in the anonymous pod's Secret, its
  environment and its secret volume. The guide documents that double entry for other pods.
- `is_gits_config` in the same file does not name `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`. An
  entry of either points git at a file that may name a credential helper.
- `crates/conductor/src/guard/rules.rs`, `anonymous_write`: a push behind `env`, `timeout`,
  `nohup`, `xargs`, `find -exec`, a variable or an alias, `git-push` by path, `git subtree push`,
  `git lfs push` and a write through the API route pass the rule, while the guide says every write
  is refused before any call.

Read, not run:

- Other entries that hand git or a forge's command line a credential stay in the pod:
  `GIT_SSH_COMMAND`, `SSH_AUTH_SOCK`, `SSH_ASKPASS`, `GH_CONFIG_DIR`, and a secret-backed variable
  under any other name.
- The same filter drops the installer's git settings that are no credential (proxy, CA).
- `crates/conductor/src/collect/checkouts.rs`: the anonymous clone clears the credential helper
  and the token variables. `url.<x>.insteadOf` and `http.extraHeader` from the image's or the
  system's git configuration still apply.
- `crates/conductor/src/collect/repositories.rs`: an `include` entry that validation accepts and
  the listing refuses fails the whole listing of an anonymous instance.
- A doc comment in `guard/rules.rs` sits on the wrong item.
- A `secret_syncs` entry is not refused beside `forge_access: anonymous`.
- The loop writes a `records-commit-refused` line each day for such an instance.

## What

- The pod of an anonymous instance carries no entry of `daemon.kubernetes.env` that names a
  secret of a daemon's forge token, under whatever variable name, and none of the variables that
  point git, ssh or a forge's command line at a credential or at a configuration file.
- The installer's git settings that carry no credential stay in an anonymous pod.
- The anonymous clone runs with no `insteadOf` rewrite and no extra header from any level of
  git's configuration.
- The write refusal covers the spellings above, or the guide's sentence says what is refused and
  why the rest cannot write.
- `secret_syncs` beside `forge_access: anonymous` is refused at the offending path.
- One `include` entry the listing refuses is one member failure.
- No member of `spec/` is added or changed; descriptions follow the code where they differ.

## Acceptance

- A case renders the pod of an anonymous instance under a daemon whose `env` names the forge
  token's secret twice, under `GH_TOKEN` and under another name: the Secret, the environment and
  the secret volume hold none of it.
- A case renders it with `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM`, `GIT_SSH_COMMAND`,
  `SSH_AUTH_SOCK`, `SSH_ASKPASS` and `GH_CONFIG_DIR` set on the daemon and reads none of them in
  the pod's spec.
- A case renders it with a proxy and a CA setting and reads both in the pod's spec.
- A case clones anonymously under a configuration that names an `insteadOf` rewrite and an extra
  header and reads neither in the git call.
- A case per write spelling reads the denial naming the member, or the guide's sentence is the
  narrower one and a case holds it to the code.
- The pod of an instance without the member is rendered byte for byte as before.
- Each case fails before the change. No test calls a real forge or a real cluster.
