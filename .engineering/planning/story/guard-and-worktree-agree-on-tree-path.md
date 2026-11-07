---
format: aep.planning-md/3
id: story:guard-and-worktree-agree-on-tree-path
kind: story
status: active
title: The guard and worktree create agree on one tree path per checkout
summary: trees made by worktree create are denied by the guard on grouped checkouts
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T10:29:01Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T10:29:01Z", actor: "human:timo", revision: 4}
---
## Why

Reported 2026-11-07 by controllers on two instances whose repositories are on a self-hosted GitLab:
`worktree create` places a managed tree at a path the guard refuses, so every controller there
works around it with a plain `git worktree add` under the guard's path.

- A checkout at `<root>/<group>/<repo>`: `worktree create` made `<trees>/<repo>/<id>`, while the
  guard allows only `<checkouts.trees>/<group>/<repo>/` (`places.trees.join(&self.repository)`,
  `crates/conductor/src/guard/rules.rs:993`, the repository being its path under the checkouts
  root): Edit and `cd` there were denied.
- A checkout three folders deep, `<a>/<b>/<repo>`: its tree `<trees>/<repo>/<id>` was denied as
  reaching another repository, the one whose checkout path is `<repo>`: the tree folder's name was
  resolved to a different repository.
- An instance whose config sets `checkouts.trees` to its own directory: `worktree create` put the
  tree under another instance's trees directory, and `git add` there was denied as outside the
  instance's managed worktrees.

Reported, not verified here: where `worktree create` takes its root and folder name from.

## Acceptance

- For a checkout at `<checkouts.root>/<path>`, `worktree create` run by a session of the instance
  makes its tree under `<checkouts.trees>/<path>/<id>`, and the guard allows Edit, `cd` and `git`
  in it; the same holds for a checkout one, two and three folders deep.
- A tree of one repository never resolves to another repository whose path ends in the same name.
- Two instances with different `checkouts.trees` each get their trees under their own directory.
