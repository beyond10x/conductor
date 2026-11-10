---
format: aep.planning-md/3
id: story:guard-direct-write-expansions
kind: story
status: draft
title: Under the direct route the guard denies a forge write whose words the shell would change
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
revision: 3
---
## Why

Found by the adversary pass on story:guard-admits-own-mr-auto-merge in wave 49 (2026-11-10), with failing cases; not run on the base, origin by reading the diff. Under `forge.route: direct` the guard reads a command line's words before the shell expands them:

- A direct write is admitted with a command substitution, a backquote or a variable in a flag value. The shell splits the value into further flags, and `glab` takes the last `-R` or `--hostname`, so `glab mr merge 7 -R <own> -m $(echo x -R <other>)` reaches another project (crates/conductor/src/guard/rules.rs:1995).
- `xargs` or `parallel` before an own `glab api` write or `glab mr merge` feeds words `glab` reads after the ones the guard read (rules.rs:1980).
- `GLAB_CONFIG_DIR` and `XDG_CONFIG_HOME` are not among `FORGE_TARGET_VARIABLES`; a config they name redirects the own host through `api_host` (rules.rs:811). No token valid elsewhere was shown.
- The pipeline POST admits the same expansions and any header.

The cases are in crates/conductor/tests/guard_rules.rs as `u15_adv_*`; those that this story owns are marked ignored with this story's id.

## Acceptance

- Under the direct route, a forge write whose line holds a command substitution, a backquote or a variable expansion in any word after the program is denied; the ignored `u15_adv_*` cases run and pass.
- A forge write fed by `xargs` or `parallel` is denied.
- `GLAB_CONFIG_DIR` and `XDG_CONFIG_HOME` on the line or exported in it deny a forge write.
- Every form admitted today without those constructs keeps its verdict.
