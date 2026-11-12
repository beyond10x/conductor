---
format: aep.planning-md/3
id: decision-blocker:release-run-shows-image
kind: decision-blocker
status: cleared
title: 'No release has run since the image jobs merged: the push and the Release notes are unobserved'
relations:
- blocks: story:release-container-image
withholds: test_result
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-11-12T00:05:02Z", actor: "human:timo", revision: 2}
---
## What is blocked

`story:release-container-image` is merged with wave 51: the image file, the release workflow's
jobs `image` and `image-notes`, and cases that run the jobs' step scripts against stand-in
programs. Two of its three acceptance lines can be observed only on a release run:

- the workflow pushes the image to the public registry under the release's version tag;
- the Release notes name the image by tag and digest, and the documented `docker run` of that
  image prints the version.

No release has run since the unit merged. Also unobserved until then: that the registry accepts
a second push of a version tag when the workflow is run again for the same release.

## What clears it

The operator decides the next release. Its workflow run shows the push and the notes; the
`docker run` line of the guide is run once against the pushed image. If one of them fails, the
finding goes back to the story.
