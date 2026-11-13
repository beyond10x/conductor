---
format: aep.planning-md/3
id: story:guard-launcher-spellings
kind: story
status: draft
title: The guard reads a program behind taskset, env -S, parallel, a shortened option of env and ARGV0
relations:
- informed_by: story:anonymous-pod-no-forge-secret
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
revision: 2
---
## Why

The guard reads the program word of a command behind a launcher (`env`, `timeout`, `sudo`,
`nice`, `xargs` and the others of `LAUNCHERS` in `crates/conductor/src/guard/rules.rs`), and since
story:anonymous-pod-no-forge-secret also a name given to the program in `argv[0]`. Two security
reviews of that story showed by cases, each with the member `forge_access: anonymous`, spellings
the reading does not cover. Each was allowed before that story too:

```
taskset ff git push origin main | Allow
env --argv=git-push git origin main | Allow
env --a=git-push git origin main | Allow
ARGV0=git-push git origin main | Allow
```

- `taskset` reads a mask that does not start with a digit; the reading takes it as the program.
- `env` accepts a long option shortened to any unique prefix (`--argv=`, `--a=`); the table of
  options that take a value names the full spelling only.
- zsh sets `argv[0]` of the command from the assignment `ARGV0=`.
- `env -S '<command>'` (`--split-string`) holds a whole command in one word. `env -S 'git push
  origin main'` is denied today; `-S` is not in the table of options that take a value, so a
  command after other words in the string was not shown either way.
- `parallel` is no launcher of the list; it was not installed where the reviews ran.
- A program that reads its name from `argv[0]` other than git and rustup (a multi-call binary) is
  read only for the names the rules know: `git-<subcommand>` and `cargo`, `task`, `npm`.

The same reading serves every rule that looks for a program word, so each of these also hides a
build from the rule of unreserved builds for an instance without the member. The pod of an
anonymous instance holds no credential a push could use; the guard is the second line there.

## What

- The reading covers the four spellings shown above and `env -S` with a command anywhere in its
  string.
- `parallel` is read as a launcher, with its options that take a value from its own manual.
- The story says, with a run each, which multi-call programs on the image answer to a name in
  `argv[0]`, and reads those.

## Acceptance

- The review's cases `rev6_pushes_in_a_split_string_or_after_a_cpu_mask_are_denied` and
  `rev7_pushes_under_argv0_spellings_beside_the_named_ones`, taken into the repository's suites,
  fail before the change and pass after it.
- No read is denied that is allowed today: the read lists of the suites `guard_rules` and
  `forge_access_anonymous_review` keep their decisions.
- For an instance without the member, every decision that changes is named in the pull request
  with the command and both verdicts.
