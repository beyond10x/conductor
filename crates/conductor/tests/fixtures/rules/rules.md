# Rules of the example-org instance

Every session reads its own section. Items are numbered once and never renumbered, so an item
added later carries a higher number wherever it sits.

## Conductor

### Session start

1. Read the newest hand-over first, then the open dispatches.
   Source: the operator, 2026-10-01.
43. **Never discard uncommitted changes.** A tree with changes is archived, never removed:
   `worktree finish --archive`.

   Ask before removing a tree that is not yours.
   Source: `decisions/2026-10.jsonl`, 2026-10-03.
2. Answer a report within one cycle.

### Briefs

51. **Builds and disk.** Build into the tree's own `target/`:
    - never set `CARGO_TARGET_DIR`;
    - every build runs under `conductor build run`.
    Source: the operator, 2026-10-05.

## Repository controller

49. **No work files in `/tmp`.** Scratch lives under `$TMPDIR` or `~/.cache/<repo>-<purpose>/`.
   Source: the operator, 2026-10-02.
3. Report `started` before the first change.

## Operator's standing rules

These hold for every session, beside its own section.

### Conductor

102. **A decision never waits on a session that is gone.** Re-dispatch it.
     Source: the operator, 2026-10-09.
