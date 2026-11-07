---
format: aep.planning-md/3
id: story:instance-worktree-profile
kind: story
status: draft
title: Each instance has a worktree profile matching its checkouts root and trees
summary: A root inside another profile workspace puts trees where the guard refuses them
revision: 2
---
## Why

The host supervisor, 2026-11-07, with evidence from one instance: its `checkouts.root` lies under the `workspace_root` of another profile in the worktree tool's host config, and no profile names the instance's own root. `worktree create` in a controller's checkout therefore placed three trees under the other profile's `worktree_root`, outside the instance's `checkouts.trees`. The controller guard (`crates/conductor/src/guard/rules.rs`, trees = `checkouts.trees`) then refused the controller's git writes in its own trees, and the controller fell back to plain `git worktree add`. Separately, `crates/conductor/src/config.rs:307` defaults `WORKTREES` to `.local/state/worktree/trees/example-org`, a placeholder organization, not a value derived from the instance.

## Acceptance

- `conductor config validate` (and `conductor doctor`) reports, per instance, the worktree profile whose `workspace_root` holds `checkouts.root` and whose `worktree_root` is `checkouts.trees`; with none, or with a profile whose roots differ, it names the YAML path and the profile and exits 1.
- `conductor init` (or a `conductor config worktree-profile` command) writes the missing profile for the instance into the worktree tool's config, keyed by the instance name, more specific than any enclosing profile; an existing profile with other roots is named and left alone.
- The default of `checkouts.trees` is derived from the instance name, not a fixed placeholder; `config.rs` holds no organization in a default.
- A test: an instance whose root lies inside another profile's workspace gets its own profile, and a `worktree create` from one of its checkouts lands under `checkouts.trees`, where the guard admits its git writes.
