# R-01 decision: Add an offline full-text search to the documentation site?
Finding: UXF-001. Target metric: site-contract/always_visible_violations@1280x800, now 144 (all search), expected 0; @390x844, now 288 (144 search, 144 theme toggle), expected 0 after UXF-002.
Yes: add a build-time local search plugin to `website/` (a new npm dependency, no keys, no external service) with its control in the navbar at every width, shot: none (a mock is not built; the build waits for the answer)
No:  the site stays without search; readers use the sidebar and the browser's find, and UXF-001 is closed as accepted with this record as the reason, shot: shots/decision-no.png (the current navbar)
Default if unanswered by the next round: yes, because 36 pages and a 40-entry sidebar already exceed what a reader can scan, search is on the always-visible list of the loop's docs adapter, and the dependency is build-time only.
Answer: pending
