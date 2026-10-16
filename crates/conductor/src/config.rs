//! The config file (`story:config-file`): one file, version `conductor.config/1`, names
//! conductor's instances, and `conductor config validate` and `conductor config show` read it.
//!
//! The file is found at `--config`, else at the path in `CONDUCTOR_CONFIG`, else at
//! `~/.b10x/conductor/conductor.yaml`. The two variables are the `conductor-config` and
//! `conductor-instance` settings the `conductor-cli` component declares in `spec/components.yaml`.
//! A file `--config` or `CONDUCTOR_CONFIG` names must be there; when the default file is not,
//! the built-in defaults apply ([`built_in`]), and they equal today's constants: nothing changes
//! until a file exists. The instance is `--instance`, else `CONDUCTOR_INSTANCE`, else the instance
//! whose tree holds the working directory ([`by_directory`], `story:instance-isolation`), else the
//! file's `default` (`story:grouped-instance`), else the only one, else an error naming the
//! instances ([`select`]). What the instances of the file share is in the host directory beside it
//! ([`host_dir`]), and a role's settings file that pins another instance is refused
//! ([`pin_refusal`]); one that pins none starts pinned ([`session_settings`]).
//!
//! The file is read into `serde_json::Value` and converted into the generated
//! `conductor.config` types ([`parse`]), which do not derive `serde`; what the types cannot say
//! is checked on the way, and every problem is named by its YAML path, such as
//! `instances[0].cadence.watch`:
//! - a key the specification does not declare is an error;
//! - the specification's three invariants of `conductor.config.Config`: the version is
//!   `conductor.config/1`, no two instances share a name, and `default` names an instance;
//! - a source is one of `github`, `local` and `gitlab`; a `local` or `gitlab` source may carry
//!   `exclude`, each entry a relative path under its root without `.` or `..` steps, and a
//!   `github` source carries none; only a `gitlab` source carries a `host`, a host name without
//!   a scheme or a path, `gitlab.com` when absent (`story:gitlab-source-host`);
//! - a duration is a whole number above 0 with `s`, `m`, `h` or `d` (held as the ISO 8601
//!   duration the generated `Duration` carries), a size a whole number with `G` (GiB), a count of
//!   tokens a whole number or thousands with `k`, a time of day `HH:MM`;
//! - a harness is `claude` or `codex`, an activity `active` or `inactive`, a decider `conductor`
//!   or `operator`, a catalog's `names` `plain` or `hex`;
//! - a `catalog` (`story:catalog-source`) names its `repository`, a path under the checkouts
//!   root, and its `path`, a directory relative to that repository's top, each without `.` or
//!   `..` steps;
//! - each of `secret_syncs` (`story:policy-secret-drift`) names its `name`, no two alike, its
//!   `source` (a `repository` under the checkouts root and a file `path` of it, each without `.`
//!   or `..` steps, and a `branch`, `main` when absent), one or more `secrets` (each an `owner`,
//!   a `name` and, for a repository's secret, its `repository`), and its `command`, and may name
//!   a `dry_run`;
//! - the most controllers working at once and the most sub-agents one controller runs at once are
//!   each a whole number of 1 or more (`Controllers`'s invariants), and so is the number of
//!   snapshots whose observations are kept (`Retention`'s);
//! - the free disk from which the full-gate watchdog resumes a gate lies above the one under
//!   which it stops it (`Thresholds`'s invariant `gate_resume > gate_stop`);
//! - a path (a role's `settings` and `profile` and `thresholds.disk_path` among them) is absolute
//!   or under `~`, which expands to the home directory;
//! - a `session_prefix` (`story:instance-session-names`) is a `conductor.config.SessionPrefix`:
//!   letters, digits and `-`, starting and ending with a letter or digit; no two instances share
//!   one, at most one instance has none, no prefix starts with another followed by `-`, which
//!   would let one session name carry both, and no prefix followed by `-` begins a fixed session
//!   name an instance without a prefix keeps (`conductor-dev`). A role's `session_name` is derived from it
//!   ([`session_name`]), never chosen: the file may write only that value;
//! - a `dashboard_port` (`story:instance-scoped-tasks`) is a whole number from 1 to 65535
//!   (`Instance`'s invariants), and no two instances share one, counting the ports filled for the
//!   instances that leave it out ([`dashboard_port`]), so each instance's dashboard binds its own;
//! - a role's `model`, `agent`, `settings` and `profile` are each a `conductor.config.CommandWord`:
//!   letters, digits and `.`, `_`, `:`, `/`, `-` only, the alphabet the specification declares,
//!   because a start command passes each to the harness as one shell word;
//! - the instance's `confinement` (`story:session-envelope-model`) has a `mode`, `strict` or
//!   `report`, and a `provider`, `systemd` or `none`; its `limits` and each role's are a
//!   `memory_max` size of 1G or more, a `cpu_weight` from 1 to 10000 and a `tasks_max` of 1 or
//!   more (`Limits`'s invariants), and a role's limit lies at or under the instance's of the same
//!   key, where the instance sets one; a role's `writable` entry is a place's word
//!   ([`PLACES`]) or a path, and its `network` is `open`;
//! - `usage.probe` (`story:usage-probe`) is a command, not empty, and `usage.watcher` a systemd
//!   unit name ([`unit_name`]);
//! - the file holds no secret: a value that looks like a token (a word starting `ghp_`,
//!   `github_pat_`, `glpat-` or `xox<letter>-`) is refused by its path, and never printed.
//!
//! An instance may leave out every key but `name`, `sources` and `checkouts`. Absent, `records`,
//! `state` and `cache` are `~/.b10x/conductor/<name>/records`, `~/.b10x/conductor/<name>/state`
//! and `~/.cache/b10x/conductor/<name>`; `roles`, `controllers`, `cadence`, `thresholds`,
//! `retention` and `authority`, and each key of the last five, are the built-in defaults;
//! `dashboard_port` is 7313 plus the instance's index in the file ([`dashboard_port`]);
//! `repositories`, `reports` and `secret_syncs` are empty; `operator`, `catalog`, `gates_policy`,
//! `usage.probe`, `usage.watcher`, `session_prefix`, and a role's
//! `agent`, `settings` and `profile`, are absent, and `config show` writes an absent one as null;
//! a catalog's `names` is `plain`. The built-in instance names no catalog and no session prefix.
//! `confinement` is `report` through `systemd` with no limit; a role sets no limit, writes its
//! default places ([`default_writable`]), and has the network `open`.
//!
//! `conductor config workspace` (`story:workspace-from-config`) writes the instance's workspace
//! file from it ([`workspace_members`], [`workspace`]).
//!
//! Nothing else reads the config yet: the global `--config` is parsed on every command and read
//! only here.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fmt::{self, Write as _};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::OnceLock;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{
    Authority, Cadence, Catalog, CatalogNames, Checkouts, CommandWord, ConductorSession, Config,
    Confinement, ConfinementMode, ConfinementProvider, Controllers, ForgeSecret, Gibibytes,
    GitHubSource, GitLabSource, Harness, Instance, InstanceName, Limits, LocalSource, Network,
    Report, RepositoryRule, Retention, Role, SecretSource, SecretSync, SessionName, SessionPrefix,
    Source, Thresholds, TimeOfDay, Tokens, Usage, WorkspaceMember, Writable,
};
use conductor_model::decision::Decider;
use conductor_model::direction::Activity;
use conductor_model::primitives::Duration;
use serde_json::{Map, Value};
use serde_yaml::{Mapping, Value as Yaml};

use crate::cli::{ShowConfigArgs, ValidateConfigArgs, WriteWorkspaceArgs};

/// The placeholder GitHub owner of the built-in instance, which applies without a config file,
/// and the directory under the home directory that holds its checkouts.
pub const ORGANIZATION: &str = "example-org";

/// The version every config file carries: `conductor.config.Config`'s first invariant.
pub const VERSION: &str = "conductor.config/1";

/// The variable of the `conductor-config` setting: the config file, when no `--config` names one.
pub const CONFIG_VARIABLE: &str = "CONDUCTOR_CONFIG";

/// The variable of the `conductor-instance` setting: the instance, when no `--instance` names one.
pub const INSTANCE_VARIABLE: &str = "CONDUCTOR_INSTANCE";

/// The config file under the home directory, when neither `--config` nor [`CONFIG_VARIABLE`]
/// names one.
pub const DEFAULT_FILE: &str = ".b10x/conductor/conductor.yaml";

/// The host of a `gitlab` source that writes none (`story:gitlab-source-host`).
pub const GITLAB_HOST: &str = "gitlab.com";

/// The session roles and the model each runs on today: `claude … --model opus` in `Taskfile.yml`
/// (`conductor`, `conductor-dev`) and in `.agents/conductor.md` (a controller).
const ROLES: [&str; 3] = ["conductor", "conductor-dev", "controller"];

/// The role of conductor's own session.
const CONDUCTOR_ROLE: &str = "conductor";

/// The role whose sessions are named per repository, `<session_prefix>-<repository>`, not after
/// the role (`story:instance-session-names`).
pub const CONTROLLER: &str = "controller";

/// The characters of a `conductor.config.SessionPrefix`, the alphabet `spec/domains/config.yaml`
/// declares for it.
const SESSION_PREFIX: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-";

/// The model every role runs on today.
const MODEL: &str = "opus";

/// The characters of a `conductor.config.CommandWord`, the alphabet `spec/domains/config.yaml`
/// declares for it: a role's `model`, `agent`, `settings` and `profile` hold only these.
const COMMAND_WORD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._:/-";

/// Hours between two cycles: the job `17 */2 * * *` of `.agents/conductor.md`.
const CYCLE_HOURS: u64 = 2;

/// The daily cycle, UTC: the job `7 8 * * *` of `.agents/conductor.md`.
const DAILY: &str = "08:07";

/// Free GiB under which no session starts (`.agents/conductor.md`, Dispatch: "under
/// `thresholds.disk_admit`, start nothing, grant nothing").
const DISK_ADMIT_GIB: i64 = 20;

/// Free GiB under which a build expected to write more than [`BUILD_SIZE_GIB`] takes a build
/// slot: the line conductor grants slots by today (`.agents/conductor.md`, Dispatch). It lies
/// above `below-disk-floor` of `conductor.dispatch.GrantResource`, 32212254720 bytes (30 GiB).
const BUILD_SLOT_GIB: i64 = 60;

/// GiB a build is expected to write above which it asks for a build slot.
const BUILD_SIZE_GIB: i64 = 10;

/// Free GiB under which the full-gate watchdog stops the gate's process group.
const GATE_STOP_GIB: i64 = 15;

/// Free GiB from which the full-gate watchdog resumes a stopped gate.
const GATE_RESUME_GIB: i64 = 17;
/// The cores `build run` leaves to the desktop (`thresholds.cpu_reserve`).
const CPU_RESERVE: i64 = 2;

/// Seconds between two reads of free disk by the full-gate watchdog.
const WATCHDOG_EVERY: u64 = 10;

/// The filesystem whose free space the disk thresholds read.
const DISK_PATH: &str = "/";

/// The managed worktrees under the home directory (`WORKTREES` of `src/guard/rules.rs`).
const WORKTREES: &str = ".local/state/worktree/trees/example-org";

/// The watch cache under the home directory that the dashboard reads (`src/dashboard.rs`).
const CACHE: &str = ".cache/conductor-watch";

/// The state directory under the working directory (`DEFAULT` of `src/state.rs`).
const STATE: &str = "state";

/// The most controllers of an instance working at once, when the file does not say: the
/// default is 5.
const MAX_WORKING: i64 = 5;

/// The most sub-agents one controller runs at once, when the file does not say.
const MAX_SUBAGENTS: i64 = 4;

/// The most pull requests of one repository that wait for review before conductor sends that
/// repository no new dispatch, when the file does not say (`story:review-limit`); `0` is no limit.
pub const MAX_OPEN_REVIEWS: i64 = 3;

/// The complete snapshots whose observations are kept, when the file does not say: a day at the
/// 2-hour cadence (`story:observation-retention`).
pub const RETENTION_SNAPSHOTS: i64 = 12;

/// Seconds from the end of one pass of `conductor watch run` to the start of the next.
pub const WATCH_EVERY: u64 = 180;

/// Seconds between two reads of `main`'s CI by `conductor watch run`.
pub const CI_EVERY: u64 = 900;

/// Tokens of a session's context above which `conductor watch run` reports it.
pub const CONTEXT_HANDOVER: u64 = 300_000;

/// Free GiB on `/` under which `conductor watch run` reports it: the default keeps 100G free.
pub const DISK_LOW_GIB: u64 = 100;

/// Free GiB on `/` from which a further fall is reported again.
pub const DISK_CLEAR_GIB: u64 = 110;

/// Minutes a live controller session, and each of its sub-agents, may write nothing to its
/// transcript before `conductor watch run` reports it silent (`story:watch-silent-controller`).
pub const SILENT_SESSION_MINUTES: u64 = 30;

/// The keys of each mapping the file holds, in the order `config show` writes them.
const CONFIG_KEYS: [&str; 3] = ["version", "default", "instances"];
const INSTANCE_KEYS: [&str; 23] = [
    "name",
    "session_prefix",
    "dashboard_port",
    "sources",
    "checkouts",
    "records",
    "state",
    "cache",
    "roles",
    "confinement",
    "controllers",
    "repositories",
    "cadence",
    "thresholds",
    "retention",
    "reports",
    "authority",
    "usage",
    "operator",
    "catalog",
    "secret_syncs",
    "gates_policy",
    "conductor",
];
const CONDUCTOR_KEYS: [&str; 2] = ["served_by", "session_name"];
/// The keys that name a source's kind; a source names exactly one.
const SOURCE_KINDS: [&str; 3] = ["github", "local", "gitlab"];
const SOURCE_KEYS: [&str; 5] = ["github", "local", "gitlab", "exclude", "host"];
const CHECKOUT_KEYS: [&str; 2] = ["root", "trees"];
const ROLE_KEYS: [&str; 12] = [
    "role",
    "harness",
    "model",
    "agent",
    "settings",
    "profile",
    "session_name",
    "read_only_tools",
    "limits",
    "writable",
    "network",
    "forge_token_env",
];
const CONFINEMENT_KEYS: [&str; 3] = ["mode", "provider", "limits"];
const LIMIT_KEYS: [&str; 3] = ["memory_max", "cpu_weight", "tasks_max"];
const CONTROLLER_KEYS: [&str; 3] = ["max_working", "max_subagents", "max_open_reviews"];
const REPOSITORY_KEYS: [&str; 2] = ["match", "activity"];
const CADENCE_KEYS: [&str; 4] = ["cycle", "daily", "watch", "ci"];
const THRESHOLD_KEYS: [&str; 12] = [
    "context_handover",
    "disk_path",
    "disk_low",
    "disk_clear",
    "disk_admit",
    "build_slot",
    "build_size",
    "gate_stop",
    "gate_resume",
    "watchdog_every",
    "silent_session",
    "cpu_reserve",
];
const RETENTION_KEYS: [&str; 1] = ["snapshots"];
const REPORT_KEYS: [&str; 3] = ["channel", "as", "when"];
const AUTHORITY_KEYS: [&str; 2] = ["class_c", "class_o"];
const CATALOG_KEYS: [&str; 3] = ["repository", "path", "names"];
const SECRET_SYNC_KEYS: [&str; 5] = ["name", "source", "secrets", "command", "dry_run"];
const SECRET_SOURCE_KEYS: [&str; 3] = ["repository", "branch", "path"];
const FORGE_SECRET_KEYS: [&str; 3] = ["owner", "repository", "name"];
/// A secret sync's `source.branch` when the file leaves it out (`story:policy-secret-drift`).
const SECRET_SOURCE_BRANCH: &str = "main";

const USAGE_KEYS: [&str; 2] = ["probe", "watcher"];

/// The suffixes of a systemd unit name: what `usage.watcher` ends with (`story:usage-probe`).
const UNIT_TYPES: [&str; 11] = [
    "service",
    "socket",
    "target",
    "timer",
    "path",
    "scope",
    "slice",
    "mount",
    "automount",
    "swap",
    "device",
];

/// How the file writes each `conductor.config.CatalogNames`.
const CATALOG_NAMES: [(&str, CatalogNames); 2] =
    [("plain", CatalogNames::Plain), ("hex", CatalogNames::Hex)];

/// How the file writes each `conductor.config.ConfinementMode`.
const MODES: [(&str, ConfinementMode); 2] = [
    ("strict", ConfinementMode::Strict),
    ("report", ConfinementMode::Report),
];

/// How the file writes each `conductor.config.ConfinementProvider`.
const PROVIDERS: [(&str, ConfinementProvider); 2] = [
    ("systemd", ConfinementProvider::Systemd),
    ("none", ConfinementProvider::None),
];

/// How the file writes each `conductor.config.Network`.
const NETWORKS: [(&str, Network); 1] = [("open", Network::Open)];

/// The words a role's `writable` may name besides a path (`conductor.config.Writable`), each a
/// place resolved per session when it starts: the session's checkout, its managed trees, the
/// build cache, scratch, the harness's own state directory, and the instance's records.
pub const PLACES: [&str; 6] = [
    "checkout",
    "trees",
    "build-cache",
    "scratch",
    "harness-state",
    RECORDS_PLACE,
];

/// The place of the instance's records directory, which the `conductor` role writes by default.
const RECORDS_PLACE: &str = "records";

/// The weights a `cpu_weight` may name, systemd's `CPUWeight=` range: `conductor.config.Limits`'s
/// invariants `cpu_weight >= 1` and `cpu_weight <= 10000`.
const CPU_WEIGHTS: std::ops::RangeInclusive<i64> = 1..=10_000;

/// The ports a `dashboard_port` may name: `conductor.config.Instance`'s invariants
/// `dashboard_port >= 1` and `dashboard_port <= 65535`.
const PORTS: std::ops::RangeInclusive<i64> = 1..=65535;

/// The beginnings of a word that looks like a token; `xox` takes a letter and `-` after it.
const TOKEN_PREFIXES: [&str; 3] = ["ghp_", "github_pat_", "glpat-"];

/// What the process gives the config: the home directory, the working directory and the two
/// settings' variables. A test builds one by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    /// `HOME`, when it is set.
    pub home: Option<PathBuf>,
    /// The working directory.
    pub cwd: PathBuf,
    /// [`CONFIG_VARIABLE`], when it is set and not empty.
    pub config: Option<OsString>,
    /// [`INSTANCE_VARIABLE`], when it is set and not empty.
    pub instance: Option<OsString>,
}

impl Environment {
    /// Reads the environment of this process.
    ///
    /// # Errors
    ///
    /// The working directory cannot be read.
    pub fn from_process() -> Result<Self> {
        Ok(Self {
            home: env::var_os("HOME").map(PathBuf::from),
            cwd: env::current_dir().context("read the working directory")?,
            config: env::var_os(CONFIG_VARIABLE).filter(|value| !value.is_empty()),
            instance: env::var_os(INSTANCE_VARIABLE).filter(|value| !value.is_empty()),
        })
    }

    /// The home directory, which `~` and the default paths are under.
    ///
    /// # Errors
    ///
    /// `HOME` is not set, or is not an absolute path.
    pub fn home(&self) -> Result<&Path> {
        match &self.home {
            Some(home) if home.is_absolute() => Ok(home),
            _ => bail!(
                "HOME is not an absolute path; the config file's `~` and its default paths are \
                 under it"
            ),
        }
    }
}

/// One problem of a config file: its YAML path, such as `instances[0].cadence.watch`, or the
/// line and column of a syntax error, and what is wrong there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The YAML path, or `line L, column C`.
    pub path: String,
    /// What is wrong there. It never holds a value that looks like a token.
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// A config file that does not convert, with every problem it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    /// The file.
    pub file: PathBuf,
    /// Each problem, in the order the file holds them.
    pub problems: Vec<Problem>,
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = self
            .problems
            .iter()
            .map(|problem| format!("{}: {problem}", self.file.display()))
            .collect();
        f.write_str(&lines.join("\n"))
    }
}

impl std::error::Error for Invalid {}

/// The effective config: the file's, or the built-in defaults when the default file is not there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// Where the file was looked for.
    pub path: PathBuf,
    /// Whether it was read; `false` only for the default file, when it is not there.
    pub read: bool,
    /// The effective config.
    pub config: Config,
}

impl Loaded {
    /// Names where the config came from, for a message.
    fn origin(&self) -> String {
        if self.read {
            format!("the config file {}", self.path.display())
        } else {
            format!(
                "the built-in defaults (no config file at {})",
                self.path.display()
            )
        }
    }
}

/// The config file's path: `flag`, else [`CONFIG_VARIABLE`], else [`DEFAULT_FILE`] under the home
/// directory; and whether it was named (by the flag or the variable) rather than defaulted.
///
/// # Errors
///
/// Neither names a file and the home directory is not known.
pub fn locate(flag: Option<&Path>, environment: &Environment) -> Result<(PathBuf, bool)> {
    if let Some(file) = flag {
        return Ok((file.to_owned(), true));
    }
    if let Some(file) = &environment.config {
        return Ok((PathBuf::from(file), true));
    }
    Ok((environment.home()?.join(DEFAULT_FILE), false))
}

/// Reads the effective config: the file [`locate`] finds, converted by [`parse`], or
/// [`built_in`] when the default file is not there.
///
/// # Errors
///
/// A named file is not there; a file cannot be read; the home directory is not known; or the
/// file does not convert, answered as [`Invalid`] with every problem.
pub fn load(flag: Option<&Path>, environment: &Environment) -> Result<Loaded> {
    let (path, named) = locate(flag, environment)?;
    let home = environment.home()?;
    match fs::read_to_string(&path) {
        Ok(text) => match parse(&text, home) {
            Ok(config) => Ok(Loaded {
                path,
                read: true,
                config,
            }),
            Err(problems) => Err(Invalid {
                file: path,
                problems,
            }
            .into()),
        },
        Err(error) if error.kind() == ErrorKind::NotFound && !named => Ok(Loaded {
            config: Config {
                version: VERSION.to_owned(),
                default: None,
                instances: vec![built_in(home, &environment.cwd)],
            },
            path,
            read: false,
        }),
        Err(error) => {
            Err(error).with_context(|| format!("read the config file {}", path.display()))
        }
    }
}

/// The instance a command runs: `flag`, else [`INSTANCE_VARIABLE`], else the instance whose tree
/// holds the working directory ([`by_directory`], `story:instance-isolation`), else the file's
/// `default`, else the config's only one.
///
/// # Errors
///
/// The name given is not an instance of the config; two instances' trees hold the working
/// directory alike (`AmbiguousInstance`); or none is given, the file names no `default` and the
/// config names more than one; the error names the instances there are.
pub fn select<'c>(
    loaded: &'c Loaded,
    flag: Option<&str>,
    environment: &Environment,
) -> Result<&'c Instance> {
    let instances = &loaded.config.instances;
    let names: Vec<&str> = instances
        .iter()
        .map(|instance| instance.name.0.as_str())
        .collect();
    let asked = match flag {
        Some(name) => Some(name.to_owned()),
        None => match &environment.instance {
            Some(name) => Some(name.to_string_lossy().into_owned()),
            None => by_directory(instances, &environment.cwd)?
                .or_else(|| loaded.config.default.as_ref().map(|name| name.0.clone())),
        },
    };
    match asked {
        Some(name) => instances
            .iter()
            .find(|instance| instance.name.0 == name)
            .ok_or_else(|| {
                anyhow!(
                    "no instance {name:?}: {} names {}",
                    loaded.origin(),
                    names.join(", ")
                )
            }),
        None => match instances.as_slice() {
            [only] => Ok(only),
            _ => bail!(
                "{} names {} instances, {}, and no default; choose one with --instance or \
                 {INSTANCE_VARIABLE}, or name it as the file's default",
                loaded.origin(),
                names.len(),
                names.join(", ")
            ),
        },
    }
}

/// The instance whose tree holds `cwd` (`story:instance-isolation`): its `records`, `state`,
/// `checkouts.root` or `checkouts.trees` is `cwd` or an ancestor of it, read as written and as
/// the file system resolves it. Where the trees of several instances hold it, the one whose tree
/// lies deepest; none where it lies in no instance's tree.
///
/// # Errors
///
/// `AmbiguousInstance`: two instances' trees hold `cwd` at the same depth, so neither is chosen
/// and the default is not taken either.
pub fn by_directory(instances: &[Instance], cwd: &Path) -> Result<Option<String>> {
    let real = fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_owned());
    let mut best: Option<(usize, Vec<&str>)> = None;
    for instance in instances {
        let roots = [
            &instance.records,
            &instance.state,
            &instance.checkouts.root,
            &instance.checkouts.trees,
        ];
        let depth = roots
            .iter()
            .flat_map(|root| {
                let root = PathBuf::from(root);
                let resolved = fs::canonicalize(&root).ok();
                [Some(root), resolved]
            })
            .flatten()
            .filter(|root| cwd.starts_with(root) || real.starts_with(root))
            .map(|root| root.components().count())
            .max();
        let Some(depth) = depth else { continue };
        let name = instance.name.0.as_str();
        match &mut best {
            Some((deepest, names)) if *deepest == depth => names.push(name),
            Some((deepest, _)) if *deepest > depth => {}
            _ => best = Some((depth, vec![name])),
        }
    }
    match best {
        None => Ok(None),
        Some((_, names)) if names.len() == 1 => Ok(Some(names[0].to_owned())),
        Some((_, names)) => bail!(
            "AmbiguousInstance: the working directory {} lies in the trees of the instances {} \
             alike, so none is chosen and the default is not taken; choose one with \
             {INSTANCE_VARIABLE}",
            cwd.display(),
            names.join(" and ")
        ),
    }
}

/// The instance this process runs: the selected instance of the config file, or the built-in
/// one when there is no file. Every command reads paths, sources and thresholds from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    /// The instance.
    pub instance: Instance,
    /// Whether a config file named it. Without a file every path and value is today's.
    pub from_file: bool,
    /// The other instances of the config file, in its order: whose `session_prefix` a session
    /// name may carry instead (`story:instance-session-names`). None without a file.
    pub others: Vec<Instance>,
    /// The file's `default` instance, whose conductor serves every instance that names no
    /// `conductor` role. None without a file, or when the file names none.
    pub default: Option<InstanceName>,
}

impl Active {
    /// The instance and every other instance of the config file, the instance first.
    #[must_use]
    pub fn all(&self) -> Vec<&Instance> {
        std::iter::once(&self.instance)
            .chain(&self.others)
            .collect()
    }
}

/// Whether `instance` names a `conductor` role: its own conductor. One that names none is served
/// by the conductor of the file's `default` instance (`.agents/conductor.md`, More than one
/// instance).
#[must_use]
pub fn has_conductor(instance: &Instance) -> bool {
    instance
        .roles
        .iter()
        .any(|role| role.role == CONDUCTOR_ROLE)
}

static ACTIVE: OnceLock<Active> = OnceLock::new();

/// The instance `flag` (the global `--config`) and `environment` select: [`load`], then
/// [`select`] by [`INSTANCE_VARIABLE`] or the only instance; [`built_in`] when no file is there.
///
/// # Errors
///
/// What [`load`] and [`select`] answer.
pub fn resolve(flag: Option<&Path>, environment: &Environment) -> Result<Active> {
    let loaded = load(flag, environment)?;
    if loaded.read {
        let instance = select(&loaded, None, environment)?.clone();
        let others = loaded
            .config
            .instances
            .iter()
            .filter(|other| other.name != instance.name)
            .cloned()
            .collect();
        Ok(Active {
            instance,
            from_file: true,
            others,
            default: loaded.config.default.clone(),
        })
    } else {
        Ok(Active {
            instance: built_in(environment.home()?, &environment.cwd),
            from_file: false,
            others: Vec::new(),
            default: None,
        })
    }
}

/// Fixes the instance of this process from the global `--config` before a command runs, so a
/// config file that does not load stops the command with its problems instead of running it on
/// the built-in values. A second call keeps the first instance.
///
/// # Errors
///
/// What [`resolve`] answers.
pub fn set_active(flag: Option<&Path>) -> Result<()> {
    let environment = Environment::from_process()?;
    let (file, _) = locate(flag, &environment)?;
    let _ = CONFIG_FILE.set(environment.cwd.join(file));
    let active = resolve(flag, &environment)?;
    let _ = ACTIVE.set(active);
    Ok(())
}

/// The config file [`set_active`] located, absolute.
static CONFIG_FILE: OnceLock<PathBuf> = OnceLock::new();

/// Why the settings file `file` does not pin the instance `instance` (`story:instance-isolation`),
/// or none when it does: its `env.CONDUCTOR_INSTANCE` is `instance`, and each of its hook commands
/// that runs `conductor` starts `CONDUCTOR_INSTANCE=<instance> `. The harness does not pass the
/// environment of the process that starts a session on to the session it forks, so only the
/// settings select the instance the session's hooks and commands act for. The reason ends with
/// what to write.
#[must_use]
pub fn pin_problem(file: &Path, instance: &str) -> Option<String> {
    let fix = pin_fix(instance);
    let settings = match read_settings(file) {
        Ok(settings) => settings,
        Err(reason) => return Some(format!("{reason}: {fix}")),
    };
    foreign_pin(&settings, instance)
        .or_else(|| unpinned(&settings, instance))
        .map(|reason| format!("{reason}: {fix}"))
}

/// Why a session cannot start with the settings file `file` for the instance `instance`
/// (`story:instance-isolation`), or none when it can, pinned or not: it cannot be read, is no
/// JSON, or pins another instance (its `env.CONDUCTOR_INSTANCE`, or a hook command that starts
/// `CONDUCTOR_INSTANCE=<other>`), whose hooks would act for that instance. A file that pins no
/// instance is no refusal: a session starts with it pinned ([`session_settings`]). The reason
/// ends with what to write.
#[must_use]
pub fn pin_refusal(file: &Path, instance: &str) -> Option<String> {
    let fix = pin_fix(instance);
    match read_settings(file) {
        Ok(settings) => foreign_pin(&settings, instance).map(|reason| format!("{reason}: {fix}")),
        Err(reason) => Some(format!("{reason}: {fix}")),
    }
}

/// What a settings file writes to pin the instance `instance`.
fn pin_fix(instance: &str) -> String {
    format!(
        "write \"env\": {{\"{INSTANCE_VARIABLE}\": \"{instance}\"}} and start every hook command \
         that runs conductor with `{INSTANCE_VARIABLE}={instance} conductor`"
    )
}

/// The settings file `file` as JSON, or why it is not.
fn read_settings(file: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(file).map_err(|error| format!("cannot be read ({error})"))?;
    serde_json::from_str(&text).map_err(|error| format!("is no JSON ({error})"))
}

/// Why `settings` pins another instance than `instance`, or none when it does not: its
/// `env.CONDUCTOR_INSTANCE` names another, or one of its hook commands starts
/// `CONDUCTOR_INSTANCE=<another> `.
fn foreign_pin(settings: &Value, instance: &str) -> Option<String> {
    if let Some(name) = settings["env"][INSTANCE_VARIABLE].as_str()
        && name != instance
    {
        return Some(format!("pins the instance {name:?}, not {instance:?}"));
    }
    let prefix = format!("{INSTANCE_VARIABLE}=");
    hook_commands(settings).into_iter().find_map(|command| {
        let name = command
            .trim_start()
            .strip_prefix(&prefix)?
            .split(char::is_whitespace)
            .next()
            .unwrap_or_default();
        (name != instance).then(|| {
            format!(
                "has the hook command {command:?}, which runs for the instance {name:?}, not \
                 {instance:?}"
            )
        })
    })
}

/// Why `settings` does not pin `instance` although it pins no other ([`foreign_pin`]), or none
/// when it pins it: it has no `env.CONDUCTOR_INSTANCE`, or a hook command that runs `conductor`
/// does not start `CONDUCTOR_INSTANCE=<instance> `.
fn unpinned(settings: &Value, instance: &str) -> Option<String> {
    if settings["env"][INSTANCE_VARIABLE].as_str().is_none() {
        return Some("pins no instance".to_owned());
    }
    let prefix = format!("{INSTANCE_VARIABLE}={instance} ");
    hook_commands(settings)
        .into_iter()
        .find(|command| runs_conductor(command) && !command.trim_start().starts_with(&prefix))
        .map(|command| {
            format!("has the hook command {command:?}, which runs conductor for no instance")
        })
}

/// The pinned copy of a session's settings (`story:instance-isolation`):
/// `<state>/sessions/<session>.settings.json`.
#[must_use]
pub fn pinned_copy(state: &Path, session: &str) -> PathBuf {
    state
        .join("sessions")
        .join(format!("{session}.settings.json"))
}

/// Writes `settings` pinned to `instance` ([`pinned`]) to the session's pinned copy
/// ([`pinned_copy`]) and answers its path.
///
/// # Errors
///
/// The directory or the file cannot be written.
pub fn write_pinned(
    settings: Value,
    instance: &str,
    state: &Path,
    session: &str,
) -> Result<PathBuf> {
    let file = pinned_copy(state, session);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(&pinned(settings, instance))
        .context("write the pinned settings")?;
    fs::write(&file, format!("{text}\n")).with_context(|| format!("write {}", file.display()))?;
    Ok(file)
}

/// The settings file a session `session` of the instance `instance` starts with when its role
/// names `file` (`story:instance-isolation`): `file` itself when it pins the instance
/// ([`pin_problem`] finds none), else a copy of its content pinned to the instance
/// ([`write_pinned`]), so a role may name a settings file that pins no instance, as the shipped
/// ones do. The caller has refused a file [`pin_refusal`] names.
///
/// # Errors
///
/// The file cannot be read or is no JSON, or the copy cannot be written.
pub fn session_settings(
    file: &Path,
    instance: &str,
    state: &Path,
    session: &str,
) -> Result<String> {
    if pin_problem(file, instance).is_none() {
        return Ok(file.display().to_string());
    }
    let settings = read_settings(file)
        .map_err(|reason| anyhow!("the settings {} {reason}", file.display()))?;
    Ok(write_pinned(settings, instance, state, session)?
        .display()
        .to_string())
}

/// `settings` pinned to `instance` (`story:instance-isolation`): its `env.CONDUCTOR_INSTANCE`
/// set to `instance`, and each hook command that runs `conductor` started with
/// `CONDUCTOR_INSTANCE=<instance> ` in place of any instance it named, so [`pin_problem`] finds
/// none. Settings that are no JSON object become one holding only the pin.
#[must_use]
pub fn pinned(mut settings: Value, instance: &str) -> Value {
    if !settings.is_object() {
        settings = Value::Object(Map::new());
    }
    let env = &mut settings["env"];
    if !env.is_object() {
        *env = Value::Object(Map::new());
    }
    env[INSTANCE_VARIABLE] = Value::String(instance.to_owned());
    let prefix = format!("{INSTANCE_VARIABLE}=");
    if let Some(events) = settings["hooks"].as_object_mut() {
        for matchers in events.values_mut().filter_map(Value::as_array_mut) {
            for matcher in matchers.iter_mut() {
                let Some(hooks) = matcher["hooks"].as_array_mut() else {
                    continue;
                };
                for hook in hooks {
                    let Some(command) = hook["command"].as_str() else {
                        continue;
                    };
                    if !runs_conductor(command) {
                        continue;
                    }
                    let mut rest = command.trim_start();
                    while let Some(after) = rest.strip_prefix(&prefix) {
                        rest = after
                            .split_once(char::is_whitespace)
                            .map_or("", |(_, more)| more.trim_start());
                    }
                    hook["command"] = Value::String(format!("{prefix}{instance} {rest}"));
                }
            }
        }
    }
    settings
}

/// Every hook command of a settings file: `hooks.<event>[].hooks[].command`.
fn hook_commands(settings: &Value) -> Vec<&str> {
    settings["hooks"]
        .as_object()
        .into_iter()
        .flat_map(|events| events.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|matcher| matcher["hooks"].as_array())
        .flatten()
        .filter_map(|hook| hook["command"].as_str())
        .collect()
}

/// Whether `command` runs the `conductor` binary: one of its words is `conductor` or a path to it.
fn runs_conductor(command: &str) -> bool {
    command.split_whitespace().any(|word| {
        let word = word.trim_matches(|c| c == '"' || c == '\'');
        word == "conductor" || word.ends_with("/conductor")
    })
}

/// The roles of `config` whose settings file is there and does not pin its instance, each by its
/// `settings` path: as problems, the files a session cannot start with ([`pin_refusal`]); as
/// notes, the files that pin no instance, which a session starts with pinned
/// ([`session_settings`]).
fn pin_problems(config: &Config) -> (Vec<Problem>, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut notes = Vec::new();
    for (index, instance) in config.instances.iter().enumerate() {
        for (at, role) in instance.roles.iter().enumerate() {
            let Some(settings) = &role.settings else {
                continue;
            };
            let file = Path::new(&settings.0);
            if !file.is_file() {
                continue;
            }
            let path = format!("instances[{index}].roles[{at}].settings");
            let name = &instance.name.0;
            if let Some(problem) = pin_refusal(file, name) {
                problems.push(Problem {
                    path,
                    message: format!("the settings file {} {problem}", file.display()),
                });
            } else if let Ok(settings) = read_settings(file)
                && let Some(reason) = unpinned(&settings, name)
            {
                notes.push(Problem {
                    path,
                    message: format!(
                        "the settings file {} {reason}; session start and spawn start the role \
                         with a copy pinned to {name:?}, {}",
                        file.display(),
                        pinned_copy(Path::new(&instance.state), "<session name>").display()
                    ),
                });
            }
        }
    }
    (problems, notes)
}

/// The host directory's name beside the config file.
pub const HOST_DIR: &str = "host";

/// The host directory of the config file `file` (`story:instance-isolation`): `host/` beside it.
/// What every instance of the file shares lives there, never under an instance's `state`: the
/// host ledger, its locks, the reclaim record and the fall's marker (`crate::host`).
#[must_use]
pub fn host_of(file: &Path) -> PathBuf {
    file.parent()
        .map_or_else(|| PathBuf::from(HOST_DIR), |parent| parent.join(HOST_DIR))
}

/// The host directory of this process's config file ([`host_of`]): of the file [`set_active`]
/// located, else of the one [`locate`] finds without a flag, `~/.b10x/conductor/host` when no
/// variable names one. A `--state-dir` does not move it.
///
/// # Errors
///
/// No file was located and the working directory or the home directory cannot be read.
pub fn host_dir() -> Result<PathBuf> {
    if let Some(file) = CONFIG_FILE.get() {
        return Ok(host_of(file));
    }
    let environment = Environment::from_process()?;
    let (file, _) = locate(None, &environment)?;
    Ok(host_of(&environment.cwd.join(file)))
}

/// The instance this process runs. [`crate::cli::Cli::run`] fixes it with [`set_active`]; a
/// caller that did not (a library test) gets what [`resolve`] finds without a flag, and the
/// built-in instance when that fails.
#[must_use]
pub fn active() -> &'static Active {
    ACTIVE.get_or_init(|| {
        let environment = Environment::from_process().ok();
        environment
            .as_ref()
            .and_then(|environment| resolve(None, environment).ok())
            .unwrap_or_else(|| {
                let home = environment
                    .as_ref()
                    .and_then(|environment| environment.home.clone())
                    .unwrap_or_else(|| PathBuf::from("/"));
                let cwd = environment
                    .map(|environment| environment.cwd)
                    .unwrap_or_else(|| PathBuf::from("."));
                Active {
                    instance: built_in(&home, &cwd),
                    from_file: false,
                    others: Vec::new(),
                    default: None,
                }
            })
    })
}

/// The built-in instance, which applies when there is no config file: each value is today's.
/// `home` is the home directory, `cwd` the working directory.
#[must_use]
pub fn built_in(home: &Path, cwd: &Path) -> Instance {
    Instance {
        name: InstanceName(ORGANIZATION.to_owned()),
        // Without a prefix the sessions keep today's names (story:instance-session-names).
        session_prefix: None,
        // The first instance's port, the dashboard's own default (story:instance-scoped-tasks).
        dashboard_port: dashboard_port(0),
        sources: vec![Source::GitHub(GitHubSource {
            owner: ORGANIZATION.to_owned(),
        })],
        checkouts: Checkouts {
            root: text_of(&home.join(ORGANIZATION)),
            trees: text_of(&home.join(WORKTREES)),
        },
        records: text_of(cwd),
        state: text_of(&cwd.join(STATE)),
        cache: text_of(&home.join(CACHE)),
        roles: default_roles(None),
        confinement: default_confinement(),
        controllers: default_controllers(),
        // Repository rules are written only by a user: without one, a repository's activity is
        // its mark or the newest snapshot's, as today.
        repositories: Vec::new(),
        cadence: default_cadence(),
        thresholds: default_thresholds(),
        retention: default_retention(),
        reports: Vec::new(),
        authority: default_authority(),
        // A usage probe and watcher are named only by a user (story:usage-probe).
        usage: no_usage(),
        // The operator is named only by a user.
        operator: None,
        // A catalog is named only by a user (story:catalog-source).
        catalog: None,
        // A secret sync is named only by a user (story:policy-secret-drift).
        secret_syncs: Vec::new(),
        // A Gates policy is named only by a user (story:gates-hooks-fresh).
        gates_policy: None,
        // Its own conductor, `conductor` (story:instance-session-names).
        conductor: ConductorSession {
            served_by: None,
            session_name: Some(SessionName(CONDUCTOR_ROLE.to_owned())),
        },
    }
}

/// The conductor that serves `instance` (`story:instance-session-names`): its own when it names
/// a `conductor` role; else the conductor of `default`, the file's default instance, when that is
/// another instance and names one; else none.
#[must_use]
pub fn conductor_session(instance: &Instance, default: Option<&Instance>) -> ConductorSession {
    if has_conductor(instance) {
        return ConductorSession {
            served_by: None,
            session_name: Some(session_name(instance, CONDUCTOR_ROLE)),
        };
    }
    match default.filter(|default| default.name != instance.name && has_conductor(default)) {
        Some(default) => ConductorSession {
            served_by: Some(default.name.clone()),
            session_name: Some(session_name(default, CONDUCTOR_ROLE)),
        },
        None => ConductorSession {
            served_by: None,
            session_name: None,
        },
    }
}

/// The `dashboard_port` of the instance at `index` in the file when it writes none
/// (`story:instance-scoped-tasks`): the dashboard's default port, 7313, for the first, and one
/// more for each later instance, so instances that leave it out each serve their own.
#[must_use]
pub fn dashboard_port(index: usize) -> i64 {
    i64::from(crate::dashboard::DEFAULT_PORT)
        .saturating_add(i64::try_from(index).unwrap_or(i64::MAX))
}

/// The effective `dashboard_port` of each instance of the file, by its index, as the file writes
/// them: the written port, or [`dashboard_port`] of its index when it writes none; `None` where
/// the written value is not a port, which [`Reader::port`] names.
fn ports(items: &[Value]) -> Vec<Option<i64>> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| match item.get("dashboard_port") {
            None | Some(Value::Null) => Some(dashboard_port(index)),
            Some(value) => value.as_i64().filter(|port| PORTS.contains(port)),
        })
        .collect()
}

/// The built-in roles, each session named after `prefix` ([`role_session_name`]).
fn default_roles(prefix: Option<&SessionPrefix>) -> Vec<Role> {
    ROLES
        .iter()
        .map(|role| Role {
            role: (*role).to_owned(),
            harness: Harness::Claude,
            model: CommandWord(MODEL.to_owned()),
            agent: None,
            settings: None,
            profile: None,
            session_name: role_session_name(prefix, role),
            read_only_tools: Vec::new(),
            limits: no_limits(),
            writable: default_writable(role),
            network: Network::Open,
            forge_token_env: Vec::new(),
        })
        .collect()
}

/// The places a session of the role `role` writes when the file names none
/// (`story:session-envelope-model`): every place of [`PLACES`] but the records, and for the
/// `conductor` role, whose session keeps them, the records too.
#[must_use]
pub fn default_writable(role: &str) -> Vec<Writable> {
    PLACES
        .iter()
        .filter(|place| **place != RECORDS_PLACE || role == CONDUCTOR_ROLE)
        .map(|place| Writable((*place).to_owned()))
        .collect()
}

/// Limits that set no limit.
fn no_limits() -> Limits {
    Limits {
        memory_max: None,
        cpu_weight: None,
        tasks_max: None,
    }
}

/// The confinement of an instance whose file writes none: a session that cannot be boxed starts
/// unboxed and is reported, boxes come from systemd, and the instance sets no limit.
fn default_confinement() -> Confinement {
    Confinement {
        mode: ConfinementMode::Report,
        provider: ConfinementProvider::Systemd,
        limits: no_limits(),
    }
}

/// The name a session of `instance` starts under for `word`, a role or a repository:
/// `<session_prefix>-<word>`, or `word` itself when the instance has no prefix
/// (`story:instance-session-names`).
#[must_use]
pub fn session_name(instance: &Instance, word: &str) -> SessionName {
    named(instance.session_prefix.as_ref(), word)
}

/// [`session_name`] under `prefix`.
fn named(prefix: Option<&SessionPrefix>, word: &str) -> SessionName {
    SessionName(match prefix {
        Some(prefix) => format!("{}-{word}", prefix.0),
        None => word.to_owned(),
    })
}

/// The `session_name` of the role `role` under `prefix`: [`named`] after the role, and none for
/// the controller role, whose sessions are named per repository.
fn role_session_name(prefix: Option<&SessionPrefix>, role: &str) -> Option<SessionName> {
    (role != CONTROLLER).then(|| named(prefix, role))
}

/// The fixed session names each instance of the file without a `session_prefix` keeps, by its
/// index, as the file writes them: its roles' names but the controller's (every built-in role's
/// when it writes no `roles`), and `conductor` and `conductor-dev`, which the start tasks use.
fn bare_names(items: &[Value]) -> Vec<(usize, String)> {
    let mut names = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if !item.get("session_prefix").is_none_or(Value::is_null) {
            continue;
        }
        let roles: Vec<String> = match item.get("roles").and_then(Value::as_array) {
            Some(roles) => roles
                .iter()
                .filter_map(|role| role["role"].as_str().map(str::to_owned))
                .collect(),
            None => ROLES.iter().map(|role| (*role).to_owned()).collect(),
        };
        let fixed = roles
            .into_iter()
            .chain([CONDUCTOR_ROLE.to_owned(), format!("{CONDUCTOR_ROLE}-dev")])
            .filter(|role| role != CONTROLLER);
        for name in fixed {
            if !names.contains(&(index, name.clone())) {
                names.push((index, name));
            }
        }
    }
    names
}

/// Whether the session name `name` carries the session prefix `prefix`: it is `<prefix>-`
/// followed by at least one more character.
#[must_use]
pub fn carries(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|rest| !rest.is_empty())
}

fn default_controllers() -> Controllers {
    Controllers {
        max_working: MAX_WORKING,
        max_subagents: MAX_SUBAGENTS,
        max_open_reviews: MAX_OPEN_REVIEWS,
    }
}

fn default_cadence() -> Cadence {
    Cadence {
        cycle: Duration(format!("PT{CYCLE_HOURS}H")),
        daily: TimeOfDay(DAILY.to_owned()),
        watch: Duration(format!("PT{WATCH_EVERY}S")),
        ci: Duration(format!("PT{CI_EVERY}S")),
    }
}

fn default_thresholds() -> Thresholds {
    Thresholds {
        context_handover: Tokens(i64::try_from(CONTEXT_HANDOVER).unwrap_or(i64::MAX)),
        disk_path: DISK_PATH.to_owned(),
        disk_low: Gibibytes(i64::try_from(DISK_LOW_GIB).unwrap_or(i64::MAX)),
        disk_clear: Gibibytes(i64::try_from(DISK_CLEAR_GIB).unwrap_or(i64::MAX)),
        disk_admit: Gibibytes(DISK_ADMIT_GIB),
        build_slot: Gibibytes(BUILD_SLOT_GIB),
        build_size: Gibibytes(BUILD_SIZE_GIB),
        gate_stop: Gibibytes(GATE_STOP_GIB),
        gate_resume: Gibibytes(GATE_RESUME_GIB),
        watchdog_every: Duration(format!("PT{WATCHDOG_EVERY}S")),
        silent_session: Duration(format!("PT{SILENT_SESSION_MINUTES}M")),
        cpu_reserve: CPU_RESERVE,
    }
}

/// Whether `unit` is a systemd unit name: letters, digits, `:`, `_`, `.`, `@` and `-`, ending
/// in `.` and one of [`UNIT_TYPES`] after a name of its own (`story:usage-probe`).
#[must_use]
pub fn unit_name(unit: &str) -> bool {
    unit.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '.' | '@' | '-'))
        && unit.rsplit_once('.').is_some_and(|(name, kind)| {
            !name.is_empty() && !name.starts_with('-') && UNIT_TYPES.contains(&kind)
        })
}

/// No usage probe and no watcher: what an instance that names neither has (`story:usage-probe`).
fn no_usage() -> Usage {
    Usage {
        probe: None,
        watcher: None,
    }
}

fn default_retention() -> Retention {
    Retention {
        snapshots: RETENTION_SNAPSHOTS,
    }
}

fn default_authority() -> Authority {
    Authority {
        class_c: Decider::Conductor,
        class_o: Decider::Operator,
    }
}

fn text_of(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Converts a config file's text into the generated types, `~` expanded against `home` and
/// every absent key filled.
///
/// # Errors
///
/// Every problem the file has, each named by its YAML path.
pub fn parse(text: &str, home: &Path) -> Result<Config, Vec<Problem>> {
    let value: Value = match serde_yaml::from_str::<Option<Value>>(text) {
        Ok(value) => value.unwrap_or(Value::Null),
        Err(error) => {
            let path = error.location().map_or_else(
                || "the file".to_owned(),
                |at| format!("line {}, column {}", at.line(), at.column()),
            );
            return Err(vec![Problem {
                path,
                message: redact(&format!("not YAML: {error}")),
            }]);
        }
    };
    let mut reader = Reader {
        home,
        problems: Vec::new(),
    };
    reader.secrets(&value, "");
    let config = reader.config(&value);
    match (config, reader.problems.is_empty()) {
        (Some(config), true) => Ok(config),
        _ => Err(reader.problems),
    }
}

/// The YAML path of `key` under `parent`.
fn join(parent: &str, key: &str) -> String {
    let key = if looks_like_token(key) {
        "<a key that looks like a token>"
    } else {
        key
    };
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    }
}

/// Whether `text` holds a word that looks like a token. A word is a run of letters, digits, `_`
/// and `-`.
fn looks_like_token(text: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .any(|word| {
            TOKEN_PREFIXES.iter().any(|prefix| word.starts_with(prefix))
                || word.strip_prefix("xox").is_some_and(|rest| {
                    let mut chars = rest.chars();
                    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                        && chars.next() == Some('-')
                })
        })
}

/// `text` with every word that looks like a token replaced.
fn redact(text: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if looks_like_token(word) {
            out.push_str("<redacted>");
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// A value as a problem quotes it; one that looks like a token is not quoted.
fn echo(value: &Value) -> String {
    match value {
        Value::String(text) if looks_like_token(text) => {
            "a value that looks like a token".to_owned()
        }
        Value::String(text) => format!("{text:?}"),
        Value::Null => "nothing".to_owned(),
        Value::Array(_) => "a list".to_owned(),
        Value::Object(_) => "a mapping".to_owned(),
        other => other.to_string(),
    }
}

/// Reads a config file's value into the generated types, collecting every problem.
struct Reader<'h> {
    home: &'h Path,
    problems: Vec<Problem>,
}

impl Reader<'_> {
    fn problem(&mut self, path: &str, message: impl Into<String>) {
        let path = if path.is_empty() { "the file" } else { path };
        self.problems.push(Problem {
            path: path.to_owned(),
            message: message.into(),
        });
    }

    /// Names every value that looks like a token, by its path.
    fn secrets(&mut self, value: &Value, path: &str) {
        match value {
            Value::String(text) if looks_like_token(text) => self.problem(
                path,
                "looks like a token; the config file holds no secret, so name where the secret \
                 is kept instead",
            ),
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    self.secrets(item, &format!("{path}[{index}]"));
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    self.secrets(item, &join(path, key));
                }
            }
            _ => {}
        }
    }

    /// `value` as a mapping whose keys are among `keys`; each other key is a problem.
    fn mapping<'v>(
        &mut self,
        value: &'v Value,
        path: &str,
        keys: &[&str],
    ) -> Option<&'v Map<String, Value>> {
        let Value::Object(map) = value else {
            self.problem(
                path,
                format!("{} is not a mapping of {}", echo(value), keys.join(", ")),
            );
            return None;
        };
        for key in map.keys() {
            if !keys.contains(&key.as_str()) {
                self.problem(
                    &join(path, key),
                    format!("unknown key; expected one of {}", keys.join(", ")),
                );
            }
        }
        Some(map)
    }

    /// The value of `key`, absent when it is not there or null.
    fn get<'v>(map: &'v Map<String, Value>, key: &str) -> Option<&'v Value> {
        map.get(key).filter(|value| !value.is_null())
    }

    /// The value of a key that must be there.
    fn required<'v>(
        &mut self,
        map: &'v Map<String, Value>,
        path: &str,
        key: &str,
    ) -> Option<&'v Value> {
        let value = Self::get(map, key);
        if value.is_none() {
            self.problem(&join(path, key), "missing");
        }
        value
    }

    /// Each item of a list, converted by `item`; `None` when any does not convert.
    fn list<T>(
        &mut self,
        value: &Value,
        path: &str,
        mut item: impl FnMut(&mut Self, &Value, &str) -> Option<T>,
    ) -> Option<Vec<T>> {
        let Value::Array(items) = value else {
            self.problem(path, format!("{} is not a list", echo(value)));
            return None;
        };
        let converted: Vec<Option<T>> = items
            .iter()
            .enumerate()
            .map(|(index, value)| item(self, value, &format!("{path}[{index}]")))
            .collect();
        converted.into_iter().collect()
    }

    fn text(&mut self, value: &Value, path: &str) -> Option<String> {
        match value {
            Value::String(text) if !text.trim().is_empty() => Some(text.clone()),
            Value::String(_) => {
                self.problem(path, "is empty");
                None
            }
            other => {
                self.problem(path, format!("{} is not text", echo(other)));
                None
            }
        }
    }

    /// A path, absolute or under `~`, made absolute.
    fn path(&mut self, value: &Value, path: &str) -> Option<String> {
        let text = self.text(value, path)?;
        if text == "~" {
            return Some(text_of(self.home));
        }
        if let Some(rest) = text.strip_prefix("~/") {
            return Some(text_of(&self.home.join(rest)));
        }
        if Path::new(&text).is_absolute() {
            return Some(text);
        }
        self.problem(
            path,
            format!(
                "{} is not a path: write it absolute (/…) or under the home directory (~/…)",
                echo(value)
            ),
        );
        None
    }

    /// `text` as a `conductor.config.CommandWord`: every character one of [`COMMAND_WORD`]. The
    /// refusal names the first other character and never the value, which may look like a token.
    fn command_word(&mut self, text: String, path: &str) -> Option<CommandWord> {
        match text.chars().find(|c| !COMMAND_WORD.contains(*c)) {
            None => Some(CommandWord(text)),
            Some(other) => {
                self.problem(
                    path,
                    format!(
                        "holds {other:?}; write only A-Z a-z 0-9 . _ : / -, since a start command \
                         passes it to the harness as one shell word"
                    ),
                );
                None
            }
        }
    }

    fn word<T: Copy>(&mut self, value: &Value, path: &str, words: &[(&str, T)]) -> Option<T> {
        let found = value
            .as_str()
            .and_then(|text| words.iter().find(|(word, _)| *word == text));
        if found.is_none() {
            let names: Vec<&str> = words.iter().map(|(word, _)| *word).collect();
            self.problem(
                path,
                format!("{} is not one of {}", echo(value), names.join(", ")),
            );
        }
        found.map(|(_, meaning)| *meaning)
    }

    fn duration(&mut self, value: &Value, path: &str) -> Option<Duration> {
        let duration = value.as_str().and_then(parse_duration);
        if duration.is_none() {
            self.problem(
                path,
                format!(
                    "{} is not a duration: write a whole number above 0 with s, m, h or d, such \
                     as 180s or 2h",
                    echo(value)
                ),
            );
        }
        duration
    }

    fn size(&mut self, value: &Value, path: &str) -> Option<Gibibytes> {
        let size = value
            .as_str()
            .and_then(|text| text.strip_suffix('G'))
            .filter(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|number| number.parse::<i64>().ok())
            .map(Gibibytes);
        if size.is_none() {
            self.problem(
                path,
                format!(
                    "{} is not a size: write a whole number of GiB with G, such as 15G",
                    echo(value)
                ),
            );
        }
        size
    }

    fn tokens(&mut self, value: &Value, path: &str) -> Option<Tokens> {
        let digits = |text: &str| {
            Some(text)
                .filter(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|number| number.parse::<i64>().ok())
        };
        let count = match value {
            Value::Number(number) => number.as_i64().filter(|count| *count >= 0),
            Value::String(text) => match text.strip_suffix('k') {
                Some(thousands) => digits(thousands).and_then(|count| count.checked_mul(1000)),
                None => digits(text),
            },
            _ => None,
        };
        if count.is_none() {
            self.problem(
                path,
                format!(
                    "{} is not a count of tokens: write a whole number, or thousands with k, \
                     such as 300k",
                    echo(value)
                ),
            );
        }
        count.map(Tokens)
    }

    fn time_of_day(&mut self, value: &Value, path: &str) -> Option<TimeOfDay> {
        let valid = value.as_str().filter(|text| {
            let bytes = text.as_bytes();
            bytes.len() == 5
                && bytes[2] == b':'
                && text[..2].parse::<u8>().is_ok_and(|hour| hour < 24)
                && text[3..].parse::<u8>().is_ok_and(|minute| minute < 60)
                && bytes[..2].iter().chain(&bytes[3..]).all(u8::is_ascii_digit)
        });
        if valid.is_none() {
            self.problem(
                path,
                format!(
                    "{} is not a time of day: write HH:MM, UTC, such as \"08:07\"",
                    echo(value)
                ),
            );
        }
        valid.map(|text| TimeOfDay(text.to_owned()))
    }

    fn config(&mut self, value: &Value) -> Option<Config> {
        let empty = Value::Object(Map::new());
        let value = if value.is_null() { &empty } else { value };
        let map = self.mapping(value, "", &CONFIG_KEYS)?;
        let version = self.required(map, "", "version").and_then(|value| {
            let version = self.text(value, "version")?;
            if version != VERSION {
                self.problem("version", format!("{} is not {VERSION}", echo(value)));
                return None;
            }
            Some(version)
        });
        let instances = self.required(map, "", "instances").and_then(|value| {
            let instances = self.list(value, "instances", Self::instance);
            if let Value::Array(items) = value {
                if items.is_empty() {
                    self.problem("instances", "names no instance");
                }
                self.distinct_names(items);
                self.distinct_prefixes(items);
                self.distinct_ports(items);
            }
            instances
        });
        let default = match Self::get(map, "default") {
            Some(value) => self
                .default(value, map.get("instances"))
                .map(|name| Some(InstanceName(name))),
            None => Some(None),
        };
        let default = default?;
        let mut instances = instances?;
        if let Some(Value::Array(items)) = map.get("instances") {
            self.conductors(&mut instances, default.as_ref(), items);
            // A port the file leaves out is its instance's by index; one it writes was read.
            for (instance, port) in instances.iter_mut().zip(ports(items)) {
                if let Some(port) = port {
                    instance.dashboard_port = port;
                }
            }
        }
        Some(Config {
            version: version?,
            default,
            instances,
        })
    }

    /// Each instance's `conductor`, derived ([`conductor_session`]); one the file writes that is
    /// not the derived one is a problem, by its path. A written section is compared whole: a key
    /// it leaves out is written as none.
    fn conductors(
        &mut self,
        instances: &mut [Instance],
        default: Option<&InstanceName>,
        items: &[Value],
    ) {
        let serving = default.and_then(|default| {
            instances
                .iter()
                .find(|instance| &instance.name == default)
                .cloned()
        });
        for (index, (instance, item)) in instances.iter_mut().zip(items).enumerate() {
            let derived = conductor_session(instance, serving.as_ref());
            let path = format!("instances[{index}].conductor");
            if let Some(written) =
                Self::get(item.as_object().unwrap_or(&Map::new()), "conductor").cloned()
                && let Some(map) = self.mapping(&written, &path, &CONDUCTOR_KEYS)
            {
                let derived_served = derived.served_by.as_ref().map(|name| name.0.as_str());
                let derived_name = derived.session_name.as_ref().map(|name| name.0.as_str());
                for (key, wanted) in [
                    ("served_by", derived_served),
                    ("session_name", derived_name),
                ] {
                    let value = Self::get(map, key);
                    if value.and_then(Value::as_str) != wanted
                        || (value.is_some() && wanted.is_none())
                    {
                        self.problem(
                            &join(&path, key),
                            format!(
                                "{} is not {}, the conductor derived for this instance (its own \
                                 conductor's session name when it names a conductor role, else \
                                 the default instance's, which serves it); leave it out",
                                value.map_or_else(|| "nothing".to_owned(), echo),
                                wanted.map_or_else(
                                    || "nothing".to_owned(),
                                    |wanted| format!("{wanted:?}")
                                )
                            ),
                        );
                    }
                }
            }
            instance.conductor = derived;
        }
    }

    /// `default`, `conductor.config.Config`'s third invariant held: it names an instance of the
    /// file, as the file writes the names, whatever else is wrong with them.
    fn default(&mut self, value: &Value, instances: Option<&Value>) -> Option<String> {
        let name = self.text(value, "default")?;
        let names: Vec<&str> = match instances {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|item| item["name"].as_str())
                .collect(),
            _ => Vec::new(),
        };
        if names.contains(&name.as_str()) {
            return Some(name);
        }
        self.problem(
            "default",
            format!(
                "{} names no instance of the file; it names {}",
                echo(value),
                if names.is_empty() {
                    "none".to_owned()
                } else {
                    names.join(", ")
                }
            ),
        );
        None
    }

    /// `conductor.config.Config`'s second invariant, over the names as the file writes them, so
    /// that a duplicate is named whatever else is wrong with either instance.
    fn distinct_names(&mut self, items: &[Value]) {
        let names: Vec<Option<&str>> = items.iter().map(|item| item["name"].as_str()).collect();
        for (index, name) in names.iter().enumerate() {
            let Some(name) = name else { continue };
            if let Some(first) = names[..index]
                .iter()
                .position(|earlier| *earlier == Some(*name))
            {
                self.problem(
                    &format!("instances[{index}].name"),
                    format!(
                        "{} is the name of instances[{first}] too; instance names are unique",
                        echo(&Value::from(*name))
                    ),
                );
            }
        }
    }

    /// `conductor.config.Config`'s fourth invariant, over the prefixes as the file writes them
    /// (`story:instance-session-names`): no two instances share a `session_prefix`, at most one
    /// has none, and no prefix starts with another followed by `-`, which would let one session
    /// name carry both. A prefix that is not text is named by [`Reader::session_prefix`].
    fn distinct_prefixes(&mut self, items: &[Value]) {
        let prefixes: Vec<Option<Option<&str>>> = items
            .iter()
            .map(|item| match item.get("session_prefix") {
                None | Some(Value::Null) => Some(None),
                Some(Value::String(prefix)) => Some(Some(prefix.as_str())),
                Some(_) => None,
            })
            .collect();
        for (index, prefix) in prefixes.iter().enumerate() {
            let at = format!("instances[{index}].session_prefix");
            let first = prefixes[..index]
                .iter()
                .position(|earlier| earlier == prefix);
            match (prefix, first) {
                (None, _) => {}
                (Some(None), Some(first)) => self.problem(
                    &at,
                    format!(
                        "missing: instances[{first}] has no session_prefix either, and at most \
                         one instance keeps the bare session names conductor, conductor-dev and \
                         <repository>"
                    ),
                ),
                (Some(Some(prefix)), Some(first)) => self.problem(
                    &at,
                    format!(
                        "{} is the session_prefix of instances[{first}] too; session prefixes \
                         are unique, since session names are one namespace for the whole machine",
                        echo(&Value::from(*prefix))
                    ),
                ),
                (Some(Some(prefix)), None) => {
                    let shorter = prefixes.iter().position(|other| {
                        other.flatten().is_some_and(|other| {
                            prefix
                                .strip_prefix(other)
                                .is_some_and(|rest| rest.starts_with('-'))
                        })
                    });
                    if let Some(other) = shorter {
                        self.problem(
                            &at,
                            format!(
                                "{} starts with the session_prefix of instances[{other}] and \
                                 '-': a session named {prefix}-<name> would carry both",
                                echo(&Value::from(*prefix))
                            ),
                        );
                    } else if let Some((other, fixed)) = bare_names(items)
                        .into_iter()
                        .find(|(_, fixed)| carries(fixed, prefix))
                    {
                        self.problem(
                            &at,
                            format!(
                                "{} followed by '-' begins {fixed:?}, a session name \
                                 instances[{other}] keeps without a session_prefix: that session \
                                 would carry this prefix too",
                                echo(&Value::from(*prefix))
                            ),
                        );
                    }
                }
                (Some(None), None) => {}
            }
        }
    }

    /// `conductor.config.Config`'s invariant on `instance.dashboard_port`
    /// (`story:instance-scoped-tasks`): no two instances share a `dashboard_port`, counting the
    /// port [`dashboard_port`] fills for an instance that writes none. A port that is not one is
    /// named by [`Reader::port`].
    fn distinct_ports(&mut self, items: &[Value]) {
        let ports = ports(items);
        for (index, port) in ports.iter().enumerate() {
            let Some(port) = port else { continue };
            if let Some(first) = ports[..index]
                .iter()
                .position(|earlier| *earlier == Some(*port))
            {
                let filled = items[index]
                    .get("dashboard_port")
                    .is_none_or(Value::is_null);
                self.problem(
                    &format!("instances[{index}].dashboard_port"),
                    format!(
                        "{port}{} is the dashboard_port of instances[{first}] too; each \
                         instance's dashboard binds a port of its own, so write a port no other \
                         instance has",
                        if filled {
                            " (filled: 7313 plus the instance's index)"
                        } else {
                            ""
                        }
                    ),
                );
            }
        }
    }

    /// A `dashboard_port`: a whole number from 1 to 65535 (`conductor.config.Instance`'s
    /// invariants).
    fn port(&mut self, value: &Value, path: &str) -> Option<i64> {
        let port = value.as_i64().filter(|port| PORTS.contains(port));
        if port.is_none() {
            self.problem(
                path,
                format!(
                    "{} is not a port: write a whole number from 1 to 65535",
                    echo(value)
                ),
            );
        }
        port
    }

    /// A `conductor.config.SessionPrefix`: letters, digits and `-` (the alphabet the
    /// specification declares), starting and ending with a letter or digit.
    fn session_prefix(&mut self, value: &Value, path: &str) -> Option<SessionPrefix> {
        let valid = value.as_str().filter(|text| {
            let bytes = text.as_bytes();
            bytes.first().is_some_and(u8::is_ascii_alphanumeric)
                && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
                && text.chars().all(|c| SESSION_PREFIX.contains(c))
        });
        if valid.is_none() {
            self.problem(
                path,
                format!(
                    "{} is not a session prefix: letters, digits and '-', starting and ending \
                     with a letter or digit",
                    echo(value)
                ),
            );
        }
        valid.map(|text| SessionPrefix(text.to_owned()))
    }

    /// `roles` with each role's `session_name` derived from `prefix` ([`role_session_name`]); a
    /// name the file wrote that is not the derived one is a problem, by its path under `path`.
    fn session_names(
        &mut self,
        roles: Vec<Role>,
        prefix: Option<&SessionPrefix>,
        path: &str,
    ) -> Vec<Role> {
        roles
            .into_iter()
            .enumerate()
            .map(|(index, mut role)| {
                let derived = role_session_name(prefix, &role.role);
                if let Some(written) = role.session_name.take()
                    && Some(&written) != derived.as_ref()
                {
                    let why = match &derived {
                        Some(derived) => format!(
                            "is not {:?}, the name derived from session_prefix \
                             (<session_prefix>-<role>, or <role> without one); leave it out",
                            derived.0
                        ),
                        None => "names no session: the controller role's sessions are named per \
                                 repository, <session_prefix>-<repository>; leave it out"
                            .to_owned(),
                    };
                    self.problem(
                        &format!("{path}[{index}].session_name"),
                        format!("{} {why}", echo(&Value::from(written.0))),
                    );
                }
                role.session_name = derived;
                role
            })
            .collect()
    }

    fn instance(&mut self, value: &Value, path: &str) -> Option<Instance> {
        let map = self.mapping(value, path, &INSTANCE_KEYS)?;
        let name = self.required(map, path, "name").and_then(|value| {
            let at = join(path, "name");
            let name = self.text(value, &at)?;
            let mut chars = name.chars();
            let valid = chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
                && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
            if !valid {
                self.problem(
                    &at,
                    format!(
                        "{} is not a name: letters, digits, '.', '_' and '-', starting with a \
                         letter or digit",
                        echo(value)
                    ),
                );
                return None;
            }
            Some(name)
        });
        let sources = self.required(map, path, "sources").and_then(|value| {
            let at = join(path, "sources");
            let sources = self.list(value, &at, Self::source)?;
            if sources.is_empty() {
                self.problem(&at, "names no source");
            }
            Some(sources)
        });
        let checkouts = self
            .required(map, path, "checkouts")
            .and_then(|value| self.checkouts(value, &join(path, "checkouts")));
        let session_prefix = match Self::get(map, "session_prefix") {
            Some(value) => self
                .session_prefix(value, &join(path, "session_prefix"))
                .map(Some),
            None => Some(None),
        };
        // Absent, the port is filled by the instance's index once every instance is read
        // (`Reader::config`); until then, the first instance's.
        let dashboard_port = match Self::get(map, "dashboard_port") {
            Some(value) => self.port(value, &join(path, "dashboard_port")),
            None => Some(dashboard_port(0)),
        };
        let home = self.home;
        let place =
            |reader: &mut Self, key: &str, default: &dyn Fn(&str) -> PathBuf| match Self::get(
                map, key,
            ) {
                Some(value) => reader.path(value, &join(path, key)),
                None => name.as_deref().map(|name| text_of(&default(name))),
            };
        let records = place(self, "records", &|name| {
            home.join(".b10x/conductor").join(name).join("records")
        });
        let state = place(self, "state", &|name| {
            home.join(".b10x/conductor").join(name).join("state")
        });
        let cache = place(self, "cache", &|name| {
            home.join(".cache/b10x/conductor").join(name)
        });
        // Each role's session name is derived from the prefix, and a written one is held to it;
        // a prefix that is no prefix has been named already, and nothing is derived from it.
        let roles = match Self::get(map, "roles") {
            Some(value) => self
                .list(value, &join(path, "roles"), Self::role)
                .map(|roles| match &session_prefix {
                    Some(prefix) => {
                        self.session_names(roles, prefix.as_ref(), &join(path, "roles"))
                    }
                    None => roles,
                }),
            None => Some(default_roles(session_prefix.clone().flatten().as_ref())),
        };
        let confinement = match Self::get(map, "confinement") {
            Some(value) => self.confinement(value, &join(path, "confinement")),
            None => Some(default_confinement()),
        };
        // Every session of the instance runs inside the instance's box, so a role asks for no
        // more than it (`story:session-envelope-model`).
        if let (Some(roles), Some(confinement)) = (&roles, &confinement) {
            self.within(roles, &confinement.limits, &join(path, "roles"));
        }
        let controllers = match Self::get(map, "controllers") {
            Some(value) => self.controllers(value, &join(path, "controllers")),
            None => Some(default_controllers()),
        };
        let repositories = match Self::get(map, "repositories") {
            Some(value) => self.list(value, &join(path, "repositories"), Self::repository),
            None => Some(Vec::new()),
        };
        let cadence = match Self::get(map, "cadence") {
            Some(value) => self.cadence(value, &join(path, "cadence")),
            None => Some(default_cadence()),
        };
        let thresholds = match Self::get(map, "thresholds") {
            Some(value) => self.thresholds(value, &join(path, "thresholds")),
            None => Some(default_thresholds()),
        };
        let retention = match Self::get(map, "retention") {
            Some(value) => self.retention(value, &join(path, "retention")),
            None => Some(default_retention()),
        };
        let reports = match Self::get(map, "reports") {
            Some(value) => self.list(value, &join(path, "reports"), Self::report),
            None => Some(Vec::new()),
        };
        let authority = match Self::get(map, "authority") {
            Some(value) => self.authority(value, &join(path, "authority")),
            None => Some(default_authority()),
        };
        let usage = match Self::get(map, "usage") {
            Some(value) => self.usage(value, &join(path, "usage")),
            None => Some(no_usage()),
        };
        let operator = match Self::get(map, "operator") {
            Some(value) => self.text(value, &join(path, "operator")).map(Some),
            None => Some(None),
        };
        let catalog = match Self::get(map, "catalog") {
            Some(value) => self.catalog(value, &join(path, "catalog")).map(Some),
            None => Some(None),
        };
        let secret_syncs = match Self::get(map, "secret_syncs") {
            Some(value) => self.secret_syncs(value, &join(path, "secret_syncs")),
            None => Some(Vec::new()),
        };
        let gates_policy = match Self::get(map, "gates_policy") {
            Some(value) => self.path(value, &join(path, "gates_policy")).map(Some),
            None => Some(None),
        };
        // The conductor that serves the instance is derived once every instance is read
        // (`Reader::conductors`); until then, its own.
        let conductor = ConductorSession {
            served_by: None,
            session_name: None,
        };
        Some(Instance {
            conductor,
            name: InstanceName(name?),
            session_prefix: session_prefix?,
            dashboard_port: dashboard_port?,
            sources: sources?,
            checkouts: checkouts?,
            records: records?,
            state: state?,
            cache: cache?,
            roles: roles?,
            confinement: confinement?,
            controllers: controllers?,
            repositories: repositories?,
            cadence: cadence?,
            thresholds: thresholds?,
            retention: retention?,
            reports: reports?,
            authority: authority?,
            usage: usage?,
            operator: operator?,
            catalog: catalog?,
            secret_syncs: secret_syncs?,
            gates_policy: gates_policy?,
        })
    }

    /// One source: exactly one of `github`, `local` and `gitlab`, and for a `local` or `gitlab`
    /// source its `exclude`, empty when absent; for a `gitlab` source its `host`, [`GITLAB_HOST`]
    /// when absent (`story:gitlab-source-host`).
    fn source(&mut self, value: &Value, path: &str) -> Option<Source> {
        let map = self.mapping(value, path, &SOURCE_KEYS)?;
        let kinds: Vec<(&str, &Value)> = SOURCE_KINDS
            .iter()
            .filter_map(|kind| Self::get(map, kind).map(|value| (*kind, value)))
            .collect();
        let exclude = Self::get(map, "exclude");
        let host = Self::get(map, "host");
        if host.is_some() && !matches!(kinds.as_slice(), [("gitlab", _)] | []) {
            self.problem(
                &join(path, "host"),
                "only a gitlab source takes a host: it names the GitLab forge the group is on",
            );
            return None;
        }
        match kinds.as_slice() {
            [("github", owner)] => {
                if exclude.is_some() {
                    self.problem(
                        &join(path, "exclude"),
                        "a github source takes no exclude: GitHub lists its owner's \
                         repositories, and the collectors that read that list do not filter it",
                    );
                }
                let owner = self.text(owner, &join(path, "github"))?;
                if exclude.is_some() {
                    return None;
                }
                Some(Source::GitHub(GitHubSource { owner }))
            }
            [("local", directory)] => {
                let directory = self.path(directory, &join(path, "local"));
                let exclude = self.exclude(exclude, &join(path, "exclude"));
                Some(Source::Local(LocalSource {
                    path: directory?,
                    exclude: exclude?,
                }))
            }
            [("gitlab", group)] => {
                let group = self.text(group, &join(path, "gitlab"));
                let host = self.host(host, &join(path, "host"));
                let exclude = self.exclude(exclude, &join(path, "exclude"));
                Some(Source::GitLab(GitLabSource {
                    group: group?,
                    host: host?,
                    exclude: exclude?,
                }))
            }
            [] => {
                if map.keys().all(|key| SOURCE_KEYS.contains(&key.as_str())) {
                    self.problem(path, "names none of github, local and gitlab");
                }
                None
            }
            _ => {
                let named: Vec<&str> = kinds.iter().map(|(kind, _)| *kind).collect();
                self.problem(
                    path,
                    format!(
                        "names {}; a source is one of github, local and gitlab",
                        named.join(" and ")
                    ),
                );
                None
            }
        }
    }

    /// A `gitlab` source's `host`: a host name, letters, digits, `.` and `-`, with an optional
    /// `:<port>`, and no scheme or path. Absent, it is [`GITLAB_HOST`].
    fn host(&mut self, value: Option<&Value>, path: &str) -> Option<String> {
        let Some(value) = value else {
            return Some(GITLAB_HOST.to_owned());
        };
        let host = self.text(value, path)?;
        let (name, port) = match host.split_once(':') {
            Some((name, port)) => (name, Some(port)),
            None => (host.as_str(), None),
        };
        let name_ok = !name.is_empty()
            && name.split('.').all(|label| {
                !label.is_empty()
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            });
        let port_ok = port.is_none_or(|port| {
            !port.is_empty()
                && port.chars().all(|c| c.is_ascii_digit())
                && port.parse::<u16>().is_ok_and(|port| port > 0)
        });
        if name_ok && port_ok {
            Some(host)
        } else {
            self.problem(
                path,
                "is not a host name: write the forge's host without a scheme or a path, such as \
                 gitlab.example.com",
            );
            None
        }
    }

    /// A source's `exclude`: a list of paths under its root, each relative, its steps neither
    /// empty nor `.` nor `..`. Absent, it is empty.
    fn exclude(&mut self, value: Option<&Value>, path: &str) -> Option<Vec<String>> {
        let Some(value) = value else {
            return Some(Vec::new());
        };
        self.list(value, path, |reader, value, at| {
            reader.relative(
                value,
                at,
                "is not a path under the root: write it relative, such as group or \
                 group/repository, without `.` or `..`",
            )
        })
    }

    /// A relative path: text that does not start with `/`, its steps neither empty nor `.` nor
    /// `..`; otherwise the problem `refusal` after the value.
    fn relative(&mut self, value: &Value, path: &str, refusal: &str) -> Option<String> {
        let entry = self.text(value, path)?;
        let relative = !entry.starts_with('/')
            && entry
                .split('/')
                .all(|step| !step.is_empty() && step != "." && step != "..");
        if relative {
            Some(entry)
        } else {
            self.problem(path, format!("{} {refusal}", echo(value)));
            None
        }
    }

    /// `conductor.config.Catalog`: a repository under the checkouts root and a directory of it,
    /// each a relative path, and how an entry's stem spells a name, `plain` when absent.
    fn catalog(&mut self, value: &Value, path: &str) -> Option<Catalog> {
        let map = self.mapping(value, path, &CATALOG_KEYS)?;
        let repository = self.required(map, path, "repository").and_then(|value| {
            self.relative(
                value,
                &join(path, "repository"),
                "is not a repository under the checkouts root: write its path under the root, \
                 such as registry or group/registry, without `.` or `..`",
            )
        });
        let directory = self.required(map, path, "path").and_then(|value| {
            self.relative(
                value,
                &join(path, "path"),
                "is not a directory of the catalog repository: write it relative to the \
                 repository's top, such as catalog/entries, without `.` or `..`",
            )
        });
        let names = match Self::get(map, "names") {
            Some(value) => self.word(value, &join(path, "names"), &CATALOG_NAMES),
            None => Some(CatalogNames::Plain),
        };
        Some(Catalog {
            repository: repository?,
            path: directory?,
            names: names?,
        })
    }

    /// The instance's `secret_syncs` (`story:policy-secret-drift`): each a
    /// `conductor.config.SecretSync`, no two of one name.
    fn secret_syncs(&mut self, value: &Value, path: &str) -> Option<Vec<SecretSync>> {
        // Two syncs of one name are named whether or not either converts.
        let mut named: Vec<&str> = Vec::new();
        let mut distinct = true;
        for (index, item) in value.as_array().into_iter().flatten().enumerate() {
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            if named.contains(&name) {
                self.problem(
                    &join(&format!("{path}[{index}]"), "name"),
                    format!("{name:?} names an earlier sync too; give each sync its own name"),
                );
                distinct = false;
            }
            named.push(name);
        }
        let syncs = self.list(value, path, Self::secret_sync)?;
        distinct.then_some(syncs)
    }

    /// `conductor.config.SecretSync`: a name, a source, one or more secrets, the command that
    /// publishes the source, and the one that shows what it would publish, when given.
    fn secret_sync(&mut self, value: &Value, path: &str) -> Option<SecretSync> {
        let map = self.mapping(value, path, &SECRET_SYNC_KEYS)?;
        let name = self
            .required(map, path, "name")
            .and_then(|value| self.text(value, &join(path, "name")));
        let source = self
            .required(map, path, "source")
            .and_then(|value| self.secret_source(value, &join(path, "source")));
        let secrets = self.required(map, path, "secrets").and_then(|value| {
            let at = join(path, "secrets");
            let secrets = self.list(value, &at, Self::forge_secret)?;
            if secrets.is_empty() {
                self.problem(&at, "names no secret; a sync feeds one or more");
                return None;
            }
            Some(secrets)
        });
        let command = self
            .required(map, path, "command")
            .and_then(|value| self.text(value, &join(path, "command")));
        let dry_run = match Self::get(map, "dry_run") {
            Some(value) => self.text(value, &join(path, "dry_run")).map(Some),
            None => Some(None),
        };
        Some(SecretSync {
            name: name?,
            source: source?,
            secrets: secrets?,
            command: command?,
            dry_run: dry_run?,
        })
    }

    /// `conductor.config.SecretSource`: a repository under the checkouts root and a file of it,
    /// each a relative path, and the branch, [`SECRET_SOURCE_BRANCH`] when absent.
    fn secret_source(&mut self, value: &Value, path: &str) -> Option<SecretSource> {
        let map = self.mapping(value, path, &SECRET_SOURCE_KEYS)?;
        let repository = self.required(map, path, "repository").and_then(|value| {
            self.relative(
                value,
                &join(path, "repository"),
                "is not a repository under the checkouts root: write its path under the root, \
                 such as policies or group/policies, without `.` or `..`",
            )
        });
        let file = self.required(map, path, "path").and_then(|value| {
            self.relative(
                value,
                &join(path, "path"),
                "is not a file of the source repository: write it relative to the repository's \
                 top, such as ci/policy.json, without `.` or `..`",
            )
        });
        let branch = match Self::get(map, "branch") {
            Some(value) => self.text(value, &join(path, "branch")),
            None => Some(SECRET_SOURCE_BRANCH.to_owned()),
        };
        Some(SecretSource {
            repository: repository?,
            branch: branch?,
            path: file?,
        })
    }

    /// `conductor.config.ForgeSecret`: an owner's secret, or its repository's when one is named.
    fn forge_secret(&mut self, value: &Value, path: &str) -> Option<ForgeSecret> {
        let map = self.mapping(value, path, &FORGE_SECRET_KEYS)?;
        let owner = self
            .required(map, path, "owner")
            .and_then(|value| self.text(value, &join(path, "owner")));
        let repository = match Self::get(map, "repository") {
            Some(value) => self.text(value, &join(path, "repository")).map(Some),
            None => Some(None),
        };
        let name = self
            .required(map, path, "name")
            .and_then(|value| self.text(value, &join(path, "name")));
        Some(ForgeSecret {
            owner: owner?,
            repository: repository?,
            name: name?,
        })
    }

    fn checkouts(&mut self, value: &Value, path: &str) -> Option<Checkouts> {
        let map = self.mapping(value, path, &CHECKOUT_KEYS)?;
        let root = self
            .required(map, path, "root")
            .and_then(|value| self.path(value, &join(path, "root")));
        let trees = self
            .required(map, path, "trees")
            .and_then(|value| self.path(value, &join(path, "trees")));
        Some(Checkouts {
            root: root?,
            trees: trees?,
        })
    }

    fn role(&mut self, value: &Value, path: &str) -> Option<Role> {
        let map = self.mapping(value, path, &ROLE_KEYS)?;
        let role = self
            .required(map, path, "role")
            .and_then(|value| self.text(value, &join(path, "role")));
        let harness = self.required(map, path, "harness").and_then(|value| {
            self.word(
                value,
                &join(path, "harness"),
                &[("claude", Harness::Claude), ("codex", Harness::Codex)],
            )
        });
        let model = self.required(map, path, "model").and_then(|value| {
            let at = join(path, "model");
            let text = self.text(value, &at)?;
            self.command_word(text, &at)
        });
        let agent = match Self::get(map, "agent") {
            Some(value) => {
                let at = join(path, "agent");
                self.text(value, &at)
                    .and_then(|text| self.command_word(text, &at))
                    .map(Some)
            }
            None => Some(None),
        };
        let settings = match Self::get(map, "settings") {
            Some(value) => {
                let at = join(path, "settings");
                self.path(value, &at)
                    .and_then(|text| self.command_word(text, &at))
                    .map(Some)
            }
            None => Some(None),
        };
        let profile = match Self::get(map, "profile") {
            Some(value) => {
                let at = join(path, "profile");
                self.path(value, &at)
                    .and_then(|text| self.command_word(text, &at))
                    .map(Some)
            }
            None => Some(None),
        };
        // A controller starts with `--setting-sources project,local`, which loads no user-level
        // agent, so a controller role that names an agent names its profile as well
        // (`story:agent-needs-user-setting-source`).
        if role.as_deref() == Some(CONTROLLER)
            && Self::get(map, "agent").is_some()
            && Self::get(map, "profile").is_none()
        {
            self.problem(
                &join(path, "profile"),
                "is absent beside an agent: a controller starts from its profile, the file its \
                 session starts with through --append-system-prompt-file, never from an agent, \
                 because --setting-sources project,local loads no user-level agent",
            );
        }
        // As the file writes it; the instance holds it to the derived name
        // (`Reader::session_names`).
        let session_name = match Self::get(map, "session_name") {
            Some(value) => self
                .text(value, &join(path, "session_name"))
                .map(|name| Some(SessionName(name))),
            None => Some(None),
        };
        // The tools the guard allows the role's sessions beyond its built-in read-only allowlist
        // (`story:guard-all-tools`): each a tool name as the harness gives it. Absent, none.
        let read_only_tools = match Self::get(map, "read_only_tools") {
            Some(value) => self.list(value, &join(path, "read_only_tools"), |reader, value, at| {
                let name = reader.text(value, at)?;
                if name.trim().is_empty() || name.chars().any(char::is_whitespace) {
                    reader.problem(
                        at,
                        format!("{} is not a tool name: write it as the harness names it, such as mcp__server__tool", echo(value)),
                    );
                    None
                } else {
                    Some(name)
                }
            }),
            None => Some(Vec::new()),
        };
        // The envelope the role's sessions ask for (`story:session-envelope-model`).
        let limits = match Self::get(map, "limits") {
            Some(value) => self.limits(value, &join(path, "limits")),
            None => Some(no_limits()),
        };
        let writable = match Self::get(map, "writable") {
            Some(value) => self.list(value, &join(path, "writable"), Self::writable),
            None => Some(default_writable(role.as_deref().unwrap_or_default())),
        };
        let network = match Self::get(map, "network") {
            Some(value) => self.word(value, &join(path, "network"), &NETWORKS),
            None => Some(Network::Open),
        };
        // The environment variables that carry the forge tokens the role's sessions may use
        // (`story:sandbox-spec`): each a variable name. Absent, none.
        let forge_token_env = match Self::get(map, "forge_token_env") {
            Some(value) => self.list(value, &join(path, "forge_token_env"), Self::variable_name),
            None => Some(Vec::new()),
        };
        Some(Role {
            role: role?,
            harness: harness?,
            model: model?,
            agent: agent?,
            settings: settings?,
            profile: profile?,
            session_name: session_name?,
            read_only_tools: read_only_tools?,
            limits: limits?,
            writable: writable?,
            network: network?,
            forge_token_env: forge_token_env?,
        })
    }

    /// One entry of a role's `forge_token_env`: an environment variable name, capital letters,
    /// digits and `_`, not starting with a digit.
    fn variable_name(&mut self, value: &Value, path: &str) -> Option<String> {
        let name = self.text(value, path)?;
        let well_formed = name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            && !name.starts_with(|c: char| c.is_ascii_digit());
        if well_formed {
            Some(name)
        } else {
            self.problem(
                path,
                format!(
                    "{} is not an environment variable name: write capital letters, digits and \
                     `_`, not starting with a digit, such as GH_TOKEN",
                    echo(value)
                ),
            );
            None
        }
    }

    /// One entry of a role's `writable`: a place's word ([`PLACES`]), kept as it is, or a path,
    /// absolute or under `~`, made absolute.
    fn writable(&mut self, value: &Value, path: &str) -> Option<Writable> {
        let text = self.text(value, path)?;
        if PLACES.contains(&text.as_str()) {
            return Some(Writable(text));
        }
        if text.starts_with('/') || text == "~" || text.starts_with("~/") {
            return self.path(value, path).map(Writable);
        }
        self.problem(
            path,
            format!(
                "{} is neither a place nor a path: write one of {}, or a path absolute (/…) or \
                 under the home directory (~/…)",
                echo(value),
                PLACES.join(", ")
            ),
        );
        None
    }

    /// `conductor.config.Limits`, its invariants held: a `memory_max` of 1G or more, a
    /// `cpu_weight` from 1 to 10000 and a `tasks_max` of 1 or more, each absent when not written.
    fn limits(&mut self, value: &Value, path: &str) -> Option<Limits> {
        let map = self.mapping(value, path, &LIMIT_KEYS)?;
        let memory_max = match Self::get(map, "memory_max") {
            Some(value) => {
                let at = join(path, "memory_max");
                match self.size(value, &at) {
                    Some(size) if size.0 >= 1 => Some(Some(size)),
                    Some(_) => {
                        self.problem(&at, "0G sets no memory at all: write 1G or more");
                        None
                    }
                    None => None,
                }
            }
            None => Some(None),
        };
        let count = |reader: &mut Self, key: &str, valid: &dyn Fn(i64) -> bool, rule: &str| {
            let Some(value) = Self::get(map, key) else {
                return Some(None);
            };
            let count = value.as_i64().filter(|count| valid(*count));
            if count.is_none() {
                reader.problem(
                    &join(path, key),
                    format!(
                        "{} is not a {key}: write a whole number {rule}",
                        echo(value)
                    ),
                );
            }
            count.map(Some)
        };
        let cpu_weight = count(
            self,
            "cpu_weight",
            &|weight| CPU_WEIGHTS.contains(&weight),
            "from 1 to 10000",
        );
        let tasks_max = count(self, "tasks_max", &|tasks| tasks >= 1, "of 1 or more");
        Some(Limits {
            memory_max: memory_max?,
            cpu_weight: cpu_weight?,
            tasks_max: tasks_max?,
        })
    }

    /// `conductor.config.Confinement`: its `mode`, `report` when absent; its `provider`,
    /// `systemd` when absent; and its `limits`, none when absent.
    fn confinement(&mut self, value: &Value, path: &str) -> Option<Confinement> {
        let map = self.mapping(value, path, &CONFINEMENT_KEYS)?;
        let defaults = default_confinement();
        let mode = match Self::get(map, "mode") {
            Some(value) => self.word(value, &join(path, "mode"), &MODES),
            None => Some(defaults.mode),
        };
        let provider = match Self::get(map, "provider") {
            Some(value) => self.word(value, &join(path, "provider"), &PROVIDERS),
            None => Some(defaults.provider),
        };
        let limits = match Self::get(map, "limits") {
            Some(value) => self.limits(value, &join(path, "limits")),
            None => Some(defaults.limits),
        };
        Some(Confinement {
            mode: mode?,
            provider: provider?,
            limits: limits?,
        })
    }

    /// Each limit of each of `roles` that lies above `instance`'s limit of the same key, where
    /// the instance sets one, is a problem by its path under `path`, the instance's `roles`.
    fn within(&mut self, roles: &[Role], instance: &Limits, path: &str) {
        let gibibytes = |size: Option<&Gibibytes>| size.map(|size| size.0);
        for (index, role) in roles.iter().enumerate() {
            for (key, own, cap, unit) in [
                (
                    "memory_max",
                    gibibytes(role.limits.memory_max.as_ref()),
                    gibibytes(instance.memory_max.as_ref()),
                    "G",
                ),
                (
                    "cpu_weight",
                    role.limits.cpu_weight,
                    instance.cpu_weight,
                    "",
                ),
                ("tasks_max", role.limits.tasks_max, instance.tasks_max, ""),
            ] {
                if let (Some(own), Some(cap)) = (own, cap)
                    && own > cap
                {
                    self.problem(
                        &format!("{path}[{index}].limits.{key}"),
                        format!(
                            "{own}{unit} is above confinement.limits.{key}, {cap}{unit}: every \
                             session of the instance runs inside the instance's box, so a role \
                             asks for no more than the instance has"
                        ),
                    );
                }
            }
        }
    }

    /// `conductor.config.Controllers`, its invariants `max_working >= 1`,
    /// `max_subagents >= 1` and `max_open_reviews >= 0` held.
    fn controllers(&mut self, value: &Value, path: &str) -> Option<Controllers> {
        let map = self.mapping(value, path, &CONTROLLER_KEYS)?;
        let defaults = default_controllers();
        let count = |reader: &mut Self, key: &str, what: &str, least: i64, default: i64| {
            let Some(value) = Self::get(map, key) else {
                return Some(default);
            };
            let count = value.as_i64().filter(|count| *count >= least);
            if count.is_none() {
                reader.problem(
                    &join(path, key),
                    format!(
                        "{} is not a number of {what}: write a whole number of {least} or more",
                        echo(value)
                    ),
                );
            }
            count
        };
        let max_working = count(self, "max_working", "controllers", 1, defaults.max_working);
        let max_subagents = count(
            self,
            "max_subagents",
            "sub-agents",
            1,
            defaults.max_subagents,
        );
        let max_open_reviews = count(
            self,
            "max_open_reviews",
            "pull requests waiting for review (0 is no limit)",
            0,
            defaults.max_open_reviews,
        );
        Some(Controllers {
            max_working: max_working?,
            max_subagents: max_subagents?,
            max_open_reviews: max_open_reviews?,
        })
    }

    /// `conductor.config.Retention`, its invariant `snapshots >= 1` held.
    fn retention(&mut self, value: &Value, path: &str) -> Option<Retention> {
        let map = self.mapping(value, path, &RETENTION_KEYS)?;
        let Some(value) = Self::get(map, "snapshots") else {
            return Some(default_retention());
        };
        let snapshots = value.as_i64().filter(|count| *count >= 1);
        if snapshots.is_none() {
            self.problem(
                &join(path, "snapshots"),
                format!(
                    "{} is not a number of snapshots: write a whole number of 1 or more",
                    echo(value)
                ),
            );
        }
        Some(Retention {
            snapshots: snapshots?,
        })
    }

    fn repository(&mut self, value: &Value, path: &str) -> Option<RepositoryRule> {
        let map = self.mapping(value, path, &REPOSITORY_KEYS)?;
        let pattern = self
            .required(map, path, "match")
            .and_then(|value| self.text(value, &join(path, "match")));
        let activity = self.required(map, path, "activity").and_then(|value| {
            self.word(
                value,
                &join(path, "activity"),
                &[
                    ("active", Activity::Active),
                    ("inactive", Activity::Inactive),
                ],
            )
        });
        Some(RepositoryRule {
            r#match: pattern?,
            activity: activity?,
        })
    }

    fn cadence(&mut self, value: &Value, path: &str) -> Option<Cadence> {
        let map = self.mapping(value, path, &CADENCE_KEYS)?;
        let defaults = default_cadence();
        let cycle = match Self::get(map, "cycle") {
            Some(value) => self.duration(value, &join(path, "cycle")),
            None => Some(defaults.cycle),
        };
        let daily = match Self::get(map, "daily") {
            Some(value) => self.time_of_day(value, &join(path, "daily")),
            None => Some(defaults.daily),
        };
        let watch = match Self::get(map, "watch") {
            Some(value) => self.duration(value, &join(path, "watch")),
            None => Some(defaults.watch),
        };
        let ci = match Self::get(map, "ci") {
            Some(value) => self.duration(value, &join(path, "ci")),
            None => Some(defaults.ci),
        };
        Some(Cadence {
            cycle: cycle?,
            daily: daily?,
            watch: watch?,
            ci: ci?,
        })
    }

    /// `conductor.config.Thresholds`, its invariant `gate_resume > gate_stop` held.
    fn thresholds(&mut self, value: &Value, path: &str) -> Option<Thresholds> {
        let map = self.mapping(value, path, &THRESHOLD_KEYS)?;
        let defaults = default_thresholds();
        let context_handover = match Self::get(map, "context_handover") {
            Some(value) => self.tokens(value, &join(path, "context_handover")),
            None => Some(defaults.context_handover),
        };
        let disk_path = match Self::get(map, "disk_path") {
            Some(value) => self.path(value, &join(path, "disk_path")),
            None => Some(defaults.disk_path),
        };
        let size = |reader: &mut Self, key: &str, default: Gibibytes| match Self::get(map, key) {
            Some(value) => reader.size(value, &join(path, key)),
            None => Some(default),
        };
        let disk_low = size(self, "disk_low", defaults.disk_low);
        let disk_clear = size(self, "disk_clear", defaults.disk_clear);
        let disk_admit = size(self, "disk_admit", defaults.disk_admit);
        let build_slot = size(self, "build_slot", defaults.build_slot);
        let build_size = size(self, "build_size", defaults.build_size);
        let gate_stop = size(self, "gate_stop", defaults.gate_stop);
        let gate_resume = size(self, "gate_resume", defaults.gate_resume);
        let watchdog_every = match Self::get(map, "watchdog_every") {
            Some(value) => self.duration(value, &join(path, "watchdog_every")),
            None => Some(defaults.watchdog_every),
        };
        let silent_session = match Self::get(map, "silent_session") {
            Some(value) => self.duration(value, &join(path, "silent_session")),
            None => Some(defaults.silent_session),
        };
        let cpu_reserve = match Self::get(map, "cpu_reserve") {
            Some(value) => {
                let cores = value.as_i64().filter(|cores| *cores >= 0);
                if cores.is_none() {
                    self.problem(
                        &join(path, "cpu_reserve"),
                        format!(
                            "{} is not a number of cores: write a whole number of 0 or more",
                            echo(value)
                        ),
                    );
                }
                cores
            }
            None => Some(defaults.cpu_reserve),
        };
        if let (Some(stop), Some(resume)) = (&gate_stop, &gate_resume)
            && resume.0 <= stop.0
        {
            self.problem(
                &join(path, "gate_resume"),
                format!(
                    "{}G is not above gate_stop, {}G: the full-gate watchdog resumes a gate it \
                     stopped only from more free disk than it stopped it under",
                    resume.0, stop.0
                ),
            );
            return None;
        }
        Some(Thresholds {
            context_handover: context_handover?,
            disk_path: disk_path?,
            disk_low: disk_low?,
            disk_clear: disk_clear?,
            disk_admit: disk_admit?,
            build_slot: build_slot?,
            build_size: build_size?,
            gate_stop: gate_stop?,
            gate_resume: gate_resume?,
            watchdog_every: watchdog_every?,
            silent_session: silent_session?,
            cpu_reserve: cpu_reserve?,
        })
    }

    fn report(&mut self, value: &Value, path: &str) -> Option<Report> {
        let map = self.mapping(value, path, &REPORT_KEYS)?;
        let channel = self
            .required(map, path, "channel")
            .and_then(|value| self.text(value, &join(path, "channel")));
        let posted_as = self
            .required(map, path, "as")
            .and_then(|value| self.text(value, &join(path, "as")));
        let when = self
            .required(map, path, "when")
            .and_then(|value| self.list(value, &join(path, "when"), Self::text));
        Some(Report {
            channel: channel?,
            r#as: posted_as?,
            when: when?,
        })
    }

    /// `conductor.config.Usage` (`story:usage-probe`): `probe`, a command, and `watcher`, a
    /// systemd unit name ([`unit_name`]); each absent when not written.
    fn usage(&mut self, value: &Value, path: &str) -> Option<Usage> {
        let map = self.mapping(value, path, &USAGE_KEYS)?;
        let probe = match Self::get(map, "probe") {
            Some(value) => self.text(value, &join(path, "probe")).map(Some),
            None => Some(None),
        };
        let watcher = match Self::get(map, "watcher") {
            Some(value) => {
                let at = join(path, "watcher");
                self.text(value, &at).and_then(|unit| {
                    if unit_name(&unit) {
                        Some(Some(unit))
                    } else {
                        self.problem(
                            &at,
                            format!(
                                "{} is not a systemd unit name: letters, digits, ':', '_', '.', \
                                 '@' and '-', ending in a unit type such as .service",
                                echo(value)
                            ),
                        );
                        None
                    }
                })
            }
            None => Some(None),
        };
        Some(Usage {
            probe: probe?,
            watcher: watcher?,
        })
    }

    fn authority(&mut self, value: &Value, path: &str) -> Option<Authority> {
        let map = self.mapping(value, path, &AUTHORITY_KEYS)?;
        let defaults = default_authority();
        let deciders = [
            ("conductor", Decider::Conductor),
            ("operator", Decider::Operator),
        ];
        let class_c = match Self::get(map, "class_c") {
            Some(value) => self.word(value, &join(path, "class_c"), &deciders),
            None => Some(defaults.class_c),
        };
        let class_o = match Self::get(map, "class_o") {
            Some(value) => self.word(value, &join(path, "class_o"), &deciders),
            None => Some(defaults.class_o),
        };
        Some(Authority {
            class_c: class_c?,
            class_o: class_o?,
        })
    }
}

/// A duration as the file writes it, `2h`, as the ISO 8601 duration `Duration` carries: `PT2H`.
/// `None` unless it is a whole number above 0 with `s`, `m`, `h` or `d`.
#[must_use]
pub fn parse_duration(text: &str) -> Option<Duration> {
    let unit = text.chars().last()?;
    let number = &text[..text.len() - unit.len_utf8()];
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let count: u64 = number.parse().ok().filter(|count| *count > 0)?;
    let iso = match unit {
        's' => format!("PT{count}S"),
        'm' => format!("PT{count}M"),
        'h' => format!("PT{count}H"),
        'd' => format!("P{count}D"),
        _ => return None,
    };
    Some(Duration(iso))
}

/// A duration's count and unit (`s`, `m`, `h` or `d`), for one [`parse_duration`] answers.
fn unit_of(duration: &Duration) -> Option<(u64, char)> {
    let iso = duration.0.as_str();
    let (body, unit) = if let Some(days) = iso
        .strip_prefix('P')
        .and_then(|rest| rest.strip_suffix('D'))
    {
        (days, 'd')
    } else {
        let rest = iso.strip_prefix("PT")?;
        let unit = rest.chars().last()?;
        let unit = match unit {
            'S' => 's',
            'M' => 'm',
            'H' => 'h',
            _ => return None,
        };
        (&rest[..rest.len() - 1], unit)
    };
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((body.parse().ok()?, unit))
}

/// The seconds of a duration [`parse_duration`] answers; `None` for any other.
#[must_use]
pub fn seconds(duration: &Duration) -> Option<u64> {
    let (count, unit) = unit_of(duration)?;
    let factor = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        _ => 86_400,
    };
    count.checked_mul(factor)
}

/// A duration as the file writes it.
fn written(duration: &Duration) -> String {
    unit_of(duration).map_or_else(
        || duration.0.clone(),
        |(count, unit)| format!("{count}{unit}"),
    )
}

/// A count of tokens as the file writes it: thousands with `k` where it is a whole thousand.
fn written_tokens(tokens: &Tokens) -> String {
    if tokens.0 != 0 && tokens.0 % 1000 == 0 {
        format!("{}k", tokens.0 / 1000)
    } else {
        tokens.0.to_string()
    }
}

/// `instance` as a config file of one instance, which is its `default`, its keys in the
/// specification's order.
#[must_use]
pub fn document(instance: &Instance) -> Yaml {
    fn mapping<const N: usize>(entries: [(&str, Yaml); N]) -> Yaml {
        let mut map = Mapping::new();
        for (key, value) in entries {
            map.insert(Yaml::from(key), value);
        }
        Yaml::Mapping(map)
    }
    fn text(value: &str) -> Yaml {
        Yaml::from(value)
    }
    /// An optional text, null when it is absent.
    fn optional(value: Option<&String>) -> Yaml {
        value.map_or(Yaml::Null, |value| text(value))
    }
    /// A number.
    fn number(value: i64) -> Yaml {
        Yaml::Number(serde_yaml::Number::from(value))
    }
    /// A box's limits, each absent one null.
    fn limits(limits: &Limits) -> Yaml {
        mapping([
            (
                "memory_max",
                limits
                    .memory_max
                    .as_ref()
                    .map_or(Yaml::Null, |size| text(&format!("{}G", size.0))),
            ),
            ("cpu_weight", limits.cpu_weight.map_or(Yaml::Null, number)),
            ("tasks_max", limits.tasks_max.map_or(Yaml::Null, number)),
        ])
    }
    /// A source's mapping with its `exclude`, written only when it holds an entry.
    fn excluding(mut source: Yaml, exclude: &[String]) -> Yaml {
        if let Yaml::Mapping(map) = &mut source
            && !exclude.is_empty()
        {
            map.insert(
                Yaml::from("exclude"),
                Yaml::Sequence(exclude.iter().map(|entry| text(entry)).collect()),
            );
        }
        source
    }
    let word = |harness: Harness| match harness {
        Harness::Claude => "claude",
        Harness::Codex => "codex",
    };
    let decider = |decider: Decider| match decider {
        Decider::Conductor => "conductor",
        Decider::Operator => "operator",
    };
    let sources = instance
        .sources
        .iter()
        .map(|source| match source {
            Source::GitHub(github) => mapping([("github", text(&github.owner))]),
            Source::Local(local) => {
                excluding(mapping([("local", text(&local.path))]), &local.exclude)
            }
            Source::GitLab(gitlab) => {
                let source = mapping([
                    ("gitlab", text(&gitlab.group)),
                    ("host", text(&gitlab.host)),
                ]);
                excluding(source, &gitlab.exclude)
            }
        })
        .collect();
    let roles = instance
        .roles
        .iter()
        .map(|role| {
            mapping([
                ("role", text(&role.role)),
                ("harness", text(word(role.harness))),
                ("model", text(&role.model.0)),
                ("agent", optional(role.agent.as_ref().map(|agent| &agent.0))),
                (
                    "settings",
                    optional(role.settings.as_ref().map(|settings| &settings.0)),
                ),
                (
                    "profile",
                    optional(role.profile.as_ref().map(|profile| &profile.0)),
                ),
                (
                    "session_name",
                    optional(role.session_name.as_ref().map(|name| &name.0)),
                ),
                (
                    "read_only_tools",
                    Yaml::Sequence(role.read_only_tools.iter().map(|tool| text(tool)).collect()),
                ),
                ("limits", limits(&role.limits)),
                (
                    "writable",
                    Yaml::Sequence(role.writable.iter().map(|place| text(&place.0)).collect()),
                ),
                (
                    "network",
                    text(match role.network {
                        Network::Open => "open",
                    }),
                ),
                (
                    "forge_token_env",
                    Yaml::Sequence(role.forge_token_env.iter().map(|name| text(name)).collect()),
                ),
            ])
        })
        .collect();
    let repositories = instance
        .repositories
        .iter()
        .map(|rule| {
            mapping([
                ("match", text(&rule.r#match)),
                (
                    "activity",
                    text(match rule.activity {
                        Activity::Active => "active",
                        Activity::Inactive => "inactive",
                    }),
                ),
            ])
        })
        .collect();
    let reports = instance
        .reports
        .iter()
        .map(|report| {
            mapping([
                ("channel", text(&report.channel)),
                ("as", text(&report.r#as)),
                (
                    "when",
                    Yaml::Sequence(report.when.iter().map(|when| text(when)).collect()),
                ),
            ])
        })
        .collect();
    let cadence = &instance.cadence;
    let thresholds = &instance.thresholds;
    let size = |size: &Gibibytes| text(&format!("{}G", size.0));
    let shown = mapping([
        ("name", text(&instance.name.0)),
        (
            "session_prefix",
            optional(instance.session_prefix.as_ref().map(|prefix| &prefix.0)),
        ),
        (
            "dashboard_port",
            Yaml::Number(serde_yaml::Number::from(instance.dashboard_port)),
        ),
        ("sources", Yaml::Sequence(sources)),
        (
            "checkouts",
            mapping([
                ("root", text(&instance.checkouts.root)),
                ("trees", text(&instance.checkouts.trees)),
            ]),
        ),
        ("records", text(&instance.records)),
        ("state", text(&instance.state)),
        ("cache", text(&instance.cache)),
        ("roles", Yaml::Sequence(roles)),
        (
            "confinement",
            mapping([
                (
                    "mode",
                    text(match instance.confinement.mode {
                        ConfinementMode::Strict => "strict",
                        ConfinementMode::Report => "report",
                    }),
                ),
                (
                    "provider",
                    text(match instance.confinement.provider {
                        ConfinementProvider::Systemd => "systemd",
                        ConfinementProvider::None => "none",
                    }),
                ),
                ("limits", limits(&instance.confinement.limits)),
            ]),
        ),
        (
            "controllers",
            mapping([
                (
                    "max_working",
                    Yaml::Number(serde_yaml::Number::from(instance.controllers.max_working)),
                ),
                (
                    "max_subagents",
                    Yaml::Number(serde_yaml::Number::from(instance.controllers.max_subagents)),
                ),
                (
                    "max_open_reviews",
                    Yaml::Number(serde_yaml::Number::from(
                        instance.controllers.max_open_reviews,
                    )),
                ),
            ]),
        ),
        ("repositories", Yaml::Sequence(repositories)),
        (
            "cadence",
            mapping([
                ("cycle", text(&written(&cadence.cycle))),
                ("daily", text(&cadence.daily.0)),
                ("watch", text(&written(&cadence.watch))),
                ("ci", text(&written(&cadence.ci))),
            ]),
        ),
        (
            "thresholds",
            mapping([
                (
                    "context_handover",
                    text(&written_tokens(&thresholds.context_handover)),
                ),
                ("disk_path", text(&thresholds.disk_path)),
                ("disk_low", size(&thresholds.disk_low)),
                ("disk_clear", size(&thresholds.disk_clear)),
                ("disk_admit", size(&thresholds.disk_admit)),
                ("build_slot", size(&thresholds.build_slot)),
                ("build_size", size(&thresholds.build_size)),
                ("gate_stop", size(&thresholds.gate_stop)),
                ("gate_resume", size(&thresholds.gate_resume)),
                ("watchdog_every", text(&written(&thresholds.watchdog_every))),
                ("silent_session", text(&written(&thresholds.silent_session))),
                (
                    "cpu_reserve",
                    Yaml::Number(serde_yaml::Number::from(thresholds.cpu_reserve)),
                ),
            ]),
        ),
        (
            "retention",
            mapping([(
                "snapshots",
                Yaml::Number(serde_yaml::Number::from(instance.retention.snapshots)),
            )]),
        ),
        ("reports", Yaml::Sequence(reports)),
        (
            "authority",
            mapping([
                ("class_c", text(decider(instance.authority.class_c))),
                ("class_o", text(decider(instance.authority.class_o))),
            ]),
        ),
        (
            "usage",
            mapping([
                ("probe", optional(instance.usage.probe.as_ref())),
                ("watcher", optional(instance.usage.watcher.as_ref())),
            ]),
        ),
        ("operator", optional(instance.operator.as_ref())),
        (
            "catalog",
            instance.catalog.as_ref().map_or(Yaml::Null, |catalog| {
                let names = match catalog.names {
                    CatalogNames::Plain => "plain",
                    CatalogNames::Hex => "hex",
                };
                mapping([
                    ("repository", text(&catalog.repository)),
                    ("path", text(&catalog.path)),
                    ("names", text(names)),
                ])
            }),
        ),
        (
            "secret_syncs",
            Yaml::Sequence(
                instance
                    .secret_syncs
                    .iter()
                    .map(|sync| {
                        mapping([
                            ("name", text(&sync.name)),
                            (
                                "source",
                                mapping([
                                    ("repository", text(&sync.source.repository)),
                                    ("branch", text(&sync.source.branch)),
                                    ("path", text(&sync.source.path)),
                                ]),
                            ),
                            (
                                "secrets",
                                Yaml::Sequence(
                                    sync.secrets
                                        .iter()
                                        .map(|secret| {
                                            mapping([
                                                ("owner", text(&secret.owner)),
                                                (
                                                    "repository",
                                                    optional(secret.repository.as_ref()),
                                                ),
                                                ("name", text(&secret.name)),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                            ("command", text(&sync.command)),
                            ("dry_run", optional(sync.dry_run.as_ref())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("gates_policy", optional(instance.gates_policy.as_ref())),
        (
            "conductor",
            mapping([
                (
                    "served_by",
                    optional(instance.conductor.served_by.as_ref().map(|name| &name.0)),
                ),
                (
                    "session_name",
                    optional(instance.conductor.session_name.as_ref().map(|name| &name.0)),
                ),
            ]),
        ),
    ]);
    mapping([
        ("version", text(VERSION)),
        ("default", text(&instance.name.0)),
        ("instances", Yaml::Sequence(vec![shown])),
    ])
}

/// `document` as `path: value` lines, one per scalar and per empty list or mapping; a line break
/// inside a value is written as a space, so each stays one line.
fn lines(document: &Yaml, path: &str, out: &mut String) {
    match document {
        Yaml::Mapping(map) if !map.is_empty() => {
            for (key, value) in map {
                let key = key.as_str().unwrap_or_default();
                lines(value, &join(path, key), out);
            }
        }
        Yaml::Sequence(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                lines(item, &format!("{path}[{index}]"), out);
            }
        }
        Yaml::Mapping(_) => {
            let _ = writeln!(out, "{path}: {{}}");
        }
        Yaml::Sequence(_) => {
            let _ = writeln!(out, "{path}: []");
        }
        Yaml::String(text) => {
            let _ = writeln!(out, "{path}: {}", text.replace(['\r', '\n'], " "));
        }
        other => {
            let _ = writeln!(
                out,
                "{path}: {}",
                serde_yaml::to_string(other).unwrap_or_default().trim_end()
            );
        }
    }
}

/// Prints each problem of an invalid file on standard error, after `what`.
fn report(what: &str, invalid: &Invalid) {
    for problem in &invalid.problems {
        eprintln!("conductor: {what}: {}: {problem}", invalid.file.display());
    }
}

/// `conductor config validate`: reads the config file and converts it, and resolves the instance
/// `--instance` or [`INSTANCE_VARIABLE`] names, if one does. Exits 0 with one line on standard
/// output naming the file and its instances, or the default file looked for and that the
/// built-in defaults apply; or 1, with one line on standard error per problem, each named by its
/// YAML path. Writes nothing.
///
/// # Errors
///
/// A named file is not there or cannot be read, the home directory is not known, or the instance
/// named is not in the config.
pub fn validate(config: Option<&Path>, args: &ValidateConfigArgs) -> Result<ExitCode> {
    const WHAT: &str = "config validate";
    let environment = Environment::from_process().context(WHAT)?;
    let loaded = match load(config, &environment) {
        Ok(loaded) => loaded,
        Err(error) => {
            return match error.downcast::<Invalid>() {
                Ok(invalid) => {
                    report(WHAT, &invalid);
                    Ok(ExitCode::FAILURE)
                }
                Err(error) => Err(error.context(WHAT)),
            };
        }
    };
    if args.instance.is_some() || environment.instance.is_some() {
        select(&loaded, args.instance.as_deref(), &environment).context(WHAT)?;
    }
    // No role's settings pin another instance (`story:instance-isolation`); one that pins none is
    // a note, since its sessions start with it pinned.
    let (problems, notes) = pin_problems(&loaded.config);
    if loaded.read {
        for note in &notes {
            eprintln!(
                "conductor: {WHAT}: {}: note: {}: {}",
                loaded.path.display(),
                note.path,
                note.message
            );
        }
    }
    if loaded.read && !problems.is_empty() {
        report(
            WHAT,
            &Invalid {
                file: loaded.path.clone(),
                problems,
            },
        );
        return Ok(ExitCode::FAILURE);
    }
    let names: Vec<&str> = loaded
        .config
        .instances
        .iter()
        .map(|instance| instance.name.0.as_str())
        .collect();
    if loaded.read {
        println!(
            "{}: valid, {} instance(s): {}",
            loaded.path.display(),
            names.len(),
            names.join(", ")
        );
    } else {
        println!(
            "{}: no such file; the built-in defaults apply, instance {}",
            loaded.path.display(),
            names.join(", ")
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// `conductor config show`: the effective config of one instance ([`select`]), after defaults,
/// as a config file of that instance: `text`, one `path: value` line per value; `json`; or
/// `yaml`, which `config validate` takes back. An invalid file exits 1 as `config validate`
/// does. Writes nothing.
///
/// # Errors
///
/// `--format` is not one of text, json, yaml; a named file is not there or cannot be read; the
/// home directory is not known; or no one instance is selected.
pub fn show(config: Option<&Path>, args: &ShowConfigArgs) -> Result<ExitCode> {
    const WHAT: &str = "config show";
    let format = args.format.as_deref().unwrap_or("text");
    if !matches!(format, "text" | "json" | "yaml") {
        bail!("{WHAT}: --format {format:?} is not one of text, json, yaml");
    }
    let environment = Environment::from_process().context(WHAT)?;
    let loaded = match load(config, &environment) {
        Ok(loaded) => loaded,
        Err(error) => {
            return match error.downcast::<Invalid>() {
                Ok(invalid) => {
                    report(WHAT, &invalid);
                    Ok(ExitCode::FAILURE)
                }
                Err(error) => Err(error.context(WHAT)),
            };
        }
    };
    let instance = select(&loaded, args.instance.as_deref(), &environment).context(WHAT)?;
    let document = document(instance);
    let rendered = match format {
        "json" => serde_json::to_string_pretty(&document).context(WHAT)? + "\n",
        "yaml" => serde_yaml::to_string(&document).context(WHAT)?,
        _ => {
            let mut out = String::new();
            lines(&document, "", &mut out);
            out
        }
    };
    print!("{rendered}");
    Ok(ExitCode::SUCCESS)
}

/// The workspace file `conductor config workspace` writes, under the instance's records directory.
pub const WORKSPACE_FILE: &str = ".engineering/workspace.yaml";

/// The version of the workspace file.
const WORKSPACE_VERSION: &str = "aep.workspace/1";

/// `path` with every link resolved: the nearest of it and its ancestors that is there,
/// canonicalised, with the rest of `path` after it. A relative path is answered as it is.
fn physical(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path;
    loop {
        if let Ok(resolved) = fs::canonicalize(at) {
            return rest.iter().rev().fold(resolved, |out, part| out.join(part));
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                at = parent;
            }
            _ => return path.to_owned(),
        }
    }
}

/// `target` relative to `base`, both absolute: `..` for each step of `base` past what they
/// share, then the rest of `target`, always ending in `target`'s own name. A `target` that holds
/// `base` (the records, from their `.engineering/`) is `../../records`, never `..`: the blockers
/// collector takes a member's repository from its source's last component.
fn relative(base: &Path, target: &Path) -> Result<String> {
    let base: Vec<_> = base.components().collect();
    let target: Vec<_> = target.components().collect();
    let shared = base
        .iter()
        .zip(&target)
        .take_while(|(left, right)| left == right)
        .count()
        .min(target.len().saturating_sub(1));
    let mut out = PathBuf::new();
    for _ in shared..base.len() {
        out.push("..");
    }
    for part in &target[shared..] {
        out.push(part);
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out.to_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{} is not UTF-8", out.display()))
}

/// The name of each git checkout (a directory holding `.git`) directly under `dir` that has an
/// `origin` remote ([`has_origin`]), `yaml` the path of the config value that names it. A
/// directory that is not there holds none.
fn checkouts_under(dir: &Path, yaml: &str) -> Result<Vec<String>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("{yaml}: read {}", dir.display()));
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.with_context(|| format!("{yaml}: read {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() && path.join(".git").exists() && has_origin(&path) {
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| {
                anyhow!("{yaml}: the checkout {} has no UTF-8 name", path.display())
            })?;
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

/// Whether the checkout `dir` names an `origin` remote, read from its git config with no network
/// read. The collectors read each member at `origin/main`, so a local-only checkout is no member:
/// its `git fetch origin` fails the snapshot (conductor's NEED N-20261012-02).
fn has_origin(dir: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "--get", "remote.origin.url"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The members found so far: by name, the `source` and the YAML path of the value that gave it.
struct Found {
    /// `<records>/.engineering`, physical: what each `source` is relative to.
    base: PathBuf,
    members: BTreeMap<String, (String, String)>,
}

impl Found {
    /// Adds the checkout `dir` as a member named after its directory `name`
    /// ([`workspace_member_name`]), given by the config value at `yaml`. The same member twice is
    /// one; one name for two checkouts is refused, by both YAML paths.
    fn add(&mut self, name: &str, dir: &Path, yaml: &str) -> Result<()> {
        let name = &workspace_member_name(name)
            .ok_or_else(|| anyhow!("{yaml}: {} gives no workspace member name", dir.display()))?;
        let source = relative(&self.base, &physical(dir))?;
        match self.members.get(name) {
            Some((known, _)) if *known == source => Ok(()),
            Some((known, first)) => bail!(
                "{first} and {yaml} each give a workspace member {name}, {known} and {source} \
                 (relative to {}); a workspace names each member once",
                self.base.display()
            ),
            None => {
                self.members
                    .insert(name.to_owned(), (source, yaml.to_owned()));
                Ok(())
            }
        }
    }
}

/// The workspace member name of a checkout directory `name`: aep takes lower-case letters, digits
/// and hyphens only, so it is `name` lower-cased with every other character a hyphen, and without
/// a leading or trailing hyphen (`Example-Org.github.io` is `example-org-github-io`); `None` when
/// nothing is left. The collectors find a member by its source, never by this name.
#[must_use]
pub fn workspace_member_name(name: &str) -> Option<String> {
    let mapped: String = name
        .chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = mapped.trim_matches('-');
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// The members of `instance`'s workspace, by name, with no network read: each git checkout
/// directly under `checkouts.root` for a `github` source; each one directly under its `path`,
/// but those its `exclude` names, for a `local` source; for a `gitlab` source, each one under
/// `checkouts.root` whose `origin` remote is on its host and under its group, but those its
/// `exclude` names or holds, named by its path under the root (`story:gitlab-workspace`), the
/// checkouts the collectors read its projects in ([`crate::collect::gitlab::checkouts`]); and the
/// records directory. Each `source` is relative to
/// `<records>/.engineering/`. `index` is the instance's place in the config file, for the YAML
/// paths a refusal names.
///
/// # Errors
///
/// Two values give one name to two directories, a directory cannot be read, or a name or path
/// is not UTF-8; each names its YAML path.
pub fn workspace_members(instance: &Instance, index: usize) -> Result<Vec<WorkspaceMember>> {
    let records = Path::new(&instance.records);
    let mut found = Found {
        base: physical(&records.join(".engineering")),
        members: BTreeMap::new(),
    };
    each_checkout(instance, index, |name, dir, yaml| {
        found.add(name, dir, yaml)
    })?;
    let yaml = format!("instances[{index}].records");
    let name = physical(records)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{yaml}: {} has no UTF-8 name", records.display()))?;
    found.add(&name, records, &yaml)?;
    Ok(found
        .members
        .into_iter()
        .map(|(name, (source, _))| WorkspaceMember { name, source })
        .collect())
}

/// Every checkout of `instance`, the instance at `index` of its config file, each once, in path
/// order (`story:gates-hooks-fresh`): the members [`workspace_members`] writes, without the
/// records directory ([`each_checkout`]).
///
/// # Errors
///
/// A directory cannot be read, named by its YAML path.
pub fn instance_checkouts(instance: &Instance, index: usize) -> Result<Vec<PathBuf>> {
    let mut found = std::collections::BTreeSet::new();
    each_checkout(instance, index, |_, dir, _| {
        found.insert(dir.to_path_buf());
        Ok(())
    })?;
    Ok(found.into_iter().collect())
}

/// Calls `add` with the directory name (for a `gitlab` source, the path under the checkouts
/// root), the directory and the YAML path of the source of each checkout of `instance`, the
/// instance at `index` of its config file: each git checkout with an `origin` directly under
/// `checkouts.root` for a `github` source; each one directly under its `path`, but its `exclude`
/// entries, for a `local` source; and each one under `checkouts.root` whose `origin` is on a
/// `gitlab` source's host and under its group, but its `exclude` entries. A checkout two sources
/// name is given twice.
fn each_checkout(
    instance: &Instance,
    index: usize,
    mut add: impl FnMut(&str, &Path, &str) -> Result<()>,
) -> Result<()> {
    let root = Path::new(&instance.checkouts.root);
    for (place, source) in instance.sources.iter().enumerate() {
        let yaml = format!("instances[{index}].sources[{place}]");
        match source {
            Source::GitHub(_) => {
                let at = format!("instances[{index}].checkouts.root");
                for name in checkouts_under(root, &at)? {
                    add(&name, &root.join(&name), &yaml)?;
                }
            }
            Source::Local(local) => {
                let dir = Path::new(&local.path);
                for name in checkouts_under(dir, &join(&yaml, "local"))? {
                    let excluded = local
                        .exclude
                        .iter()
                        .any(|entry| Path::new(entry) == Path::new(&name));
                    if !excluded {
                        add(&name, &dir.join(&name), &yaml)?;
                    }
                }
            }
            Source::GitLab(gitlab) => {
                for checkout in crate::collect::gitlab::checkouts(
                    root,
                    &gitlab.host,
                    &gitlab.group,
                    &gitlab.exclude,
                ) {
                    add(&checkout.path, &checkout.dir, &yaml)?;
                }
            }
        }
    }
    Ok(())
}

/// One YAML scalar, quoted only where YAML needs it.
fn scalar(text: &str) -> String {
    serde_yaml::to_string(&Yaml::from(text))
        .map(|rendered| rendered.trim_end().to_owned())
        .unwrap_or_else(|_| Value::from(text).to_string())
}

/// The workspace file of `members`.
#[must_use]
pub fn workspace_text(members: &[WorkspaceMember]) -> String {
    let mut text = format!("version: {WORKSPACE_VERSION}\n");
    if members.is_empty() {
        text.push_str("members: []\n");
    } else {
        text.push_str("members:\n");
        for member in members {
            let _ = write!(
                text,
                "  - name: {}\n    source: {}\n",
                scalar(&member.name),
                scalar(&member.source)
            );
        }
    }
    text
}

/// The members a workspace file holds, by name, and its version. A file that is not there holds
/// none, at version none.
fn written_members(file: &Path) -> Result<(Option<String>, BTreeMap<String, String>)> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok((None, BTreeMap::new())),
        Err(error) => return Err(error).with_context(|| format!("read {}", file.display())),
    };
    let document: Yaml =
        serde_yaml::from_str(&text).with_context(|| format!("read {}", file.display()))?;
    let version = document["version"].as_str().map(str::to_owned);
    let mut members = BTreeMap::new();
    if let Some(listed) = document["members"].as_sequence() {
        for (place, member) in listed.iter().enumerate() {
            let (Some(name), Some(source)) = (member["name"].as_str(), member["source"].as_str())
            else {
                bail!(
                    "{}: members[{place}] has no `name` and `source`",
                    file.display()
                );
            };
            members.insert(name.to_owned(), source.to_owned());
        }
    }
    Ok((version, members))
}

/// One line per difference between the members of the config and those of the file.
fn differences(
    version: Option<&str>,
    written: &BTreeMap<String, String>,
    members: &[WorkspaceMember],
) -> Vec<String> {
    let mut lines = Vec::new();
    match version {
        Some(WORKSPACE_VERSION) => {}
        Some(other) => lines.push(format!("version {other}, not {WORKSPACE_VERSION}")),
        None => lines.push(format!("no version {WORKSPACE_VERSION}")),
    }
    for member in members {
        match written.get(&member.name) {
            None => lines.push(format!(
                "member {}: missing, the config gives source {}",
                member.name, member.source
            )),
            Some(source) if *source != member.source => lines.push(format!(
                "member {}: source {source}, the config gives {}",
                member.name, member.source
            )),
            Some(_) => {}
        }
    }
    for (name, source) in written {
        if !members.iter().any(|member| member.name == *name) {
            lines.push(format!("member {name}: source {source}, not in the config"));
        }
    }
    lines
}

/// `conductor config workspace`: writes `<records>/.engineering/workspace.yaml` of the instance
/// ([`select`]) from its config ([`workspace_members`]), replacing the file, and prints one
/// `member:` line per member and one `wrote:` line. With `--check` it writes nothing: it exits 0
/// with one line when the file holds exactly those members, else 1 with one line on standard
/// error per member that differs. A config file must name the instance, and its records
/// directory must be there.
///
/// # Errors
///
/// The config does not load or names no such instance, there is no config file, the records
/// directory is not a directory, [`workspace_members`] refuses, or the file cannot be read or
/// written.
pub fn workspace(config: Option<&Path>, args: &WriteWorkspaceArgs) -> Result<ExitCode> {
    const WHAT: &str = "config workspace";
    let environment = Environment::from_process().context(WHAT)?;
    let loaded = match load(config, &environment) {
        Ok(loaded) => loaded,
        Err(error) => {
            return match error.downcast::<Invalid>() {
                Ok(invalid) => {
                    report(WHAT, &invalid);
                    Ok(ExitCode::FAILURE)
                }
                Err(error) => Err(error.context(WHAT)),
            };
        }
    };
    if !loaded.read {
        bail!(
            "{WHAT}: no config file at {}; the workspace is written from an instance of one",
            loaded.path.display()
        );
    }
    let instance = select(&loaded, args.instance.as_deref(), &environment).context(WHAT)?;
    let index = loaded
        .config
        .instances
        .iter()
        .position(|other| other.name == instance.name)
        .unwrap_or_default();
    let records = Path::new(&instance.records);
    if !records.is_dir() {
        bail!(
            "{WHAT}: instances[{index}].records: {} is not a directory; `conductor init` writes \
             it",
            records.display()
        );
    }
    let members = workspace_members(instance, index).context(WHAT)?;
    let file = records.join(WORKSPACE_FILE);
    if args.check.unwrap_or(false) {
        let (version, written) = written_members(&file).context(WHAT)?;
        let lines = differences(version.as_deref(), &written, &members);
        if lines.is_empty() {
            println!("{}: up to date, {} members", file.display(), members.len());
            return Ok(ExitCode::SUCCESS);
        }
        for line in &lines {
            eprintln!("conductor: {WHAT}: {}: {line}", file.display());
        }
        eprintln!(
            "conductor: {WHAT}: {} differs from the config; `conductor config workspace` \
             rewrites it",
            file.display()
        );
        return Ok(ExitCode::FAILURE);
    }
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("{WHAT}: create {}", dir.display()))?;
    }
    fs::write(&file, workspace_text(&members))
        .with_context(|| format!("{WHAT}: write {}", file.display()))?;
    for member in &members {
        println!("member: {} {}", member.name, member.source);
    }
    println!("wrote: {}", file.display());
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_take_s_m_h_or_d_and_read_back() {
        for (written_text, iso, secs) in [
            ("180s", "PT180S", 180),
            ("15m", "PT15M", 900),
            ("2h", "PT2H", 7200),
            ("1d", "P1D", 86_400),
        ] {
            let duration = parse_duration(written_text).expect("a duration");
            assert_eq!(duration.0, iso);
            assert_eq!(seconds(&duration), Some(secs));
            assert_eq!(written(&duration), written_text);
        }
        for refused in ["", "h", "0s", "2", "2 h", "2H", "1.5h", "-1m", "2hh", "é"] {
            assert_eq!(parse_duration(refused), None, "{refused:?}");
        }
    }

    #[test]
    fn a_token_is_a_word_with_a_token_prefix() {
        for token in [
            "ghp_x",
            "github_pat_x",
            "glpat-x",
            "xoxb-1",
            "xoxp-1",
            "slack:xoxb-1",
            "https://ghp_x@host",
            "~/glpat-x",
        ] {
            assert!(looks_like_token(token), "{token}");
        }
        for plain in [
            "slack:#example-channel",
            "~/xoxo",
            "opus",
            "example-org",
            "aghp_x",
            "xox",
            "xoxb",
        ] {
            assert!(!looks_like_token(plain), "{plain}");
        }
        assert_eq!(redact("at ghp_abc, then"), "at <redacted>, then");
    }
}
