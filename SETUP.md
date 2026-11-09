# Set conductor up in a repository

The steps an agent follows when a person asks it to set conductor up in the repository it works
in. Run them in order, from the repository's top directory. Each step says three things:

- **Check:** a command whose output says whether the step is already done. When it is, go to the
  next step, so the whole list is safe to run again.
- **Fix:** the exact command that does the step.
- **Hands:** what only the person can do, such as a login, an install that needs root, or
  approving a store migration; or none. Ask the person for it, say why, and wait.

Every span in backticks on a Check or Fix line is a command; paths and output are written without
backticks there. `<version>`, `<target>`, `<host>` and the other words in angle brackets are values
to fill in, and each step says where they come from. `<target>` is the build of this machine:
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` or
`aarch64-apple-darwin`. On macOS, `shasum -a 256` stands in for `sha256sum`.

## Steps

1. **Find the forge.** An origin on github.com is GitHub. An origin on another host is GitLab on
   that host, `<host>`, such as gitlab.example.com; ask the person when the host is neither. No
   origin is no forge: the instance gets a `local` source, and step 2 is skipped.
   - Check: `git remote get-url origin`
   - Fix: none; this step only reads.
   - Hands: none.

2. **Install the forge's CLI and log it in to that host.** `gh` for GitHub, `glab` for GitLab.
   - Check: `gh auth status --hostname github.com` for GitHub, or
     `glab auth status --hostname <host>` for GitLab; it names the account it is logged in as.
   - Fix: install the CLI with the system's package manager (gh from https://cli.github.com, glab
     from https://gitlab.com/gitlab-org/cli), then `gh auth login --hostname github.com` or
     `glab auth login --hostname <host>`.
   - Hands: the login, which opens a browser or asks for a token; an install that needs root.

3. **Install conductor.** `<version>` is the newest release listed at
   https://github.com/beyond10x/conductor/releases.
   - Check: `conductor --version`; it prints conductor and its version.
   - Fix: download the archive and the release's checksums, check the archive, and unpack the
     binary:
     `curl -fsSLO https://github.com/beyond10x/conductor/releases/download/v<version>/conductor-<version>-<target>.tar.gz`,
     `curl -fsSLO https://github.com/beyond10x/conductor/releases/download/v<version>/SHA256SUMS`,
     `sha256sum --check --ignore-missing SHA256SUMS` (it must print OK for the archive; stop
     when it does not), then
     `tar -xzf conductor-<version>-<target>.tar.gz -C ~/.local/bin --strip-components=1 conductor-<version>-<target>/conductor`.
     When the release has no archive for this machine's target, stop and tell the person: a build
     from source is in https://beyond10x.github.io/conductor/docs/develop/onboarding#build-from-source.
     Once a conductor is installed, `conductor upgrade` does all of this for the newest release
     and keeps the old binary as conductor.previous beside it.
   - Hands: none when `~/.local/bin` is on `PATH`; otherwise the person adds it.

4. **Install `aep`, `ess`, `worktree` and `substrate-daemon` at conductor's pins.**
   `conductor doctor` names each pin, on a line such as `aep: 0.69.1 is older than the build's pin 0.79`,
   and under the row of each one missing or too old prints where to get it, such as
   `  install: follow https://github.com/beyond10x/agentplugins/releases/latest/download/SETUP.md (needs aep 0.79)`;
   for `jq`, `gh` and `claude` it prints this machine's command the same way, such as
   `  install: brew install jq`. Install the four with this step's Fix, not that page: the page
   installs the `b10x` plugins as well and asks the person first, and this step asks nothing.
   A pin `x.y` takes the newest `x.y.z` release of that
   program, `<v>` below. `<name>` is the program's name and `<repo>` its repository: `aep`, `ess`
   and `worktree` for the first three, `substrate` for `substrate-daemon`, whose releases carry
   `x86_64-unknown-linux-gnu` only. The releases are listed at
   https://github.com/beyond10x/<repo>/releases; their tags carry no `v`. Doctor needs
   `substrate-daemon` once the config of step 6 names `placement: systemd`, as the file step 6
   writes does: install it here even while doctor prints `substrate-daemon: not needed`.
   - Check: `conductor doctor`; each of the four meets the build's pin, or its pin under a
     b10x.toml.
   - Fix: for each program that does not,
     `curl -fsSLO https://github.com/beyond10x/<repo>/releases/download/<v>/<name>-<v>-<target>.tar.gz`,
     `curl -fsSLO https://github.com/beyond10x/<repo>/releases/download/<v>/SHA256SUMS`,
     `sha256sum --check --ignore-missing SHA256SUMS`, then
     `tar -xzf <name>-<v>-<target>.tar.gz -C ~/.local/bin --strip-components=1 <name>-<v>-<target>/<name>`.
   - Hands: none.

5. **Bring the planning store up to date.** The store is `.engineering/` in the repository; its
   `version` line reads `aep.project/5` when it is current.
   - Check: `git show HEAD:.engineering/project.yaml`; when there is no such file the repository
     has no planning store, and there is nothing to migrate.
   - Fix: `aep plan store migrate git --dry-run`; show the person what it would change, and only
     after they agree, `aep plan store migrate git --verify`.
   - Hands: approving the migration, which rewrites the store's files.

6. **Write the instance config for the repository.** The config file is
   `~/.b10x/conductor/conductor.yaml` unless `CONDUCTOR_CONFIG` names another. An instance names
   the repository when its checkouts root is the directory that holds the repository. A file that
   is not there yet is this one, with the forge step 1 found:

   ```yaml
   version: conductor.config/2
   daemon:
     store: sqlite:~/.b10x/conductor/conductor.sqlite
   instances:
     - name: <repository's directory name>
       placement: systemd
       session_prefix: <repository's directory name>
       sources:
         - github: <owner>
       checkouts:
         root: <directory holding the repository>
         trees: ~/.local/state/worktree/trees/<repository's directory name>
   ```

   For GitLab the source is `gitlab: <group>` with `host: <host>` under it; with no forge it is
   `local: <directory holding the repository>`. A file that is there already gets the instance
   under its `instances:`, with a `session_prefix` no other instance has.
   - Check: `conductor config show`; one instance's checkouts root is the directory that holds
     the repository.
   - Fix: write the file or the instance above, then `conductor config validate`, which names
     each problem by its YAML path. A file whose version is conductor.config/1 is rewritten
     first with `conductor config migrate`.
   - Hands: none.

7. **Write the records directory.** `<records>` is the instance's records directory, the
   `records` value `conductor config show` prints.
   - Check: `git -C <records> log --oneline -1`; a records directory that is there prints the
     commit Seed the records directory with conductor init.
   - Fix: `conductor init --instance <name>`
   - Hands: none.

8. **Trust the checkouts and the records** as Claude Code workspaces.
   - Check: none is needed; `conductor trust --instance <name>` is safe to repeat.
   - Fix: `conductor trust --instance <name>`
   - Hands: none.

9. **Start the substrate daemon.** With `placement: systemd`, conductord runs the instance's
   runtime in a box it asks the local `substrate-daemon` for. Conductord does not start
   `substrate-daemon`, and doctor checks only its version. It runs inside a delegated cgroup
   subtree carrying cpu, memory and pids, passed as `--cgroup-root`
   (https://github.com/beyond10x/substrate#serving-exec), and serves the socket conductord opens,
   `$XDG_RUNTIME_DIR/substrate/daemon.sock`. `<state>` is the instance's `state` value in
   `conductor config show`; create `<state>/sessions` and ~/.local/state/substrate/workspaces mode
   700 before the daemon starts, since it refuses a writable root that does not exist. `<uid>` is the user's numeric id.
   - Check: `curl --silent --unix-socket $XDG_RUNTIME_DIR/substrate/daemon.sock http://localhost/v1/machine`;
     it answers, and its exec.writable-roots.host fact lists sessions-<name>.
   - Fix: `substrate-daemon --socket $XDG_RUNTIME_DIR/substrate/daemon.sock --state ~/.local/state/substrate/state.db --workspaces ~/.local/state/substrate/workspaces --deployment personal --allow-uid <uid> --cgroup-root <delegated cgroup root> --writable-root sessions-<name>=<state>/sessions --egress-aperture conductor=api.anthropic.com:443/tcp --ca-bundle /etc/ssl/certs/ca-certificates.crt`,
     run inside that subtree and kept running.
   - Hands: the delegated cgroup subtree, when the user's systemd does not delegate one.

10. **Start conductord.** It starts the instance's runtime in that box, and the runtime runs the
    instance's loop.
    - Check: `conductor doctor`; it prints daemon unit: conductord.service running.
    - Fix: `conductor daemon serve --install`, then
      `curl --silent http://127.0.0.1:7300/v1/instances`, which lists the instance. A box
      substrate refused shows as PlacementRefused with substrate's reason.
    - Hands: none.

11. **Check the repository as a whole, and apply what is safe.** `conductor setup` prints doctor's
    rows, then the repository's: its planning store, the instance naming it, its records and its
    trust, and ends with one verdict. `--apply` writes the step 6 file when there is no config
    file, and runs `conductor init` and `conductor trust` when they are missing. It never logs in,
    installs a program or migrates the store: it prints each of those as a `to run:` or
    `to install:` line, which goes back to step 2, 4 or 5.
    - Check: `conductor setup`; its last line is ready.
    - Fix: `conductor setup --apply`
    - Hands: what each `to run:` and `to install:` line it prints needs: a login, an install that
      needs root, approving the store migration.

12. **Run doctor until it is ready.** Its last line is `ready`, or `not ready:` and what failed.
    - Check: `conductor doctor`
    - Fix: go back to the step for each program the not-ready line names: gh or glab to step 2,
      aep, ess, worktree or substrate-daemon too old to step 4. Install any other missing program
      (git, claude, jq) with the system's package manager, then check again.
    - Hands: what the step it goes back to needs.
    - Next: once doctor is ready, [one task in an existing repository](https://beyond10x.github.io/conductor/docs/guides/first-task-existing-repo)
      takes the person from here to a first merged pull request.

## When it is done

Tell the person what was installed, the account each forge CLI is logged in as, the instance's
name, and whether `/v1/instances` shows its runtime running.
[Run conductor](https://beyond10x.github.io/conductor/docs/guides/start-conductor) says how to
operate it from there.
