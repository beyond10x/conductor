//! The guard's rule table: which tool calls a session may make, by the repository it runs in.
//!
//! Where the repositories and conductor's records are comes from the instance the process runs
//! ([`Places`], read from `crate::config::active`): the checkouts root `<root>`, the managed-worktree
//! root `<trees>`, and conductor's records `<records>`. Without a config file they are the built-in
//! instance's, and `<records>` is the checkout of the repository named `conductor`. Conductor's
//! config, `<config>`, is the file the process resolves (`config::locate`: `CONDUCTOR_CONFIG`, else
//! [`config::DEFAULT_FILE`] under the home directory; [`Places::with_config_file`]), the default
//! file, the directory of the default file, `<config-dir>`, itself (not what is under it), and
//! with a config file the instance's `state` directory and anything under it. A controller does not
//! write `<config>`, so that it cannot give itself conductor's rules or a wider checkout
//! (`story:guard-write-forms-and-config-scope`). The rest of `<config-dir>` is not the config: an
//! instance's `<records>` are there by default, and keep their own rule.
//!
//! `<settings>` is the `settings` file of any role of the instance (`config::active`), the files
//! that wire the guard, and `<profile>` the `profile` file of any role, the system prompt its
//! sessions start with.
//!
//! | tool | controller: denied when | conductor: denied when |
//! |---|---|---|
//! | Edit, Write, NotebookEdit | the target is `<config>`, wherever it is; is outside `<root>/<repo>/` and `<trees>/<repo>/`; is in `<records>`; is a `<settings>` or a `<profile>`; or is a `.claude/` settings file (a `.json` whose name holds `settings`: `settings.json`, `settings.local.json`, `controller-settings.json`, `conductor-settings.json`) inside them | the target is a `<settings>`, a `<profile>` or a `.claude/` settings file, or is not one of conductor's records |
//! | SendMessage | the recipient is anyone but `<conductor>` or `<conductor> [<ref>]` ([`named`]); an agent the session started, by its id ([`is_own_agent_id`]); or a socket address `uds:<path>/<pid>.sock` whose pid, where the path lands, the session list ([`SessionList`]) gives a live session named `<conductor>` | with a `session_prefix` `<p>`: the recipient is anyone but a session named `<p>-<name>` (or `<p>-<name> [<ref>]`); an agent the session started; or a socket address whose pid the session list gives a live session named `<p>-<name>`. Without one: never |
//!
//! `<conductor>` is the instance's conductor session name (`story:instance-session-names`):
//! `<p>-conductor` when the instance has a `session_prefix` `<p>`, else `conductor`. A denied
//! recipient whose name carries another instance's prefix is named with both instances. The
//! instances a recipient is read against are those of the config file and, for a SendMessage
//! only, those of every other file the host registry lists (`story:instance-names-unique`,
//! [`Places::with_registered`]): a conductor without a prefix is no longer alone when another
//! file's instance is registered, and a message to that instance's session is denied.
//! | Bash | a `cd`, `pushd` or `git -C` (`--git-dir`, `--work-tree`, `GIT_DIR=`, `GIT_WORK_TREE=`) reaches another repository's checkout or managed worktrees, or `<records>`; a git subcommand that writes ([`git_reads`]) has a target (the directory `-C` leaves it in, else the cwd; `--git-dir`, `--work-tree`, `GIT_DIR`, `GIT_WORK_TREE`) outside `<root>/<repo>/`, `<trees>/<repo>/` and the session's scratch; a `gh` or `glab` call that is not a read ([`forge_write`]); a run of the bot route `b10x-gates gh` that is not `release create|upload|edit|view`, `run rerun` or `workflow run` with `--repository` naming the repository the checkout's `origin` names ([`bot_route`]); the command writes a `.claude/` settings file or a `<settings>` ([`Places::writes_settings`]); the command writes a `<profile>` ([`Places::writes_profile`]); the command names `<config>` and holds a write form ([`Places::names_config`], [`holds_write_form`]); it runs a `conductor` leaf the command tree does not declare a controller may run ([`cli::LEAVES`], [`conductor_run`]), or one whose leaf a shell expansion, `xargs` or a program word a substitution names decides at run time; or the command writes `<records>` or a path under it ([`Places::writes_records`]) | a `gh` or `glab` call that is not a read; a run of the bot route as for a controller, its own repository the one the `origin` of `<records>` names; the command writes a `.claude/` settings file or a `<settings>`; the command writes a `<profile>` |
//!
//! A Bash command that kills by pattern, name or group is denied for every role, before the rows
//! above ([`kill_denial`], `story:guard-no-pattern-kill`): `pkill`, `killall`, and a `kill` that
//! is not given numeric process ids (rules.md item 147).
//!
//! Monitor runs a shell command, and is decided by the Bash row. Any other tool has no row: it is
//! allowed from the built-in read-only allowlist (`READ_ONLY` in `guard.rs`) or the session's
//! role's `read_only_tools` (`conductor` for conductor's rules, `controller` for a controller's),
//! and denied otherwise (`story:guard-all-tools`).
//!
//! The Bash row is a heuristic over the command line (design § 7: it catches the common forms,
//! not all). Its accepted limits are pinned by `tests/adv_guard.rs`. A Bash command is decided on
//! its write targets for the records, the settings and the profile rows ([`writes_named`]): the
//! targets of its redirections and the operands of its writing programs, a path named only as a
//! read operand, in a quoted program or in a here-document's body being no write; a target the
//! guard cannot read (a variable, a substitution) or a shell or interpreter on the line falls back
//! to the name and the write form. The config row still names a path by a substring test and holds
//! a write form anywhere in the command: a redirection read by the shell's grammar (outside
//! quotes, to a file: `2>&1`, `>&2` and `>/dev/null` are none), or a write command or flag as a
//! word or substring, so a command that names the config while it writes another file is denied.
//! `cd .claude && … > settings.json` and `cd <config-dir> && cp x conductor.yaml` are not caught,
//! nor is a protected file written by a relative path from the cwd.
//!
//! A path belongs to a repository when it is under the repository's checkout, `<root>/<repo>/` or
//! `<root>/<group>/<repo>/` (the directory holding `.git` at depth 1 or 2,
//! `story:grouped-instance`), or under its managed worktrees, `<trees>/<repo>/` or
//! `<trees>/<group>/<repo>/` ([`Places::repository_of`]): a controller's workers run in the
//! latter, and their calls are decided as the controller's own. A group directory itself belongs
//! to no repository, and a session in a repository a source excludes runs under no rule set.
//!
//! Paths are compared where they land: `~` is the home directory, a relative path is taken from
//! the session's cwd (or the directory a `cd` before it moved to), and every symbolic link on the
//! way is followed, dangling or not, before the comparison.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use conductor_model::config::{Instance, RoleName, SessionPrefix};
use conductor_model::dispatch::Verdict;
use serde_json::Value;

use super::shell;
use crate::cli::{self, Access};
use crate::collect::{self, Under};
use crate::config::{self, Active};

/// How long reading the session list may take before a socket address is denied.
const SESSION_LIST_BOUND: Duration = Duration::from_secs(5);

/// Where a controller's SendMessage to a socket address is checked: the command line that prints
/// the Claude session list as one JSON array, and how long it may run. [`SessionList::default`]
/// gives the real one; a test replaces both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionList {
    /// The program and its arguments.
    pub command: Vec<OsString>,
    /// How long the command may run before it is stopped.
    pub bound: Duration,
}

impl Default for SessionList {
    /// `claude agents --json`, the list the sessions collector reads, bounded at 5 s.
    fn default() -> Self {
        Self {
            command: ["claude", "agents", "--json"].map(OsString::from).to_vec(),
            bound: SESSION_LIST_BOUND,
        }
    }
}

/// Conductor's name: the repository whose sessions conductor's rule decides when there is no
/// config file, the repository a session in conductor's records is recorded under, and the name a
/// controller messages, `<session_prefix>-conductor` when the instance has a prefix
/// ([`Places::conductor_session`]).
pub(super) const CONDUCTOR: &str = "conductor";

/// Conductor's records: directories, under its records.
const RECORD_DIRS: [&str; 5] = [
    "decisions",
    "dispatches",
    "charters",
    "docs/handoff",
    "docs/analysis",
];

/// Conductor's records: files, under its records.
const RECORD_FILES: [&str; 4] = ["goals.jsonl", "STATUS.md", "NORTHSTAR.md", "REPORT.md"];

/// Where the rule table looks, from the instance the process runs: the home directory `~` names,
/// the directory holding the repositories' checkouts, the one holding their managed worktrees,
/// conductor's records when a config file names them, and the config file the process resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    home: PathBuf,
    checkouts: PathBuf,
    trees: PathBuf,
    /// The instance's `records`, with a config file; `None` without one.
    records: Option<PathBuf>,
    /// The instance's `state` directory, with a config file; `None` without one, when the state
    /// is under conductor's records.
    state: Option<PathBuf>,
    /// The instance's own `cache` directory: a controller's scratch is `<cache>/scratch/<repo>`
    /// (`story:scratch-per-instance`).
    cache: PathBuf,
    /// The config file the process resolves ([`config::locate`]); [`config::DEFAULT_FILE`] under
    /// the home directory until [`Places::with_config_file`] names another.
    config: PathBuf,
    /// The `exclude` entries of the instance's sources under the checkouts root
    /// ([`collect::excluded_under`]): a session in one of them is in no repository.
    exclude: Vec<String>,
    /// The session's temporary directory, `$TMPDIR` of the hook's process where it lands
    /// ([`real`]); `None` until [`Places::with_tmp`] names one.
    tmp: Option<PathBuf>,
    /// The `settings` file of each role of the instance: the files that wire the guard, which no
    /// session writes.
    settings: Vec<PathBuf>,
    /// The `profile` file of each role of the instance: the system prompt its sessions start with,
    /// which no session writes.
    profiles: Vec<PathBuf>,
    /// The instance's name, for a denial to name.
    instance: String,
    /// The instance's `session_prefix` (`story:instance-session-names`); `None` keeps today's
    /// session names and rules.
    prefix: Option<String>,
    /// Every instance of the config file, the instance first, each checkouts root and managed
    /// trees where they land ([`real`]): where a recipient belongs is read against them
    /// ([`collect::session_instance`]).
    instances: Vec<Instance>,
    /// The file's `default` instance, whose conductor serves every instance that names no
    /// `conductor` role.
    default: Option<String>,
    /// The `read_only_tools` of the instance's `conductor` role and of its `controller` role
    /// (`story:guard-all-tools`): the tools each rule set allows beyond the built-in read-only
    /// allowlist.
    read_only: [Vec<String>; 2],
    /// The instance's `dashboard_port`: a Bash call that posts to it is denied
    /// (`story:decisions-batch-answer`).
    dashboard_port: i64,
}

impl Places {
    /// The built-in instance's places ([`config::built_in`]) under `home`, which apply without a
    /// config file: conductor's session is the one in the repository named `conductor`, and its
    /// records are under that repository's checkout.
    #[must_use]
    pub fn built_in(home: &Path) -> Self {
        Self::of(home, &config::built_in(home, home), false, &[], None)
    }

    /// The places of `active`, `home` being the home directory: with a config file, its
    /// `checkouts`, and conductor's session is the one whose cwd is in its `records`, where its
    /// records are; without one, [`Places::built_in`].
    #[must_use]
    pub fn active(home: &Path, active: &Active) -> Self {
        Self::of(
            home,
            &active.instance,
            active.from_file,
            &active.others,
            active.default.as_ref().map(|name| name.0.clone()),
        )
    }

    fn of(
        home: &Path,
        instance: &Instance,
        from_file: bool,
        others: &[Instance],
        default: Option<String>,
    ) -> Self {
        // What reading a recipient needs of an instance: its name, prefix, roles and checkouts,
        // the checkouts where they land; its records, state and cache are left out.
        let landing = |instance: &Instance| {
            let mut instance = instance.clone();
            instance.records = String::new();
            instance.state = String::new();
            instance.cache = String::new();
            instance.checkouts.root = real(Path::new(&instance.checkouts.root))
                .to_string_lossy()
                .into_owned();
            instance.checkouts.trees = real(Path::new(&instance.checkouts.trees))
                .to_string_lossy()
                .into_owned();
            instance
        };
        Self {
            home: home.to_path_buf(),
            checkouts: PathBuf::from(&instance.checkouts.root),
            trees: PathBuf::from(&instance.checkouts.trees),
            records: from_file.then(|| PathBuf::from(&instance.records)),
            state: from_file.then(|| PathBuf::from(&instance.state)),
            cache: PathBuf::from(&instance.cache),
            config: home.join(config::DEFAULT_FILE),
            exclude: collect::excluded_under(instance, Path::new(&instance.checkouts.root)),
            tmp: None,
            settings: instance
                .roles
                .iter()
                .filter_map(|role| role.settings.as_ref())
                .map(|settings| PathBuf::from(&settings.0))
                .collect(),
            profiles: instance
                .roles
                .iter()
                .filter_map(|role| role.profile.as_ref())
                .map(|profile| PathBuf::from(&profile.0))
                .collect(),
            instance: instance.name.0.clone(),
            prefix: instance
                .session_prefix
                .as_ref()
                .map(|SessionPrefix(prefix)| prefix.clone()),
            instances: std::iter::once(instance)
                .chain(others)
                .map(landing)
                .collect(),
            default,
            read_only: [Rules::Conductor, Rules::Controller].map(|rules| {
                instance
                    .roles
                    .iter()
                    .filter(|role| config::is_role(&role.role, rules.role_name()))
                    .flat_map(|role| role.read_only_tools.iter().cloned())
                    .collect()
            }),
            dashboard_port: instance.dashboard_port,
        }
    }

    /// Whether the Bash `command` answers for the operator through the dashboard
    /// (`story:decisions-batch-answer`): it names the answers endpoint, `/api/answers`, or names the
    /// instance's dashboard port as `:<port>` (whatever spells the loopback address before it:
    /// `127.0.0.1`, `localhost`, `[::1]`, `0x7f000001`, none) and sends a POST or a body by any client
    /// ([`posts`]). A substring test, a speed bump and not a boundary: a script the command runs, a
    /// port or path a variable holds, or a percent-encoded path is not caught.
    fn posts_to_dashboard(&self, command: &str) -> bool {
        let port = format!(":{}", self.dashboard_port);
        let names_port = command.match_indices(&port).any(|(at, _)| {
            !command[at + port.len()..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_digit())
        });
        command.contains(crate::dashboard::ANSWERS_PATH) || (names_port && posts(command))
    }

    /// The tools `rules` allows beyond the built-in read-only allowlist: its role's
    /// `read_only_tools`, `conductor` for conductor's rules and `controller` for a controller's.
    pub(super) fn read_only_tools(&self, rules: Rules) -> &[String] {
        match rules {
            Rules::Conductor => &self.read_only[0],
            Rules::Controller => &self.read_only[1],
        }
    }

    /// The instance itself.
    fn this(&self) -> Option<&Instance> {
        self.instances.first()
    }

    /// The names the director's session a controller of the instance messages may carry for this
    /// release (`story:role-name-aliases`): [`Places::conductor_session`] under the old name
    /// `conductor` and under `director`, the old one first, `<prefix>-conductor` and
    /// `<prefix>-director`.
    fn director_sessions(&self) -> Vec<String> {
        let mut names = config::session_names_of(RoleName::Director, &self.conductor_session());
        let old = config::old_role_word(RoleName::Director);
        names.sort_by_key(|name| !name.ends_with(old));
        names
    }

    /// The name of the conductor session a controller of the instance messages: the instance's
    /// `conductor.session_name` (`config::conductor_session`), its own `<prefix>-conductor` or
    /// `conductor`, or the conductor of the file's `default` instance when that serves it.
    fn conductor_session(&self) -> String {
        self.this()
            .and_then(|this| this.conductor.session_name.as_ref())
            .map_or_else(
                || match &self.prefix {
                    Some(prefix) => format!("{prefix}-{CONDUCTOR}"),
                    None => CONDUCTOR.to_owned(),
                },
                |name| name.0.clone(),
            )
    }

    /// The instance the session named `name`, working in `cwd` where known, belongs to
    /// ([`collect::session_instance`]).
    fn owner(&self, name: &str, cwd: Option<&Path>) -> Option<&Instance> {
        let instances: Vec<&Instance> = self.instances.iter().collect();
        collect::session_instance(&instances, name, cwd)
    }

    /// Whether conductor of this instance messages the sessions of `instance`: its own, and, as
    /// the conductor of the file's `default` instance, those of every instance that names no
    /// `conductor` role.
    fn serves(&self, instance: &Instance) -> bool {
        instance.name.0 == self.instance
            || (self.default.as_deref() == Some(self.instance.as_str())
                && !config::has_conductor(instance))
    }

    /// Whether where a recipient works can decide whose it is against its name: another instance
    /// keeps the bare names, so a name that seems to carry a prefix may be that instance's own
    /// controller, named after its repository.
    fn reads_directories(&self) -> bool {
        self.instances
            .iter()
            .skip(1)
            .any(|instance| instance.session_prefix.is_none())
    }

    /// What a denial of a message to `name`, working in `cwd` where known, adds when it is
    /// another instance's session: both instances, named.
    fn other_instance(&self, name: &str, cwd: Option<&Path>) -> String {
        self.owner(name, cwd)
            .filter(|owner| owner.name.0 != self.instance)
            .map_or_else(String::new, |owner| {
                let prefix = owner.session_prefix.as_ref().map_or_else(
                    || "no session prefix".to_owned(),
                    |prefix| format!("session prefix {}", prefix.0),
                );
                format!(
                    " (a session of instance {}, {prefix}; this session is instance {}'s)",
                    owner.name.0, self.instance
                )
            })
    }

    /// These places, with `dir` as the session's temporary directory when it is absolute and,
    /// where it lands, neither the home directory nor a directory holding it (`/home`, `/`), which
    /// would make the whole home directory scratch. The hook gives it `$TMPDIR` of its own
    /// environment, which the session's own tools share.
    #[must_use]
    pub fn with_tmp(mut self, dir: Option<&Path>) -> Self {
        let home = real(&self.home);
        self.tmp = dir
            .filter(|dir| dir.is_absolute())
            .map(real)
            .filter(|dir| !home.starts_with(dir));
        self
    }

    /// These places, with `file` as the config file: the absolute path the process resolves with
    /// [`config::locate`], `CONDUCTOR_CONFIG` or the default file. The hook gives it the file of
    /// its own environment.
    #[must_use]
    pub fn with_config_file(mut self, file: &Path) -> Self {
        self.config = file.to_path_buf();
        self
    }

    /// These places with the instances of every other config file the host registry lists
    /// after the file's own (`story:instance-names-unique`, [`crate::registry::others`]), each
    /// with the checkouts and prefix it registered; `None` without a config file, or when the
    /// registry lists no other file or cannot be read.
    #[must_use]
    pub fn with_registered(&self) -> Option<Self> {
        self.records.as_ref()?;
        let file = config::physical(&self.config);
        let others = crate::registry::others(&self.home, &file).ok()?;
        if others.is_empty() {
            return None;
        }
        let mut widened = self.clone();
        for (_, other) in others {
            let mut instance = config::built_in(&self.home, &self.home);
            instance.name = other.name;
            instance.session_prefix = other.session_prefix;
            instance.dashboard_port = other.dashboard_port;
            instance.records = String::new();
            instance.state = String::new();
            instance.cache = String::new();
            instance.checkouts = other.checkouts;
            widened.instances.push(instance);
        }
        Some(widened)
    }

    /// Conductor's records, as the config writes them: the instance's `records`, or without a
    /// config file the checkout of the repository named `conductor`.
    fn records_dir(&self) -> PathBuf {
        self.records
            .clone()
            .unwrap_or_else(|| self.checkouts.join(CONDUCTOR))
    }

    /// Where conductor's records land ([`real`]).
    fn records(&self) -> PathBuf {
        real(&self.records_dir())
    }

    /// The repository and the rule set of a session whose cwd lands on `cwd`, or `None` when it
    /// runs where no rule set applies. With a config file, a cwd in its `records` is conductor's,
    /// and any other in a repository that no source excludes is that repository's controller;
    /// without one, a cwd in the repository named `conductor` is conductor's.
    pub(super) fn session(&self, cwd: &Path) -> Option<(String, Rules)> {
        if let Some(records) = &self.records {
            if cwd.starts_with(real(records)) {
                return Some((CONDUCTOR.to_owned(), Rules::Conductor));
            }
            return self
                .repository_of(cwd)
                .filter(|repository| {
                    let steps: Vec<&str> = repository.split('/').collect();
                    !collect::is_excluded(&steps, &self.exclude)
                })
                .map(|repository| (repository, Rules::Controller));
        }
        let repository = self.repository_of(cwd)?;
        let rules = if repository == CONDUCTOR {
            Rules::Conductor
        } else {
            Rules::Controller
        };
        Some((repository, rules))
    }

    /// The repository the landing `path` ([`real`]) belongs to, by its steps under the checkouts
    /// root or under the managed-worktree root, the deeper of the two when both hold it: the
    /// directory holding `.git` at depth 1 or 2, named by its path under the root
    /// ([`collect::repository_of`]). `None` under neither, at either root itself, or in a group
    /// directory itself.
    pub(super) fn repository_of(&self, path: &Path) -> Option<String> {
        let checkouts = real(&self.checkouts);
        let trees = real(&self.trees);
        let (under, rest) = [(Under::Checkouts, &checkouts), (Under::Trees, &trees)]
            .into_iter()
            .filter_map(|(under, root)| Some((root, under, path.strip_prefix(root).ok()?)))
            .max_by_key(|(root, _, _)| root.components().count())
            .map(|(_, under, rest)| (under, rest))?;
        let steps: Vec<String> = rest
            .components()
            .map(|step| step.as_os_str().to_string_lossy().into_owned())
            .collect();
        let steps: Vec<&str> = steps.iter().map(String::as_str).collect();
        collect::repository_of(&checkouts, &trees, under, &steps)
    }

    /// Where a session's cwd must be for a rule set to apply, for a denial to name.
    pub(super) fn described(&self) -> String {
        let repositories = format!(
            "{}/<repository>/ and {}/<repository>/",
            self.shown(&self.checkouts),
            self.shown(&self.trees)
        );
        match &self.records {
            Some(records) => format!("{}/, {repositories}", self.shown(records)),
            None => repositories,
        }
    }

    /// `path` as a message writes it: under the home directory as `~/…`.
    fn shown(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        }
    }

    /// The landing `path` as [`Places::shown`] writes it, `~` standing for the home directory
    /// where it lands.
    fn shown_landing(&self, path: &Path) -> String {
        match path.strip_prefix(real(&self.home)) {
            Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => self.shown(path),
        }
    }

    /// The default config file, [`config::DEFAULT_FILE`] under the home directory.
    fn default_config(&self) -> PathBuf {
        self.home.join(config::DEFAULT_FILE)
    }

    /// The config directory, `<config-dir>`: the directory of [`config::DEFAULT_FILE`] under the
    /// home directory. Only the directory itself is the config's (moving or removing it takes the
    /// default file along); what is under it is not, but for that file and `<state>`.
    fn config_dir(&self) -> Option<PathBuf> {
        config_dir_in_home().map(|dir| self.home.join(dir))
    }

    /// Whether the absolute `path` is conductor's config: the config file, the default config
    /// file, `<config-dir>` itself, or `<state>` or anything under it, each as written
    /// ([`lexical`]) or where it lands ([`real`]).
    fn is_config(&self, path: &Path) -> bool {
        let is = |file: &Path| path == lexical(file) || path == real(file);
        is(&self.config)
            || is(&self.default_config())
            || self.config_dir().is_some_and(|dir| is(&dir))
            || self.state.as_ref().is_some_and(|state| {
                path.starts_with(lexical(state)) || path.starts_with(real(state))
            })
    }

    /// Whether the Bash `command` names conductor's config. A plain substring test for the files:
    /// `$CONDUCTOR_CONFIG` or `${CONDUCTOR_CONFIG`, [`config::DEFAULT_FILE`] as the home directory
    /// leaves it, or any of the [`Places::spellings`] of the config file or the default file. A
    /// whole-path test ([`names_path`]) for the directories: `<config-dir>` itself, and `<state>`
    /// or a path under it, so that a path elsewhere under `<config-dir>`, such as the records an
    /// instance keeps there by default, is not the config.
    fn names_config(&self, command: &str) -> bool {
        let variable = config::CONFIG_VARIABLE;
        let mut files = vec![
            format!("${variable}"),
            format!("${{{variable}"),
            config::DEFAULT_FILE.to_owned(),
        ];
        files.extend(self.spellings(&self.config));
        files.extend(self.spellings(&self.default_config()));
        files.iter().any(|file| command.contains(file.as_str()))
            || self.config_dir().is_some_and(|dir| {
                self.spellings(&dir)
                    .iter()
                    .any(|spelling| names_path(command, spelling, false))
            })
            || self.state.as_ref().is_some_and(|state| {
                self.spellings(state)
                    .iter()
                    .any(|spelling| names_path(command, spelling, true))
            })
    }

    /// Whether the Bash `command` writes conductor's records ([`writes_named`]): a write target
    /// names `<records>` or a path under it ([`names_path`]) by any of its [`Places::spellings`].
    fn writes_records(&self, command: &str) -> bool {
        let spellings = self.spellings(&self.records_dir());
        writes_named(command, |text| {
            spellings
                .iter()
                .any(|spelling| names_path(text, spelling, true))
        })
    }

    /// The ways a command line writes the absolute `path`: as written or where it lands, as an
    /// absolute path or under the home directory as `~/…`, `$HOME/…`, `${HOME}/…`, `"$HOME"/…` or
    /// `"${HOME}"/…`.
    fn spellings(&self, path: &Path) -> Vec<String> {
        let mut spellings = Vec::new();
        for landing in [lexical(path), real(path)] {
            spellings.push(landing.display().to_string());
            if let Ok(rest) = landing.strip_prefix(&self.home)
                && !rest.as_os_str().is_empty()
            {
                let rest = rest.display();
                spellings.extend([
                    format!("~/{rest}"),
                    format!("$HOME/{rest}"),
                    format!("${{HOME}}/{rest}"),
                    format!("\"$HOME\"/{rest}"),
                    format!("\"${{HOME}}\"/{rest}"),
                ]);
            }
        }
        spellings
    }

    /// Whether the absolute `path` is the `settings` file of a role of the instance, as written
    /// ([`lexical`]) or where it lands ([`real`]).
    fn is_role_settings(&self, path: &Path) -> bool {
        self.settings
            .iter()
            .any(|settings| path == lexical(settings) || path == real(settings))
    }

    /// Whether the Bash `command` writes a settings file that wires the guard ([`writes_named`]):
    /// a write target names a `.claude/` settings file ([`names_claude_settings`]) or, by any of
    /// its [`Places::spellings`], the `settings` file of a role of the instance. A substring test
    /// on each target, not a resolution: `cd .claude && … > settings.json` is not caught, nor is a
    /// role's file written by a path relative to the cwd.
    fn writes_settings(&self, command: &str) -> bool {
        let spellings: Vec<String> = self
            .settings
            .iter()
            .flat_map(|settings| self.spellings(settings))
            .collect();
        writes_named(command, |text| {
            names_claude_settings(text)
                || spellings
                    .iter()
                    .any(|spelling| text.contains(spelling.as_str()))
        })
    }

    /// Whether the Bash `command` names the guard's copy of the conductor lease or the state
    /// directory it is kept in (`story:lease-guard`, security review F1): the file's name
    /// ([`crate::session::LEASE_FILE`]) anywhere in it, or, by any of its [`Places::spellings`],
    /// `state` (the directory the hook reads the lease from) or the instance's `state`, or a path
    /// under either ([`names_path`]). Such a command can remove, empty or lock the lease, whatever
    /// program it runs (`chmod`, `unlink`, `find -delete`, an interpreter), so the hook decides it
    /// as a write of a conductor-role session. A plain substring test, as
    /// [`Places::writes_settings`] is: a path relative to the cwd is caught only by the file's name.
    pub(super) fn names_lease(&self, command: &str, state: Option<&Path>) -> bool {
        command.contains(crate::session::LEASE_FILE)
            || state.into_iter().chain(self.state.as_deref()).any(|dir| {
                let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
                self.spellings(&dir)
                    .iter()
                    .any(|spelling| names_path(command, spelling, true))
            })
    }

    /// Whether the absolute `path` is the `profile` file of a role of the instance, as written
    /// ([`lexical`]) or where it lands ([`real`]).
    fn is_role_profile(&self, path: &Path) -> bool {
        self.profiles
            .iter()
            .any(|profile| path == lexical(profile) || path == real(profile))
    }

    /// Whether the Bash `command` writes the `profile` file of a role of the instance
    /// ([`writes_named`]): a write target names one by any of its [`Places::spellings`]. A
    /// substring test on each target, as [`Places::writes_settings`] is: a profile written by a
    /// path relative to the cwd is not caught.
    fn writes_profile(&self, command: &str) -> bool {
        let spellings: Vec<String> = self
            .profiles
            .iter()
            .flat_map(|profile| self.spellings(profile))
            .collect();
        writes_named(command, |text| {
            spellings
                .iter()
                .any(|spelling| text.contains(spelling.as_str()))
        })
    }

    /// Conductor's config, for a denial to name: the config file, the default config file,
    /// `<config-dir>` itself, and `<state>`.
    fn config_described(&self) -> String {
        let mut parts = vec![self.shown(&self.config)];
        let default = self.default_config();
        if default != self.config {
            parts.push(self.shown(&default));
        }
        if let Some(dir) = self.config_dir() {
            parts.push(format!("the directory {}/ itself", self.shown(&dir)));
        }
        if let Some(state) = &self.state {
            parts.push(format!("anything under {}/", self.shown(state)));
        }
        match parts.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{}, or {last}", rest.join(", ")),
            _ => parts.join(""),
        }
    }
}

/// The `gh` verbs that read, after any command group; everything else from `gh` is a GitHub
/// write, apart from the other reads [`forge_write`] names.
const GH_READ_VERBS: [&str; 7] = [
    "view", "list", "status", "diff", "checks", "search", "download",
];

/// The verbs of `gh run` that read.
const GH_RUN_READ_VERBS: [&str; 4] = ["view", "list", "watch", "download"];

/// The `gh` groups whose writes are credentials: a write in one is class H, the operator's hands,
/// whatever route would carry it.
const GH_CREDENTIAL_GROUPS: [&str; 4] = ["secret", "auth", "ssh-key", "gpg-key"];

/// The `glab` calls that read, each group with its verbs that read; everything else from `glab`
/// is a GitLab write, apart from the other reads [`forge_write`] names
/// (`story:guard-glab-writes`). Unlike `gh`, no verb reads after any group: a `glab` group not
/// named here makes only writes.
const GLAB_READS: [(&str, &[&str]); 5] = [
    ("mr", &["view", "list", "diff"]),
    ("issue", &["view", "list"]),
    ("release", &["view", "list", "download"]),
    ("repo", &["view", "list"]),
    ("ci", &["view", "list", "status", "trace", "get"]),
];

/// The `glab` groups whose writes are credentials, as [`GH_CREDENTIAL_GROUPS`] are for `gh`: CI
/// variables (GitLab's secrets), the login, keys and access tokens, the CLI's own config (`glab
/// config set token …` writes the login's access token) and CI secure files (signing keys,
/// certificates).
const GLAB_CREDENTIAL_GROUPS: [&str; 8] = [
    "variable",
    "auth",
    "ssh-key",
    "gpg-key",
    "deploy-key",
    "token",
    "config",
    "securefile",
];

/// Commands that name `gh` or `glab` without running it, or run it through the bot route, which
/// this rule leaves to [`bot_route`]: only as the simple command's program
/// ([`names_without_running`]).
const GH_NOT_RUN: [&str; 6] = [BOT_ROUTE, "which", "type", "whereis", "man", "hash"];

/// The program of the bot route, which a `gh` call is left to ([`bot_route`]).
const BOT_ROUTE: &str = "b10x-gates";

/// The `gh` calls the bot route carries, each group with its verbs, for the session's own
/// repository only: its release (the operator's decision of 2026-10-15) and a re-run of its CI
/// (the coordinator's decision the same day: a controller re-runs failed CI through the route).
const ROUTE_CARRIES: [(&str, &[&str]); 3] = [
    ("release", &["create", "upload", "edit", "view"]),
    ("run", &["rerun"]),
    ("workflow", &["run"]),
];

/// The options of `b10x-gates gh` that take the next word as their value.
const ROUTE_VALUES: [&str; 5] = ["--repo", "--repository", "--policy", "--key", "--gitleaks"];

/// A run of the bot route `b10x-gates gh [opts] -- <gh args>`: the repository its
/// `--repository` names, the `gh` arguments, and whether it asks for its own help.
struct BotRoute<'w> {
    repository: Option<&'w str>,
    args: Vec<String>,
    help: bool,
}

/// The bot route the simple command `words` runs, if it runs one: its program, after reserved
/// words and assignments, is `b10x-gates`, and its subcommand, the first word after it that is no
/// flag, is `gh`. The `gh` arguments are the words after `--`, and before it every word that is
/// no option nor an option's value.
fn bot_route(words: &[String]) -> Option<BotRoute<'_>> {
    let words = command_words(words);
    let (program, rest) = words.split_first()?;
    if shell::program(program) != BOT_ROUTE {
        return None;
    }
    let at = rest.iter().position(|word| !word.starts_with('-'))?;
    if rest[at] != "gh" {
        return None;
    }
    let rest = &rest[at + 1..];
    let mut route = BotRoute {
        repository: None,
        args: Vec::new(),
        help: false,
    };
    let mut next = 0;
    while let Some(word) = rest.get(next) {
        next += 1;
        if word == "--" {
            route.args.extend(rest[next..].iter().cloned());
            break;
        }
        if let Some(value) = word.strip_prefix("--repository=") {
            route.repository = Some(value);
        } else if word == "--repository" {
            route.repository = rest.get(next).map(String::as_str);
            next += 1;
        } else if ROUTE_VALUES.contains(&word.as_str()) {
            next += 1;
        } else if word == "-h" || word == "--help" {
            route.help = true;
        } else if !word.starts_with('-') {
            route.args.push(word.clone());
        }
    }
    Some(route)
}

/// A forge whose command-line tool the guard reads: `gh` for GitHub, `glab` for GitLab. A write
/// through either is held alike for each role ([`forge_write`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Forge {
    GitHub,
    GitLab,
}

impl Forge {
    /// Every forge, in the order a command's words are read for each.
    const ALL: [Self; 2] = [Self::GitHub, Self::GitLab];

    /// The program that reaches the forge.
    fn program(self) -> &'static str {
        match self {
            Self::GitHub => "gh",
            Self::GitLab => "glab",
        }
    }

    /// Whether the command word `word` runs the forge's program, by any path.
    fn runs(self, word: &str) -> bool {
        shell::program(word) == self.program()
    }

    /// The forge's name in a denial.
    fn name(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
        }
    }

    /// The groups whose writes are credentials.
    fn credential_groups(self) -> &'static [&'static str] {
        match self {
            Self::GitHub => &GH_CREDENTIAL_GROUPS,
            Self::GitLab => &GLAB_CREDENTIAL_GROUPS,
        }
    }

    /// The calls that read, as a denial names them.
    fn reads(self) -> String {
        match self {
            Self::GitHub => format!(
                "{} after any group; run {}; api GET; auth status; help; --help",
                GH_READ_VERBS.join(", "),
                GH_RUN_READ_VERBS.join(", ")
            ),
            Self::GitLab => {
                let groups: Vec<String> = GLAB_READS
                    .iter()
                    .map(|(group, verbs)| format!("{group} {}", verbs.join("|")))
                    .collect();
                format!(
                    "{}; api GET; auth status; --help; --version",
                    groups.join("; ")
                )
            }
        }
    }
}

/// The write forms that, in a controller's Bash command that names `.claude/settings` or
/// conductor's config, deny it: as substrings. A redirection is a write form too, read by the
/// shell's grammar rather than as a substring ([`shell::written`]).
const WRITE_SUBSTRINGS: [&str; 7] = [
    "sed -i",
    "sed --in-place",
    "yq -i",
    "yq --inplace",
    "perl -i",
    "perl -pi",
    " of=",
];

/// The write forms that, in a controller's Bash command that names `.claude/settings` or
/// conductor's config, deny it: as words, a substring with no letter, digit, `_`, `-` or `.` on
/// either side.
const WRITE_WORDS: [&str; 10] = [
    "tee", "cp", "mv", "rm", "ln", "install", "truncate", "touch", "rsync", "patch",
];

/// `<config-dir>` as the home directory leaves it: the directory of [`config::DEFAULT_FILE`], when
/// it names one.
fn config_dir_in_home() -> Option<&'static Path> {
    Path::new(config::DEFAULT_FILE)
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
}

/// Why a controller's write to conductor's config is denied (wave 08 W5): rewriting `records` to
/// its own cwd would give it conductor's rules and write leaves, and `checkouts.root: /` would
/// widen where it writes.
const NOT_CONFIG: &str = "a controller does not write conductor's config";

/// The global options of the `conductor` command tree that take a value (`src/cli.rs`), so that
/// the value is not read as a group or a leaf. `tests/guard_rules.rs` runs each leaf with every
/// global option of the tree that takes one.
const GLOBAL_VALUES: [&str; 2] = ["--state-dir", "--config"];

/// The [`Access`] the `conductor` command tree declares for the leaf named by `path`, the words
/// after `conductor` ([`cli::LEAVES`]), or `None` when the tree has no such leaf.
fn declared(path: &[&str]) -> Option<Access> {
    cli::LEAVES
        .iter()
        .find(|(leaf, _)| *leaf == path)
        .map(|&(_, access)| access)
}

/// Whether `word` is a group of the `conductor` command tree: the first of the two words of a
/// leaf of [`cli::LEAVES`]. A `conductor` word followed by one is a run of the binary.
fn is_group(word: &str) -> bool {
    cli::LEAVES
        .iter()
        .any(|(leaf, _)| matches!(leaf, [group, _] if *group == word))
}

/// Which rule set decides a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rules {
    /// A repository controller.
    Controller,
    /// Conductor itself.
    Conductor,
}

impl Rules {
    /// The config role whose sessions this rule set decides, under its name or its old name
    /// (`story:role-name-aliases`).
    pub(super) const fn role_name(self) -> RoleName {
        match self {
            Self::Controller => RoleName::Lead,
            Self::Conductor => RoleName::Director,
        }
    }

    /// The name of [`Rules::role_name`].
    pub(super) const fn role(self) -> &'static str {
        config::role_word(self.role_name())
    }
}

/// Where a session runs: the places of the instance, its cwd, and the repository the cwd belongs
/// to; and where the session list is read when a SendMessage needs it.
pub(super) struct Session<'a> {
    pub places: &'a Places,
    pub cwd: PathBuf,
    pub repository: String,
    pub rules: Rules,
    pub sessions: &'a SessionList,
}

impl Session<'_> {
    fn home(&self) -> &Path {
        &self.places.home
    }

    fn checkout(&self) -> PathBuf {
        real(&self.places.checkouts.join(&self.repository))
    }

    fn worktrees(&self) -> PathBuf {
        real(&self.places.trees.join(&self.repository))
    }

    /// `path` as a file tool takes it, unresolved: `~` is the home directory, a relative path is
    /// taken from the cwd.
    fn file_target(&self, path: &str) -> PathBuf {
        match path.strip_prefix('~') {
            Some("") => self.home().to_path_buf(),
            Some(rest) if rest.starts_with('/') => self.home().join(&rest[1..]),
            _ => self.cwd.join(path),
        }
    }

    /// Whether the landing `path` is a session's scratch (`story:guard-scratch`): under `$TMPDIR`
    /// for every session, or for a controller also under `<cache>/scratch/<repo>/` of its own
    /// instance (`story:scratch-per-instance`), `<repo>` its repository with `/` written `-`. Never a repository's checkout or managed tree,
    /// conductor's records, its config, a `.claude/` settings file, a role's settings file or a
    /// role's profile file, wherever the temporary directory points.
    fn is_scratch(&self, landing: &Path) -> bool {
        let places = self.places;
        if places.repository_of(landing).is_some()
            || landing.starts_with(places.records())
            || places.is_config(landing)
            || settings_file(landing)
            || places.is_role_settings(landing)
            || places.is_role_profile(landing)
        {
            return false;
        }
        if places
            .tmp
            .as_ref()
            .is_some_and(|tmp| landing.starts_with(tmp) && landing != tmp)
        {
            return true;
        }
        if self.rules != Rules::Controller {
            return false;
        }
        let scratch = self.scratch_dir();
        landing.starts_with(&scratch) && landing != scratch
    }

    /// A controller's scratch directory, `<cache>/scratch/<repo>` of its own instance, where it
    /// lands ([`real`]).
    fn scratch_dir(&self) -> PathBuf {
        crate::confine::places::scratch_of(&real(&self.places.cache), &self.repository)
    }

    /// The scratch directory as a denial names it, the home directory written `~`.
    fn scratch_shown(&self) -> String {
        let dir = crate::confine::places::scratch_of(&self.places.cache, &self.repository);
        match dir.strip_prefix(self.home()) {
            Ok(under) => format!("~/{}/", under.display()),
            Err(_) => format!("{}/", dir.display()),
        }
    }

    /// Where a denied write may go instead, for a denial to name (`story:guard-write-targets`):
    /// a controller's scratch, under `$TMPDIR` or `<cache>/scratch/<repo>/`.
    fn scratch_named(&self) -> String {
        format!("scratch is under $TMPDIR or {}", self.scratch_shown())
    }

    /// Edit, Write and NotebookEdit: the verdict on writing `target`.
    pub(super) fn file(&self, tool: &str, target: &str) -> (Verdict, String) {
        let written = self.file_target(target);
        let landing = real(&written);
        let resolves = if Path::new(target) == landing {
            String::new()
        } else {
            format!(" (resolves to {})", landing.display())
        };
        match self.rules {
            Rules::Controller => {
                let places = self.places;
                let checkout = places.shown(&places.checkouts.join(&self.repository));
                let inside =
                    landing.starts_with(self.checkout()) || landing.starts_with(self.worktrees());
                // Conductor's config first, wherever it is: a config file can be named inside a
                // checkout, and `<config-dir>` can be a link into one.
                if places.is_config(&lexical(&written)) || places.is_config(&landing) {
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is conductor's config ({}): {}",
                            places.config_described(),
                            NOT_CONFIG
                        ),
                    )
                // A role's settings file wires the guard, wherever the config puts it.
                } else if places.is_role_settings(&lexical(&written))
                    || places.is_role_settings(&landing)
                {
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is the settings file of a role of \
                             the config, which wires the guard: a controller does not write it"
                        ),
                    )
                // A role's profile is the system prompt its sessions start with, wherever it is.
                } else if places.is_role_profile(&lexical(&written))
                    || places.is_role_profile(&landing)
                {
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is the profile file of a role of the \
                             config, the system prompt its sessions start with: a controller does \
                             not write it"
                        ),
                    )
                // Both as written and where it lands: Claude Code reads the settings by name, so a
                // `.claude` that is a link still counts, and so does a link to the settings.
                } else if inside && (settings_file(&lexical(&written)) || settings_file(&landing)) {
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is Claude Code's project settings, \
                             which can switch the guard off: a controller does not write them"
                        ),
                    )
                } else if !inside && self.is_scratch(&landing) {
                    (
                        Verdict::Allow,
                        format!(
                            "{tool} target{resolves} is scratch: under $TMPDIR or {}",
                            self.scratch_shown()
                        ),
                    )
                } else if inside && landing.starts_with(places.records()) {
                    // Only a config file can put conductor's records in a controller's checkout or
                    // worktrees; they stay conductor's.
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is in conductor's records {}/: a \
                             controller does not write them; {}",
                            places.shown(&places.records_dir()),
                            self.scratch_named()
                        ),
                    )
                } else if inside {
                    (
                        Verdict::Allow,
                        format!(
                            "{tool} target{resolves} is inside {checkout}/ or its managed worktrees"
                        ),
                    )
                } else {
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is outside the repository's checkout \
                             {checkout}/ and its managed worktrees {}/; {}",
                            places.shown(&places.trees.join(&self.repository)),
                            self.scratch_named()
                        ),
                    )
                }
            }
            Rules::Conductor => {
                let places = self.places;
                let as_written = lexical(&written);
                if settings_file(&as_written)
                    || settings_file(&landing)
                    || places.is_role_settings(&as_written)
                    || places.is_role_settings(&landing)
                {
                    return (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is a settings file, which can wire \
                             or switch off the guard: conductor writes only its records, and no \
                             settings file among them"
                        ),
                    );
                }
                if places.is_role_profile(&as_written) || places.is_role_profile(&landing) {
                    return (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is the profile file of a role of the \
                             config, the system prompt its sessions start with: conductor writes \
                             only its records, and no role's profile among them"
                        ),
                    );
                }
                let records = places.records();
                let record = RECORD_DIRS.iter().any(|dir| {
                    let dir = records.join(dir);
                    landing.starts_with(&dir) && landing != dir
                }) || RECORD_FILES
                    .iter()
                    .any(|file| landing == records.join(file));
                if record {
                    (
                        Verdict::Allow,
                        format!("{tool} target{resolves} is one of conductor's records"),
                    )
                } else if self.is_scratch(&landing) {
                    (
                        Verdict::Allow,
                        format!("{tool} target{resolves} is scratch under $TMPDIR"),
                    )
                } else {
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is not one of conductor's records: \
                             conductor writes only its records ({}, {}) and scratch under $TMPDIR",
                            RECORD_DIRS.map(|dir| format!("{dir}/")).join(", "),
                            RECORD_FILES.join(", ")
                        ),
                    )
                }
            }
        }
    }

    /// SendMessage: the verdict on messaging `to`, whose `recipient` field, when present, must
    /// name the same session: in either form of its name ([`named`]), otherwise as `to` does.
    /// Whose a recipient is, is read against the instances of every config file the host
    /// registry lists (`story:instance-names-unique`, [`Places::with_registered`]), so a session
    /// of another file's instance is refused too; only this call reads the registry.
    pub(super) fn message(&self, to: Option<&str>, recipient: Option<&str>) -> (Verdict, String) {
        match self.places.with_registered() {
            Some(widened) => Session {
                places: &widened,
                cwd: self.cwd.clone(),
                repository: self.repository.clone(),
                rules: self.rules,
                sessions: self.sessions,
            }
            .message_among(to, recipient),
            None => self.message_among(to, recipient),
        }
    }

    /// [`Session::message`] against the instances of its places.
    fn message_among(&self, to: Option<&str>, recipient: Option<&str>) -> (Verdict, String) {
        match (self.rules, &self.places.prefix) {
            // Alone in its file, a conductor without a prefix keeps today's rule.
            (Rules::Conductor, None) if self.places.instances.len() <= 1 => (
                Verdict::Allow,
                "conductor's SendMessage is not restricted".to_owned(),
            ),
            (Rules::Conductor, prefix) => self.conductor_message(prefix.as_deref(), to, recipient),
            (Rules::Controller, _) => self.controller_message(to, recipient),
        }
    }

    /// A controller's SendMessage: to its instance's conductor session ([`Places::conductor_session`])
    /// by name, ref or socket, under either name of the director's session for this release
    /// ([`Places::director_sessions`]), or to an agent it started.
    fn controller_message(&self, to: Option<&str>, recipient: Option<&str>) -> (Verdict, String) {
        let names = self.places.director_sessions();
        let conductor = names.join(" or ");
        let only = format!("a controller messages only {conductor}");
        let Some(to) = to else {
            return (
                Verdict::Deny,
                format!("SendMessage names no recipient: {only}"),
            );
        };
        let denied = |why: &str| {
            let base = named_base(to);
            (
                Verdict::Deny,
                format!(
                    "SendMessage to {to:?}{why}: {only}{}",
                    self.places.other_instance(base, None)
                ),
            )
        };
        if let Some((name, reference)) = names
            .iter()
            .find_map(|name| named(to, name).map(|reference| (name.as_str(), reference)))
        {
            let same = |other: Option<&str>| match (reference, other) {
                (Some(reference), Some(other)) => reference == other,
                _ => true,
            };
            return match recipient {
                Some(recipient) if !named(recipient, name).is_some_and(same) => {
                    denied(&format!(" with recipient {recipient:?}"))
                }
                _ => (
                    Verdict::Allow,
                    format!("SendMessage to {to:?}: a session named {name}"),
                ),
            };
        }
        if let Some(recipient) = recipient.filter(|recipient| *recipient != to) {
            return denied(&format!(" with recipient {recipient:?}"));
        }
        if is_own_agent_id(to) {
            return (
                Verdict::Allow,
                format!("SendMessage to {to:?}: an agent this session started"),
            );
        }
        match to.strip_prefix(SOCKET_SCHEME) {
            Some(path) => self.socket(to, path, &conductor, &only, &|name, _| {
                names.iter().any(|director| director == name)
            }),
            None => denied(""),
        }
    }

    /// Conductor's SendMessage, its instance having the session prefix `prefix`: to a session
    /// named `<prefix>-<name>` by name, ref or socket, or to an agent it started.
    fn conductor_message(
        &self,
        prefix: Option<&str>,
        to: Option<&str>,
        recipient: Option<&str>,
    ) -> (Verdict, String) {
        let places = self.places;
        let only = match prefix {
            Some(prefix) => format!(
                "conductor of instance {} messages only sessions named {prefix}-<name>, and those \
                 of the instances it serves",
                places.instance
            ),
            None => format!(
                "conductor of instance {} messages its own sessions and those of the instances it \
                 serves, and no session of an instance that names its own conductor role",
                places.instance
            ),
        };
        // Whose sessions it messages: those of the instances it serves; one no instance claims is
        // its own when it keeps the bare names (correction round 2, decision 1).
        let allowed = |owner: Option<&Instance>| match owner {
            Some(owner) => places.serves(owner),
            None => prefix.is_none(),
        };
        let Some(to) = to else {
            return (
                Verdict::Deny,
                format!("SendMessage names no recipient: {only}"),
            );
        };
        let base = named_base(to);
        if let Some(recipient) = recipient.filter(|recipient| named_base(recipient) != base) {
            return (
                Verdict::Deny,
                format!("SendMessage to {to:?} with recipient {recipient:?}: {only}"),
            );
        }
        if is_own_agent_id(to) {
            return (
                Verdict::Allow,
                format!("SendMessage to {to:?}: an agent this session started"),
            );
        }
        if let Some(path) = to.strip_prefix(SOCKET_SCHEME) {
            let served = |name: &str, cwd: Option<&Path>| allowed(places.owner(name, cwd));
            return self.socket(to, path, "of an instance it serves", &only, &served);
        }
        // Where the recipient works, from the session list, when another instance keeps the bare
        // names: its own controller `b-tools`, in its checkout `b-tools`, is not the session of
        // an instance whose prefix is `b`.
        let cwd = if places.reads_directories() {
            match self.listed_directory(to) {
                Ok(cwd) => cwd,
                Err(why) => {
                    return (
                        Verdict::Deny,
                        format!("SendMessage to {to:?}: {why}; {only}"),
                    );
                }
            }
        } else {
            None
        };
        let owner = places.owner(base, cwd.as_deref());
        match owner {
            None if prefix.is_none() => (
                Verdict::Allow,
                format!(
                    "SendMessage to {to:?}: a session no other instance claims, instance {}'s own",
                    places.instance
                ),
            ),
            Some(owner) if places.serves(owner) => (
                Verdict::Allow,
                format!(
                    "SendMessage to {to:?}: a session of instance {}",
                    owner.name.0
                ),
            ),
            _ => (
                Verdict::Deny,
                format!(
                    "SendMessage to {to:?}: {only}{}",
                    places.other_instance(base, cwd.as_deref())
                ),
            ),
        }
    }

    /// Where the listed session `to` addresses works (correction round 2, decisions 2 and 3):
    /// `to` is `<name>` or `<name> [<ref>]`, and the ref, the start of a listed session's `id`,
    /// picks that session; a listed session's working directory counts whether or not it has a
    /// pid. The error says why no one session is meant: the list cannot be read, the bare name is
    /// listed more than once, the ref matches none or more than one, or the list holds no session
    /// of that name.
    fn listed_directory(&self, to: &str) -> Result<Option<PathBuf>, String> {
        let base = named_base(to);
        let reference = named(to, base).flatten();
        let live = live_sessions(self.sessions).map_err(|error| {
            format!(
                "the session list could not be read, so where {base} works is not known: \
                 {error:#}"
            )
        })?;
        let matching: Vec<Live> = live
            .into_iter()
            .filter(|session| {
                session.name == base
                    && reference.is_none_or(|reference| {
                        session
                            .id
                            .as_deref()
                            .is_some_and(|id| id.starts_with(reference))
                    })
            })
            .collect();
        match (matching.as_slice(), reference) {
            ([one], _) => Ok(one.cwd.clone()),
            ([], Some(reference)) => Err(format!(
                "the ref [{reference}] of {base} matches no listed session"
            )),
            ([], None) => Err(format!(
                "{base} is in no listed session, so where it works is not known"
            )),
            (_, Some(reference)) => Err(format!(
                "the ref [{reference}] of {base} matches more than one listed session"
            )),
            (_, None) => Err(format!(
                "{base} matches more than one listed session; name one with its [<ref>]"
            )),
        }
    }

    /// SendMessage to the socket address `to`, `uds:` and `path`: allowed when `path` lands on
    /// `<pid>.sock` and the session list holds a live session with that pid whose name and
    /// working directory `wanted` accepts, a session `shown_name` describes; denied with `only`
    /// otherwise.
    fn socket(
        &self,
        to: &str,
        path: &str,
        shown_name: &str,
        only: &str,
        wanted: &dyn Fn(&str, Option<&Path>) -> bool,
    ) -> (Verdict, String) {
        let denied = |why: String| (Verdict::Deny, format!("SendMessage to {to:?}{why}: {only}"));
        let path = Path::new(path);
        if !path.is_absolute() {
            return denied(format!(" is no socket address {SOCKET_FORM}"));
        }
        let landing = real(path);
        let resolves = if landing == path {
            String::new()
        } else {
            format!(" (resolves to {})", landing.display())
        };
        let Some(pid) = socket_pid(&landing) else {
            return denied(format!("{resolves} is no socket address {SOCKET_FORM}"));
        };
        let shown = command_line(&self.sessions.command);
        match live_pids(self.sessions, wanted) {
            Ok(pids) if pids.contains(&pid) => (
                Verdict::Allow,
                format!(
                    "SendMessage to {to:?}{resolves}: the live session named {shown_name} has pid \
                     {pid}"
                ),
            ),
            Ok(_) => denied(format!(
                "{resolves}: no live session named {shown_name} has pid {pid} in the session list \
                 `{shown}`"
            )),
            Err(error) => denied(format!(
                "{resolves}: the session list could not be read, so pid {pid} is not known to be \
                 {shown_name}'s: {error:#}"
            )),
        }
    }

    /// Bash: the verdict on running `command`.
    pub(super) fn bash(&self, command: &str) -> (Verdict, String) {
        let commands = shell::commands(command);
        // Whether a variable the line assigns may hold the forge's program, so that a program
        // word that expands may run it (`g=glab; $g mr merge 12`).
        let assigned = |forge: Forge| {
            commands.iter().flatten().any(|word| {
                is_assignment(word)
                    && word
                        .split_once('=')
                        .is_some_and(|(_, value)| contains_word(value, forge.program()))
            })
        };
        if let Some(reason) = kill_denial(&commands) {
            return (Verdict::Deny, reason);
        }
        for words in &commands {
            if let Some(gh) = bot_route(words)
                && let Some(reason) = self.bot_route_denial(&gh)
            {
                return (Verdict::Deny, reason);
            }
            for forge in Forge::ALL {
                let Some(write) = forge_write(words, forge, assigned(forge)) else {
                    continue;
                };
                if write
                    .split(' ')
                    .nth(1)
                    .is_some_and(|group| forge.credential_groups().contains(&group))
                {
                    return (
                        Verdict::Deny,
                        format!(
                            "`{write}` is a {} write of a credential: class H, the operator's \
                             hands, whoever authorized it; no session runs it by any route. Hand \
                             the operator the exact command to run: ! {}",
                            forge.name(),
                            command.trim()
                        ),
                    );
                }
                return (
                    Verdict::Deny,
                    format!(
                        "`{write}` is a {} write: {} here only reads ({}); writes go through the \
                         bot route",
                        forge.name(),
                        forge.program(),
                        forge.reads()
                    ),
                );
            }
        }
        if self.places.posts_to_dashboard(command) {
            return (
                Verdict::Deny,
                format!(
                    "the command posts to the dashboard (it names {} or posts to port {}): only \
                     the operator sends the operator's answers, from the Decisions page; relay \
                     his words with `conductor decision answer-escalated-as-operator`",
                    crate::dashboard::ANSWERS_PATH,
                    self.places.dashboard_port
                ),
            );
        }
        if self.places.writes_settings(command) {
            return (
                Verdict::Deny,
                "the command writes a .claude/ settings file or a role's settings file (a write \
                 target names it, or one the guard cannot read): no session writes Claude Code's \
                 project settings or the settings that wire the guard, which can switch it off"
                    .to_owned(),
            );
        }
        if self.places.writes_profile(command) {
            return (
                Verdict::Deny,
                "the command writes a role's profile file (a write target names it, or one the \
                 guard cannot read): no session writes the system prompt a role's sessions start \
                 with"
                    .to_owned(),
            );
        }
        if self.rules == Rules::Controller
            && self.places.names_config(command)
            && holds_write_form(command)
        {
            return (
                Verdict::Deny,
                format!(
                    "the command names conductor's config ({}) and holds a write form: {}",
                    self.places.config_described(),
                    NOT_CONFIG
                ),
            );
        }
        if self.rules == Rules::Controller
            && let Some(run) = commands.iter().find_map(|words| conductor_run(words))
        {
            return (
                Verdict::Deny,
                match run {
                    DeniedRun::Writes(leaf) => format!(
                        "`conductor {leaf}` writes conductor's records; only conductor runs it"
                    ),
                    DeniedRun::Unknown(shown) => format!(
                        "`{shown}` may run a conductor leaf that writes conductor's records: the \
                         shell expands a word, or another program hands conductor its words, \
                         before conductor reads which leaf to run; a controller writes the \
                         program, the group and the leaf as plain words"
                    ),
                },
            );
        }
        if let Some(reason) = timeout_snapshot_denial(&commands) {
            return (Verdict::Deny, reason);
        }
        if self.rules == Rules::Controller
            && let Some((run, size)) = unreserved_build(&commands)
        {
            return (
                Verdict::Deny,
                format!(
                    "`{run}` builds, tests or gates outside the host's disk ledger: run it as \
                     `conductor build run --reserve {size} -- {run}`, which reserves its disk \
                     first; a refusal names the shortfall: report it, do not retry"
                ),
            );
        }
        if self.rules == Rules::Conductor {
            return (Verdict::Allow, "no GitHub or GitLab write".to_owned());
        }
        let mut here = self.cwd.clone();
        // `GIT_DIR=` and `GIT_WORK_TREE=` an `export` earlier on the line put in the environment.
        let mut exported: Vec<String> = Vec::new();
        for raw in &commands {
            let words = command_words(raw);
            if words.first().is_some_and(|name| name == "export") {
                exported.extend(words[1..].iter().filter(|w| is_git_location(w)).cloned());
            }
            if let Some(name) = words.first().map(String::as_str)
                && (name == "cd" || name == "pushd")
            {
                let argument = words[1..]
                    .iter()
                    .find(|word| !word.starts_with('-') || word.as_str() == "-");
                let target = match argument.map(String::as_str) {
                    None => Some(self.home().to_path_buf()),
                    Some(word) => self.word_path(word, &here),
                };
                if let Some(target) = target {
                    if let Some(place) = self.elsewhere(&target) {
                        let shown = argument.map_or(name.to_owned(), |arg| format!("{name} {arg}"));
                        return (
                            Verdict::Deny,
                            format!(
                                "`{shown}` reaches {place}: read it by absolute path, without cd"
                            ),
                        );
                    }
                    here = target;
                }
            }
            for (at, word) in raw.iter().enumerate() {
                if is_assignment(word) || shell::program(word) != "git" {
                    continue;
                }
                let mut env = exported.clone();
                env.extend(raw[..at].iter().filter(|w| is_git_location(w)).cloned());
                if let Some(reason) = self.git_reach(&raw[at..], &env, &here) {
                    return (Verdict::Deny, reason);
                }
            }
        }
        if self.places.writes_records(command) {
            return (
                Verdict::Deny,
                format!(
                    "the command writes in conductor's records {}/ (a write target names them, \
                     or one the guard cannot read): a controller does not write them; {}",
                    self.places.shown(&self.places.records_dir()),
                    self.scratch_named()
                ),
            );
        }
        (
            Verdict::Allow,
            "no cd or git -C into another repository, no git write outside the repository's \
             checkout and managed worktrees, and no GitHub or GitLab write"
                .to_owned(),
        )
    }

    /// Why a run of the bot route, `b10x-gates gh`, is denied, if it is (the operator's decision
    /// of 2026-10-15): the route carries only `release create|upload|edit|view`, `run rerun` and
    /// `workflow run` ([`ROUTE_CARRIES`]) of the session's own repository, the `<owner>/<name>` the
    /// `origin` of its checkout names (conductor's: of its records), which `--repository` names
    /// and any `-R`/`--repo` among the `gh` arguments names too. A credential group is class H,
    /// as a bare `gh` write of one is. No `gh` arguments, or a help request, runs nothing on the
    /// forge.
    fn bot_route_denial(&self, gh: &BotRoute) -> Option<String> {
        let shown = format!("b10x-gates gh -- {}", gh.args.join(" "));
        let mut operands: Vec<&str> = Vec::new();
        let mut repos: Vec<&str> = Vec::new();
        let mut help = gh.help;
        // A flag between the group and the verb may take the verb's place as its value, as in
        // [`forge_write`]: the verb read would not be the one that runs.
        let mut flag_before_verb = false;
        let mut next = 0;
        while let Some(word) = gh.args.get(next) {
            next += 1;
            if word == "-R" || word == "--repo" {
                repos.extend(gh.args.get(next).map(String::as_str));
                next += 1;
            } else if let Some(repo) = word.strip_prefix("--repo=") {
                repos.push(repo);
            } else if word == "--help" {
                help = true;
            } else if word.starts_with('-') && word.len() > 1 {
                flag_before_verb |= operands.len() == 1;
            } else {
                operands.push(word);
            }
        }
        if help || operands.is_empty() {
            return None;
        }
        let group = operands[0];
        let verb = operands.get(1).copied().unwrap_or_default();
        if Forge::GitHub.credential_groups().contains(&group) {
            return Some(format!(
                "`{shown}` is a GitHub write of a credential through the bot route: class H, the \
                 operator's hands, whoever authorized it; no session runs it by any route"
            ));
        }
        let calls: Vec<String> = ROUTE_CARRIES
            .iter()
            .map(|(group, verbs)| format!("`{group} {}`", verbs.join("|")))
            .collect();
        let carried = format!(
            "the bot route carries only {} of the session's own repository",
            calls.join(", ")
        );
        let carries = ROUTE_CARRIES
            .iter()
            .any(|(name, verbs)| *name == group && verbs.contains(&verb));
        if !carries || flag_before_verb {
            return Some(format!(
                "`gh {group} {verb}` through the bot route is denied: {carried}, written with \
                 no flag between the group and the verb"
            ));
        }
        let checkout = match self.rules {
            Rules::Controller => self.checkout(),
            Rules::Conductor => self.places.records(),
        };
        let Some((host, own)) = collect::gitlab::origin(&checkout) else {
            return Some(format!(
                "`{shown}` is denied: {carried}, the one the origin of {} names, and it names \
                 none",
                self.places.shown(&checkout)
            ));
        };
        let is_own = |named: &str| {
            named.eq_ignore_ascii_case(&own) || named.eq_ignore_ascii_case(&format!("{host}/{own}"))
        };
        let Some(repository) = gh.repository else {
            return Some(format!(
                "`{shown}` names no --repository: {carried}, named as `b10x-gates gh --repository \
                 {own} -- release …`"
            ));
        };
        if let Some(other) = std::iter::once(repository)
            .chain(repos)
            .find(|r| !is_own(r))
        {
            return Some(format!(
                "`{shown}` names the repository {other}: {carried}, {own}"
            ));
        }
        None
    }

    /// Why a `git` command, `words` from the program word on and `env` the `GIT_DIR=` and
    /// `GIT_WORK_TREE=` assignments it runs with, is denied from `here`, if it is:
    ///
    /// - a global option, `-C`, `--git-dir` or `--work-tree`, each taken from where the `-C`
    ///   before it left off, or an assignment, taken from where the last `-C` left off, reaches
    ///   another place ([`Session::elsewhere`]);
    /// - or its subcommand writes ([`git_reads`]) and its target lands outside the repository's
    ///   checkout, its managed worktrees and its scratch ([`Session::is_scratch`]): the directory
    ///   the last `-C` left it in (`here` without one), and each `--git-dir`, `--work-tree`,
    ///   `GIT_DIR` and `GIT_WORK_TREE` (`story:guard-git-c-outside`).
    ///
    /// A target whose landing depends on a word the shell expands ([`Session::word_path`]) is not
    /// decided.
    fn git_reach(&self, words: &[String], env: &[String], here: &Path) -> Option<String> {
        let reaches = |shown: &str, place: String| format!("`{shown}` reaches {place}");
        let mut base = here.to_path_buf();
        // Whether `base` is where git runs: a `-C` that expands leaves it unknown.
        let mut known = true;
        // The `--git-dir`, `--work-tree`, `GIT_DIR` and `GIT_WORK_TREE` targets, where they land.
        let mut targets: Vec<PathBuf> = Vec::new();
        let mut subcommand = None;
        let mut at = 1;
        while let Some(word) = words.get(at) {
            let (flag, value, used) = match word.as_str() {
                "-C" | "--git-dir" | "--work-tree" => {
                    (word.as_str(), words.get(at + 1).map(String::as_str), 2)
                }
                "-c" | "--namespace" | "--exec-path" | "--config-env" => {
                    at += 2;
                    continue;
                }
                other => match other.split_once('=') {
                    Some((flag @ ("--git-dir" | "--work-tree"), value)) => (flag, Some(value), 1),
                    _ if other.starts_with('-') => {
                        at += 1;
                        continue;
                    }
                    _ => {
                        subcommand = Some(at);
                        break;
                    }
                },
            };
            at += used;
            let value = value?;
            let Some(target) = self.word_path(value, &base) else {
                if flag == "-C" {
                    known = false;
                }
                continue;
            };
            if let Some(place) = self.elsewhere(&target) {
                return Some(reaches(&format!("git {flag} {value}"), place));
            }
            if flag == "-C" {
                base = target;
                known = known || is_absolute_word(value);
            } else if known || is_absolute_word(value) {
                targets.push(target);
            }
        }
        for assignment in env {
            let Some((_, value)) = assignment.split_once('=') else {
                continue;
            };
            let Some(target) = self.word_path(value, &base) else {
                continue;
            };
            if let Some(place) = self.elsewhere(&target) {
                return Some(reaches(assignment.as_str(), place));
            }
            if known || is_absolute_word(value) {
                targets.push(target);
            }
        }
        let subcommand = subcommand?;
        if git_reads(&words[subcommand], &words[subcommand + 1..]) {
            return None;
        }
        if known {
            targets.insert(0, base);
        }
        let outside = targets.into_iter().find(|target| {
            !(target.starts_with(self.checkout())
                || target.starts_with(self.worktrees())
                || self.is_scratch(target))
        })?;
        let places = self.places;
        let shown = env
            .iter()
            .chain(&words[..=subcommand])
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        Some(format!(
            "`{shown}` writes in {}, outside the repository's checkout {}/ and its managed \
             worktrees {}/: a controller's git writes stay in its own repository and its scratch",
            places.shown_landing(&outside),
            places.shown(&places.checkouts.join(&self.repository)),
            places.shown(&places.trees.join(&self.repository))
        ))
    }

    /// A shell word naming a directory, where it lands from `here`: `~`, `$HOME` and `${HOME}`
    /// are the home directory. `None` for a word whose landing depends on anything else the shell
    /// would expand (another variable, a substitution, a glob, `-`).
    fn word_path(&self, word: &str, here: &Path) -> Option<PathBuf> {
        let home = self.home().to_string_lossy();
        let expanded = if word == "~" || word == "$HOME" || word == "${HOME}" {
            home.to_string()
        } else if let Some(rest) = word
            .strip_prefix("~/")
            .or_else(|| word.strip_prefix("$HOME/"))
            .or_else(|| word.strip_prefix("${HOME}/"))
        {
            format!("{home}/{rest}")
        } else {
            word.to_owned()
        };
        if word == "-" || expanded.contains(['$', '`', '*', '?', '[']) {
            return None;
        }
        Some(real(&here.join(expanded)))
    }

    /// Where the landing `path` is, when a controller's Bash may not reach it: another repository
    /// ([`Places::repository_of`]) than this session's own, or conductor's records.
    fn elsewhere(&self, path: &Path) -> Option<String> {
        let places = self.places;
        if let Some(other) = places
            .repository_of(path)
            .filter(|name| *name != self.repository)
        {
            return Some(format!("another repository, {other}"));
        }
        path.starts_with(places.records()).then(|| {
            format!(
                "conductor's records {}/",
                places.shown(&places.records_dir())
            )
        })
    }
}

/// The words of a simple command from its command name on: reserved words, variable assignments
/// and `builtin`, `command` and `exec` before it are dropped.
fn command_words(words: &[String]) -> &[String] {
    const RESERVED: [&str; 15] = [
        "!", "{", "}", "if", "then", "else", "elif", "fi", "do", "done", "while", "until", "time",
        "builtin", "command",
    ];
    let start = words
        .iter()
        .position(|word| {
            !(RESERVED.contains(&word.as_str()) || word == "exec" || is_assignment(word))
        })
        .unwrap_or(words.len());
    &words[start..]
}

fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !name.starts_with(|c: char| c.is_ascii_digit())
    })
}

/// Whether `word` assigns `GIT_DIR` or `GIT_WORK_TREE`, which move a git command's target as
/// `--git-dir` and `--work-tree` do.
fn is_git_location(word: &str) -> bool {
    word.starts_with("GIT_DIR=") || word.starts_with("GIT_WORK_TREE=")
}

/// Whether the directory word `word` lands where it does wherever it is taken from: `/`, `~` or
/// `$HOME` first.
fn is_absolute_word(word: &str) -> bool {
    ["/", "~", "$HOME", "${HOME}"]
        .iter()
        .any(|start| word.starts_with(start))
}

/// The git subcommands that only read, whatever their arguments (`story:guard-git-c-outside`).
const GIT_READS: &[&str] = &[
    "status",
    "log",
    "diff",
    "show",
    "rev-parse",
    "ls-files",
    "ls-tree",
    "ls-remote",
    "cat-file",
    "blame",
    "annotate",
    "describe",
    "shortlog",
    "grep",
    "rev-list",
    "show-ref",
    "for-each-ref",
    "name-rev",
    "merge-base",
    "count-objects",
    "version",
    "help",
    "var",
    "check-ignore",
    "check-attr",
    "check-ref-format",
    "diff-tree",
    "diff-files",
    "diff-index",
    "whatchanged",
    "verify-commit",
    "verify-tag",
    "show-branch",
    "range-diff",
    "cherry",
];

/// The `git branch` options that make it write: delete, move, copy, force or set an upstream.
const GIT_BRANCH_WRITES: &[&str] = &[
    "-d",
    "-D",
    "--delete",
    "-m",
    "-M",
    "--move",
    "-c",
    "-C",
    "--copy",
    "-f",
    "--force",
    "-u",
    "--set-upstream-to",
    "--unset-upstream",
    "--edit-description",
    "-t",
    "--track",
];

/// The `git branch` options that make it list, so that a word after them is no new branch.
const GIT_BRANCH_LISTS: &[&str] = &[
    "-l",
    "--list",
    "-a",
    "--all",
    "-r",
    "--remotes",
    "--show-current",
    "--contains",
    "--no-contains",
    "--merged",
    "--no-merged",
    "--points-at",
];

/// Whether the git subcommand `subcommand` with `arguments` only reads: one of [`GIT_READS`], or
/// the listing and reading forms of `config`, `remote`, `branch`, `tag`, `stash`, `worktree`,
/// `notes`, `reflog` and `submodule`. Anything else, an alias included, writes.
fn git_reads(subcommand: &str, arguments: &[String]) -> bool {
    fn holds(arguments: &[String], names: &[&str]) -> bool {
        arguments
            .iter()
            .any(|argument| names.contains(&argument.as_str()))
    }
    let has = |names: &[&str]| holds(arguments, names);
    let first = arguments
        .iter()
        .find(|argument| !argument.starts_with('-'))
        .map(String::as_str);
    match subcommand {
        read if GIT_READS.contains(&read) => true,
        "config" => {
            has(&[
                "--get",
                "--get-all",
                "--get-regexp",
                "--get-urlmatch",
                "--get-color",
                "--get-colorbool",
                "--list",
                "-l",
            ]) || matches!(first, Some("get" | "list"))
        }
        "remote" => matches!(first, None | Some("show" | "get-url")),
        "branch" => {
            !(has(GIT_BRANCH_WRITES)
                || arguments
                    .iter()
                    .any(|argument| argument.starts_with("--set-upstream-to=")))
                && (first.is_none() || has(GIT_BRANCH_LISTS))
        }
        "tag" => {
            !has(&[
                "-d",
                "--delete",
                "-f",
                "--force",
                "-a",
                "--annotate",
                "-s",
                "--sign",
                "-m",
                "-F",
            ]) && (first.is_none() || has(&["-l", "--list", "-v", "--verify"]))
        }
        "stash" => matches!(first, Some("list" | "show")),
        "worktree" => first == Some("list"),
        "notes" => matches!(first, None | Some("list" | "show")),
        "reflog" => matches!(first, None | Some("show" | "exists")),
        "submodule" => matches!(first, None | Some("status" | "summary")),
        _ => false,
    }
}

/// A `conductor` run a controller is denied ([`conductor_run`]).
enum DeniedRun {
    /// A leaf the tree declares a controller does not run, or a word under a group that names no
    /// leaf of it, as `<group> <leaf>` or `<leaf>`.
    Writes(String),
    /// A run whose leaf is decided after the guard reads the line, by the shell or by another
    /// program: the words as written, from the program word to the last one read.
    Unknown(String),
}

/// The characters by which the shell turns a written word into other words before a program
/// reads them: a parameter or a command substitution (`$`, a backquote), brace expansion (`{`)
/// and a glob (`*`, `?`, `[`). The guard reads words after quote removal, so a quoted literal
/// holding one counts too.
const EXPANSIONS: [char; 6] = ['$', '`', '{', '*', '?', '['];

/// The programs that run their first word with words they read at run time: a `conductor` they
/// run receives its group and leaf from their input.
const FEEDERS: [&str; 2] = ["xargs", "parallel"];

/// The programs that run the program word written after their own options and values.
const LAUNCHERS: [&str; 12] = [
    "env", "timeout", "nice", "nohup", "sudo", "doas", "setsid", "stdbuf", "ionice", "taskset",
    "xargs", "parallel",
];

/// The `find` actions whose next word is a program `find` runs.
const FIND_RUNS: [&str; 4] = ["-exec", "-execdir", "-ok", "-okdir"];

/// Whether `words[at]` is the program a simple command runs: its command name (`words[name]`, from
/// [`command_words`]); the word after a [`FIND_RUNS`] action; or the first word after a chain of
/// [`LAUNCHERS`] starting at the command name, in which every other word is an option, an
/// assignment, a value that starts with a digit (`timeout 5`, `nice -n 10`) or `{}`. A word that
/// follows any other program's words (`grep -rn conductor`) is an argument, not a program.
fn is_program_word(words: &[String], name: usize, at: usize) -> bool {
    if at <= name {
        return at == name;
    }
    if FIND_RUNS.contains(&words[at - 1].as_str()) {
        return true;
    }
    LAUNCHERS.contains(&shell::program(&words[name]))
        && words[name + 1..at].iter().all(|word| {
            LAUNCHERS.contains(&shell::program(word))
                || word.starts_with('-')
                || is_assignment(word)
                || word.starts_with(|c: char| c.is_ascii_digit())
                || word == "{}"
        })
}

/// Whether the shell expands `word` into other words before a program reads it ([`EXPANSIONS`]).
fn expands(word: &str) -> bool {
    word.contains(EXPANSIONS)
}

/// The `conductor` run of a simple command that a controller may not run.
///
/// A run is a word naming the `conductor` program, wherever it stands (after `env`, `timeout` or
/// a path), followed by a group of the tree and a leaf, or by a leaf with no group: the first two
/// words after it that are not flags, each of [`GLOBAL_VALUES`] taking its value. It is
/// [`DeniedRun::Writes`] when the leaf's [`Access`] in [`cli::LEAVES`] does not let a controller
/// run it, or when a word under a group names no leaf of the group. A `conductor` word that no
/// leaf follows runs none (`which conductor`, `conductor --help`, a path ending in `conductor`).
///
/// It is [`DeniedRun::Unknown`] (`story:guard-expanded-conductor-words`) when the shell or another
/// program decides the leaf after the guard has read the line, for a `conductor` that is the
/// command's program word ([`is_program_word`]; elsewhere, `grep -rn conductor *.md`, it is a
/// pattern or a path, held to the declared leaves only):
/// - a word read before the leaf is decided (the group word, the leaf word, a flag or a flag's
///   value) [`expands`]: brace expansion, `$"…"`, `${…}`, `$(…)`, `"$@"` or a glob can each
///   become a group and a leaf. Words after the leaf choose no other leaf;
/// - one of [`FEEDERS`] runs `conductor`, and the words after it name no leaf: the rest arrives
///   on its input;
/// - the command's program word expands and mentions `conductor` (`$(command -v conductor)`,
///   `` `which conductor` ``).
///
/// A program word that expands without mentioning `conductor` (`$CMD`), and a word elsewhere that
/// expands and mentions it, are held to the declared leaves only: the words after it are read as
/// a run of `conductor`, and a leaf a controller may not run is [`DeniedRun::Writes`]. Each check
/// runs only on a word that names `conductor` or holds an expansion character, and allocates only
/// to name a denial.
fn conductor_run(words: &[String]) -> Option<DeniedRun> {
    let name = words.len() - command_words(words).len();
    words.iter().enumerate().find_map(|(at, word)| {
        if shell::program(word) == "conductor" {
            if !is_program_word(words, name, at) {
                return leaf_after(&words[at..], false, false);
            }
            let fed = words[name..at]
                .iter()
                .any(|before| FEEDERS.contains(&shell::program(before)));
            return leaf_after(&words[at..], true, fed);
        }
        if !expands(word) {
            return None;
        }
        let mentions = word.contains("conductor");
        if at == name && mentions {
            return Some(DeniedRun::Unknown(words[at..].join(" ")));
        }
        if at == name || mentions {
            return leaf_after(&words[at..], false, false);
        }
        None
    })
}

/// The denial of the run whose program word is `words[0]`, read as `conductor`
/// ([`conductor_run`]). With `strict`, a word that [`expands`] before the leaf is decided makes
/// the run [`DeniedRun::Unknown`]; with `fed`, so does a run whose words name no leaf.
fn leaf_after(words: &[String], strict: bool, fed: bool) -> Option<DeniedRun> {
    let mut operands = [""; 2];
    let mut count = 0;
    let mut expanded = false;
    let mut at = 1;
    // The leaf is decided by a group and its leaf, or by a first operand that is no group.
    let mut decided = false;
    while !decided && let Some(word) = words.get(at) {
        at += 1;
        expanded |= expands(word);
        if GLOBAL_VALUES.contains(&word.as_str()) {
            expanded |= words.get(at).is_some_and(|value| expands(value));
            at += 1;
        } else if !(word.starts_with('-') && word.len() > 1) {
            operands[count] = word.as_str();
            count += 1;
            decided = count == 2 || !is_group(word);
        }
    }
    match operands[..count] {
        [group, leaf]
            if is_group(group)
                && !declared(&[group, leaf]).is_some_and(Access::controller_runs) =>
        {
            return Some(DeniedRun::Writes(format!("{group} {leaf}")));
        }
        // A leaf with no group a controller may not run: `trust` (`~/.claude.json`) and `init`
        // (a records directory). `doctor` reads.
        [leaf] if declared(&[leaf]).map(Access::controller_runs) == Some(false) => {
            return Some(DeniedRun::Writes(leaf.to_owned()));
        }
        _ => {}
    }
    // Two operands are a group and a leaf a controller runs; one names a leaf when it is one.
    let named = count == 2 || (count == 1 && declared(&operands[..1]).is_some());
    ((strict && expanded) || (fed && !named))
        .then(|| DeniedRun::Unknown(words[..at.min(words.len())].join(" ")))
}

/// The reservation the guard proposes for a build, test or gate: `auto`, which `conductor build
/// run` resolves to the largest peak its host ledger recorded for the repository and command plus
/// half (`story:reserve-from-peak`). The guard answers without opening the store, so it names no
/// size of its own (`story:guard-requires-build-run`).
const BUILD_RESERVE: &str = "auto";

/// The cargo subcommands that compile into a target directory, aliases included. `fmt`,
/// `metadata` and `tree` compile nothing.
const CARGO_BUILDS: [&str; 15] = [
    "build", "b", "test", "t", "check", "c", "clippy", "nextest", "install", "run", "r", "bench",
    "doc", "rustc", "fix",
];

/// The cargo options that take the next word as their value.
const CARGO_VALUES: [&str; 5] = ["--manifest-path", "--config", "--color", "-Z", "-C"];

/// The tasks that build, test or gate, with the reservation each is proposed.
const TASK_BUILDS: [(&str, &str); 6] = [
    ("check", BUILD_RESERVE),
    ("gate", BUILD_RESERVE),
    ("install", BUILD_RESERVE),
    ("regen", BUILD_RESERVE),
    ("docs-generate", BUILD_RESERVE),
    ("ui", BUILD_RESERVE),
];

/// The `task` options that take the next word as their value.
const TASK_VALUES: [&str; 8] = [
    "-d",
    "--dir",
    "-t",
    "--taskfile",
    "-o",
    "--output",
    "-C",
    "--concurrency",
];

/// The npm options that take the next word as their value.
const NPM_VALUES: [&str; 4] = ["--prefix", "-C", "-w", "--workspace"];

/// The first build, test or gate among the simple commands `commands` ([`shell::commands`]) that
/// does not run under `conductor build run`, as written from its program word on, and the
/// reservation it takes by default (`story:guard-requires-build-run`).
///
/// A command is a build when its program word ([`is_program_word`]: after `env`, `timeout` and
/// the other launchers too) is `cargo` with a subcommand of [`CARGO_BUILDS`], `task` with a task of
/// [`TASK_BUILDS`] before `--`, or npm running `ci`, `test` or the `build` or `test` script. A
/// `conductor build run` covers the words after its `--` and every command they run: a shell's
/// `-c` script, an `eval`. A command substitution on its line runs before it, in the session's
/// shell, and is held as any other command.
fn unreserved_build(commands: &[Vec<String>]) -> Option<(String, &'static str)> {
    let mut covered: Vec<Vec<String>> = Vec::new();
    for words in commands {
        if let Some(run) = build_run(words) {
            let script: Vec<String> = run.iter().map(|word| shell::quoted(word)).collect();
            covered.extend(shell::commands(&script.join(" ")));
        }
    }
    for words in commands {
        if build_run(words).is_some() {
            continue;
        }
        if let Some(at) = covered.iter().position(|run| run == words) {
            covered.swap_remove(at);
            continue;
        }
        let name = words.len() - command_words(words).len();
        for at in name..words.len() {
            if !is_program_word(words, name, at) {
                continue;
            }
            if let Some(size) = build_reserve(shell::program(&words[at]), &words[at + 1..]) {
                let run: Vec<String> = words[at..].iter().map(|word| shell::quoted(word)).collect();
                return Some((run.join(" "), size));
            }
        }
    }
    None
}

/// A `timeout` bounding `conductor snapshot` (`story:rules-into-code`): the program word is
/// `timeout` (as in [`is_program_word`], after other launchers too) and a later word runs
/// `conductor` with `snapshot start-snapshot` as its operands (the views read and end at once). A timeout kills the snapshot mid-write. The
/// answer is the denial's reason.
fn timeout_snapshot_denial(commands: &[Vec<String>]) -> Option<String> {
    for words in commands {
        let name = words.len() - command_words(words).len();
        if name >= words.len() {
            continue;
        }
        let Some(at) = (name..words.len()).find(|&at| {
            shell::program(&words[at]) == "timeout" && is_program_word(words, name, at)
        }) else {
            continue;
        };
        for (run, word) in words.iter().enumerate().skip(at + 1) {
            if shell::program(word) != "conductor" {
                continue;
            }
            let mut rest = words[run + 1..].iter();
            let mut operands: Vec<&str> = Vec::new();
            while operands.len() < 2
                && let Some(next) = rest.next()
            {
                if GLOBAL_VALUES.contains(&next.as_str()) {
                    rest.next();
                } else if !next.starts_with('-') {
                    operands.push(next.as_str());
                }
            }
            if operands == ["snapshot", "start-snapshot"] {
                return Some(
                    "`timeout` around `conductor snapshot start-snapshot` kills the snapshot \
                     mid-write and leaves a partial record (story rules-into-code): run it \
                     without a timeout; it ends on its own"
                        .to_owned(),
                );
            }
        }
    }
    None
}

/// The programs that kill by name or pattern.
const KILL_BY_NAME: [&str; 4] = ["pkill", "killall", "killall5", "skill"];

/// The options of `kill` that list signals and signal nobody.
const KILL_LISTS: [&str; 6] = ["-l", "-L", "-t", "--list", "--table", "--help"];

/// A kill of something the caller did not start, by pattern or by name (`story:guard-no-pattern-
/// kill`): the run of `pkill`, `killall`, `killall5` or `skill`, and of `kill` unless every
/// operand is a numeric process id (or `$!`, the last background job the shell started) after at
/// most one signal (`-9`, `-TERM`, `-s TERM`, `-n 9`, `--signal=TERM`); an operand that is a
/// name, `0`, a negative id (`-1`, a group), a `%job`, an expansion or a command substitution; or
/// a `kill` whose operands `xargs` supplies. A program word is read as in [`is_program_word`]; and
/// after a launcher (`sudo -u root pkill x`) every word, since its own options take values. The
/// answer is the denial's reason.
fn kill_denial(commands: &[Vec<String>]) -> Option<String> {
    for words in commands {
        let name = words.len() - command_words(words).len();
        if name >= words.len() {
            continue;
        }
        let launched = LAUNCHERS.contains(&shell::program(&words[name]));
        for at in name..words.len() {
            if !(at == name || launched || FIND_RUNS.contains(&words[at - 1].as_str())) {
                continue;
            }
            let program = shell::program(&words[at]);
            let by_name = KILL_BY_NAME.contains(&program);
            let fed = words[name..at]
                .iter()
                .any(|word| matches!(shell::program(word), "xargs" | "parallel"));
            if by_name || (program == "kill" && (fed || !kills_pids(&words[at + 1..]))) {
                let run: Vec<String> = words[at..].iter().map(|word| shell::quoted(word)).collect();
                return Some(format!(
                    "`{}` kills by pattern, by name or by group, so it can kill another \
                     session's processes (rules.md item 147): never `pkill`, `killall` or `kill` \
                     by pattern, name, `-1`, a negative group or `%job`. Stop what you started \
                     with TaskStop, or `kill <pid>` / `kill -<SIG> <pid>` with the numeric pid of \
                     a process you started",
                    run.join(" ")
                ));
            }
        }
    }
    None
}

/// Whether the arguments of a `kill` name only numeric process ids: an optional signal first,
/// then one or more ids, each digits and not `0` (the caller's own group), or `$!`; or only list
/// signals ([`KILL_LISTS`]).
fn kills_pids(arguments: &[String]) -> bool {
    if arguments
        .first()
        .is_some_and(|word| KILL_LISTS.contains(&word.as_str()))
    {
        return true;
    }
    let mut rest = arguments;
    if let Some(first) = rest.first() {
        if first == "-s" || first == "-n" || first == "--signal" {
            rest = rest.get(2..).unwrap_or_default();
        } else if first.starts_with("--signal=")
            || (first.len() > 1
                && first.starts_with('-')
                && first[1..].chars().all(|c| c.is_ascii_alphanumeric()))
        {
            rest = &rest[1..];
        }
    }
    if rest.first().is_some_and(|word| word == "--") {
        rest = &rest[1..];
    }
    !rest.is_empty()
        && rest.iter().all(|word| {
            word == "$!"
                || (!word.is_empty()
                    && word.chars().all(|c| c.is_ascii_digit())
                    && word.chars().any(|c| c != '0'))
        })
}

/// The words a `conductor build run` in `words` runs: those after its `--`, or after `run`
/// when it has none. `None` when `words` runs no `conductor build run`.
fn build_run(words: &[String]) -> Option<&[String]> {
    let name = words.len() - command_words(words).len();
    let at = (name..words.len()).find(|&at| {
        shell::program(&words[at]) == "conductor" && is_program_word(words, name, at)
    })?;
    let mut next = at + 1;
    let mut operands = Vec::new();
    while operands.len() < 2
        && let Some(word) = words.get(next)
    {
        next += 1;
        if GLOBAL_VALUES.contains(&word.as_str()) {
            next += 1;
        } else if !word.starts_with('-') {
            operands.push(word.as_str());
        }
    }
    if operands != ["build", "run"] {
        return None;
    }
    let rest = words.get(next..).unwrap_or_default();
    Some(match rest.iter().position(|word| word == "--") {
        Some(dash) => &rest[dash + 1..],
        None => rest,
    })
}

/// The reservation the run of `program` with `arguments` takes by default when it builds, tests
/// or gates ([`unreserved_build`]).
fn build_reserve(program: &str, arguments: &[String]) -> Option<&'static str> {
    match program {
        "cargo" => build_operands(arguments, &CARGO_VALUES)
            .first()
            .filter(|subcommand| CARGO_BUILDS.contains(*subcommand))
            .map(|_| BUILD_RESERVE),
        "task" => build_operands(arguments, &TASK_VALUES)
            .into_iter()
            .find_map(|task| {
                TASK_BUILDS
                    .iter()
                    .find(|&&(name, _)| name == task)
                    .map(|&(_, size)| size)
            }),
        "npm" => match build_operands(arguments, &NPM_VALUES).as_slice() {
            [
                "ci" | "clean-install" | "install-clean" | "test" | "t" | "tst",
                ..,
            ] => Some(BUILD_RESERVE),
            ["run" | "run-script" | "rum" | "urn", "build" | "test", ..] => Some(BUILD_RESERVE),
            _ => None,
        },
        _ => None,
    }
}

/// The operands of a program's `arguments` before `--`: no option, no `+toolchain`, no
/// assignment, and no value of an option in `values`.
fn build_operands<'a>(arguments: &'a [String], values: &[&str]) -> Vec<&'a str> {
    let mut operands = Vec::new();
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        if values.contains(&word.as_str()) {
            words.next();
        } else if !(word.starts_with('-') || word.starts_with('+') || is_assignment(word)) {
            operands.push(word.as_str());
        }
    }
    operands
}

/// Whether `command` names a file in a `.claude/` directory whose name holds `settings`: the
/// path after `.claude/`, up to a blank, a quote or a shell operator, ends in such a name.
fn names_claude_settings(command: &str) -> bool {
    const DIR: &str = ".claude/";
    command.match_indices(DIR).any(|(at, _)| {
        let rest = &command[at + DIR.len()..];
        let end = rest
            .find(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        '"' | '\'' | '`' | ';' | '|' | '&' | '<' | '>' | '(' | ')'
                    )
            })
            .unwrap_or(rest.len());
        rest[..end]
            .rsplit('/')
            .next()
            .is_some_and(|name| name.contains("settings"))
    })
}

/// The editors that write their file operands only with an in-place flag ([`in_place`]).
const IN_PLACE_EDITORS: [&str; 2] = ["sed", "yq"];

/// The programs that run a script of their own, which can write a file the command names without
/// naming it again (`sh -c 'cp x "$2"' sh <file>`, `python3 -c '…' <file>`, `xargs cp x`): the
/// shells, the interpreters, and the programs that run another with words read at run time. `awk`
/// is none: its file operands are read, and a write from inside its quoted program is not read.
const SCRIPT_RUNNERS: [&str; 16] = [
    "sh", "bash", "zsh", "dash", "ksh", "fish", "python", "python3", "node", "ruby", "perl",
    "xargs", "parallel", "eval", "source", ".",
];

/// The variables a word may start with and still land where the guard reads it, as long as the
/// command does not set them ([`fixed`]): `$HOME` (and `~`) the home directory, `$TMPDIR` the
/// session's scratch.
const KNOWN_VARIABLES: [&str; 2] = ["HOME", "TMPDIR"];

/// Whether the Bash `command` writes a file `names` holds for (`story:guard-write-targets`): it
/// names one and holds a write form ([`holds_write_form`]), and one of its simple commands
/// ([`shell::simple_commands`])
/// - redirects output to a target `names` holds for, or to one the guard cannot read
///   ([`unread`]);
/// - runs a writing program, one of [`WRITE_WORDS`], `dd`, or one of [`IN_PLACE_EDITORS`] with an
///   in-place flag, with an operand `names` holds for or one the guard cannot read: every operand
///   counts, the source of `cp` or `mv` too;
/// - or runs one of [`SCRIPT_RUNNERS`], whose script can reach any word of the command.
///
/// So a path that appears only as a read operand (`awk … <path> > brief.md`), inside a quoted
/// program or string, or in a here-document's body is not written; a target held in a variable is
/// decided as before, by the name and the write form alone.
fn writes_named(command: &str, names: impl Fn(&str) -> bool) -> bool {
    if !names(command) || !holds_write_form(command) {
        return false;
    }
    let known: Vec<&str> = KNOWN_VARIABLES
        .into_iter()
        .filter(|variable| fixed(command, variable))
        .collect();
    let written = |word: &str| names(word) || unread(word, &known);
    shell::simple_commands(command)
        .iter()
        .any(|(words, writes)| {
            writes.iter().any(|target| written(target)) || {
                let name = words.len() - command_words(words).len();
                (name..words.len()).any(|at| {
                    if !is_program_word(words, name, at) {
                        return false;
                    }
                    let program = shell::program(&words[at]);
                    let operands = &words[at + 1..];
                    SCRIPT_RUNNERS.contains(&program)
                        || ((WRITE_WORDS.contains(&program)
                            || program == "dd"
                            || (IN_PLACE_EDITORS.contains(&program)
                                && operands.iter().any(|word| in_place(word))))
                            && operands.iter().any(|word| written(word)))
                })
            }
        })
}

/// Whether the command-line flag `word` edits in place: `--in-place`, `--inplace`, or a short
/// flag cluster holding `i` (`-i`, `-i.bak`, `-ni`).
fn in_place(word: &str) -> bool {
    word.starts_with("--in-place")
        || word.starts_with("--inplace")
        || (word.starts_with('-') && !word.starts_with("--") && word[1..].contains('i'))
}

/// Whether `command` leaves the variable `variable` as the session's environment has it: every
/// mention of the name is an expansion (`$NAME`, `${NAME}`), none an assignment, an `export` or a
/// loop variable.
fn fixed(command: &str, variable: &str) -> bool {
    command.match_indices(variable).all(|(at, _)| {
        let before = &command[..at];
        before.ends_with('$') || before.ends_with("${")
    })
}

/// Whether the guard cannot read where the shell word `word` lands: it is empty (an output
/// operator with no word after it), `{}` (a file `find` or `xargs` supplies), starts with `~` while
/// the command sets `HOME`, or holds a parameter, a substitution or a backquote other than a
/// leading `$HOME`, `${HOME}`, `$TMPDIR` or `${TMPDIR}` of the `known` variables.
fn unread(word: &str, known: &[&str]) -> bool {
    if word.is_empty() || word == "{}" || (word.starts_with('~') && !known.contains(&"HOME")) {
        return true;
    }
    let rest = known
        .iter()
        .flat_map(|variable| [format!("${variable}"), format!("${{{variable}}}")])
        .find_map(|prefix| {
            word.strip_prefix(prefix.as_str())
                .filter(|rest| rest.is_empty() || rest.starts_with('/'))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| word.to_owned());
    rest.contains(['$', '`'])
}

/// Whether the Bash `command` is one the guard treats as a write (`story:lease-guard`): it holds
/// a write form ([`holds_write_form`]), or runs a `conductor` leaf a controller is denied as one
/// that writes or may write conductor's records ([`conductor_run`]). A write the guard does not
/// recognise as one (`git commit`, a program writing a file by its own means) is not.
pub(super) fn bash_writes(command: &str) -> bool {
    holds_write_form(command)
        || shell::commands(command)
            .iter()
            .any(|words| conductor_run(words).is_some())
}

/// Whether `command` holds a write form: a redirection that writes a file ([`shell::written`]:
/// outside quotes, and not a descriptor duplication such as `2>&1` or `/dev/null`), one of
/// [`WRITE_SUBSTRINGS`], or one of [`WRITE_WORDS`] as a word.
fn holds_write_form(command: &str) -> bool {
    !shell::written(command).is_empty()
        || WRITE_SUBSTRINGS.iter().any(|form| command.contains(form))
        || WRITE_WORDS.iter().any(|form| contains_word(command, form))
        || edits_in_place(command)
}

/// Whether `command` runs an in-place editor (`sed`, `yq`, `perl`) with an in-place flag among the
/// flags that follow it ([`in_place`]): `sed -ni`, `sed -n -i`, `perl -0pi`, which the substrings
/// of [`WRITE_SUBSTRINGS`] miss.
fn edits_in_place(command: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    words.iter().enumerate().any(|(at, word)| {
        matches!(shell::program(word), "sed" | "yq" | "perl")
            && words[at + 1..]
                .iter()
                .take_while(|flag| flag.starts_with('-'))
                .any(|flag| in_place(flag))
    })
}

/// Whether `command` names the path `spelling` as a whole: `spelling` followed by the end of the
/// command, a blank, a quote or a shell operator, or by `/` and then one of those; with `under`,
/// by `/` and anything, a path under it. A path that merely starts with the same letters
/// (`records-old` beside `records`) is not named.
fn names_path(command: &str, spelling: &str, under: bool) -> bool {
    let ends = |rest: &str| {
        rest.chars().next().is_none_or(|c| {
            c.is_whitespace()
                || matches!(
                    c,
                    '"' | '\'' | '`' | ';' | '|' | '&' | '<' | '>' | '(' | ')'
                )
        })
    };
    command.match_indices(spelling).any(|(at, _)| {
        let rest = &command[at + spelling.len()..];
        ends(rest)
            || rest
                .strip_prefix('/')
                .is_some_and(|rest| under || ends(rest))
    })
}

/// The programs whose own words a POST or a body is read from: curl, wget and the httpie family.
const HTTP_CLIENTS: [&str; 6] = ["curl", "wget", "wget2", "http", "https", "xh"];

/// Text that sends a body or a POST from an interpreter's client, compared without case: Python's
/// urllib (`data=`) and requests (`.post(`, `json=`), and a `fetch` or an axios call
/// (`method: 'POST'`, `body:`).
const INTERPRETER_POSTS: [&str; 8] = [
    "data=",
    "json=",
    ".post(",
    "'post'",
    "\"post\"",
    "body:",
    "body=",
    "method=post",
];

/// Whether the Bash `command` sends an HTTP POST or a request body
/// (`story:decisions-batch-answer`): an HTTP client's own simple command ([`HTTP_CLIENTS`], after
/// any launcher such as `timeout` or `env`) holds `POST` (`-X POST`, `--request POST`, httpie's
/// `http POST`), `-XPOST`, `--request=POST` or `--method=POST`, a body flag (`-d…`, `--data…`,
/// `--json`, `-F`, `--form…`, `-T`, `--upload-file`, `--post-data`, `--post-file`), or, for the
/// httpie family, a request item (`name=value`, `name:=json`); or the line holds an interpreter's
/// POST or body ([`INTERPRETER_POSTS`]). A flag of another program (`tr -d`, `cut -d`, `date -d`,
/// `sort -d`) is no body.
fn posts(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    if INTERPRETER_POSTS.iter().any(|post| lower.contains(post)) {
        return true;
    }
    shell::commands(command).iter().any(|words| {
        let words = command_words(words);
        let Some(at) = words
            .iter()
            .position(|word| HTTP_CLIENTS.contains(&shell::program(word)))
        else {
            return false;
        };
        let httpie = !matches!(shell::program(&words[at]), "curl" | "wget" | "wget2");
        words[at + 1..].iter().any(|word| {
            let upper = word.to_ascii_uppercase();
            upper == "POST"
                || upper == "-XPOST"
                || upper.ends_with("=POST")
                || (word.starts_with("-d") && !word.starts_with("--"))
                || [
                    "--data",
                    "--json",
                    "--form",
                    "--upload-file",
                    "--post-data",
                    "--post-file",
                ]
                .iter()
                .any(|flag| word.starts_with(flag))
                || matches!(word.as_str(), "-F" | "-T")
                || (httpie && !word.starts_with('-') && !word.contains("://") && word.contains('='))
        })
    })
}

/// Whether `text` holds `word` with no letter, digit, `_`, `-` or `.` on either side.
fn contains_word(text: &str, word: &str) -> bool {
    let joins = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.');
    text.match_indices(word).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(joins)
            && !text[at + word.len()..].chars().next().is_some_and(joins)
    })
}

/// Whether `path` is a settings file in a directory named `.claude`: a `.json` file whose name
/// holds `settings`, which covers Claude Code's project settings (`settings.json`,
/// `settings.local.json`) and the files that wire the guard (`controller-settings.json`,
/// `conductor-settings.json`).
fn settings_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|name| name.to_str());
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    parent == Some(".claude")
        && name.is_some_and(|name| name.ends_with(".json") && name.contains("settings"))
}

/// The absolute `path` with `.` and `..` taken by name, no link followed.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::from("/");
    for step in steps(path) {
        match step {
            Step::Up => {
                out.pop();
            }
            Step::Name(name) => out.push(name),
        }
    }
    out
}

/// The forge write a simple command makes through `forge`'s program: any call that is not a read,
/// as `<program> <group> <verb>`. The group and the verb are the first two words after the
/// program that are not flags, `-R`/`--repo` taking its value. A program named by [`GH_NOT_RUN`]
/// or `command -v`, as the simple command's program, does not run it here
/// ([`names_without_running`]).
///
/// The program is a word naming `forge`'s program anywhere in the command, or a program word that
/// [`expands`] ([`is_program_word`], or any word after a [`LAUNCHERS`] command name) and so may
/// become it. As [`conductor_run`] holds an expanding program word, one that mentions the forge's
/// program (`$(which glab)`, `"$(command -v gh)"`) is a write whatever follows it: the shell, not
/// the guard, decides what runs. One that does not mention it (`$g`) is read as the forge's program
/// when `assigned`, a variable the line assigns may hold it (`g=glab; $g mr merge 12`).
///
/// Any flag but `-R`/`--repo`, `--help` and `--version`, written between a group other than `api`
/// and its verb, makes the call a write: it may take the next word as its value (`glab mr -m view
/// note 12` posts the note `view`), so the verb read is not the one that runs. `--help` is a help
/// request only when the word before it is no flag that may take it as its value (`glab mr note
/// 12 -m --help` posts the note `--help`).
///
/// A `gh` read is `<group> view|list|status|diff|checks|search|download`,
/// `run view|list|watch|download`, `api` with GET and no request fields, `api graphql` without a
/// mutation, `auth status`, `search …`, `status`, `help`, any call carrying `--help`, or
/// `--version`/`-h` with no group: with one, `-h` may be another flag (`gh auth login -h
/// github.com` is `--hostname`).
///
/// A `glab` read is `mr view|list|diff`, `issue view|list`, `release view|list|download`,
/// `repo view|list`, `ci view|list|status|trace|get` ([`GLAB_READS`]), `api` with GET and no
/// request fields, `auth status`, any call carrying `--help`, or `--version` with no group
/// (`story:guard-glab-writes`).
///
/// The `api` call is read with every flag on the line, those before the `api` group too: the leaf
/// parses them all (`glab -XDELETE api projects/1` deletes).
fn forge_write(words: &[String], forge: Forge, assigned: bool) -> Option<String> {
    let name = words.len() - command_words(words).len();
    let launched = words
        .get(name)
        .is_some_and(|word| LAUNCHERS.contains(&shell::program(word)));
    let at = (0..words.len()).find(|&at| {
        let word = &words[at];
        forge.runs(word)
            || (expands(word)
                && !is_assignment(word)
                && (is_program_word(words, name, at) || (launched && at > name))
                && (assigned || contains_word(word, forge.program())))
    })?;
    if !forge.runs(&words[at]) && contains_word(&words[at], forge.program()) {
        return Some(words[at..].join(" "));
    }
    if names_without_running(&words[..at]) {
        return None;
    }
    let rest = &words[at + 1..];
    let mut operands: Vec<(usize, &str)> = Vec::new();
    let mut flags: Vec<(usize, &str)> = Vec::new();
    let mut help = false;
    // A flag written after the group and before the verb, and whether one was before the verb.
    let mut pending = false;
    let mut flag_before_verb = false;
    let mut next = 0;
    while let Some(word) = rest.get(next) {
        next += 1;
        if word == "-R" || word == "--repo" {
            next += 1;
        } else if word.starts_with('-') && word.len() > 1 {
            if word == "--help" {
                let previous = next.checked_sub(2).map(|before| rest[before].as_str());
                help |= !previous.is_some_and(|before| before.starts_with('-') && before.len() > 1);
            }
            if operands.len() == 1
                && !(matches!(word.as_str(), "--help" | "--version") || word.starts_with("--repo="))
            {
                pending = true;
            }
            flags.push((next - 1, word.as_str()));
        } else {
            if operands.len() == 1 {
                flag_before_verb = pending;
            }
            operands.push((next - 1, word.as_str()));
        }
    }
    let group = operands.first().map(|(_, word)| *word);
    let verb = operands.get(1).map(|(_, word)| *word);
    let has_flag = |wanted: &str| flags.iter().any(|(_, flag)| *flag == wanted);
    // The words `api` reads: every flag before the `api` group, then the words after it.
    let api_arguments = || -> Vec<String> {
        let api = operands[0].0;
        flags
            .iter()
            .filter(|(index, _)| *index < api)
            .map(|(_, flag)| (*flag).to_owned())
            .chain(rest[api + 1..].iter().cloned())
            .collect()
    };
    // A verb a flag's value may have displaced is no verb the guard can read.
    let displaced = flag_before_verb && group != Some("api");
    let read = help
        || (!displaced
            && match forge {
                Forge::GitHub => match (group, verb) {
                    (None, _) => has_flag("--version") || has_flag("-h"),
                    (Some("help" | "search"), _)
                    | (Some("status"), None)
                    | (Some("auth"), Some("status")) => true,
                    (Some("api"), _) => api_reads(&api_arguments(), forge),
                    (Some("run"), Some(verb)) => GH_RUN_READ_VERBS.contains(&verb),
                    (Some(_), Some(verb)) => GH_READ_VERBS.contains(&verb),
                    (Some(_), None) => false,
                },
                Forge::GitLab => match (group, verb) {
                    (None, _) => has_flag("--version"),
                    (Some("auth"), Some("status")) => true,
                    (Some("api"), _) => api_reads(&api_arguments(), forge),
                    (Some(group), Some(verb)) => glab_reads(group, verb),
                    (Some(_), None) => false,
                },
            });
    (!read).then(|| {
        if flag_before_verb && let Some(&(verb_at, _)) = operands.get(1) {
            // The flag and the value it may take, as written.
            return format!("{} {}", forge.program(), rest[..=verb_at].join(" "));
        }
        let named: Vec<&str> = operands.iter().take(2).map(|(_, word)| *word).collect();
        let shown = format!("{} {}", forge.program(), named.join(" "));
        shown.trim_end().to_owned()
    })
}

/// Whether the words before a `gh` or `glab` word, `before`, make a command that names it without
/// running it: the simple command's program, after reserved words and variable assignments, is
/// one of [`GH_NOT_RUN`] (`which glab`, `b10x-gates gh -- pr create`), or is `command` with only
/// flags after it, one of them `-v` or `-V` (`command -v gh`). The same name anywhere else, as an
/// assignment's value or a launcher's argument (`env -C /usr/share/man glab`), runs it.
fn names_without_running(before: &[String]) -> bool {
    const RESERVED: [&str; 15] = [
        "!", "{", "}", "if", "then", "else", "elif", "fi", "do", "done", "while", "until", "time",
        "builtin", "exec",
    ];
    let start = before
        .iter()
        .position(|word| !(RESERVED.contains(&word.as_str()) || is_assignment(word)))
        .unwrap_or(before.len());
    let Some(program) = before.get(start) else {
        return false;
    };
    if GH_NOT_RUN.contains(&shell::program(program)) {
        return true;
    }
    let options = &before[start + 1..];
    program == "command"
        && options
            .iter()
            .all(|word| word.starts_with('-') && word.len() > 1 && !word.starts_with("--"))
        && options.iter().any(|word| word[1..].contains(['v', 'V']))
}

/// Whether `glab <group> <verb>` is one of the reads [`GLAB_READS`] names.
fn glab_reads(group: &str, verb: &str) -> bool {
    GLAB_READS
        .iter()
        .any(|(name, verbs)| *name == group && verbs.contains(&verb))
}

/// Whether the arguments of `gh api` or `glab api` make a read: GET (no method or GET named, by
/// `-X`, `--method` or a flag cluster holding `X`) and no request field (`-f`, `-F`, `--field`,
/// `--raw-field`, `--input`), or, for `gh` only, `graphql` without a mutation. A query from a
/// file (`=@…`) or `--input` may be a mutation, and is taken as one. `glab api graphql` reads only
/// as any other endpoint does: by GET with no field.
fn api_reads(arguments: &[String], forge: Forge) -> bool {
    /// Flags of `gh api` that take the next word as their value.
    const GH_VALUED: &[&str] = &[
        "-H",
        "--header",
        "-q",
        "--jq",
        "-t",
        "--template",
        "-p",
        "--preview",
        "--hostname",
        "--cache",
    ];
    /// The short flags of `gh api` that take a value, attached or as the next word.
    const GH_SHORT_VALUED: &[char] = &['f', 'F', 'H', 'q', 't', 'p'];
    /// Flags of `glab api` that take the next word as their value, besides the method and the
    /// request fields.
    const GLAB_VALUED: &[&str] = &["-H", "--header", "--hostname"];
    /// The short flags of `glab api` that take a value, attached or as the next word.
    const GLAB_SHORT_VALUED: &[char] = &['f', 'F', 'H'];
    let (valued, short_valued) = match forge {
        Forge::GitHub => (GH_VALUED, GH_SHORT_VALUED),
        Forge::GitLab => (GLAB_VALUED, GLAB_SHORT_VALUED),
    };
    let mut method: Option<&str> = None;
    let mut fields = false;
    let mut endpoint = None;
    let mut next = 0;
    while let Some(word) = arguments.get(next) {
        let word = word.as_str();
        next += 1;
        match word {
            "-X" | "--method" => {
                method = arguments.get(next).map(String::as_str);
                next += 1;
            }
            "-f" | "-F" | "--field" | "--raw-field" | "--input" => {
                fields = true;
                next += 1;
            }
            _ if valued.contains(&word) => next += 1,
            _ if word.starts_with("--method=") => method = word.strip_prefix("--method="),
            _ if ["--field=", "--raw-field=", "--input="]
                .iter()
                .any(|flag| word.starts_with(flag)) =>
            {
                fields = true;
            }
            _ if word.starts_with("--") => {}
            _ if word.starts_with('-') && word.len() > 1 => {
                for (at, flag) in word[1..].char_indices() {
                    let value = &word[1 + at + flag.len_utf8()..];
                    match flag {
                        'X' => {
                            method = if value.is_empty() {
                                next += 1;
                                arguments.get(next - 1).map(String::as_str)
                            } else {
                                Some(value)
                            };
                            break;
                        }
                        _ if short_valued.contains(&flag) => {
                            fields |= matches!(flag, 'f' | 'F');
                            if value.is_empty() {
                                next += 1;
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                endpoint.get_or_insert(word);
            }
        }
    }
    let mutation = arguments.iter().any(|word| {
        word.contains("mutation")
            || word.contains("=@")
            || word == "--input"
            || word.starts_with("--input=")
    });
    if forge == Forge::GitHub && endpoint == Some("graphql") {
        return !mutation;
    }
    method.is_none_or(|method| method.eq_ignore_ascii_case("GET")) && !fields
}

/// One step of a path still to resolve.
enum Step {
    Up,
    Name(OsString),
}

/// Where the absolute `path` lands: `.` and `..` are taken as the kernel takes them, after every
/// symbolic link before them, and every symbolic link is followed whether or not its target
/// exists. A component that does not exist is kept as written.
pub(super) fn real(path: &Path) -> PathBuf {
    /// How many links are followed before the rest is kept as written (the kernel's `ELOOP`).
    const LINKS: usize = 40;
    let mut pending: VecDeque<Step> = steps(path).collect();
    let mut out = PathBuf::from("/");
    let mut followed = 0;
    while let Some(step) = pending.pop_front() {
        match step {
            Step::Up => {
                out.pop();
            }
            Step::Name(name) => {
                let next = out.join(&name);
                let link = fs::symlink_metadata(&next)
                    .is_ok_and(|meta| meta.file_type().is_symlink())
                    .then(|| fs::read_link(&next).ok())
                    .flatten();
                match link {
                    Some(target) if followed < LINKS => {
                        followed += 1;
                        if target.is_absolute() {
                            out = PathBuf::from("/");
                        }
                        let mut ahead: VecDeque<Step> = steps(&target).collect();
                        ahead.extend(pending.drain(..));
                        pending = ahead;
                    }
                    _ => out = next,
                }
            }
        }
    }
    out
}

fn steps(path: &Path) -> impl Iterator<Item = Step> + '_ {
    path.components().filter_map(|component| match component {
        Component::ParentDir => Some(Step::Up),
        Component::Normal(name) => Some(Step::Name(name.to_owned())),
        Component::RootDir | Component::CurDir | Component::Prefix(_) => None,
    })
}

/// The scheme of a session's socket address in SendMessage's `to`.
const SOCKET_SCHEME: &str = "uds:";

/// The form a socket address `to` must have, as a denial names it.
const SOCKET_FORM: &str = "of the form uds:<absolute path>/<pid>.sock";

/// Whether `to` names the session called `name`, and by which ref: `Some(None)` for `name`,
/// `Some(Some(ref))` for `name [<ref>]`, the ref being one or more lower-case hexadecimal
/// characters as the session list prints it (wave 07 U8 brief, decision 1: the harness refuses a
/// ref that does not belong to a session of that name).
fn named<'t>(to: &'t str, name: &str) -> Option<Option<&'t str>> {
    if to == name {
        return Some(None);
    }
    let reference = to
        .strip_prefix(name)?
        .strip_prefix(" [")?
        .strip_suffix(']')?;
    let hexadecimal = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    (!reference.is_empty() && reference.bytes().all(hexadecimal)).then_some(Some(reference))
}

/// The session name `to` addresses: `to` without a trailing ` [<ref>]` ([`named`]).
fn named_base(to: &str) -> &str {
    to.rsplit_once(" [")
        .filter(|(base, _)| named(to, base).is_some())
        .map_or(to, |(base, _)| base)
}

/// The pid a socket path names: its file name is `<pid>.sock`, the pid written in decimal with
/// no sign and no leading zero.
fn socket_pid(path: &Path) -> Option<u64> {
    let stem = path.file_name()?.to_str()?.strip_suffix(".sock")?;
    let pid: u64 = stem.parse().ok()?;
    (pid.to_string() == stem).then_some(pid)
}

/// The pids of the live sessions in the session list whose name and working directory (where it
/// lands) `wanted` accepts. The list is read once ([`live_sessions`]).
fn live_pids(
    sessions: &SessionList,
    wanted: &dyn Fn(&str, Option<&Path>) -> bool,
) -> Result<Vec<u64>> {
    Ok(live_sessions(sessions)?
        .into_iter()
        .filter(|session| wanted(&session.name, session.cwd.as_deref()))
        .filter_map(|session| session.pid)
        .collect())
}

/// One session of the session list: its name, its `id` (else `sessionId`), its pid when it
/// runs, and its working directory where it lands.
struct Live {
    name: String,
    id: Option<String>,
    pid: Option<u64>,
    cwd: Option<PathBuf>,
}

/// The sessions of the session list: its entries with a `name`, with a `pid` or without one. The
/// list is read once, through [`collect::run`] under its bound.
fn live_sessions(sessions: &SessionList) -> Result<Vec<Live>> {
    let shown = command_line(&sessions.command);
    let (program, arguments) = sessions
        .command
        .split_first()
        .context("no session-list command is given")?;
    let output = collect::run(Command::new(program).args(arguments), sessions.bound)
        .with_context(|| format!("run `{shown}`"))?;
    match output.status.code() {
        Some(0) => {}
        Some(code) => bail!("`{shown}` exited {code}"),
        None => bail!("`{shown}` was ended by a signal"),
    }
    let Value::Array(entries) = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("`{shown}` printed no JSON"))?
    else {
        bail!("`{shown}` printed no JSON array");
    };
    Ok(entries
        .iter()
        .filter_map(|entry| {
            Some(Live {
                name: entry["name"].as_str()?.to_owned(),
                id: entry["id"]
                    .as_str()
                    .or_else(|| entry["sessionId"].as_str())
                    .map(str::to_owned),
                pid: entry["pid"].as_u64(),
                cwd: entry["cwd"]
                    .as_str()
                    .filter(|cwd| Path::new(cwd).is_absolute())
                    .map(|cwd| real(Path::new(cwd))),
            })
        })
        .collect())
}

/// A command line as one string, its words joined by spaces.
fn command_line(command: &[OsString]) -> String {
    command
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `to` is an in-process agent id (`a` and 16 lowercase hex digits, such as
/// `a0123456789abcdef`), which addresses an agent this session started. Other sessions are
/// addressed by name (Claude Code's SendMessage description: "Use the raw agentId … only when the
/// agent has no name").
pub(super) fn is_own_agent_id(to: &str) -> bool {
    to.len() == 17
        && to.starts_with('a')
        && to[1..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
