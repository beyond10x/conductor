---
format: aep.planning-md/3
id: story:pod-git-reaches-forge
kind: story
status: draft
title: git over HTTPS in an instance pod authenticates to the instance's forge with the pod's token
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read on 2026-11-12 on the branch of `story:instance-forge-token`: an instance pod carries its
forge token in `GH_TOKEN` or `GITLAB_TOKEN`, which `gh` and `glab` read. Nothing wires git to
it: the image installs git, gh and glab and sets no credential helper and no `insteadOf`
(`Dockerfile`), no source file sets one, and the pod gets only a git identity
(`crates/conductor/src/daemon/pods.rs`, `GIT_IDENTITY`). A `git clone`, `git fetch` or
`git push` over HTTPS in the pod therefore has no credential, whichever of the two setups gave
the pod its token.

## Outcome

In an instance pod that carries a forge token, git over HTTPS to the forge of the instance's
sources authenticates with that token, and git offers the token to no other host.

## Acceptance

- For an instance with a `github` source whose pod carries `GH_TOKEN`, and for one with a
  `gitlab` source whose pod carries `GITLAB_TOKEN`, from the instance's `forge_token` or from
  `daemon.kubernetes.env` alike, the pod's environment configures a git credential helper for
  that forge's HTTPS origin and no other: `https://github.com`, or `https://<host>` of the
  `gitlab` source.
- With that environment, `git credential fill` for the forge's host answers the token, read
  from the variable at the time of the call; for any other host it answers none. A test runs
  git itself for both, with no network.
- The helper's text holds no secret, and the token appears in no file the supervisor writes
  other than the instance's Secret.
- A pod that carries no forge token, and a pod of an instance with local sources only, gets no
  helper and is unchanged, byte for byte. Where `daemon.kubernetes.env` sets `GIT_CONFIG_COUNT`
  itself, the supervisor adds nothing: the installer owns git's configuration.
- On a local cluster: in the pod of each setup, `git credential fill` for the forge's host
  answers a value whose digest equals the configured secret's, shown without printing a value.
- The guide says what a pod's git reaches with the token, and that SSH remotes are not covered.

## Not in this story

A clone, a push and a merge request against a real forge; SSH remotes; a token per source.
