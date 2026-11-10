# R-01 report: the dashboard

Commit 8eaa9f159 (30 changed paths under crates/conductor/dashboard, listed in round.json), 2026-11-10T01:12:59.663Z. 5 scenarios (decisions, health, kpis, overview, repository), 21 states, viewports 1440x900*, 1280x800*, 1024x768, 768x1024, 390x844 (* gating), themes light and dark: 210 state cases. Levels L1 to L3; L0 (no guidance cue), L4, L5, L6, L7 and L8 are not run by this command.

## Gating metrics (lower is better)

| scenario | metric | value | baseline | last round |
|---|---|---|---|---|
| decisions | hscroll_states@1440x900 | 0 | 0 | none |
| decisions | overflow_states@1440x900 | 0 | 0 | none |
| decisions | always_visible_violations@1440x900 | 6 | 6 | none |
| decisions | axe_serious_critical_nodes@1440x900 | 0 | 0 | none |
| decisions | axe_rule_cases@1440x900 | 0 | 0 | none |
| decisions | theme_not_applied@1440x900 | 0 | 0 | none |
| decisions | presence_misses@1440x900 | 0 | 0 | none |
| decisions | truncated_states@1440x900 | 0 | 0 | none |
| decisions | reduced_motion_violations@1440x900 | 0 | 0 | none |
| decisions | network_requests@1440x900 | 0 | 0 | none |
| decisions | unanswered_requests@1440x900 | 0 | 0 | none |
| decisions | completion_miss@1440x900 | 0 | 0 | none |
| decisions | false_complete@1440x900 | 0 | 0 | none |
| decisions | over_budget@1440x900 | 0 | 0 | none |
| decisions | screen_only.actions@1440x900 | 4 | 4 | none |
| decisions | keyboard.tabs@1440x900 | 44 | 44 | none |
| decisions | keyboard.miss@1440x900 | 0 | 0 | none |
| decisions | hscroll_states@1280x800 | 0 | 0 | none |
| decisions | overflow_states@1280x800 | 0 | 0 | none |
| decisions | always_visible_violations@1280x800 | 6 | 6 | none |
| decisions | axe_serious_critical_nodes@1280x800 | 0 | 0 | none |
| decisions | axe_rule_cases@1280x800 | 0 | 0 | none |
| decisions | theme_not_applied@1280x800 | 0 | 0 | none |
| decisions | presence_misses@1280x800 | 0 | 0 | none |
| decisions | truncated_states@1280x800 | 0 | 0 | none |
| decisions | reduced_motion_violations@1280x800 | 0 | 0 | none |
| decisions | network_requests@1280x800 | 0 | 0 | none |
| decisions | unanswered_requests@1280x800 | 0 | 0 | none |
| decisions | completion_miss@1280x800 | 0 | 0 | none |
| decisions | false_complete@1280x800 | 0 | 0 | none |
| decisions | over_budget@1280x800 | 0 | 0 | none |
| decisions | screen_only.actions@1280x800 | 4 | 4 | none |
| decisions | keyboard.tabs@1280x800 | 44 | 44 | none |
| decisions | keyboard.miss@1280x800 | 0 | 0 | none |
| health | hscroll_states@1440x900 | 0 | 0 | none |
| health | overflow_states@1440x900 | 0 | 0 | none |
| health | always_visible_violations@1440x900 | 0 | 0 | none |
| health | axe_serious_critical_nodes@1440x900 | 0 | 0 | none |
| health | axe_rule_cases@1440x900 | 0 | 0 | none |
| health | theme_not_applied@1440x900 | 0 | 0 | none |
| health | presence_misses@1440x900 | 0 | 0 | none |
| health | truncated_states@1440x900 | 0 | 0 | none |
| health | reduced_motion_violations@1440x900 | 0 | 0 | none |
| health | network_requests@1440x900 | 0 | 0 | none |
| health | unanswered_requests@1440x900 | 0 | 0 | none |
| health | hscroll_states@1280x800 | 0 | 0 | none |
| health | overflow_states@1280x800 | 0 | 0 | none |
| health | always_visible_violations@1280x800 | 0 | 0 | none |
| health | axe_serious_critical_nodes@1280x800 | 0 | 0 | none |
| health | axe_rule_cases@1280x800 | 0 | 0 | none |
| health | theme_not_applied@1280x800 | 0 | 0 | none |
| health | presence_misses@1280x800 | 0 | 0 | none |
| health | truncated_states@1280x800 | 0 | 0 | none |
| health | reduced_motion_violations@1280x800 | 0 | 0 | none |
| health | network_requests@1280x800 | 0 | 0 | none |
| health | unanswered_requests@1280x800 | 0 | 0 | none |
| kpis | hscroll_states@1440x900 | 0 | 0 | none |
| kpis | overflow_states@1440x900 | 0 | 0 | none |
| kpis | always_visible_violations@1440x900 | 0 | 0 | none |
| kpis | axe_serious_critical_nodes@1440x900 | 0 | 0 | none |
| kpis | axe_rule_cases@1440x900 | 0 | 0 | none |
| kpis | theme_not_applied@1440x900 | 0 | 0 | none |
| kpis | presence_misses@1440x900 | 0 | 0 | none |
| kpis | truncated_states@1440x900 | 0 | 0 | none |
| kpis | reduced_motion_violations@1440x900 | 0 | 0 | none |
| kpis | network_requests@1440x900 | 0 | 0 | none |
| kpis | unanswered_requests@1440x900 | 0 | 0 | none |
| kpis | hscroll_states@1280x800 | 0 | 0 | none |
| kpis | overflow_states@1280x800 | 0 | 0 | none |
| kpis | always_visible_violations@1280x800 | 0 | 0 | none |
| kpis | axe_serious_critical_nodes@1280x800 | 0 | 0 | none |
| kpis | axe_rule_cases@1280x800 | 0 | 0 | none |
| kpis | theme_not_applied@1280x800 | 0 | 0 | none |
| kpis | presence_misses@1280x800 | 0 | 0 | none |
| kpis | truncated_states@1280x800 | 0 | 0 | none |
| kpis | reduced_motion_violations@1280x800 | 0 | 0 | none |
| kpis | network_requests@1280x800 | 0 | 0 | none |
| kpis | unanswered_requests@1280x800 | 0 | 0 | none |
| overview | hscroll_states@1440x900 | 0 | 0 | none |
| overview | overflow_states@1440x900 | 0 | 0 | none |
| overview | always_visible_violations@1440x900 | 0 | 0 | none |
| overview | axe_serious_critical_nodes@1440x900 | 2 | 2 | none |
| overview | axe_rule_cases@1440x900 | 2 | 2 | none |
| overview | theme_not_applied@1440x900 | 0 | 0 | none |
| overview | presence_misses@1440x900 | 0 | 0 | none |
| overview | truncated_states@1440x900 | 0 | 0 | none |
| overview | reduced_motion_violations@1440x900 | 0 | 0 | none |
| overview | network_requests@1440x900 | 0 | 0 | none |
| overview | unanswered_requests@1440x900 | 0 | 0 | none |
| overview | hscroll_states@1280x800 | 0 | 0 | none |
| overview | overflow_states@1280x800 | 0 | 0 | none |
| overview | always_visible_violations@1280x800 | 0 | 0 | none |
| overview | axe_serious_critical_nodes@1280x800 | 2 | 2 | none |
| overview | axe_rule_cases@1280x800 | 2 | 2 | none |
| overview | theme_not_applied@1280x800 | 0 | 0 | none |
| overview | presence_misses@1280x800 | 0 | 0 | none |
| overview | truncated_states@1280x800 | 0 | 0 | none |
| overview | reduced_motion_violations@1280x800 | 0 | 0 | none |
| overview | network_requests@1280x800 | 0 | 0 | none |
| overview | unanswered_requests@1280x800 | 0 | 0 | none |
| repository | hscroll_states@1440x900 | 0 | 0 | none |
| repository | overflow_states@1440x900 | 0 | 0 | none |
| repository | always_visible_violations@1440x900 | 0 | 0 | none |
| repository | axe_serious_critical_nodes@1440x900 | 4 | 4 | none |
| repository | axe_rule_cases@1440x900 | 2 | 2 | none |
| repository | theme_not_applied@1440x900 | 0 | 0 | none |
| repository | presence_misses@1440x900 | 0 | 0 | none |
| repository | truncated_states@1440x900 | 0 | 0 | none |
| repository | reduced_motion_violations@1440x900 | 0 | 0 | none |
| repository | network_requests@1440x900 | 0 | 0 | none |
| repository | unanswered_requests@1440x900 | 0 | 0 | none |
| repository | hscroll_states@1280x800 | 0 | 0 | none |
| repository | overflow_states@1280x800 | 0 | 0 | none |
| repository | always_visible_violations@1280x800 | 0 | 0 | none |
| repository | axe_serious_critical_nodes@1280x800 | 4 | 4 | none |
| repository | axe_rule_cases@1280x800 | 2 | 2 | none |
| repository | theme_not_applied@1280x800 | 0 | 0 | none |
| repository | presence_misses@1280x800 | 0 | 0 | none |
| repository | truncated_states@1280x800 | 0 | 0 | none |
| repository | reduced_motion_violations@1280x800 | 0 | 0 | none |
| repository | network_requests@1280x800 | 0 | 0 | none |
| repository | unanswered_requests@1280x800 | 0 | 0 | none |

## Measured only (not gated)

| viewport | hscroll states | overflow states | always-visible violations | truncated states | theme not on screen | presence misses | states with a scrolling block | states with text under 14 px |
|---|---|---|---|---|---|---|---|---|
| 1024x768 | 0 | 0 | 6 | 0 | 0 | 0 | 6 | 36 |
| 768x1024 | 0 | 0 | 6 | 0 | 0 | 0 | 2 | 36 |
| 390x844 | 0 | 0 | 6 | 0 | 0 | 0 | 12 | 36 |

## Detail at 1440x900 (gating)

Always-visible defects (kind: why: state cases):
- header: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- instance-selector: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- navigation: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])

axe serious or critical, by rule (state cases, nodes, first target):
- link-in-text-block (serious), The link has insufficient color contrast of 1.54:1 with the surrounding text. (Minimum contrast is 3:1, link text: #7cc4ff, surrounding text: #e6e9ee): 1 cases, 1 nodes, `a[rel="noreferrer"][data-v-34d6433b=""]` (overview/full[dark])
- link-in-text-block (serious), The link has insufficient color contrast of 2.67:1 with the surrounding text. (Minimum contrast is 3:1, link text: #0b62b0, surrounding text: #1a1f26): 1 cases, 1 nodes, `a[rel="noreferrer"][data-v-34d6433b=""]` (overview/full[light])
- scrollable-region-focusable (serious), Element should have focusable content: 2 cases, 4 nodes, `.stack.session.card:nth-child(2) > .s13.row[data-v-d220f67f=""]:nth-child(5) > .transcript` (repository/full[dark], repository/full[light])

Elements past their box: none
Text cut off without its full value: none
Elements a state names and does not show (shown of named): none
State cases whose theme is not on screen (not pinned, or the background of the other theme): none
Requests off the loopback origin: none
Requests no fixture answers: none
State cases with text under 14 px in main: 36 of 42

## Detail at 1280x800 (gating)

Always-visible defects (kind: why: state cases):
- header: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- instance-selector: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- navigation: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])

axe serious or critical, by rule (state cases, nodes, first target):
- link-in-text-block (serious), The link has insufficient color contrast of 1.54:1 with the surrounding text. (Minimum contrast is 3:1, link text: #7cc4ff, surrounding text: #e6e9ee): 1 cases, 1 nodes, `a[rel="noreferrer"][data-v-34d6433b=""]` (overview/full[dark])
- link-in-text-block (serious), The link has insufficient color contrast of 2.67:1 with the surrounding text. (Minimum contrast is 3:1, link text: #0b62b0, surrounding text: #1a1f26): 1 cases, 1 nodes, `a[rel="noreferrer"][data-v-34d6433b=""]` (overview/full[light])
- scrollable-region-focusable (serious), Element should have focusable content: 2 cases, 4 nodes, `.stack.session.card:nth-child(2) > .s13.row[data-v-d220f67f=""]:nth-child(5) > .transcript` (repository/full[dark], repository/full[light])

Elements past their box: none
Text cut off without its full value: none
Elements a state names and does not show (shown of named): none
State cases whose theme is not on screen (not pinned, or the background of the other theme): none
Requests off the loopback origin: none
Requests no fixture answers: none
State cases with text under 14 px in main: 36 of 42

## Detail at 1024x768

Always-visible defects (kind: why: state cases):
- header: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- instance-selector: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- navigation: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])

Elements past their box: none
Text cut off without its full value: none
Elements a state names and does not show (shown of named): none
State cases whose theme is not on screen (not pinned, or the background of the other theme): none
Requests off the loopback origin: none
Requests no fixture answers: none
State cases with text under 14 px in main: 36 of 42

## Detail at 768x1024

Always-visible defects (kind: why: state cases):
- header: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- instance-selector: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- navigation: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])

Elements past their box: none
Text cut off without its full value: none
Elements a state names and does not show (shown of named): none
State cases whose theme is not on screen (not pinned, or the background of the other theme): none
Requests off the loopback origin: none
Requests no fixture answers: none
State cases with text under 14 px in main: 36 of 42

## Detail at 390x844

Always-visible defects (kind: why: state cases):
- header: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- instance-selector: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])
- navigation: outside viewport: 2 (decisions/answered[dark], decisions/answered[light])

Elements past their box: none
Text cut off without its full value: none
Elements a state names and does not show (shown of named): none
State cases whose theme is not on screen (not pinned, or the background of the other theme): none
Requests off the loopback origin: none
Requests no fixture answers: none
State cases with text under 14 px in main: 36 of 42

## Screen-only and keyboard-only users (L3)

| scenario | viewport | theme | completed | actions | looks | path | keyboard tabs | keyboard completed |
|---|---|---|---|---|---|---|---|---|
| decisions | 1440x900 | light | yes | 2 | 1 | button "Yes" > button "Send 1 answer" | 22 | yes |
| decisions | 1440x900 | dark | yes | 2 | 1 | button "Yes" > button "Send 1 answer" | 22 | yes |
| decisions | 1280x800 | dark | yes | 2 | 1 | button "Yes" > button "Send 1 answer" | 22 | yes |
| decisions | 1280x800 | light | yes | 2 | 1 | button "Yes" > button "Send 1 answer" | 22 | yes |

## Worst moments

- decisions/answered 1280x800 dark: score 6, shots/worst-decisions-answered-1280x800-dark.png
- decisions/answered 1280x800 light: score 6, shots/worst-decisions-answered-1280x800-light.png
- decisions/answered 1440x900 dark: score 6, shots/worst-decisions-answered-1440x900-dark.png
- decisions/answered 1440x900 light: score 6, shots/worst-decisions-answered-1440x900-light.png

Key moments (each scenario's last state at 1280x800): shots/decisions-answered-1280x800-dark.png, shots/decisions-answered-1280x800-light.png, shots/health-full-1280x800-dark.png, shots/health-full-1280x800-light.png, shots/kpis-full-1280x800-dark.png, shots/kpis-full-1280x800-light.png, shots/overview-full-1280x800-dark.png, shots/overview-full-1280x800-light.png, shots/repository-full-1280x800-dark.png, shots/repository-full-1280x800-light.png

Contact sheets: shots/contact-sheet-light.png, shots/contact-sheet-dark.png

## Ratchet

ok

## Missing inputs

- questionnaire scores: no people in this round
- L5 probe misses: not run (paid)
