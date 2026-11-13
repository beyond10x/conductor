---
format: aep.planning-md/3
id: story:anonymous-fetch-worktree-config
kind: story
status: draft
title: The anonymous fetch reads the worktree's configuration file and follows no rewrite of the environment
relations:
- informed_by: story:anonymous-pod-no-forge-secret
scope:
- confidence: cited
  path: crates/conductor/src/collect/checkouts.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
revision: 2
---
## Why

Before an anonymous fetch (`forge_access: anonymous`) the runtime reads the checkout's own
configuration for keys the fetch would send or follow (an extra header, a cookie file, a TLS
client certificate or key, a URL rewrite), removes them, and refuses the fetch when one stays
(`without_own_credential` in `crates/conductor/src/collect/checkouts.rs`). It reads with
`git config --local --includes`, which does not read `.git/config.worktree`
(`extensions.worktreeConfig`). A security review of story:anonymous-pod-no-forge-secret showed by
a case that an extra header written there is sent:

```
the anonymous fetch sends the worktree's header (2 requests)
```

Nothing is known to write a checkout's worktree configuration in a pod: conductor clones the
checkout itself and no credential holder writes its configuration. The path existed before that
story.

## What

- The read before an anonymous fetch covers the worktree's configuration file too, and so do the
  removal and the refusal.

## Acceptance

- The review's case
  `the_anonymous_fetch_sends_no_header_of_the_worktrees_configuration` fails before the change and
  passes after it: with `extensions.worktreeConfig` on and an extra header in
  `.git/config.worktree`, the anonymous fetch sends no such header or is refused with a line that
  names the key and never its value.
- A fetch of an instance without the member leaves both configuration files byte for byte.

## Also read

- A later review of the same story showed that an `insteadOf` rewrite given in the process's own
  environment (`GIT_CONFIG_COUNT` and its pairs) is followed by the read of the origin's URL and
  by the fetch alike; a rewrite there into a URL with a login sends it (`4 requests, a login:
  true`). The pod of an anonymous instance keeps only proxy and CA pairs of that environment, so
  no pod carries one; a runtime outside a pod could. The story takes it in: the anonymous route
  follows no rewrite of the environment, or refuses while one is there.
- The plain `ssh` the anonymous route gives git still reads the home's ssh configuration file
  and its default identity files. The anonymous clone is over HTTPS; for an `ssh` origin the
  story gives `ssh` no configuration file and no identity (`-F /dev/null -o IdentitiesOnly=yes`),
  with a run that shows what it changes.
