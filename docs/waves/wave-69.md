# Wave 69 (2026-11-20: one command for the supervisor's watch)

The fourteenth wave of the goal that fixes the friction the agent box filed: issue #254, which
folds in #247. It forks from `main` after release 1.20.1. The wave before is
[wave 68](wave-68.md). The critic panel's result: `review-result:w69-critic-panel`.

Width: one unit. The forge triggers (`story:supervisor-watch-forge-triggers`) depend on the
command and share a file with it, so they follow once it merges; #245 and #246 are being
drafted for wave 70 meanwhile.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | supervisor-watch-command | #254, #247 | merged |

## Outcome

`conductor supervisor watch` replaces the supervisor's hand-written watch: it blocks until a
trigger of any instance fires (MAIL, LOG, DAEMON, INBOX, DISK, UNPUSHED, IDLE) or its timeout
ends (TIMEOUT), prints one line per fired trigger, and keeps its cursors under the host
directory, so a restart skips nothing. The supervisor profile arms it in one background call
and names no watch of its own. The config key `globals.supervisor` names the product checkout.

An adversary review found ways a trigger could be lost: the log of an instance in a unit is its
unit's journal, not a file; a mail file could land below the cursor or be unreadable; the idle
trigger matched other repositories' trees by name prefix and skipped a controller with no
transcript; a quiet pass or a timeout dropped its cursor moves; the disk baseline moved on every
pass; a commit on a detached head was not seen; a huge timeout panicked. Each is fixed with a
test that failed first. The tests read a fake journal, never the host's.

In the box: the specification, the binding, a fresh generation, clippy and the supervisor watch,
profile, CLI tree and config suites; `api_parity` and `dashboard_config` bind a socket, and CI
proves them.
