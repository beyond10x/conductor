# Wave 71 (2026-11-20: the supervisor watch reads the forge)

The sixteenth wave of the goal that fixes the friction the agent box filed: the forge triggers of
#254, and #247. It forks from wave 69's head, whose command it extends. The wave before is
[wave 69](wave-69.md). The critic panel's result: `review-result:w69-critic-panel`, which split
these triggers off.

Width: one unit (the forge triggers extend the command of wave 69 and share its module).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | supervisor-watch-forge-triggers | #247, #254 | merged |

## Outcome

`conductor supervisor watch` fires `GREEN` for an open request that is not a draft, has no
auto-merge and whose checks ended green over 15 min ago, once per head sha unless `--hold` names
it, and `CIEND` when a running controller's newest report is `blocked ci <ref>` and that request's
checks ended after the report. It reads GitHub and GitLab with the host's `gh` and `glab` at most
every 2 min per instance, within a budget per pass; a refused read is one note and backs the
instance off to at most 16 min, and the other triggers go on.

An adversary review found that a GitLab pipeline run again on the same head was never seen, that a
ref of another repository matched the controller's own, that serial forge reads could hold a pass
past its timeout, that requests with no pipeline were read every pass, that a refused pass kept
stale reads over fresh ones, and that an error in the forge cursors stopped every trigger. Each is
fixed with a test that failed first.
