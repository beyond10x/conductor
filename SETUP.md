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
     When the release has no archive for this machine's target:
     `cargo install --git https://github.com/beyond10x/conductor --tag v<version> --locked conductor-cli`.
     Once a conductor is installed, `conductor upgrade` does all of this for the newest release:
     it downloads the archive and the checksums, checks the sum, and replaces the binary, keeping
     the old one as conductor.previous beside it; `conductor upgrade --check` only says whether a
     newer release exists.
   - Hands: none when `~/.local/bin` is on `PATH`; otherwise the person adds it.

4. **Install `aep`, `ess` and `worktree` at conductor's pins.** `conductor doctor` names each pin,
   on a line such as `aep: 0.69.1 is older than the build's pin 0.71`, and under the row of each
   one missing or too old prints where to get it, such as
   `  install: follow https://github.com/beyond10x/agentplugins/releases/latest/download/SETUP.md (needs aep 0.71)`;
   for `task`, `jq`, `gh` and `claude` it prints this machine's command the same way, such as
   `  install: brew install go-task`. A pin `x.y` takes the
   newest `x.y.z` release of that program, `<v>` below, listed at
   https://github.com/beyond10x/aep/releases, https://github.com/beyond10x/ess/releases and
   https://github.com/beyond10x/worktree/releases; their tags carry no `v`. `<name>` below is the
   program's name.
   - Check: `conductor doctor`; each of the three meets the build's pin, or its pin under a
     b10x.toml.
   - Fix: for each program that does not,
     `curl -fsSLO https://github.com/beyond10x/<name>/releases/download/<v>/<name>-<v>-<target>.tar.gz`,
     `curl -fsSLO https://github.com/beyond10x/<name>/releases/download/<v>/SHA256SUMS`,
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
   the repository when its checkouts root is the directory that holds the repository. Add one
   under `instances:`, with the source step 1 found; a file that is not there yet starts with
   `version: conductor.config/1`:

   ```yaml
   version: conductor.config/1
   instances:
     - name: <repository's directory name>
       sources:
         - github: <owner>                 # GitHub
         # - gitlab: <group>               # GitLab
         #   host: <host>
         # - local: <directory holding the repository>   # no forge
       checkouts:
         root: <directory holding the repository>
         trees: ~/.local/state/worktree/trees/<repository's directory name>
   ```

   - Check: `conductor config show`; one instance's checkouts root is the directory that holds
     the repository.
   - Fix: write the instance above into the config file, then `conductor config validate`, which
     names each problem by its YAML path.
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

9. **Check the repository as a whole, and apply what is safe.** `conductor setup` prints doctor's
   rows, then the repository's: its planning store, the instance naming it, its records and its
   trust, and ends with one verdict. `--apply` writes the instance config when there is no config
   file, and runs `conductor init` and `conductor trust` when they are missing. It never logs in,
   installs a program or migrates the store: it prints each of those as a `to run:` or
   `to install:` line, which goes back to step 2, 4 or 5.
   - Check: `conductor setup`; its last line is ready.
   - Fix: `conductor setup --apply`
   - Hands: what each `to run:` and `to install:` line it prints needs: a login, an install that
     needs root, approving the store migration.

10. **Run doctor until it is ready.** Its last line is `ready`, or `not ready:` and what failed.
    - Check: `conductor doctor`
    - Fix: go back to the step for each program the not-ready line names: gh or glab to step 2,
      aep, ess or worktree too old to step 4. Install any other missing program (git, claude,
      jq, task) with the system's package manager, then check again.
    - Hands: what the step it goes back to needs.
    - Next: once doctor is ready, [one task in an existing repository](https://beyond10x.github.io/conductor/docs/guides/first-task-existing-repo)
      takes the person from here to a first merged pull request.

## When it is done

Tell the person what was installed, the account each forge CLI is logged in as, and the
instance's name. [Start conductor](https://beyond10x.github.io/conductor/docs/guides/start-conductor)
starts its loop with `conductor run --install`.
