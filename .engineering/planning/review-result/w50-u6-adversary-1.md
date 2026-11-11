---
format: aep.planning-md/3
id: review-result:w50-u6-adversary-1
kind: review-result
status: active
title: Wave 50 U6 getting-started-substrate-steps-portable, adversary pass 1
relations:
- reviews: story:getting-started-substrate-steps-portable
revision: 1
---
Adversary pass 1 on `story:getting-started-substrate-steps-portable`, commit 584b3964b on base 1b6dc5490.

Verdict: CONFIRMED. Three findings against the unit's own test, all introduced; the doc change itself was not broken.

Cases added (`mod adversary` in `crates/conductor/tests/docs_substrate_steps.rs`), all red on their first run:

- `adv_a_a_resolved_path_two_of_the_three_distributions_lack_is_found`
- `adv_b_a_realpath_that_resolves_no_link_is_found`
- `adv_c_setup_md_telling_mode_755_is_not_held_by_the_reason_beside_it`

Suite after the cases: `39 tests run: 36 passed, 3 failed, 0 skipped`.

Findings:

- F1 (warning): `bundle_problems` accepts any path inside `$(readlink -f …)` or `$(realpath …)`.
- F2 (note): the SETUP.md test is satisfied by "mode 700" in the reason clause, so the instruction can tell another mode.
- F3 (note): a dead operand in `bundle_problems`.

Attacked, not broken: dropping the parent directory or naming the symlink again goes red; the SETUP.md
sentence matches substrate 0.20.0 (`crates/substrate-daemon/src/runtime.rs`: the parent of `--state`
must be mode 0700 or stricter; a symlink bundle is refused); `install -d -m 700` on a parent already at
0755 changes only that directory; `readlink -f` on Alpine yields a regular file; with no bundle
installed it fails with exit 1 and empty output; no other page repeats either instruction.

```findings
- file: crates/conductor/tests/docs_substrate_steps.rs
  line: 69
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: '"bundle_problems accepts any path inside $(readlink -f …) or $(realpath …), so a Fedora-only bundle path or realpath -s stays green while the acceptance breaks on Debian and Arch"'
- file: crates/conductor/tests/docs_substrate_steps.rs
  line: 111
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"the SETUP.md test is satisfied by ''mode 700'' in the reason clause on SETUP.md:145, so the instruction can tell mode 755 or no mode and the test stays green"'
- file: crates/conductor/tests/docs_substrate_steps.rs
  line: 69
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"the operand value.starts_with(\"/etc/ssl/certs/\") is dead: every such value is already !resolved"'
```
