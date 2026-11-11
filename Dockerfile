# The container image of a conductor release, for linux/amd64: the release's own `conductor`
# binary and every program `conductor doctor` checks for an instance with a forge source.
#
# The build context holds one file, `conductor`, the binary of the release's
# `x86_64-unknown-linux-gnu` archive; `.github/workflows/release.yml` builds and pushes the image
# from it. Nothing is compiled here.
#
# `aep`, `ess` and `worktree` come from their release archives at the pins of `b10x.toml`, and
# the Gates CLI from its newest release, each by the command `conductor doctor` prints as
# `install:` with `/usr/local/bin` in place of `~/.local/bin`. `gh` and `glab` come from their
# own releases, checked against the release's checksums; `git` and `jq` from the distribution;
# the harness from its installer, as the image's user. Every `tar` here runs as root and takes
# root's owner and the umask, not the archive's, so no other uid can replace a program.
# `crates/conductor/tests/release_image.rs` holds this file to that.
FROM debian:13-slim

ARG AEP_VERSION=0.79.1
ARG ESS_VERSION=0.90.0
ARG WORKTREE_VERSION=0.18.1
ARG GH_VERSION=2.103.3
ARG GLAB_VERSION=1.130.0
ARG CLAUDE_CODE_VERSION=latest

LABEL org.opencontainers.image.title="conductor" \
      org.opencontainers.image.description="conductor and the programs it starts" \
      org.opencontainers.image.source="https://github.com/beyond10x/conductor" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update \
 && apt-get install --yes --no-install-recommends ca-certificates curl git jq openssh-client \
 && rm -rf /var/lib/apt/lists/* \
 && git --version \
 && jq --version

RUN mkdir /fetch && cd /fetch \
 && curl -fsSLO https://github.com/beyond10x/aep/releases/download/${AEP_VERSION}/aep-${AEP_VERSION}-x86_64-unknown-linux-gnu.tar.gz \
 && curl -fsSLO https://github.com/beyond10x/aep/releases/download/${AEP_VERSION}/SHA256SUMS \
 && sha256sum --check --ignore-missing SHA256SUMS \
 && tar --no-same-owner --no-same-permissions -xzf aep-${AEP_VERSION}-x86_64-unknown-linux-gnu.tar.gz -C /usr/local/bin --strip-components=1 aep-${AEP_VERSION}-x86_64-unknown-linux-gnu/aep \
 && cd / && rm -r /fetch \
 && aep --version

RUN mkdir /fetch && cd /fetch \
 && curl -fsSLO https://github.com/beyond10x/ess/releases/download/${ESS_VERSION}/ess-${ESS_VERSION}-x86_64-unknown-linux-gnu.tar.gz \
 && curl -fsSLO https://github.com/beyond10x/ess/releases/download/${ESS_VERSION}/SHA256SUMS \
 && sha256sum --check --ignore-missing SHA256SUMS \
 && tar --no-same-owner --no-same-permissions -xzf ess-${ESS_VERSION}-x86_64-unknown-linux-gnu.tar.gz -C /usr/local/bin --strip-components=1 ess-${ESS_VERSION}-x86_64-unknown-linux-gnu/ess \
 && cd / && rm -r /fetch \
 && ess --version

RUN mkdir /fetch && cd /fetch \
 && curl -fsSLO https://github.com/beyond10x/worktree/releases/download/${WORKTREE_VERSION}/worktree-${WORKTREE_VERSION}-x86_64-unknown-linux-gnu.tar.gz \
 && curl -fsSLO https://github.com/beyond10x/worktree/releases/download/${WORKTREE_VERSION}/SHA256SUMS \
 && sha256sum --check --ignore-missing SHA256SUMS \
 && tar --no-same-owner --no-same-permissions -xzf worktree-${WORKTREE_VERSION}-x86_64-unknown-linux-gnu.tar.gz -C /usr/local/bin --strip-components=1 worktree-${WORKTREE_VERSION}-x86_64-unknown-linux-gnu/worktree \
 && cd / && rm -r /fetch \
 && worktree --version

RUN mkdir /fetch && cd /fetch \
 && curl -fsSLO https://github.com/beyond10x/gates/releases/latest/download/b10x-gates-linux-x86_64 \
 && curl -fsSLO https://github.com/beyond10x/gates/releases/latest/download/SHA256SUMS \
 && sha256sum --check --ignore-missing SHA256SUMS \
 && install -m 755 b10x-gates-linux-x86_64 /usr/local/bin/b10x-gates \
 && cd / && rm -r /fetch \
 && b10x-gates --version

RUN mkdir /fetch && cd /fetch \
 && curl -fsSLO https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_linux_amd64.tar.gz \
 && curl -fsSLO https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_checksums.txt \
 && sha256sum --check --ignore-missing gh_${GH_VERSION}_checksums.txt \
 && tar --no-same-owner --no-same-permissions -xzf gh_${GH_VERSION}_linux_amd64.tar.gz -C /usr/local/bin --strip-components=2 gh_${GH_VERSION}_linux_amd64/bin/gh \
 && cd / && rm -r /fetch \
 && gh --version

RUN mkdir /fetch && cd /fetch \
 && curl -fsSLO https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/glab_${GLAB_VERSION}_linux_amd64.tar.gz \
 && curl -fsSLO https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/checksums.txt \
 && sha256sum --check --ignore-missing checksums.txt \
 && tar --no-same-owner --no-same-permissions -xzf glab_${GLAB_VERSION}_linux_amd64.tar.gz -C /usr/local/bin --strip-components=1 bin/glab \
 && cd / && rm -r /fetch \
 && glab --version

# The user every command of the image runs as: not root, named by number so a cluster that
# requires a non-root user can check it, with a home directory for the config file, the logins
# and the harness. The home is under `/var/lib`, as a service account's is.
RUN groupadd --gid 10001 conductor \
 && useradd --uid 10001 --gid 10001 --create-home --home-dir /var/lib/conductor --shell /bin/bash conductor
USER 10001:10001
ENV HOME=/var/lib/conductor \
    PATH=/var/lib/conductor/.local/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
WORKDIR /var/lib/conductor

# The harness installs into the home directory of the user who runs its installer. The version
# check fails the build when the download did not arrive.
RUN curl -fsSL https://claude.ai/install.sh | bash -s -- "${CLAUDE_CODE_VERSION}" \
 && claude --version

# Last, so a new binary rebuilds no layer above.
COPY --chmod=755 conductor /usr/local/bin/conductor

# No entrypoint: `docker run <image> conductor --version` runs conductor, and so does any other
# command line. Nothing here needs a terminal.
CMD ["conductor", "--help"]
