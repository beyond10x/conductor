---
format: aep.planning-md/3
id: approval-record:bg-launch-reads-user-settings
kind: approval-record
status: accepted
title: 'Approved: the none provider''s --bg launches read user settings'
relations:
- decides: story:bg-launch-reads-user-settings
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-11-06T15:42:49Z", actor: "human:timo", revision: 2}
---
## Decision

The operator's orchestrator approved this fix on 2026-11-06 as one consumer-blocked release: "fix
https://github.com/beyond10x/conductor/issues/83 and release it as one consumer-blocked fix",
"Find the narrowest fix by probing the installed `claude` … Only if none works, add `user` to
--setting-sources for the launches that need it, and record the trade-off against
story:sessions-without-global-config / story:agent-needs-user-setting-source in the store."

No form of the acceptance outside user or managed settings satisfied `claude --bg` (probes in
`story:bg-launch-reads-user-settings`), so `user` is added for the `none` provider's `--bg`
launches only. The trade-off is recorded in that story.
