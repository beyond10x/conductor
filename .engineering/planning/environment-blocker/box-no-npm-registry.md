---
format: aep.planning-md/3
id: environment-blocker:box-no-npm-registry
kind: environment-blocker
status: open
title: The agent box resolves no npm registry (#198)
summary: registry.npmjs.org does not resolve in the box, so no npm ci, Vitest or dashboard dist/ build runs there; the supervisor does not build in this repository
refs:
- provider: github
  reference: beyond10x/conductor#198
relations:
- blocks: story:dashboard-unit-merge-setting
revision: 1
---
# The agent box resolves no npm registry (#198)
