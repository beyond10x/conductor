# R-01 report review

## Verdict

Not fit to send as is. The numbers reproduce, but the three headline counts do not mean what the report says. The hard gates are only ratchets, so the first baseline would freeze 144 and 60 violations as "accepted". Default action: send it only with the § 4 caveats and the § 3 T1-T4 fixes listed as pending. Do not run `gate.mjs --update` until T1-T4 are fixed.

Method. I drove the built site (commit dad6b16b9 plus the uncommitted tree, see T9) with my own Playwright and axe scripts, not through measure.mjs or contract.mjs. The review scripts are not kept.
- Site contract: not sampled. All 36 sitemap pages, 1280x800 and 390x844, both themes, 144 cases, axe with the round's tags.
- Screen-only user: sampled. I did not re-run its three scenarios. I probed the home page by hand: link targets and tab counts per viewport.
- Not recomputed: layout shift, reduced motion, the non-gating viewports.
- The task said 6 pages in `ux/scenarios`; there are 3 scenario directories.
- There are no recordings, questionnaire, operator notes or earlier round, so § 1 compares against round.json only.

## 1. Are the numbers right and meaningful?

Recomputed vs round.json:

| number | round.json | mine | note |
|---|---|---|---|
| search missing | 72 per state | 72 of 72 cases, no search element at all | same |
| always_visible@1280x800 | 144 | 144 (72 x load + scroll-end) | same, but one defect |
| always_visible@390x844 | 288 | 288 | 3 defects, see below |
| axe_serious_critical@1280x800 | 60 | 60 (58 link-in-text-block, 2 label) | same |
| axe_serious_critical@390x844 | 58 | 58 (56 + 2) | same |
| hscroll, overflow, truncated | 0 | 0 (own check: no horizontal document scroll, no element past the viewport outside pre/table) | same |
| theme applied in dark runs | not asserted | `data-theme` dark, dark background | dark runs are real |

No headline number differs. The meaning is the problem:

1. `always_visible_violations` is not a count of problems. It is page x theme x state. 144 at 1280x800 is one defect (the site has no search) counted 72 times and then doubled for the load and scroll-end states. At 390x844, 288 is search x 2 states plus the theme toggle x 2 states, each x 72. The theme toggle is not "missing": Docusaurus puts it in the drawer on mobile (I confirmed it exists, at x=-116, inside `.navbar-sidebar`). The check flags a design choice as a violation on every page. Two distinct defects are reported as 432 (trend.md).
2. `axe_serious_critical` counts (rule, page, theme), not nodes. 60 here is 330 failing nodes. One root cause, link-in-text-block (prose links told apart from text by colour only), is 58 of the 60. Distinct pages affected: 29 of 36 at 1280x800. A page with 6 bad links scores 1; adding a 7th bad link is invisible to the ratchet.
3. The report's axe detail groups by the first node's CSS path. One cause shows up as about 20 lines (`p:nth-child(3) > a`, `p:nth-child(8) > a`, ...). The reader cannot see it is one fix.
4. "Pages with main-content text under 14 px: 72" is page x theme, not pages (36 pages), and it is 72 of 72 (every page): a threshold that discriminates nothing. It is not gated.
5. Screen-only user. `configure-an-instance` completes in 1 action because the navbar link "Guides" points directly at the target page (`/docs/guides/configure-an-instance`, which is the first guide), and the card lists "Guides" as a label worth 5 points. That scenario cannot fail in its current form. The labels and `via` in each card are written with the site's own link text ("Run conductor", "Get started"), so the bot confirms the cards, not a first-time reader's path.
6. `steers` is not "the cue is wrong, steer back": there is no cue on a docs site. It counts any click that does not land on the target, including a necessary hop. `start-the-daemon` at 1280x800 has steers=1 only because Guides is a detour, the intended route.
7. `completion_miss` / `false_complete`: completion is URL = target and the h1 in view. `false_complete` checks whether the truth string is anywhere in `document.body.innerText`, including text far below the fold or in collapsed blocks. A page that buries the command passes.
8. `keyboard.*`: 80 tabs for `start-the-daemon` at 1280x800 (my direct count from the home page to the "Start conductor" link: 57) is a march through every link with a keyword score; focus visibility is never checked. It runs only in the light theme. The gate "task can be completed by keyboard alone" is met by any page where tabbing eventually reaches a link. `keyboard.tabs` is a count that moves with unrelated nav changes (a new navbar item adds 1 to every scenario).
9. `glance_cost.p95` over 1-7 samples is the maximum. `looks` includes the final confirming look.
10. `layout_shift_max` and `reduced_motion_violations` are read once after load (300 ms settle). Both are 0 at the gating viewports, and the reduced-motion check cannot see the drawer or theme-toggle transitions. As built they can only fail on an infinite animation or `scroll-behavior: smooth`.

## 2. Findings the report misses

Confirmed by my own runs:

- The primary home call to action ("Get started →") is below the fold at 1280x800 (top 786, bottom 835, viewport 800) and at 390x844 (top 817, bottom 866). The bot's "scroll Get started →" in the table is this. The report does not say it. It is the most user-visible fact in the screen-only table.
- Code blocks and tables scroll horizontally inside their box: 34 page-theme cases at 1280x800, 44 at 390x844 (my count of `pre`/`table` with `scrollWidth > clientWidth`). `innerScroll` is computed in contract.mjs and thrown away (T6). Hard gate 1 in SKILL.md § 4 says no scrollable element.
- The mobile theme toggle is two taps away (hamburger, then the drawer), and in the drawer it reads "currently system mode"; the tri-state label is not described in the round.
- The sitemap has 36 URLs but the build has 77 html files. The extra ones (`concepts/architecture`, `instances-and-config`, `concepts/the-guard`, `guides/read-the-dashboard`) are not tested: they look like redirect stubs, so no page loss, but the report should say "sitemap" is the page set. `404.html` is not tested either.
- Both critical `label` violations are one unlabeled task-list checkbox on `/docs/security/hardening`, the page the report names as the worst moment. It is the only critical finding and is buried in 58 link rows.
- Dark and light give identical axe counts (30/30 at 1280x800): the themes are not exercised independently by colour-contrast findings, which is plausible but worth saying.

## 3. Tool defects

- T1 `ux/contract.mjs` `inPage`/`checkPage` and `ux/measure.mjs` `flatten`: the always-visible gate counts per page x theme x state. The same missing element is reported 144 / 288 times. The metric moves by 72 per defect fixed or introduced, so a baseline of 144 hides which defect changed. Input: any page at 1280x800. Fix direction: count distinct (kind, why) per viewport, or per page only.
- T2 `ux/contract.mjs` `need('theme-toggle', ...)` queries the first `button[aria-label^="Switch between dark and light mode"]` without regard to the drawer. On mobile, `querySelector` returns the toggle inside the closed drawer and reports "not visible" on all 72 cases: false positive for a deliberate mobile layout, true negative for anyone who wants the toggle in the bar. The gate cannot distinguish "hidden under a menu" from "gone".
- T3 `ux/measure.mjs` `flatten` and `ux/contract.mjs`: axe metric counts rule per page, not nodes (60 vs 330 nodes); a regression inside a page that already has the rule is a false negative for the ratchet. The report groups by first-node path, which fragments one cause into 20 lines (`ux/round.mjs` `details`).
- T4 `ux/gate.mjs` and `ux/ratchet.mjs`: the hard gates 2, 3 and 5 from SKILL.md § 4 (0 always-visible, `completion_miss`/`false_complete` 0, 0 serious/critical axe) are not enforced as absolutes, only as "not worse than baseline". `gate.mjs --update` writes 144 / 60 as the accepted value without a decision id (the decision is attached only when a value is raised over an existing one). The first baseline therefore freezes the current failures as green.
- T5 `ux/ratchet.mjs` `mainBaseline`/`worseThanMain`: when `origin/main:ux/baseline.json` is missing (new file, no remote, shallow clone) the `catch` returns null and `worseThanMain` returns no problems. The "baseline made worse without a decision id" check then passes silently.
- T6 `ux/contract.mjs`: `innerScroll` (horizontally scrolling elements) and `themeApplied` are collected and never reported, gated or asserted. 34 and 44 cases exist (§ 2). If a theme failed to apply, the dark round would be a second light round and no number would show it (it does apply today).
- T7 `ux/scenarios/*/scenario.json` and `ux/bot.mjs`: `expectations.maxActions` (4) is never read; hard gate 4 "every fix at most 2 actions" is not checked. `anchor` and `rotating` are not read either. Labels and `via` use the site's own link text, so scenario success is partly circular (§ 1.5). `configure-an-instance` passes by label coincidence.
- T8 `ux/bot.mjs` `onTarget`/`runBot`: completion needs only the URL and the h1 in view; `truthText` is searched in the whole page text, visible or not. `false_complete` cannot fire for a buried or collapsed answer. The bot and the keyboard pass run in the light theme only. `keyboard.tabs` at the cap of 80 loop iterations is indistinguishable from "finished in 80".
- T9 `ux/round.mjs`: the report records the commit `dad6b16b9`, but the worktree is dirty (`website/docusaurus.config.ts`, `package.json`, `package-lock.json` modified; `ux/` and `website/src/` untracked). The commit does not identify what was measured, and the round cannot be reproduced from it.
- T10 `ux/round.mjs` report text: "Pages with main-content text under 14 px: 72" is page x theme; "no baseline yet" / "none" columns are blank for every row, so the report has no comparison at all; "(see findings)" points at findings UXF-nnn that the report never cites by id.
- T11 `ux/measure.mjs` `takeShots`: "worst moments" score is `violations*2 + axe + ...`, and with a constant 72-case search violation and a link-in-text-block present on most pages the ranking is mostly a tie-break; the six "worst" shots are three pages, both themes, at 390x844 only. The 1280x800 view of those pages is not shown.
- T12 `ux/measure.mjs` `checkReducedMotion`: reads running animations and `scroll-behavior` once, at load. It cannot detect transitions started by user action (drawer, toggle, tab switch).

## 4. Caveats to send with the round

- One defect is counted many times: 432 "always-visible violations" are two defects (no search anywhere; theme toggle in the mobile drawer). 118 axe findings are 2 rules and 330 nodes.
- No baseline exists; every "baseline" and "last round" cell is empty. The ratchet has not run.
- The screen-only user is three scenarios, deterministic, one run each, light theme, written with the site's own labels. `configure-an-instance` passes trivially. The "at most 2 actions" gate is not computed.
- The recorded commit does not match the measured tree.
- L0, L4, L5, L6, L8 are not run: no people, no probe, no questionnaire. Nothing here measures whether a reader understands the pages.
- Not measured and not in the report: horizontally scrolling code blocks and tables, visible focus, the below-the-fold primary call to action.
- Sampling in this review: site contract recomputed in full (144 cases); bot, keyboard, layout shift and reduced motion only probed or not re-run.
