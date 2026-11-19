---
format: aep.planning-md/3
id: story:rules-show-unnumbered-items
kind: story
status: active
title: rules show reads unnumbered bullets as items and never lists the section it refuses
summary: 'A rules.md of - bullets: --section prints them, --brief and json too'
refs:
- provider: github
  reference: beyond10x/conductor#221
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/rules.rs
- confidence: cited
  path: crates/conductor/tests/rules.rs
- confidence: cited
  path: spec/domains/direction.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:24:19Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-19T18:24:19Z", actor: "human:unknown", revision: 5}
---
## Why

Issue #221. An instance's `rules.md` written as unnumbered `- ` bullets under `##` role headings
has no item for `conductor rules show`: `parse` opens an item only on `<n>. ` at the margin
(`crates/conductor/src/rules.rs:95`, `item_number` at `:173`), so `--section Director` is refused
with `RuleSectionNotFound`, and the refusal lists `Director` among the file's sections, because
`sections` (`:136`) names every heading whether or not an item sits under it. A controller's
worker briefs then carry no rules item.

## Outcome

- Under a heading of level 2 or deeper, a line that opens with `- ` at the margin opens an
  unnumbered item, read as a numbered one is (continuation lines indented, up to the next item,
  heading or margin line). A bullet above every section heading is no item.
- `RulesRow.item` is `Optional<Integer>` (`spec/domains/direction.yaml`): absent for an
  unnumbered item; `--format json` writes `null`. `--brief` prints `- <title>` for one; the
  title is read after the `- `. `--item` names numbered items only.
- `--changed-since` keeps an unnumbered item when no item of the revision has its text.
- The `RuleSectionNotFound` refusal names the headings with an item under them only, so it never
  names the heading it refuses.

## Acceptance (fail on main)

- `crates/conductor/tests/rules.rs` `a_section_of_unnumbered_bullets_prints_its_items`: text,
  `--brief`, `--format json` of a bullets-only `rules.md`, and the refusal of an empty section
  naming `Director, Maintainer` only.
- The existing `rules.rs` cases pass unchanged.

## Boundaries

No item numbers are assigned to bullets (a position would change with every edit), so `--item`
still cannot cite them; numbering the file's bullets is the file's owner's choice. The `--role`
path (`rules/<role>.md`) is unchanged.
