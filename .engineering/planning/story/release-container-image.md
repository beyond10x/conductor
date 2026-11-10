---
format: aep.planning-md/3
id: story:release-container-image
kind: story
status: draft
title: Each release publishes a container image of conductor
relations:
- decomposes: epic:deployable-in-kubernetes
scope:
- confidence: cited
  path: .github/workflows/release.yml
revision: 2
---
## Why

A consumer control plane that wants to run one conductor instance in a local cluster asked first for a published image (2026-11-10). The release workflow attaches binary archives only; the repository holds no image build.

## Acceptance

- The release workflow builds a container image for linux/amd64 from the release's own binary and pushes it to a public registry under the release's version tag.
- The Release notes name the image by tag and digest, and a documented `docker run <image> conductor --version` prints the version.
- The image runs as a non-root user and holds the programs `conductor doctor` needs for an instance with a forge source.
