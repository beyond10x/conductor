//! `conductor supervisor watch` (story:supervisor-watch-command, issues #254 and #247): the host
//! supervisor's one background command, in place of the watch its profile asked it to write by
//! hand in a shell each time it re-armed.
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `supervisor-watch` and its input `conductor.host.WatchSupervisor`
//! (`spec/domains/host.yaml`). It blocks until the first trigger fires on any instance of the
//! config file (or each `--instance`), reads every trigger once more, and prints one line per
//! fired trigger, sorted by trigger then instance:
//!
//! ```text
//! <TRIGGER> <instance> <subject> <detail>
//! ```
//!
//! or one `conductor.host.SupervisorTrigger` object per line with `--format json`. Past
//! `--timeout` with no other trigger the one line is `TIMEOUT - - -`.
//!
//! | trigger | reads | fires |
//! |---|---|---|
//! | `MAIL` | `<state>/mailbox/conductor/`, file names (ULIDs) past the mail cursor, and those within [`local::MAIL_WINDOW_MS`] below it not handled yet | each new file but a report `progress`, `started`, `unblocked` or `blocked ci` whose text holds neither `waits on` nor `waiting for`; a file that cannot be read, with its error |
//! | `LOG` | a boxed instance's `<state>/sessions/runtime.log` past the log cursor (offset, inode and first bytes: a replaced or rewritten log is read from its start); an unboxed one's unit journal past the journal cursor ([`Sources::journal`]) | each complete line carrying one of [`local::LOG_STRINGS`]; a missing log is a note |
//! | `DAEMON` | the daemon's `Reconciliation` records of the instance past the daemon cursor, in the store `daemon.store` writes | reason `LostRuntime`, or action `Upgrade` with reason `OperatorCommand` |
//! | `INBOX` | the files `globals.supervisor.inbox` and `globals.supervisor.outbox` | the inbox holds a request id the outbox does not, once per new id; off unless both are set |
//! | `DISK` | free bytes on the instance's `thresholds.disk_path` | under `thresholds.disk_low` (once per fall), or 5 GiB under the baseline, which moves only when a DISK line fires or once an hour |
//! | `UNPUSHED` | `git` in each checkout under `checkouts.root`, each managed tree, each `--checkout`, and each worktree's detached HEAD | a local branch or detached HEAD whose commits no remote branch holds, the oldest over 30 min old, once per head |
//! | `GREEN` | the forge of each `github` and `gitlab` source of the instance, with the host's `gh` and `glab`, at most every [`forge::FORGE_EVERY`] ([`forge`]) | an open request, no draft and no auto-merge, whose checks or head pipeline ended green over 15 min ago, once per head sha, unless a `--hold` names it |
//! | `IDLE` | the instance's store (a `Running` controller with an open dispatch), the transcripts of its checkout and its own trees and its newest report | no transcript write for 15 min (without a transcript, 15 min since the dispatch was sent), unless the newest report waits on a person (a `blocked ci` report never does); at most once per 30 min per controller |
//! | `CIEND` | the newest report of each repository a `Running` controller works on, and the forge as `GREEN` reads it | the report is `blocked ci <ref>` and that open request's checks ended after it; once per ref and report |
//! | `TIMEOUT` | the clock | `--timeout` passed with no other trigger |
//!
//! **State.** The cursors and baselines are small files under `supervisor-watch/<instance>/` of
//! the host directory (`supervisor-watch/-/` for what belongs to no instance), each written to a
//! temporary file and renamed into place. One run per host directory holds the exclusive lock
//! `supervisor-watch/lock`, which the kernel lets go when the process dies. A cursor moves
//! ([`Watch::commit`]) only after the lines it covers are written and flushed: delivery is at
//! least once; a pass that fires nothing moves its cursors at once. A first run with no cursor for an instance starts its mail, log and daemon cursors
//! at what is there, and names on standard error how many mail files it passed over (class C: a
//! new host gets no flood of old mail).
//!
//! **Reads.** The mail and log files are read on a change (inotify on the mailbox and sessions
//! directories) and every trigger every [`Sources::interval`] (30 s). A source that cannot be read
//! is one note on standard error, once per run, and the watch reads the others. It opens only
//! stores that exist, read only (AGENTS.md rule 7), writes nothing but `supervisor-watch/`, and
//! touches neither the guard nor `conductor.lease` (rule 6).

pub mod forge;
pub mod local;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read as _, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use conductor_model::config::{Config, Instance};
use conductor_model::host::{
    InvalidWatchTimeout, SupervisorTrigger, SupervisorTriggerKind, SupervisorWatchRunning,
    WatchStateUnwritable, WatchSupervisor,
};
use conductor_model::instance::UnknownInstance;
use conductor_model::primitives::Timestamp;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::SupervisorWatchArgs;
use crate::config::{self, Environment};

/// The directory under the host directory that holds the watch's lock, cursors and baselines.
pub const DIR: &str = "supervisor-watch";

/// The lock file in [`DIR`]: one run per host directory.
pub const LOCK: &str = "lock";

/// The instance of a trigger that belongs to no instance, and the name of its cursor directory.
pub const NO_INSTANCE: &str = "-";

/// `--timeout` when none is given.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// The longest `--timeout`: a day.
pub const MAX_TIMEOUT: Duration = Duration::from_secs(24 * 3600);

/// How often every trigger is read while nothing changed.
pub const READ_EVERY: Duration = Duration::from_secs(30);

/// The shortest time between two reads of the mail and log files a change woke.
const FILE_GAP: Duration = Duration::from_secs(1);

/// What the watch reads beside the config file, the one seam a test points at fixtures.
/// [`Sources::new`] gives the real ones.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The host directory: [`DIR`] is under it.
    pub host: PathBuf,
    /// Claude Code's transcript directory of an instance whose state holds none of its own
    /// (`<state>/claude/projects`, a box's).
    pub projects: PathBuf,
    /// The `daemon.store` the config file writes, where the daemon's `Reconciliation` records
    /// are; none: `DAEMON` is off.
    pub daemon_store: Option<String>,
    /// The current time.
    pub clock: fn() -> OffsetDateTime,
    /// The bytes free on the file system holding a path.
    pub free_bytes: fn(&Path) -> Result<u64>,
    /// How often every trigger is read while nothing changed ([`READ_EVERY`]).
    pub interval: Duration,
    /// The `git` program.
    pub git: PathBuf,
    /// The program and arguments that print the journal of the unit an unboxed instance's
    /// runtime runs in, `{unit}` filled in, with `--show-cursor` (its last line `-- cursor: <c>`);
    /// the watch adds `--after-cursor <c>`, or `-n 1` on a first run ([`JOURNAL`]).
    pub journal: Vec<String>,
    /// The program, and any words before its own, that `GREEN` and `CIEND` read GitHub with:
    /// `gh` ([`forge`]).
    pub gh: Vec<String>,
    /// The program, and any words before its own, that `GREEN` and `CIEND` read GitLab with:
    /// `glab` ([`forge`]).
    pub glab: Vec<String>,
    /// How long the forge reads of one pass may take together ([`forge::FORGE_BUDGET`]).
    pub forge_budget: Duration,
}

/// The journal of an unboxed instance's runtime unit, as [`Sources::journal`] reads it.
pub const JOURNAL: [&str; 9] = [
    "journalctl",
    "--user",
    "-u",
    "{unit}",
    "-o",
    "short-iso",
    "--no-pager",
    "--quiet",
    "--show-cursor",
];

impl Sources {
    /// The real sources: the host directory `host`, `~/.claude/projects` under `home`, the
    /// written `daemon_store`, the system clock, `statvfs`, [`READ_EVERY`], `git`, `gh` and
    /// `glab`.
    #[must_use]
    pub fn new(host: PathBuf, home: &Path, daemon_store: Option<String>) -> Self {
        Self {
            host,
            projects: crate::trust::claude_projects(home),
            daemon_store,
            clock: OffsetDateTime::now_utc,
            free_bytes: |path| crate::disk::space(path).map(|space| space.available),
            interval: READ_EVERY,
            git: PathBuf::from("git"),
            journal: JOURNAL.iter().map(|word| (*word).to_owned()).collect(),
            gh: vec!["gh".to_owned()],
            glab: vec!["glab".to_owned()],
            forge_budget: forge::FORGE_BUDGET,
        }
    }
}

/// Why the watch did not start: each is the specification's error, named on one line of standard
/// error, and the exit is 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `conductor.host.InvalidWatchTimeout`.
    InvalidWatchTimeout(InvalidWatchTimeout),
    /// `conductor.instance.UnknownInstance`.
    UnknownInstance(UnknownInstance),
    /// `conductor.host.WatchStateUnwritable`.
    WatchStateUnwritable(WatchStateUnwritable),
    /// `conductor.host.SupervisorWatchRunning`.
    SupervisorWatchRunning(SupervisorWatchRunning),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWatchTimeout(refusal) => write!(
                f,
                "conductor.host.InvalidWatchTimeout: --timeout {:?} is not <n>s, <n>m or <n>h \
                 with n above 0, at most 24h; nothing was read",
                refusal.timeout
            ),
            Self::UnknownInstance(refusal) => write!(
                f,
                "conductor.instance.UnknownInstance: the config file names no instance {:?}; \
                 nothing was read",
                refusal.name.0
            ),
            Self::WatchStateUnwritable(refusal) => write!(
                f,
                "conductor.host.WatchStateUnwritable: {}: {}; nothing was read",
                refusal.path, refusal.detail
            ),
            Self::SupervisorWatchRunning(refusal) => write!(
                f,
                "conductor.host.SupervisorWatchRunning: the supervisor watch of pid {} holds the \
                 lock of this host directory; nothing was read",
                refusal.pid
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// `--timeout`: `<n>s`, `<n>m` or `<n>h`, `n` a whole number above 0, at most [`MAX_TIMEOUT`].
///
/// # Errors
///
/// [`Refusal::InvalidWatchTimeout`] for any other text.
pub fn timeout(text: &str) -> Result<Duration, Refusal> {
    let refused = || {
        Refusal::InvalidWatchTimeout(InvalidWatchTimeout {
            timeout: text.to_owned(),
        })
    };
    let unit = text.chars().last().ok_or_else(refused)?;
    let seconds = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        _ => return Err(refused()),
    };
    let number = &text[..text.len() - 1];
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return Err(refused());
    }
    let n: u64 = number.parse().map_err(|_| refused())?;
    if n == 0 {
        return Err(refused());
    }
    n.checked_mul(seconds)
        .map(Duration::from_secs)
        .filter(|timeout| *timeout <= MAX_TIMEOUT)
        .ok_or_else(refused)
}

/// The name a trigger is printed under.
#[must_use]
pub const fn trigger_name(kind: SupervisorTriggerKind) -> &'static str {
    match kind {
        SupervisorTriggerKind::Mail => "MAIL",
        SupervisorTriggerKind::Log => "LOG",
        SupervisorTriggerKind::Daemon => "DAEMON",
        SupervisorTriggerKind::Inbox => "INBOX",
        SupervisorTriggerKind::Disk => "DISK",
        SupervisorTriggerKind::Unpushed => "UNPUSHED",
        SupervisorTriggerKind::Green => "GREEN",
        SupervisorTriggerKind::Idle => "IDLE",
        SupervisorTriggerKind::CiEnd => "CIEND",
        SupervisorTriggerKind::Timeout => "TIMEOUT",
    }
}

/// The variant's name as the specification spells it.
const fn kind_name(kind: SupervisorTriggerKind) -> &'static str {
    match kind {
        SupervisorTriggerKind::Mail => "Mail",
        SupervisorTriggerKind::Log => "Log",
        SupervisorTriggerKind::Daemon => "Daemon",
        SupervisorTriggerKind::Inbox => "Inbox",
        SupervisorTriggerKind::Disk => "Disk",
        SupervisorTriggerKind::Unpushed => "Unpushed",
        SupervisorTriggerKind::Green => "Green",
        SupervisorTriggerKind::Idle => "Idle",
        SupervisorTriggerKind::CiEnd => "CiEnd",
        SupervisorTriggerKind::Timeout => "Timeout",
    }
}

/// The trigger's place in the printed order: the story's numbering.
const fn rank(kind: SupervisorTriggerKind) -> u8 {
    match kind {
        SupervisorTriggerKind::Mail => 1,
        SupervisorTriggerKind::Log => 2,
        SupervisorTriggerKind::Daemon => 3,
        SupervisorTriggerKind::Inbox => 4,
        SupervisorTriggerKind::Disk => 5,
        SupervisorTriggerKind::Unpushed => 6,
        SupervisorTriggerKind::Green => 7,
        SupervisorTriggerKind::Idle => 8,
        SupervisorTriggerKind::CiEnd => 9,
        SupervisorTriggerKind::Timeout => 10,
    }
}

/// `trigger` as one printed line, without its line break: `<TRIGGER> <instance> <subject>
/// <detail>`, or with `json` one `conductor.host.SupervisorTrigger` object.
#[must_use]
pub fn render(trigger: &SupervisorTrigger, json: bool) -> String {
    if json {
        return json!({
            "trigger": kind_name(trigger.trigger),
            "instance": trigger.instance,
            "subject": trigger.subject,
            "detail": trigger.detail,
            "at": trigger.at.0,
        })
        .to_string();
    }
    format!(
        "{} {} {} {}",
        trigger_name(trigger.trigger),
        trigger.instance,
        trigger.subject,
        trigger.detail
    )
}

/// What one reading of the triggers found: the fired triggers, and the cursor files to write
/// once they are printed.
#[derive(Debug, Default, Clone)]
pub struct Pass {
    /// The fired triggers, in the order they were read.
    pub triggers: Vec<SupervisorTrigger>,
    /// Each cursor file and its new content, written by [`Watch::commit`]; `None` removes it.
    writes: BTreeMap<PathBuf, Option<String>>,
}

impl Pass {
    /// Whether no trigger fired.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }

    /// The triggers in printed order: by trigger, then instance, each kind and instance in the
    /// order read.
    #[must_use]
    pub fn sorted(&self) -> Vec<SupervisorTrigger> {
        let mut sorted = self.triggers.clone();
        sorted.sort_by(|one, other| {
            (rank(one.trigger), &one.instance).cmp(&(rank(other.trigger), &other.instance))
        });
        sorted
    }

    /// `later` with every trigger of `self` it does not hold (by trigger, instance and subject),
    /// and every cursor write of `self` it does not make.
    fn merged_into(self, mut later: Self) -> Self {
        let key = |trigger: &SupervisorTrigger| {
            (
                rank(trigger.trigger),
                trigger.instance.clone(),
                trigger.subject.clone(),
            )
        };
        let held: BTreeSet<_> = later.triggers.iter().map(key).collect();
        for trigger in self.triggers {
            if !held.contains(&key(&trigger)) {
                later.triggers.push(trigger);
            }
        }
        for (file, content) in self.writes {
            later.writes.entry(file).or_insert(content);
        }
        later
    }

    fn fire(
        &mut self,
        kind: SupervisorTriggerKind,
        instance: &str,
        subject: &str,
        detail: &str,
        at: &Timestamp,
    ) {
        let subject: String = subject.split_whitespace().collect::<Vec<_>>().join("_");
        let detail = detail.split(['\n', '\r']).collect::<Vec<_>>().join(" ");
        self.triggers.push(SupervisorTrigger {
            trigger: kind,
            instance: instance.to_owned(),
            subject: if subject.is_empty() {
                NO_INSTANCE.to_owned()
            } else {
                subject
            },
            detail: if detail.trim().is_empty() {
                NO_INSTANCE.to_owned()
            } else {
                detail
            },
            at: at.clone(),
        });
    }

    fn write(&mut self, file: PathBuf, content: impl Into<String>) {
        self.writes.insert(file, Some(content.into()));
    }

    fn remove(&mut self, file: PathBuf) {
        self.writes.insert(file, None);
    }
}

/// One supervisor watch over the instances it reads.
#[derive(Debug)]
pub struct Watch {
    instances: Vec<Instance>,
    inbox: Option<PathBuf>,
    outbox: Option<PathBuf>,
    checkouts: Vec<PathBuf>,
    holds: Vec<String>,
    timeout: Duration,
    sources: Sources,
    /// The notes printed in this run, each once.
    noted: BTreeSet<String>,
    /// When the running [`Watch::run`] times out: the forge reads of a pass end by then.
    deadline: Option<Instant>,
}

impl Watch {
    /// The watch of `args` over the config `config` (whose relative `--checkout` paths are under
    /// `cwd`), reading through `sources`.
    ///
    /// # Errors
    ///
    /// `--timeout` does not parse, or an `--instance` names no instance of `config`.
    pub fn new(
        config: &Config,
        args: &WatchSupervisor,
        cwd: &Path,
        sources: Sources,
    ) -> Result<Self, Refusal> {
        let timeout = match &args.timeout {
            Some(text) => timeout(text)?,
            None => DEFAULT_TIMEOUT,
        };
        let mut instances = Vec::new();
        if args.instance.is_empty() {
            instances.clone_from(&config.instances);
        } else {
            for name in &args.instance {
                let found = config
                    .instances
                    .iter()
                    .find(|instance| &instance.name.0 == name)
                    .ok_or_else(|| {
                        Refusal::UnknownInstance(UnknownInstance {
                            name: conductor_model::config::InstanceName(name.clone()),
                        })
                    })?;
                if !instances
                    .iter()
                    .any(|held: &Instance| held.name == found.name)
                {
                    instances.push(found.clone());
                }
            }
        }
        let supervisor = config.globals.supervisor.as_ref();
        Ok(Self {
            instances,
            inbox: supervisor.and_then(|s| s.inbox.as_ref()).map(PathBuf::from),
            outbox: supervisor
                .and_then(|s| s.outbox.as_ref())
                .map(PathBuf::from),
            checkouts: args.checkout.iter().map(|path| cwd.join(path)).collect(),
            holds: args.hold.clone(),
            timeout,
            sources,
            noted: BTreeSet::new(),
            deadline: None,
        })
    }

    /// The `--hold` refs, which `GREEN` passes over ([`forge`]).
    #[must_use]
    pub fn holds(&self) -> &[String] {
        &self.holds
    }

    /// The watch's directory under the host directory.
    #[must_use]
    pub fn dir(&self) -> PathBuf {
        self.sources.host.join(DIR)
    }

    /// The cursor directory of `instance` ([`NO_INSTANCE`] for none).
    fn cursors(&self, instance: &str) -> PathBuf {
        self.dir().join(instance)
    }

    /// Takes the lock of the host directory for this run, creating [`DIR`].
    ///
    /// # Errors
    ///
    /// [`Refusal::WatchStateUnwritable`] when [`DIR`] or its lock cannot be written,
    /// [`Refusal::SupervisorWatchRunning`] when another run holds the lock.
    pub fn lock(&self) -> Result<File, Refusal> {
        let dir = self.dir();
        let unwritable = |path: &Path, error: &io::Error| {
            Refusal::WatchStateUnwritable(WatchStateUnwritable {
                path: path.display().to_string(),
                detail: error.to_string(),
            })
        };
        fs::create_dir_all(&dir).map_err(|error| unwritable(&dir, &error))?;
        let path = dir.join(LOCK);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| unwritable(&path, &error))?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                let mut text = String::new();
                let _ = file.read_to_string(&mut text);
                return Err(Refusal::SupervisorWatchRunning(SupervisorWatchRunning {
                    pid: text.trim().parse().unwrap_or(0),
                }));
            }
            Err(TryLockError::Error(error)) => return Err(unwritable(&path, &error)),
        }
        let written = file
            .set_len(0)
            .and_then(|()| writeln!(file, "{}", std::process::id()))
            .and_then(|()| file.flush());
        written.map_err(|error| unwritable(&path, &error))?;
        Ok(file)
    }

    /// The current time, as a trigger records it.
    fn now(&self) -> (OffsetDateTime, Timestamp) {
        let now = (self.sources.clock)();
        let at = now.format(&Rfc3339).unwrap_or_default();
        (now, Timestamp(at))
    }

    /// One note on standard error, once per run.
    fn note(&mut self, notes: &mut dyn Write, text: String) {
        if self.noted.insert(text.clone()) {
            let _ = writeln!(notes, "supervisor watch: {text}");
        }
    }

    /// Every trigger read once. Nothing moves until [`Watch::commit`]; a first run's baselines
    /// and the disk's last reading are written at once, since they cover no printed line.
    ///
    /// # Errors
    ///
    /// A cursor or baseline cannot be written.
    pub fn pass(&mut self, notes: &mut dyn Write) -> Result<Pass> {
        self.reading(notes, true)
    }

    /// [`Watch::pass`], reading the forge only when `forge`. A forge trigger that cannot keep
    /// its cursors is one note, and the other triggers fire.
    fn reading(&mut self, notes: &mut dyn Write, forge: bool) -> Result<Pass> {
        let mut pass = Pass::default();
        self.files(&mut pass, notes)?;
        let (now, at) = self.now();
        let instances = self.instances.clone();
        local::daemon(self, &instances, &mut pass, &at, notes)?;
        local::inbox(self, &mut pass, &at, notes);
        for instance in &instances {
            local::disk(self, instance, &mut pass, now, &at, notes)?;
        }
        local::unpushed(self, &instances, &mut pass, now, &at, notes);
        for instance in &instances {
            local::idle(self, instance, &mut pass, now, &at, notes);
            if forge && let Err(error) = forge::forge(self, instance, &mut pass, now, &at, notes) {
                let name = &instance.name.0;
                self.note(notes, format!("{name}: GREEN, CIEND: {error:#}"));
            }
        }
        Ok(pass)
    }

    /// The mail and log triggers alone, as a change to their files wakes them.
    fn files(&mut self, pass: &mut Pass, notes: &mut dyn Write) -> Result<()> {
        let (_, at) = self.now();
        for instance in self.instances.clone() {
            local::mail(self, &instance, pass, &at, notes)?;
            local::log(self, &instance, pass, &at, notes)?;
        }
        Ok(())
    }

    /// Writes the cursors `pass` moves, each to a temporary file renamed into place.
    ///
    /// # Errors
    ///
    /// A cursor cannot be written.
    pub fn commit(&self, pass: &Pass) -> Result<()> {
        for (file, content) in &pass.writes {
            match content {
                Some(content) => put(file, content)?,
                None => match fs::remove_file(file) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(error).with_context(|| format!("remove {}", file.display()));
                    }
                },
            }
        }
        Ok(())
    }

    /// Blocks until a trigger fires or the timeout passes, reads every trigger once more, prints
    /// one line per fired trigger on `out` (`json`: one object each), flushes it, and only then
    /// moves the cursors; a pass that fired nothing moves its cursors at once. Answers what it
    /// printed.
    ///
    /// # Errors
    ///
    /// The output cannot be written, or a cursor cannot.
    pub fn run(
        &mut self,
        json: bool,
        out: &mut dyn Write,
        notes: &mut dyn Write,
    ) -> Result<Vec<SupervisorTrigger>> {
        let started = Instant::now();
        let deadline = started + self.timeout;
        self.deadline = Some(deadline);
        let mut changes = local::Changes::new(&self.instances);
        let mut fired = self.pass(notes)?;
        let mut last_full = Instant::now();
        let mut last_files = last_full;
        while fired.is_empty() {
            // A pass that fired nothing prints nothing: its cursors move at once, so what it
            // read is not read again.
            self.commit(&fired)?;
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let next_full = (last_full + self.sources.interval).min(deadline);
            let changed = changes.wait(next_full.saturating_duration_since(now));
            let now = Instant::now();
            if now >= next_full {
                fired = self.pass(notes)?;
                last_full = now;
                last_files = now;
            } else if changed {
                let gap = self.sources.interval.min(FILE_GAP);
                let ready = last_files + gap;
                if now < ready {
                    std::thread::sleep(ready - now);
                }
                let mut pass = Pass::default();
                self.files(&mut pass, notes)?;
                fired = pass;
                last_files = Instant::now();
            }
        }
        let pass = if fired.is_empty() {
            let (_, at) = self.now();
            let mut pass = Pass::default();
            pass.fire(SupervisorTriggerKind::Timeout, NO_INSTANCE, "", "", &at);
            pass
        } else {
            // Lines are pending: this last reading reads no forge, which could hold them.
            let again = self.reading(notes, false)?;
            fired.merged_into(again)
        };
        let printed = pass.sorted();
        for trigger in &printed {
            writeln!(out, "{}", render(trigger, json)).context("write a trigger line")?;
        }
        out.flush().context("write the trigger lines")?;
        self.commit(&pass)?;
        Ok(printed)
    }
}

/// Writes `content` to `file` through a temporary file beside it renamed into place, creating
/// its directory.
fn put(file: &Path, content: &str) -> Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let mut next = file.as_os_str().to_owned();
    next.push(format!(".{}.next", std::process::id()));
    let next = PathBuf::from(next);
    fs::write(&next, content).with_context(|| format!("write {}", next.display()))?;
    fs::rename(&next, file).with_context(|| format!("write {}", file.display()))
}

/// The text of `file`, or `None` when it is not there.
fn read(file: &Path) -> Result<Option<String>> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", file.display())),
    }
}

/// The `daemon.store` the config file's `text` writes, if any: where the daemon keeps its
/// `Reconciliation` records.
#[must_use]
pub fn written_daemon_store(text: &str) -> Option<String> {
    serde_yaml::from_str::<serde_yaml::Value>(text)
        .ok()?
        .get("daemon")?
        .get("store")?
        .as_str()
        .map(str::to_owned)
        .filter(|url| !url.is_empty())
}

/// `conductor supervisor watch`: the input `conductor.host.WatchSupervisor`, over the config file
/// `config_flag`, `CONDUCTOR_CONFIG` or the default one.
///
/// # Errors
///
/// The config file does not load, or a cursor or the output cannot be written. A refusal of the
/// specification is one line on standard error and exit 1.
pub fn supervisor_watch(
    config_flag: Option<&Path>,
    args: &SupervisorWatchArgs,
) -> Result<ExitCode> {
    const WHAT: &str = "supervisor watch";
    let input = WatchSupervisor {
        instance: args.instance.clone(),
        checkout: args.checkout.clone(),
        timeout: args.timeout.clone(),
        hold: args.hold.clone(),
        format: args.format.clone(),
    };
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => {
            anyhow::bail!("{WHAT}: --format {other:?} is neither text nor json; nothing was read")
        }
    };
    if let Some(text) = &input.timeout
        && let Err(refusal) = timeout(text)
    {
        eprintln!("{refusal}");
        return Ok(ExitCode::FAILURE);
    }
    let environment = Environment::from_process().context(WHAT)?;
    let home = environment.home().context(WHAT)?.to_owned();
    let loaded = config::load(config_flag, &environment).context(WHAT)?;
    let file = environment.cwd.join(&loaded.path);
    let daemon_store = if loaded.read {
        fs::read_to_string(&file)
            .ok()
            .and_then(|text| written_daemon_store(&text))
    } else {
        None
    };
    let sources = Sources::new(config::host_of(&file), &home, daemon_store);
    let mut watch = match Watch::new(&loaded.config, &input, &environment.cwd, sources) {
        Ok(watch) => watch,
        Err(refusal) => {
            eprintln!("{refusal}");
            return Ok(ExitCode::FAILURE);
        }
    };
    let _lock = match watch.lock() {
        Ok(lock) => lock,
        Err(refusal) => {
            eprintln!("{refusal}");
            return Ok(ExitCode::FAILURE);
        }
    };
    let stdout = io::stdout();
    let stderr = io::stderr();
    watch
        .run(json, &mut stdout.lock(), &mut stderr.lock())
        .context(WHAT)?;
    Ok(ExitCode::SUCCESS)
}
