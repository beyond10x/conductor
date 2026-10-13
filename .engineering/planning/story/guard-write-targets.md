---
format: aep.planning-md/3
id: story:guard-write-targets
kind: story
status: draft
title: The guard decides a Bash write on its target, and a denial names the alternative
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/guard/shell.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 2
---
## Why

On 2026-10-13 the guard denied nine legitimate Bash calls. Six were controllers copying rules.md
items into a brief file in their own scratch: `writes_records` is "the command names the records"
and "it holds a write form anywhere", so a read of the records redirected into scratch is a write to
the records. After the denial one controller rewrote the excerpt from memory, so the "verbatim" rule
in its worker brief was a model's copy. Three were conductor appending a charter section whose body
quotes the controller profile's path: `writes_profile` matches the path anywhere in the command,
heredoc body included, while the Edit tool writes the same text unchecked.
`story:guard-bash-write-targets`, which would decide on the write target, was archived on
2026-10-08. Denial messages also
named no allowed alternative: two Write denials did not say where scratch is, and five read-only
`cd` denials did not say to read by absolute path.

## Acceptance

- `writes_records`, `writes_profile` and the settings rule decide on the write targets
  (`shell::written`) and the file operands of the writing programs (`tee`, `cp`, `mv`, `sed -i`,
  `perl -i`, `install`), never on a path that appears only as a read operand or inside a quoted or
  heredoc body. Cases in `crates/conductor/tests/guard_rules.rs`: `awk … <records>/rules.md >
  ~/.cache/<repo>-x/brief.md` and `sed -n … <records>/rules.md > $TMPDIR/x` allowed for a
  controller; `… > <records>/rules.md` and `sed -i … <records>/rules.md` denied; `cat >>
  charters/x.md <<'EOF'` whose body names the controller profile's path allowed for conductor; a
  target held in a variable keeps today's verdict.
- A Write or Edit denial names `$TMPDIR` and `~/.cache/<repo>-*/` as scratch, and a `cd` denial
  into another checkout adds "read it by absolute path, without cd"; cases in
  `crates/conductor/tests/guard_rules.rs` read both messages.
- Design § 7's Bash paragraph and the README's security model say the write target decides.

## Notes

- Supersedes the archived `story:guard-bash-write-targets`, whose acceptance it keeps.
- Shares `crates/conductor/src/guard/rules.rs` and `crates/conductor/tests/guard_rules.rs` with
  `story:rules-show` and `story:request-ids-allocated`; this story first.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/guard/shell.rs`,
`crates/conductor/tests/guard_rules.rs`, `docs/design/conductor.md`, `README.md`.
