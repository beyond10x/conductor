# R-01 heuristic review

7 pass, 7 partial, 0 fail (2 not observed: 6, 13)

Evidence base: contact sheets (5 pages x 2 sizes x 2 themes), moment and worst shots, report.md, scenario cards, and my own probe shots `shots/review-*.png`. No recordings, questionnaire or operator notes exist for this round, so verdicts rest on screens and metrics only. Paths below are relative to `shots/` unless they start with report.md.

| # | heuristic | verdict | evidence |
|---|---|---|---|
| 1 | One next thing | pass | moment-home-390x844-dark.png: one Get started button under the lede; TOC ends in Next (moment-docs_getting-started-1280x800-light.png); steers 0 to 1, report.md:23-80 |
| 2 | State in words | partial | links differ from body only by hue (UXF-102); report.md:106; moment-docs_getting-started-1280x800-light.png |
| 3 | Nothing required hidden | partial | no search (UXF-101); theme toggle only in the drawer at 390 (review-menu-390x844-light.png, report.md:128); table clipped (UXF-105, moment-docs_reference_cli-390x844-light.png) |
| 4 | Stable layout | pass | layout_shift_max 0 at both gating sizes, report.md:13,20; 0.016 worst at 768x1024 |
| 5 | Provenance of every value | pass | source-file line under every title (contact-sheet-light.png); cost in UXF-104 |
| 6 | Errors cheap | not observed | no error or wrong-turn state in any shot |
| 7 | Words the user can say | pass | scenario labels (Guides, Run conductor, Get started) matched the nav text; 3 of 3 scenarios completed at both sizes, report.md:165-170 |
| 8 | Reader's time respected | partial | header card plus source list fills the first phone screen; install command at y=1186 on 390 (UXF-104, worst-docs_concepts_config-390x844-dark.png); start-the-daemon took 6 actions at 390 (report.md:169-170) |
| 9 | Keyboard and pointer parity | partial | keyboard_completed yes everywhere, miss 0, but 64 to 92 tabs vs 1 to 3 clicks (UXF-106); skip link present (review-focus-1280x800-light.png) |
| 10 | Recognition over recall | partial | sidebar and breadcrumbs support it (moment-docs_guides_start-conductor-1280x800-light.png); no search, and the checklist cannot be ticked (UXF-101, UXF-103, review-hardening-1280x800-dark.png) |
| 11 | Calm by default | pass | reduced_motion_violations 0 at both sizes, report.md:14,21; no motion or banners in any shot; the one warning admonition is cause-driven (start-conductor) |
| 12 | One state vocabulary | pass | one set of eyebrow, title, summary, source blocks on every page (contact-sheet-dark.png); small gap: page title Run conductor vs URL start-conductor |
| 13 | A wrong cue recovered in one action | not observed | no wrong turn recorded; breadcrumbs and a Back to main menu control exist (review-menu-390x844-light.png) |
| 14 | Unmistakable end state | pass | completion_miss 0 and false_complete 0 on all six scenario runs, report.md:29-80; truth text on target page |
| 15 | Readable in every theme; key data at 14 px or more | partial | both themes legible (contact-sheet-light.png, contact-sheet-dark.png); code and meta text below 14 px on 72 pages (UXF-107, report.md:122) |
| 16 | Focus follows the work | partial | first Tab lands on Skip to main content (review-focus-1280x800-light.png); no search entry to move to (UXF-101) |

## Findings filed

| id | title | severity | class | sessions | cost |
|---|---|---|---|---|---|
| UXF-101 | The site has no search control | S2 | affordance | all 3 scenarios, both sizes | M |
| UXF-102 | In-text links are told apart by colour alone | S2 | cue | whole site, 58 to 60 axe hits | S |
| UXF-103 | Hardening checklist boxes are disabled and unlabelled | S3 | affordance | security/hardening (and any task list) | S |
| UXF-104 | Source-file list in every page header pushes the answer below the fold on a phone | S2 | noise | start-the-daemon and configure-an-instance at 390x844 | S |
| UXF-105 | CLI reference table is clipped at 390x844 with no scroll cue | S3 | affordance | reference/cli at 390x844 | S |
| UXF-106 | Reaching content by keyboard costs 64 to 92 tabs and the theme toggle hides behind the menu at 390x844 | S3 | affordance | configure-an-instance 390, start-the-daemon 1280 and 390 | M |
| UXF-107 | Main-content text under 14 px on every page | S3 | noise | 72 pages | S |

Note: the round has no recordings, so no session had a replayable second; evidence_recording is none on every record.
