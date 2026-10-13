# Wave 16

Coordinator: conductor-dev, `aep:implementing` 0.22.8, wave mode. Approved up front (rules item 99).
One managed tree, `conductor-wave-16`, branch `wave/16`; implementors wrote without a local build
and CI gated the round. Epic: `epic:conductor-autonomy`.

## Units

| unit | stories | commit |
|---|---|---|
| U1 | prebuilt-binaries | `cb5dff0` |
| U2 | status-board | `cb5dff0`, `368103d` |
| U3 | resumable-gates, work-beside-long-gate, claims-cite-source, operator-items-age, gc-before-pause, in that order | `cc0f30f` |
| U4 | self-arming-restart, after U2 and U3 | `285e936` |

U1, U2 and U3 touched disjoint files and ran at once; U3's stories share `.agents/conductor.md`
and `crates/conductor/tests/profiles.rs` and ran one after another; U4 shares `Taskfile.yml` and
`crates/conductor/tests/taskfile.rs` with U2 and the profile files with U3, so it ran last.

## Off-plan

- `conductor gc` has no `--apply`: a run without `--dry-run` removes. The profile and the test use
  that run; the strip is called with `worktree prune-archives --strip-build-output --apply`.
- The age of a hands item shows in the `text` and `markdown` formats of `decision hands-todo`; the
  `json` and `jsonl` rows keep the specification's fields, so the age is not in them.
- The restart prompt names the keys `cadence.cycle` and `cadence.daily` and has the session read
  their values from `conductor config show`, rather than putting the values in the prompt.
- `PublishBoard` takes `deviation_rows` as an input, which the board counts itself: the command
  refuses the flag. Dropping the input from the specification is a follow-up.
- `story:version-flag` filed: `conductor --version` is refused.

## Outcome

`task check`, documentation and the Gates job green in CI on `368103d`
(https://github.com/beyond10x/conductor/actions/runs/38374319466); the first run failed two
status tests whose table reader kept the separator row, fixed in the test. Seven stories moved to
`implemented` on that record.

v0.6.0 released from `a2db4fa`. The release workflow
(https://github.com/beyond10x/conductor/actions/runs/38376261294) built all four archives, but
its publish step failed on its first try: it created the Release with `--notes-from-tag`, which
needs the tag in a checkout the job does not have. The Release was then created with its notes
through the API, and a rerun of the failed job attached the four archives and `SHA256SUMS` (five
assets). The x86_64 Linux archive verified against `SHA256SUMS`, ran `--help` with exit 0 and was
installed on the host without a source build. `story:prebuilt-binaries` moved to `implemented` on
that run; the workflow now reads the tag's message through the API.
