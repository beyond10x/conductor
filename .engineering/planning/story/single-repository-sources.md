---
format: aep.planning-md/3
id: story:single-repository-sources
kind: story
status: draft
title: A source can name one repository, with no group listing, exclude list or depth limit
summary: sources take one repository per entry, its checkout found at its path, origins compared after insteadOf
revision: 6
---
## Why

The operator, 2026-11-07: "why not just one entry per repository ... let's not even bother about
this namespace stuff".

Selecting four projects of a self-hosted GitLab instance took three group sources with exclude
lists of 9 to 60 entries, and still missed two of them:

1. `gitlab: <group>` lists `groups/<group>/projects?include_subgroups=true`
   (`crates/conductor/src/collect/gitlab.rs:493`); a project path there lists nothing.
2. `under()` (`collect/gitlab.rs:804`) needs a non-empty rest after the group, so a project path
   as the group matches no checkout.
3. `checkouts()` (`collect/gitlab.rs:837`) looks one and two directories deep, so a checkout at
   `<root>/a/b/c` never becomes a workspace member.
4. `origin()` (`collect/gitlab.rs:909`) reads the raw `remote.origin.url` and ignores git's
   `url.<base>.insteadOf`, so after a forge host moved, no checkout whose origin names the old
   host matched, though git itself reached them.

Observed again 2026-11-07 on a second instance whose repositories are on a self-hosted GitLab:
five checkouts were invisible to its GitLab sources until their origins were rewritten by hand,
because the sources compare the raw origin and ignore `url.*.insteadOf` (point 4).

Observed 2026-11-07: `conductor spawn` refuses a repository whose checkout is three directories
deep under `checkouts.root`. `checkout()` in `crates/conductor/src/spawn.rs:632` takes only
`if !matches!(steps.len(), 1 | 2)`, so `spawn --repository <a>/<b>/<c>` is refused `NoCheckout:
... is not <repo> or <group>/<repo>`; `collect::repository_of`
(`crates/conductor/src/collect/mod.rs:235`) reads at most two steps. Reported, not verified: the
activity row of that checkout prints `<b>/<c>`, and `spawn --repository <b>/<c>` finds no checkout
at `<root>/<b>/<c>`. The instance works around it with a symlink.
## Proposal

A source may name one repository:

```yaml
sources:
  - gitlab: group/sub/project     # the project's path on the forge
    host: gitlab.example.com
    path: other/checkout/path     # optional, under checkouts.root; default the project path
  - github: owner/repo
```

No group listing, no exclude list, no depth limit; the checkout is found at its path. Origins are
compared after git's `insteadOf` rewriting (`git config --get-regexp '^url\..*\.insteadof$'`, or
`git remote get-url origin`, which applies it).

## Acceptance

- A config whose sources are four single repositories, one three directories deep, collects all
  four as members.
- A checkout whose origin names an old host that `insteadOf` rewrites to the source's host matches.
- The group form keeps working unchanged.
- `conductor spawn --repository <a>/<b>/<c>` starts a controller for a checkout three directories
  deep that a source names, with no symlink, and the activity row names it `<a>/<b>/<c>`.
