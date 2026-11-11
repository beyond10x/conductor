---
title: Run conductor in a container
sidebar_position: 10
description: Pull the container image each conductor release publishes for linux/amd64, run conductor from it, and read what the image holds and which user it runs as.
lede: Each release pushes one image with the release's own binary and the programs conductor starts, and it runs as a user that is not root.
source: Dockerfile; .github/workflows/release.yml; crates/conductor/src/doctor.rs; b10x.toml
---

## The image

Each release after 1.2.0 pushes a container image for linux/amd64 to
`ghcr.io/beyond10x/conductor`. Pulling it needs no login.

| name | is |
|---|---|
| `ghcr.io/beyond10x/conductor:<version>` | the image of one release, by the version without the `v` of its tag |
| `ghcr.io/beyond10x/conductor:latest` | the image of the newest release |
| `ghcr.io/beyond10x/conductor@sha256:<digest>` | exactly one image, whatever a tag names later |

```console
docker run --rm ghcr.io/beyond10x/conductor:latest conductor --version
```

It prints `conductor <version>`, the version of the release the image belongs to.

The notes of each GitHub Release end with a section `## Container image`. It names the release's
tag, `ghcr.io/beyond10x/conductor:<version>`, and the digest of the image behind it.

## What the image holds

The `conductor` binary is the one in the release's `x86_64-unknown-linux-gnu` archive. The image
is built around that file and compiles nothing.

Beside it are the programs `conductor doctor` checks for an instance with a source on a forge:

| program | from |
|---|---|
| `git`, `jq` | the packages of the image's distribution, Debian 13 |
| `gh`, `glab` | a release of each, checked against that release's checksums |
| `claude` | the harness's own installer |
| `aep`, `ess`, `worktree` | their release archives at the pins of `b10x.toml`, checked against `SHA256SUMS` |
| `b10x-gates` | its newest release when the image is built, checked against `SHA256SUMS` |

`substrate-daemon` is not in the image. Only an instance whose `placement` opens a box needs it.

## The user

Every command runs as the user `conductor`, user id 10001, which is not root. Its home
directory is `/var/lib/conductor`, and the image starts there. The image has no entrypoint, so
the words after the image's name are the command, and nothing in it needs a terminal:

```console
docker run --rm ghcr.io/beyond10x/conductor:latest conductor doctor
```

The image holds no config file and no login. Without a config file, `conductor doctor` lists
`gh` and `glab` as `not needed` and every other program above as `present`, and ends `ready`.
