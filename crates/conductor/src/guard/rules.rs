//! The guard's rule table: which tool calls a session may make, by the repository it runs in.
//!
//! Where the repositories and conductor's records are comes from the instance the process runs
//! ([`Places`], read from `crate::config::active`): the checkouts root `<root>`, the managed-worktree
//! root `<trees>`, and conductor's records `<records>`. Without a config file they are the built-in
//! instance's, and `<records>` is the checkout of the repository named `conductor`. Conductor's
//! config is the file the process resolves, `<config>` (`config::locate`: `CONDUCTOR_CONFIG`, else
//! [`config::DEFAULT_FILE`] under the home directory; [`Places::with_config_file`]), and the
//! directory of that default file, `<config-dir>`, which a controller writes neither of, so that it
//! cannot give itself conductor's rules or a wider checkout.
//!
//! `<settings>` is the `settings` file of any role of the instance (`config::active`), the files
//! that wire the guard, and `<profile>` the `profile` file of any role, the system prompt its
//! sessions start with.
//!
//! | tool | controller: denied when | conductor: denied when |
//! |---|---|---|
//! | Edit, Write, NotebookEdit | the target is `<config>` or under `<config-dir>`, wherever they are; is outside `<root>/<repo>/` and `<trees>/<repo>/`; is in `<records>`; is a `<settings>` or a `<profile>`; or is a `.claude/` settings file (a `.json` whose name holds `settings`: `settings.json`, `settings.local.json`, `controller-settings.json`, `conductor-settings.json`) inside them | the target is a `<settings>`, a `<profile>` or a `.claude/` settings file, or is not one of conductor's records |
//! | SendMessage | the recipient is anyone but `conductor` or `conductor [<ref>]` ([`conductor_name`]); an agent the session started, by its id ([`is_own_agent_id`]); or a socket address `uds:<path>/<pid>.sock` whose pid, where the path lands, the session list ([`SessionList`]) gives a live session named `conductor` | never |
//! | Bash | a `cd`, `pushd` or `git -C` (`--git-dir`, `--work-tree`) reaches another repository's checkout or managed worktrees, or `<records>`; a `gh` call that is not a read ([`github_write`]); the command names a `.claude/` settings file or a `<settings>` and holds a write form ([`Places::writes_settings`]); the command names a `<profile>` and holds a write form ([`Places::writes_profile`]); the command names `<config>` or `<config-dir>` and holds a write form ([`Places::names_config`], [`holds_write_form`]); or it runs a `conductor` leaf that is not one of [`READ_LEAVES`] ([`conductor_write`]) | a `gh` call that is not a read; the command names a `.claude/` settings file or a `<settings>` and holds a write form; the command names a `<profile>` and holds a write form |
//!
//! The Bash row is a heuristic over the command line (design § 7: it catches the common forms,
//! not all). Its accepted limits are pinned by `tests/adv_guard.rs`. The two write-form tests are
//! plain substring tests, not parses: `cd .claude && … > settings.json` and
//! `cd <config-dir> && cp x conductor.yaml` are not caught, nor is a config file written by a
//! relative path from the cwd.
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
use conductor_model::config::Instance;
use conductor_model::dispatch::Verdict;
use serde_json::Value;

use super::shell;
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
/// controller messages.
pub(super) const CONDUCTOR: &str = "conductor";

/// Conductor's records: directories, under its records.
const RECORD_DIRS: [&str; 4] = ["decisions", "dispatches", "charters", "docs/handoff"];

/// Conductor's records: files, under its records.
const RECORD_FILES: [&str; 3] = ["goals.jsonl", "STATUS.md", "NORTHSTAR.md"];

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
}

impl Places {
    /// The built-in instance's places ([`config::built_in`]) under `home`, which apply without a
    /// config file: conductor's session is the one in the repository named `conductor`, and its
    /// records are under that repository's checkout.
    #[must_use]
    pub fn built_in(home: &Path) -> Self {
        Self::of(home, &config::built_in(home, home), false)
    }

    /// The places of `active`, `home` being the home directory: with a config file, its
    /// `checkouts`, and conductor's session is the one whose cwd is in its `records`, where its
    /// records are; without one, [`Places::built_in`].
    #[must_use]
    pub fn active(home: &Path, active: &Active) -> Self {
        Self::of(home, &active.instance, active.from_file)
    }

    fn of(home: &Path, instance: &Instance, from_file: bool) -> Self {
        Self {
            home: home.to_path_buf(),
            checkouts: PathBuf::from(&instance.checkouts.root),
            trees: PathBuf::from(&instance.checkouts.trees),
            records: from_file.then(|| PathBuf::from(&instance.records)),
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
        }
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

    /// The config directory, `<config-dir>`: the directory of [`config::DEFAULT_FILE`] under the
    /// home directory, which also holds the instances' default records and state.
    fn config_dir(&self) -> Option<PathBuf> {
        config_dir_in_home().map(|dir| self.home.join(dir))
    }

    /// Whether the absolute `path` is conductor's config: the config file, or `<config-dir>` or
    /// anything under it, each as written ([`lexical`]) or where it lands ([`real`]).
    fn is_config(&self, path: &Path) -> bool {
        path == lexical(&self.config)
            || path == real(&self.config)
            || self
                .config_dir()
                .is_some_and(|dir| path.starts_with(lexical(&dir)) || path.starts_with(real(&dir)))
    }

    /// Whether the Bash `command` names conductor's config, by a plain substring test: the config
    /// directory as the home directory leaves it (`<config-dir>` without `~/`), which every
    /// spelling of it or of a file under it holds; `$CONDUCTOR_CONFIG` or `${CONDUCTOR_CONFIG`;
    /// or the config directory or the config file as written or where it lands, as an absolute
    /// path or under the home directory as `~/…`, `$HOME/…` or `${HOME}/…`.
    fn names_config(&self, command: &str) -> bool {
        let variable = config::CONFIG_VARIABLE;
        let mut spellings = vec![format!("${variable}"), format!("${{{variable}")];
        spellings.extend(config_dir_in_home().map(|dir| dir.display().to_string()));
        let paths = self.config_dir().into_iter().chain([self.config.clone()]);
        for path in paths {
            spellings.extend(self.spellings(&path));
        }
        spellings
            .iter()
            .any(|spelling| command.contains(spelling.as_str()))
    }

    /// The ways a command line writes the absolute `path`: as written or where it lands, as an
    /// absolute path or under the home directory as `~/…`, `$HOME/…` or `${HOME}/…`.
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

    /// Whether the Bash `command` writes a settings file that wires the guard: it names a
    /// `.claude/` settings file ([`names_claude_settings`]) or, by any of its [`Places::spellings`],
    /// the `settings` file of a role of the instance, and holds a write form
    /// ([`holds_write_form`]). A plain substring test, not a parse: `cd .claude && … >
    /// settings.json` is not caught, nor is a role's file written by a path relative to the cwd.
    fn writes_settings(&self, command: &str) -> bool {
        let named = names_claude_settings(command)
            || self.settings.iter().any(|settings| {
                self.spellings(settings)
                    .iter()
                    .any(|spelling| command.contains(spelling.as_str()))
            });
        named && holds_write_form(command)
    }

    /// Whether the absolute `path` is the `profile` file of a role of the instance, as written
    /// ([`lexical`]) or where it lands ([`real`]).
    fn is_role_profile(&self, path: &Path) -> bool {
        self.profiles
            .iter()
            .any(|profile| path == lexical(profile) || path == real(profile))
    }

    /// Whether the Bash `command` writes the `profile` file of a role of the instance: it names one
    /// by any of its [`Places::spellings`] and holds a write form ([`holds_write_form`]). A plain
    /// substring test, as [`Places::writes_settings`] is: a profile written by a path relative to
    /// the cwd is not caught.
    fn writes_profile(&self, command: &str) -> bool {
        self.profiles.iter().any(|profile| {
            self.spellings(profile)
                .iter()
                .any(|spelling| command.contains(spelling.as_str()))
        }) && holds_write_form(command)
    }

    /// Conductor's config, for a denial to name: the config file, and `<config-dir>`.
    fn config_described(&self) -> String {
        match self.config_dir() {
            Some(dir) => format!(
                "{}, or anything under {}/",
                self.shown(&self.config),
                self.shown(&dir)
            ),
            None => self.shown(&self.config),
        }
    }
}

/// The `gh` verbs that read, after any command group; everything else from `gh` is a GitHub
/// write, apart from the other reads [`github_write`] names.
const GH_READ_VERBS: [&str; 7] = [
    "view", "list", "status", "diff", "checks", "search", "download",
];

/// The verbs of `gh run` that read.
const GH_RUN_READ_VERBS: [&str; 4] = ["view", "list", "watch", "download"];

/// Commands that name `gh` without running it, or run it through the bot route, which this rule
/// leaves alone.
const GH_NOT_RUN: [&str; 6] = ["b10x-gates", "which", "type", "whereis", "man", "hash"];

/// The write forms that, in a controller's Bash command that names `.claude/settings` or
/// conductor's config, deny it: as substrings.
const WRITE_SUBSTRINGS: [&str; 8] = [
    ">",
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

/// The groups of the `conductor` command tree (`src/cli.rs`): a `conductor` word followed by one
/// of them is a run of the binary. `tests/guard_rules.rs` holds this list to the tree.
const GROUPS: [&str; 12] = [
    "snapshot",
    "goal",
    "repository",
    "decision",
    "controller",
    "dispatch",
    "message",
    "resource",
    "guard",
    "config",
    "dashboard",
    "watch",
];

/// The global options of the `conductor` command tree that take a value (`src/cli.rs`), so that
/// the value is not read as a group or a leaf. `tests/guard_rules.rs` runs each leaf with every
/// global option of the tree that takes one.
const GLOBAL_VALUES: [&str; 2] = ["--state-dir", "--config"];

/// The `conductor` leaves a controller's session may run, as `(group, leaf)`: every view,
/// `decision show`, and the guard hook's own `guard record-guard-decision`. Every other leaf
/// writes conductor's records, and only conductor runs it (wave 06 U7 brief, decision 1). The
/// generated model does not say which leaves are views, so this is a list, and
/// `tests/guard_rules.rs` holds it to the command tree: every view is in it, and no command but
/// the hook's is.
const READ_LEAVES: [(&str, &str); 24] = [
    ("snapshot", "blockers"),
    ("snapshot", "boards"),
    ("snapshot", "pull-requests"),
    ("snapshot", "repositories"),
    ("snapshot", "sessions"),
    ("snapshot", "snapshots"),
    ("snapshot", "specifications"),
    ("snapshot", "workflow-runs"),
    ("snapshot", "releases"),
    ("snapshot", "merged-pull-requests"),
    ("goal", "servings"),
    ("goal", "goals"),
    ("repository", "repository-marks"),
    ("repository", "activity"),
    ("decision", "decisions"),
    ("decision", "hands-todo"),
    ("decision", "requests"),
    ("decision", "show"),
    ("controller", "controllers"),
    ("dispatch", "dispatches"),
    ("message", "messages"),
    ("resource", "resource-requests"),
    ("guard", "guard-decisions"),
    ("guard", "record-guard-decision"),
];

/// Which rule set decides a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rules {
    /// A repository controller.
    Controller,
    /// Conductor itself.
    Conductor,
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
    /// for every session, or for a controller also under `~/.cache/<repo>-<anything>/`, `<repo>`
    /// its repository with `/` written `-`. Never a repository's checkout or managed tree,
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
        let cache = real(&self.home().join(".cache"));
        let Ok(rest) = landing.strip_prefix(&cache) else {
            return false;
        };
        let prefix = format!("{}-", self.repository.replace('/', "-"));
        let mut steps = rest.components();
        steps
            .next()
            .is_some_and(|first| first.as_os_str().to_string_lossy().starts_with(&prefix))
            && steps.next().is_some()
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
                            "{tool} target{resolves} is scratch: under $TMPDIR or \
                             ~/.cache/{}-*/",
                            self.repository.replace('/', "-")
                        ),
                    )
                } else if inside && landing.starts_with(places.records()) {
                    // Only a config file can put conductor's records in a controller's checkout or
                    // worktrees; they stay conductor's.
                    (
                        Verdict::Deny,
                        format!(
                            "{tool} target {target}{resolves} is in conductor's records {}/: a \
                             controller does not write them",
                            places.shown(&places.records_dir())
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
                             {checkout}/ and its managed worktrees {}/",
                            places.shown(&places.trees.join(&self.repository))
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
    /// name the same session: in either form of conductor's name ([`conductor_name`]), otherwise
    /// as `to` does.
    pub(super) fn message(&self, to: Option<&str>, recipient: Option<&str>) -> (Verdict, String) {
        if self.rules == Rules::Conductor {
            return (
                Verdict::Allow,
                "conductor's SendMessage is not restricted".to_owned(),
            );
        }
        let Some(to) = to else {
            return (
                Verdict::Deny,
                "SendMessage names no recipient: a controller messages only conductor".to_owned(),
            );
        };
        let denied = |why: &str| {
            (
                Verdict::Deny,
                format!("SendMessage to {to:?}{why}: a controller messages only conductor"),
            )
        };
        if let Some(reference) = conductor_name(to) {
            let same = |other: Option<&str>| match (reference, other) {
                (Some(reference), Some(other)) => reference == other,
                _ => true,
            };
            return match recipient {
                Some(recipient) if !conductor_name(recipient).is_some_and(same) => {
                    denied(&format!(" with recipient {recipient:?}"))
                }
                _ => (
                    Verdict::Allow,
                    format!("SendMessage to {to:?}: a session named conductor"),
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
            Some(path) => self.socket(to, path),
            None => denied(""),
        }
    }

    /// SendMessage to the socket address `to`, `uds:` and `path`: allowed when `path` lands on
    /// `<pid>.sock` and the session list holds a live session named `conductor` with that pid.
    fn socket(&self, to: &str, path: &str) -> (Verdict, String) {
        let denied = |why: String| {
            (
                Verdict::Deny,
                format!("SendMessage to {to:?}{why}: a controller messages only conductor"),
            )
        };
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
        match live_pids(self.sessions, CONDUCTOR) {
            Ok(pids) if pids.contains(&pid) => (
                Verdict::Allow,
                format!(
                    "SendMessage to {to:?}{resolves}: the live session named conductor has pid {pid}"
                ),
            ),
            Ok(_) => denied(format!(
                "{resolves}: no live session named conductor has pid {pid} in the session list \
                 `{shown}`"
            )),
            Err(error) => denied(format!(
                "{resolves}: the session list could not be read, so pid {pid} is not known to be \
                 conductor's: {error:#}"
            )),
        }
    }

    /// Bash: the verdict on running `command`.
    pub(super) fn bash(&self, command: &str) -> (Verdict, String) {
        let commands = shell::commands(command);
        for words in &commands {
            if let Some(write) = github_write(words) {
                return (
                    Verdict::Deny,
                    format!(
                        "`{write}` is a GitHub write: gh here only reads ({} after any group; \
                         run {}; api GET; auth status; help; --help); writes go through the bot \
                         route",
                        GH_READ_VERBS.join(", "),
                        GH_RUN_READ_VERBS.join(", ")
                    ),
                );
            }
        }
        if self.places.writes_settings(command) {
            return (
                Verdict::Deny,
                "the command names a .claude/ settings file or a role's settings file and holds a \
                 write form: no session writes Claude Code's project settings or the settings \
                 that wire the guard, which can switch it off"
                    .to_owned(),
            );
        }
        if self.places.writes_profile(command) {
            return (
                Verdict::Deny,
                "the command names a role's profile file and holds a write form: no session \
                 writes the system prompt a role's sessions start with"
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
            && let Some(leaf) = commands.iter().find_map(|words| conductor_write(words))
        {
            return (
                Verdict::Deny,
                format!("`conductor {leaf}` writes conductor's records; only conductor runs it"),
            );
        }
        if self.rules == Rules::Conductor {
            return (Verdict::Allow, "no GitHub write".to_owned());
        }
        let mut here = self.cwd.clone();
        for words in &commands {
            let words = command_words(words);
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
                        return (Verdict::Deny, format!("`{shown}` reaches {place}"));
                    }
                    here = target;
                }
            }
            for (at, word) in words.iter().enumerate() {
                if shell::program(word) != "git" {
                    continue;
                }
                if let Some((shown, place)) = self.git_reach(&words[at..], &here) {
                    return (Verdict::Deny, format!("`{shown}` reaches {place}"));
                }
            }
        }
        (
            Verdict::Allow,
            "no cd or git -C into another repository and no GitHub write".to_owned(),
        )
    }

    /// The place ([`Session::elsewhere`]) a `git` command's global options point it at, if any:
    /// `-C`, `--git-dir` and `--work-tree`, each taken from where the one before it left off.
    fn git_reach(&self, words: &[String], here: &Path) -> Option<(String, String)> {
        let mut base = here.to_path_buf();
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
                    _ => return None,
                },
            };
            at += used;
            let value = value?;
            let Some(target) = self.word_path(value, &base) else {
                continue;
            };
            if let Some(place) = self.elsewhere(&target) {
                return Some((format!("git {flag} {value}"), place));
            }
            if flag == "-C" {
                base = target;
            }
        }
        None
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

/// The `conductor` leaf a simple command runs that is not one of [`READ_LEAVES`], as
/// `<group> <leaf>`. A run is a word naming the `conductor` program, wherever it stands (after
/// `env`, `timeout` or a path), followed by a group of [`GROUPS`] and a leaf: the first two words
/// after it that are not flags, each of [`GLOBAL_VALUES`] taking its value. A `conductor` word that
/// no group and leaf follow runs none (`which conductor`, `conductor --help`, a path ending in
/// `conductor`).
fn conductor_write(words: &[String]) -> Option<String> {
    words
        .iter()
        .enumerate()
        .filter(|(_, word)| shell::program(word) == "conductor")
        .find_map(|(at, _)| {
            let mut operands = Vec::new();
            let mut rest = words[at + 1..].iter();
            while operands.len() < 2
                && let Some(word) = rest.next()
            {
                if GLOBAL_VALUES.contains(&word.as_str()) {
                    rest.next();
                } else if !(word.starts_with('-') && word.len() > 1) {
                    operands.push(word.as_str());
                }
            }
            match operands[..] {
                [group, leaf]
                    if GROUPS.contains(&group) && !READ_LEAVES.contains(&(group, leaf)) =>
                {
                    Some(format!("{group} {leaf}"))
                }
                _ => None,
            }
        })
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

/// Whether `command` holds a write form: one of [`WRITE_SUBSTRINGS`], or one of [`WRITE_WORDS`]
/// as a word.
fn holds_write_form(command: &str) -> bool {
    WRITE_SUBSTRINGS.iter().any(|form| command.contains(form))
        || WRITE_WORDS.iter().any(|form| contains_word(command, form))
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

/// The GitHub write a simple command makes through `gh`: any `gh` call that is not a read, as
/// `gh <group> <verb>`. A read is `<group> view|list|status|diff|checks|search|download`,
/// `run view|list|watch|download`, `api` with GET and no request fields, `api graphql` without a
/// mutation, `auth status`, `search …`, `status`, `help`, any call carrying `--help`, or
/// `--version`/`-h` with no group: with one, `-h` may be another flag (`gh auth login -h
/// github.com` is `--hostname`). The group and the verb are the first two words after `gh` that
/// are not flags, `-R`/`--repo` taking its value.
/// `gh` named by [`GH_NOT_RUN`] before it, or by `command -v`, is not run here.
fn github_write(words: &[String]) -> Option<String> {
    let at = words.iter().position(|word| shell::program(word) == "gh")?;
    let before = &words[..at];
    if before
        .iter()
        .any(|word| GH_NOT_RUN.contains(&shell::program(word)))
        || (before.iter().any(|word| word == "command")
            && before.iter().any(|word| word == "-v" || word == "-V"))
    {
        return None;
    }
    let rest = &words[at + 1..];
    let mut operands = Vec::new();
    let mut flags = Vec::new();
    let mut next = 0;
    while let Some(word) = rest.get(next) {
        next += 1;
        if word == "-R" || word == "--repo" {
            next += 1;
        } else if word.starts_with('-') && word.len() > 1 {
            flags.push(word.as_str());
        } else {
            operands.push((next - 1, word.as_str()));
        }
    }
    let group = operands.first().map(|(_, word)| *word);
    let verb = operands.get(1).map(|(_, word)| *word);
    let read = flags.contains(&"--help")
        || match (group, verb) {
            (None, _) => flags.iter().any(|flag| matches!(*flag, "--version" | "-h")),
            (Some("help" | "search"), _)
            | (Some("status"), None)
            | (Some("auth"), Some("status")) => true,
            (Some("api"), _) => api_reads(&rest[operands[0].0 + 1..]),
            (Some("run"), Some(verb)) => GH_RUN_READ_VERBS.contains(&verb),
            (Some(_), Some(verb)) => GH_READ_VERBS.contains(&verb),
            (Some(_), None) => false,
        };
    (!read).then(|| {
        let named: Vec<&str> = operands.iter().take(2).map(|(_, word)| *word).collect();
        format!("gh {}", named.join(" ")).trim_end().to_owned()
    })
}

/// Whether the arguments of `gh api` make a read: GET (no method or GET named, by `-X`,
/// `--method` or a flag cluster holding `X`) and no request field (`-f`, `-F`, `--field`,
/// `--raw-field`, `--input`), or `graphql` without a mutation. A query from a file
/// (`=@…`) or `--input` may be a mutation, and is taken as one.
fn api_reads(arguments: &[String]) -> bool {
    /// Flags of `gh api` that take the next word as their value.
    const VALUED: [&str; 10] = [
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
            _ if VALUED.contains(&word) => next += 1,
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
                        'f' | 'F' | 'H' | 'q' | 't' | 'p' => {
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
    if endpoint == Some("graphql") {
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

/// Whether `to` names a session called conductor, and by which ref: `Some(None)` for
/// `conductor`, `Some(Some(ref))` for `conductor [<ref>]`, the ref being one or more lower-case
/// hexadecimal characters as the session list prints it (wave 07 U8 brief, decision 1: the harness
/// refuses a ref that does not belong to a session of that name).
fn conductor_name(to: &str) -> Option<Option<&str>> {
    if to == CONDUCTOR {
        return Some(None);
    }
    let reference = to
        .strip_prefix(CONDUCTOR)?
        .strip_prefix(" [")?
        .strip_suffix(']')?;
    let hexadecimal = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    (!reference.is_empty() && reference.bytes().all(hexadecimal)).then_some(Some(reference))
}

/// The pid a socket path names: its file name is `<pid>.sock`, the pid written in decimal with
/// no sign and no leading zero.
fn socket_pid(path: &Path) -> Option<u64> {
    let stem = path.file_name()?.to_str()?.strip_suffix(".sock")?;
    let pid: u64 = stem.parse().ok()?;
    (pid.to_string() == stem).then_some(pid)
}

/// The pids of the live sessions named `name` in the session list: its entries with that `name`
/// and a `pid`. The list is read once, through [`collect::run`] under its bound.
fn live_pids(sessions: &SessionList, name: &str) -> Result<Vec<u64>> {
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
        .filter(|entry| entry["name"] == name)
        .filter_map(|entry| entry["pid"].as_u64())
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
