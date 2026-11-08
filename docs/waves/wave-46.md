# Wave 46 (one target directory per repository)

Every managed tree built into its own `target/`, and most of each held the same dependency
artifacts: on 2026-11-07 twelve `target/` directories held 77 GB while free space was under the
disk floor. `conductor build run` now gives each repository one target directory under the
instance's cache, shared by its trees under a lock held for the whole command. One tree, one
release.

## Units

| unit | story | note |
|---|---|---|
| U1 | build-shared-target-dir | `build run` sets `CARGO_TARGET_DIR` to `<cache>/target/<repository key>` and holds `<cache>/target/.locks/<key>.lock` for the whole command; a run that waits says its queue position once; a target last built from another tree has the tree's files marked modified (submodules included), so cargo rebuilds the repository's own crates; `conductor gc` removes a target whose lock it takes |

A `build run` killed with SIGKILL releases its lock with it, while its command, in its own process
group, goes on writing the target. The lock file names that group on its second line while the
command runs; a run that takes the lock and finds the group alive lets it go and waits, and `gc`
keeps the target. A group id the kernel reuses for an unrelated group makes a run wait until that
group ended.

## Metric

Bytes of every `target/` directory under the checkouts and the managed trees
(`find -maxdepth 2|4 -name target` and `du -sb`), plus the shared targets under the cache after
the release.

| | before (2026-11-08T07:57Z) | after |
|---|---|---|
| `target/` directories | 43 | |
| bytes | 34.06 GB | |
