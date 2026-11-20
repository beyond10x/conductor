---
format: aep.planning-md/3
id: upstream-blocker:substrate-aperture-bound
kind: upstream-blocker
status: open
title: Substrate takes at most 4 egress apertures per box start
summary: 'MAX_EGRESS_APERTURES is 4; a GitHub source with the model host and the dial-in takes all 4, so no registry fits; asked of beyond10x/substrate: raise the bound, or one proxy aperture serving several registries'
refs:
- provider: github
  reference: beyond10x/conductor#198
relations:
- blocks: story:box-registry-egress
revision: 1
---
# Substrate takes at most 4 egress apertures per box start
