---
format: aep.planning-md/3
id: story:config-fields-read
kind: story
status: draft
title: Every config field conductor accepts is read by something
revision: 1
---
## Why

Drafting the public documentation found fields `conductor config validate` accepts that no command
reads: `controllers`, `repositories`, `cadence.cycle`, `cadence.daily`, `thresholds.disk_admit`,
`thresholds.build_slot`, `reports`, `authority`. A user who sets them sees no effect. The draft of
generic profiles adds 9 more keys (agent, settings file, most sub-agents, gate watchdog levels).

## Acceptance

Each field is read by the command or profile text it configures (`config show` feeding the
Taskfile and the profiles counts), or is removed from `spec/domains/config.yaml`.
