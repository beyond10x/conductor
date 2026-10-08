//! `conductor watch run`: wake conductor on the first change (`story:watch-command`).
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `watch-run` and its input `conductor.observation.WatchRun`. Like `dashboard
//! serve`, it is no placed command of the `cli:` block: it writes no store record. It replaces
//! the stopgap loop conductor ran with Bash `run_in_background`, and keeps its probes, its lines
//! and its state files.
//!
//! It watches the instance the process runs ([`config::active`]): its checkouts, its records,
//! its first `github` owner, its cadence (`cadence.watch` and `cadence.ci`, which `--every` and
//! `--ci-every` override; [`Options::of`]) and its thresholds (`thresholds.context_handover`,
//! `disk_low` and `disk_clear`; [`Settings::of`]), and names it in the usage-limit
//! notification's title. Without a config file that is the built-in instance.
//!
//! It runs cheap probes, not snapshots. A pass, every [`Options::every`]:
//!
//! | probe | source ([`Sources`]) | line |
//! |---|---|---|
//! | sessions | the session list (`claude agents --json`); only the entries whose working directory lies in the instance's checkouts root or managed trees, or in its records when a config file names them, so no other session is ever named | `session exited (blocked): <name> <id>`, `session gone: <name> <id>` |
//! | usage limit | the last [`TAIL`] lines of each such session's transcript | `usage limit: <text>; <n> sessions: <names>` |
//! | context | the newest line with a usage in the transcript of each such session that has a `pid` | `context: <repo> <n>k tokens (session <id>)` |
//! | `main` CI, on the first pass and then every [`Options::ci_every`] | the newest completed `main` run per workflow of each repository the newest complete snapshot holds Active ([`crate::repository::activity`]) | `CI red on main: <repo> / <workflow> (run <id>)`, `CI green again on main: <repo> / <workflow>` |
//! | free disk | `df` on `/` | `disk low: <n>G free on /` |
//!
//! `<id>` is the first eight characters of a session's `sessionId`. A session is reported:
//! - **exited** when it turns `blocked` without a `pid`, which is how the list keeps a session
//!   stopped at the usage limit; **gone** when it leaves the list, unless it was already
//!   exited. Either only when no line of the dispatch log names its id, or its pid as a word,
//!   written since the kept list was taken or in the last [`EXPLAINED_WITHIN`], whichever is
//!   earlier, since conductor itself stopped it then; and each session id once (`exited.seen`)
//!   until the list shows it with a `pid` again, so its next exit is reported once more. A start
//!   is no change.
//! - **usage limit** for a `"error":"rate_limit"` line of the last [`LIMIT_WITHIN`], once per
//!   limit text per [`LIMIT_AGAIN`] from its last report, with a desktop notification to the
//!   operator through [`Sources::notify`], titled `<instance>: usage limit`, because conductor is
//!   stopped by the same limit.
//! - **context** above the instance's `context_handover` tokens, once per session
//!   (`story:controller-context`). `<repo>` is the directory the session works in under the
//!   managed trees or the checkouts root, else the session's name; conductor's line reads
//!   `context: conductor` either way.
//! - **CI** when a workflow's newest `main` run that decided something (completed, `success` or
//!   one of [`RED`], as the github collector maps them) is not older than the one kept and turns
//!   it red (one of [`RED`] after another conclusion), or green again (`success` after one of
//!   [`RED`]): by its conclusion, so a failed run re-run to success is reported with the same run
//!   id. Every other conclusion (`cancelled`, `skipped`, empty, `neutral`, `action_required`,
//!   `stale`, and one GitHub adds) decides nothing; it, and a workflow missing from the list,
//!   keep the last run as it was. `ci.state` keeps the conclusion as GitHub printed it. While
//!   the store under the state directory holds no complete snapshot, no run list is read.
//! - **disk** under the instance's `disk_low` GiB, once per fall: the next report waits until
//!   free disk was back at its `disk_clear` GiB.
//!
//! An empty or unreadable session list skips the three session probes for that pass. A source
//! that does not answer is named once on standard error, `conductor watch: <error>`, until it
//! answers again; the watch goes on. A CI pass runs each repository's run list once, and a
//! repository whose list does not answer is such a source of its own, named in its line; a
//! store without a complete snapshot is one too.
//!
//! [`Watch::run`] exits after the first pass that printed a line, with every line of that pass;
//! with [`Options::follow`] it prints and goes on, for the harness `Monitor` tool. Its state is
//! kept under `<state>/watch/`: `sessions.json` (the kept list and the instant it was taken),
//! `exited.seen`, `limit.reported` (`<instant> <text>`, the last report of each limit text),
//! `disk-low`, and, in the formats of the stopgap, `limit.new` (`<text>\t<name>`), `limit.seen`
//! (`YYYY-MM-DD <text>`), `context-reported` (a `sessionId` per line) and `ci.state`
//! (`repo|workflow|run|conclusion|createdAt`). The dashboard reads the last four from
//! `<conductor repository>/state/watch/` once that holds `ci.state`, which a watch run in the
//! conductor repository without `--state-dir` writes; until then it reads the stopgap's
//! `~/.cache/conductor-watch/`.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read as _, Seek as _, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use conductor_model::behaviour::SnapshotStorage;
use conductor_model::config::{Instance, Source};
use conductor_model::direction::Activity;
use conductor_model::observation::SnapshotState;

use crate::cli::WatchRunArgs;
use crate::collect::repositories::{WIDTH, each, filled, words};
use crate::collect::{self, BOUND};
use crate::config::{self, Active};
use crate::{repository, state};

/// How recent a dispatch line must be to explain a session's exit.
pub const EXPLAINED_WITHIN: Duration = Duration::from_secs(15 * 60);

/// How recent a rate-limit line must be to be reported.
pub const LIMIT_WITHIN: Duration = Duration::from_secs(30 * 60);

/// How long after its last report a limit text is reported again.
pub const LIMIT_AGAIN: Duration = Duration::from_secs(24 * 60 * 60);

/// The lines at the end of a transcript searched for a rate limit.
pub const TAIL: usize = 20;

/// How long the notifier may run.
const NOTIFY_BOUND: Duration = Duration::from_secs(10);

/// The bytes a transcript is read by, from its end.
const CHUNK: u64 = 64 * 1024;

/// The characters of a run list's standard error a note carries.
const EXCERPT: usize = 300;

/// The conclusions of a completed run that turn `main` red: those the github collector maps to a
/// `Failure`.
pub const RED: [&str; 3] = ["failure", "timed_out", "startup_failure"];

/// The conclusion of a completed run that turns `main` green.
const GREEN: &str = "success";

/// Whether a completed run concluded `conclusion` decided something about `main`: green or red.
/// Every other conclusion decides nothing and keeps the last run.
fn decided(conclusion: &str) -> bool {
    conclusion == GREEN || RED.contains(&conclusion)
}

fn red(conclusion: &str) -> bool {
    RED.contains(&conclusion)
}

/// The state files under `<state>/watch/`.
const SESSIONS: &str = "sessions.json";
const EXITED_SEEN: &str = "exited.seen";
const LIMIT_NEW: &str = "limit.new";
const LIMIT_SEEN: &str = "limit.seen";
const LIMIT_REPORTED: &str = "limit.reported";
const CONTEXT_REPORTED: &str = "context-reported";
const CI_STATE: &str = "ci.state";
const DISK_LOW: &str = "disk-low";

/// The source a CI pass names on standard error while no complete snapshot names its
/// repositories.
const SNAPSHOT: &str = "snapshot";

/// What the watch reads, the one seam a test points at fixtures. [`Sources::new`] gives the real
/// ones.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The program and arguments that print the session list as one JSON array.
    pub agents: Vec<OsString>,
    /// The instance's checkouts root: a session working in `<root>/<repo>` is watched, and works
    /// on `<repo>`.
    pub root: PathBuf,
    /// The instance's managed trees: a session working in `<trees>/<repo>/…` is watched, and
    /// works on `<repo>`.
    pub trees: PathBuf,
    /// The instance's records, when a config file names the instance: a session working in them,
    /// conductor's, is watched too.
    pub records: Option<PathBuf>,
    /// Claude Code's transcript directory: `<projects>/<cwd, each character but a letter or digit
    /// written `-`>/<sessionId>.jsonl`.
    pub projects: PathBuf,
    /// The dispatch log, `YYYY-MM.jsonl` per month.
    pub dispatches: PathBuf,
    /// The GitHub owner, filled into [`Sources::runs`] as `{organization}`.
    pub organization: String,
    /// The state directory whose store names the repositories whose `main` CI is read: the
    /// Active ones of its newest complete snapshot ([`crate::repository::activity`]), read on
    /// each CI pass. `None`: [`Sources::repositories`].
    pub store: Option<PathBuf>,
    /// The repositories whose `main` CI is read when [`Sources::store`] is `None`.
    pub repositories: Vec<String>,
    /// Prints one repository's `main` runs as one JSON array of objects with `databaseId`,
    /// `workflowName`, `status`, `conclusion` and `createdAt`; `{repository}` is filled in.
    pub runs: Vec<String>,
    /// The program and arguments that print the free bytes on `/` as the last line.
    pub disk: Vec<OsString>,
    /// The program and arguments of the desktop notifier; a title and a body are added.
    pub notify: Vec<OsString>,
    /// The current time.
    pub clock: fn() -> OffsetDateTime,
    /// How long the session list, `df` and one run list may take before they are stopped.
    pub bound: Duration,
}

impl Sources {
    /// The sources of the instance `active`, for the home directory `home` and the state
    /// directory `state`: `claude agents --json`; the instance's checkouts root and managed
    /// trees, and its records when a config file names it; `~/.claude/projects`; the dispatch log
    /// under the records when a config file names them, else under `conductor/` of the checkouts
    /// root, as before a config file existed; the owner of the instance's first `github` source,
    /// whose run lists `gh` prints for the Active repositories of the store under `state` (none
    /// for an instance without a `github` source); `df` on `/`; `notify-send -u critical`; the
    /// system clock; and [`BOUND`].
    #[must_use]
    pub fn new(home: PathBuf, active: &Active, state: PathBuf) -> Self {
        let instance = &active.instance;
        let root = PathBuf::from(&instance.checkouts.root);
        let records = PathBuf::from(&instance.records);
        let owner = instance.sources.iter().find_map(|source| match source {
            Source::GitHub(github) => Some(github.owner.clone()),
            Source::Local(_) | Source::GitLab(_) => None,
        });
        Self {
            agents: ["claude", "agents", "--json"].map(OsString::from).to_vec(),
            trees: PathBuf::from(&instance.checkouts.trees),
            dispatches: if active.from_file {
                records.join("dispatches")
            } else {
                root.join("conductor/dispatches")
            },
            records: active.from_file.then_some(records),
            root,
            projects: home.join(".claude/projects"),
            store: owner.is_some().then_some(state),
            organization: owner.unwrap_or_default(),
            repositories: Vec::new(),
            runs: words(&[
                "gh",
                "run",
                "list",
                "-R",
                "{organization}/{repository}",
                "--branch",
                "main",
                "--limit",
                "100",
                "--json",
                "workflowName,conclusion,status,createdAt,databaseId",
            ]),
            disk: ["df", "-B1", "--output=avail", "/"]
                .map(OsString::from)
                .to_vec(),
            notify: ["notify-send", "-u", "critical"]
                .map(OsString::from)
                .to_vec(),
            clock: OffsetDateTime::now_utc,
            bound: BOUND,
        }
    }
}

/// What the instance sets for the watch besides its sources: its name and its thresholds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The instance's name, which the usage-limit notification's title names.
    pub instance: String,
    /// Tokens of a session's context above which it is reported: `thresholds.context_handover`.
    pub context_handover: u64,
    /// Free GiB on `/` under which it is reported: `thresholds.disk_low`.
    pub disk_low: u64,
    /// Free GiB on `/` from which a further fall is reported again: `thresholds.disk_clear`.
    pub disk_clear: u64,
}

impl Settings {
    /// The name and the thresholds of `instance`.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        let count = |value: i64| u64::try_from(value).unwrap_or(0);
        let thresholds = &instance.thresholds;
        Self {
            instance: instance.name.0.clone(),
            context_handover: count(thresholds.context_handover.0),
            disk_low: count(thresholds.disk_low.0),
            disk_clear: count(thresholds.disk_clear.0),
        }
    }
}

impl Default for Settings {
    /// The built-in instance's ([`config::built_in`]).
    fn default() -> Self {
        Self::of(&config::built_in(Path::new("/"), Path::new("/")))
    }
}

/// How [`Watch::run`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// From the end of one pass to the start of the next.
    pub every: Duration,
    /// Between two reads of `main`'s CI.
    pub ci_every: Duration,
    /// Print each change and go on, rather than end after the first pass that found one.
    pub follow: bool,
}

impl Default for Options {
    /// The built-in cadence: [`config::WATCH_EVERY`] and [`config::CI_EVERY`].
    fn default() -> Self {
        Self {
            every: Duration::from_secs(config::WATCH_EVERY),
            ci_every: Duration::from_secs(config::CI_EVERY),
            follow: false,
        }
    }
}

impl Options {
    /// The instance's `cadence.watch` and `cadence.ci`, each overridden by its flag, and
    /// `--follow`.
    #[must_use]
    pub fn of(instance: &Instance, args: &WatchRunArgs) -> Self {
        let cadence = &instance.cadence;
        let every = args
            .every
            .or_else(|| config::seconds(&cadence.watch))
            .unwrap_or(config::WATCH_EVERY);
        let ci_every = args
            .ci_every
            .or_else(|| config::seconds(&cadence.ci))
            .unwrap_or(config::CI_EVERY);
        Self {
            every: Duration::from_secs(every),
            ci_every: Duration::from_secs(ci_every),
            follow: args.follow.unwrap_or(false),
        }
    }
}

/// `conductor watch run`: watches until the first pass that finds a change, prints its lines and
/// exits 0; with `--follow`, prints each change and goes on.
///
/// # Errors
///
/// `HOME` is not an absolute path, the working directory cannot be read (without
/// `--state-dir`), the state directory cannot be written, or standard output is closed.
pub fn watch_run(state: Option<&Path>, args: &WatchRunArgs) -> Result<ExitCode> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .context("HOME is not an absolute path; the watch reads the transcripts under ~/.claude")?;
    let active = config::active();
    let options = Options::of(&active.instance, args);
    let state = state::dir(state)?;
    let mut watch = Watch::new(
        Sources::new(home, active, state.clone()),
        state.join("watch"),
    )
    .with_settings(Settings::of(&active.instance));
    watch.run(&options, &mut io::stdout().lock(), &mut io::stderr().lock())?;
    Ok(ExitCode::SUCCESS)
}

/// The watch over its [`Sources`], keeping its state in one directory.
#[derive(Debug)]
pub struct Watch {
    sources: Sources,
    settings: Settings,
    dir: PathBuf,
    /// The sources named on standard error that have not answered since.
    failing: BTreeSet<String>,
}

/// `sessions.json`: the session list of the last pass that read one, and when it was taken.
#[derive(Debug, Serialize, Deserialize)]
struct Kept {
    /// The watch's clock when the list was read, RFC 3339.
    at: String,
    sessions: Vec<Session>,
}

/// One session of the list, as the watch keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Session {
    /// The first eight characters of its `sessionId`.
    id: String,
    /// Its `sessionId`, else its `id`.
    session: String,
    name: String,
    pid: Option<u64>,
    /// Its `status`, else its `state`, else `-`.
    state: String,
    cwd: String,
}

impl Session {
    /// Whether the list keeps it as stopped: `blocked`, without a `pid`.
    fn exited(&self) -> bool {
        self.pid.is_none() && self.state == "blocked"
    }
}

/// The newest completed `main` run of one workflow, one line of `ci.state`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Latest {
    repository: String,
    workflow: String,
    run: i64,
    conclusion: String,
    at: String,
}

impl Latest {
    fn parse(line: &str) -> Option<Self> {
        let mut fields = line.split('|');
        let latest = Self {
            repository: fields.next()?.to_owned(),
            workflow: fields.next()?.to_owned(),
            run: fields.next()?.parse().ok()?,
            conclusion: fields.next()?.to_owned(),
            at: fields.next()?.to_owned(),
        };
        fields.next().is_none().then_some(latest)
    }

    fn key(&self) -> (String, String) {
        (self.repository.clone(), self.workflow.clone())
    }

    fn line(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.repository, self.workflow, self.run, self.conclusion, self.at
        )
    }
}

/// One run, as `gh run list --json` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Run {
    database_id: i64,
    workflow_name: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    conclusion: Option<String>,
    created_at: String,
}

impl Watch {
    /// A watch over `sources` keeping its state in `dir`, which the first pass creates, on the
    /// built-in instance's [`Settings`].
    #[must_use]
    pub fn new(sources: Sources, dir: PathBuf) -> Self {
        Self {
            sources,
            settings: Settings::default(),
            dir,
            failing: BTreeSet::new(),
        }
    }

    /// This watch on `settings`.
    #[must_use]
    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    /// Passes every `options.every` (`main`'s CI on the first and then every
    /// `options.ci_every`), writing each change as a line to `out`, until the first pass that
    /// found one; with `options.follow`, for ever. A source that does not answer is named on
    /// `notes`.
    ///
    /// # Errors
    ///
    /// The state directory cannot be written, or `out` cannot be.
    pub fn run(
        &mut self,
        options: &Options,
        out: &mut dyn Write,
        notes: &mut dyn Write,
    ) -> Result<()> {
        let mut ci_read: Option<Instant> = None;
        loop {
            let ci = ci_read.is_none_or(|at| at.elapsed() >= options.ci_every);
            if ci {
                ci_read = Some(Instant::now());
            }
            let lines = self.pass(ci, notes)?;
            for line in &lines {
                writeln!(out, "{line}").context("write a change to standard output")?;
            }
            out.flush().context("write a change to standard output")?;
            if !lines.is_empty() && !options.follow {
                return Ok(());
            }
            thread::sleep(options.every);
        }
    }

    /// One pass over every probe, `main`'s CI only when `ci`; the changes it found, one line
    /// each, in the order of the probes. A source that does not answer is named on `notes` once,
    /// until it answers again.
    ///
    /// # Errors
    ///
    /// The state directory cannot be read or written.
    pub fn pass(&mut self, ci: bool, notes: &mut dyn Write) -> Result<Vec<String>> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("create the watch's directory {}", self.dir.display()))?;
        let now = (self.sources.clock)();
        let mut lines = Vec::new();
        match self.sessions() {
            Ok(sessions) => {
                self.answered("sessions");
                if !sessions.is_empty() {
                    self.session_changes(now, &sessions, &mut lines)?;
                    self.limits(now, &sessions, &mut lines, notes)?;
                    self.context(&sessions, &mut lines)?;
                }
            }
            Err(error) => self.failed("sessions", &error, notes),
        }
        if ci {
            self.ci(&mut lines, notes)?;
        }
        self.disk(&mut lines, notes)?;
        Ok(lines)
    }

    /// Names `error` on `notes`, unless `source` was named and has not answered since.
    fn failed(&mut self, source: &str, error: &anyhow::Error, notes: &mut dyn Write) {
        if self.failing.insert(source.to_owned()) {
            let _ = writeln!(
                notes,
                "conductor watch: {}",
                one_line(&format!("{error:#}"))
            );
        }
    }

    fn answered(&mut self, source: &str) {
        self.failing.remove(source);
    }

    fn state(&self, file: &str) -> PathBuf {
        self.dir.join(file)
    }

    // -----------------------------------------------------------------------------------------
    // Sessions
    // -----------------------------------------------------------------------------------------

    /// The watched sessions of the list, by id.
    fn sessions(&self) -> Result<Vec<Session>> {
        let shown = command_line(&self.sources.agents);
        let (program, arguments) = self
            .sources
            .agents
            .split_first()
            .context("no session-list command is given")?;
        let output = collect::run(Command::new(program).args(arguments), self.sources.bound)
            .with_context(|| format!("read the session list `{shown}`"))?;
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
        let mut sessions = Vec::new();
        for entry in &entries {
            let text = |key: &str| entry[key].as_str().filter(|text| !text.is_empty());
            let Some(cwd) = text("cwd").filter(|cwd| self.sources.watches(cwd)) else {
                continue;
            };
            let Some(session) = text("sessionId").or_else(|| text("id")) else {
                continue;
            };
            sessions.push(Session {
                id: session.chars().take(8).collect(),
                session: session.to_owned(),
                name: one_line(text("name").unwrap_or("unnamed")),
                pid: entry["pid"].as_u64(),
                state: text("status")
                    .or_else(|| text("state"))
                    .unwrap_or("-")
                    .to_owned(),
                cwd: cwd.to_owned(),
            });
        }
        sessions.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(sessions)
    }

    /// The sessions that exited or left the list since the kept list, then forgets the reports of
    /// the sessions listed with a `pid` and keeps `current`, taken at `now`.
    fn session_changes(
        &self,
        now: OffsetDateTime,
        current: &[Session],
        lines: &mut Vec<String>,
    ) -> Result<()> {
        let file = self.state(SESSIONS);
        let kept: Option<Kept> =
            read_optional(&file)?.and_then(|text| serde_json::from_str(&text).ok());
        if let Some(Kept {
            at,
            sessions: previous,
        }) = kept
        {
            let window = now - EXPLAINED_WITHIN;
            let since =
                OffsetDateTime::parse(&at, &Rfc3339).map_or(window, |taken| taken.min(window));
            let recent = self.dispatches_since(since, now);
            let before: BTreeMap<&str, &Session> = previous
                .iter()
                .map(|session| (session.id.as_str(), session))
                .collect();
            for session in current {
                let Some(before) = before.get(session.id.as_str()) else {
                    continue;
                };
                if session.exited()
                    && !before.exited()
                    && !explained(&recent, &session.id, before.pid)
                    && !self.reported(&session.id)?
                {
                    lines.push(format!(
                        "session exited (blocked): {} {}",
                        session.name, session.id
                    ));
                }
            }
            let listed: BTreeSet<&str> =
                current.iter().map(|session| session.id.as_str()).collect();
            for before in &previous {
                if listed.contains(before.id.as_str()) || before.exited() {
                    continue;
                }
                if !explained(&recent, &before.id, before.pid) && !self.reported(&before.id)? {
                    lines.push(format!("session gone: {} {}", before.name, before.id));
                }
            }
        }
        let returned: BTreeSet<&str> = current
            .iter()
            .filter(|session| session.pid.is_some())
            .map(|session| session.id.as_str())
            .collect();
        self.forget(&returned)?;
        let kept = Kept {
            at: now
                .format(&Rfc3339)
                .context("write the watch's clock as RFC 3339")?,
            sessions: current.to_vec(),
        };
        let kept = serde_json::to_string(&kept).context("write the session list as JSON")?;
        replace(&file, &format!("{kept}\n"))
    }

    /// The lines of the dispatch log written from `since` on, by their `at`, else `sent_at`, else
    /// `decided_at`, read from the months of `since` through `now`.
    fn dispatches_since(&self, since: OffsetDateTime, now: OffsetDateTime) -> Vec<String> {
        let mut recent = Vec::new();
        for month in months(since, now) {
            let Ok(text) =
                fs::read_to_string(self.sources.dispatches.join(format!("{month}.jsonl")))
            else {
                continue;
            };
            for line in text.lines() {
                let Ok(Value::Object(record)) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                let written = ["at", "sent_at", "decided_at"]
                    .iter()
                    .find_map(|key| record.get(*key).and_then(Value::as_str))
                    .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok());
                if written.is_some_and(|at| at >= since) {
                    recent.push(line.to_owned());
                }
            }
        }
        recent
    }

    /// Whether the session `id` was reported already; marks it reported when it was not.
    fn reported(&self, id: &str) -> Result<bool> {
        let file = self.state(EXITED_SEEN);
        if holds_line(&file, id)? {
            return Ok(true);
        }
        append_line(&file, id)?;
        Ok(false)
    }

    /// Takes the session ids `returned` out of `exited.seen`, so the next exit of each is reported.
    fn forget(&self, returned: &BTreeSet<&str>) -> Result<()> {
        let file = self.state(EXITED_SEEN);
        let Some(text) = read_optional(&file)? else {
            return Ok(());
        };
        if !text.lines().any(|id| returned.contains(id)) {
            return Ok(());
        }
        let kept: String = text
            .lines()
            .filter(|id| !returned.contains(id))
            .map(|id| format!("{id}\n"))
            .collect();
        replace(&file, &kept)
    }

    // -----------------------------------------------------------------------------------------
    // Usage limit
    // -----------------------------------------------------------------------------------------

    /// Each usage limit a session's transcript tail shows, once per text per [`LIMIT_AGAIN`]
    /// from its last report, with a notification.
    fn limits(
        &mut self,
        now: OffsetDateTime,
        sessions: &[Session],
        lines: &mut Vec<String>,
        notes: &mut dyn Write,
    ) -> Result<()> {
        let since = now - LIMIT_WITHIN;
        let found: Vec<(String, &str)> = sessions
            .iter()
            .filter_map(|session| {
                rate_limit(&self.transcript(session), since)
                    .map(|text| (text, session.name.as_str()))
            })
            .collect();
        let new: String = found
            .iter()
            .map(|(text, name)| format!("{text}\t{name}\n"))
            .collect();
        replace(&self.state(LIMIT_NEW), &new)?;
        let today = day(now);
        let reported_file = self.state(LIMIT_REPORTED);
        let mut reported: BTreeMap<String, OffsetDateTime> = read_optional(&reported_file)?
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let (at, text) = line.split_once(' ')?;
                let at = OffsetDateTime::parse(at, &Rfc3339).ok()?;
                Some((text.to_owned(), at))
            })
            .filter(|(_, at)| now - *at < LIMIT_AGAIN)
            .collect();
        let texts: BTreeSet<&str> = found.iter().map(|(text, _)| text.as_str()).collect();
        let mut again = false;
        for text in texts {
            if reported.contains_key(text) {
                continue;
            }
            reported.insert(text.to_owned(), now);
            again = true;
            let names: BTreeSet<&str> = found
                .iter()
                .filter(|(limit, _)| limit == text)
                .map(|(_, name)| *name)
                .collect();
            let count = names.len();
            let names: Vec<&str> = names.into_iter().collect();
            lines.push(format!(
                "usage limit: {text}; {count} sessions: {}",
                names.join(" ")
            ));
            append_line(&self.state(LIMIT_SEEN), &format!("{today} {text}"))?;
            let body = format!(
                "{text}. {count} sessions stopped. After rotating or the reset, tell conductor \
                 to resume."
            );
            let title = format!("{}: usage limit", self.settings.instance);
            match self.notify(&title, &body) {
                Ok(()) => self.answered("notify"),
                Err(error) => self.failed("notify", &error, notes),
            }
        }
        if again {
            let kept = reported
                .iter()
                .map(|(text, at)| Ok(format!("{} {text}\n", at.format(&Rfc3339)?)))
                .collect::<Result<String, time::error::Format>>()
                .context("write the watch's clock as RFC 3339")?;
            replace(&reported_file, &kept)?;
        }
        Ok(())
    }

    fn notify(&self, title: &str, body: &str) -> Result<()> {
        let shown = command_line(&self.sources.notify);
        let (program, arguments) = self
            .sources
            .notify
            .split_first()
            .context("no notifier is given")?;
        let output = collect::run(
            Command::new(program).args(arguments).arg(title).arg(body),
            NOTIFY_BOUND,
        )
        .with_context(|| format!("notify the operator through `{shown}`"))?;
        if !output.status.success() {
            bail!("`{shown}` exited {:?}", output.status.code());
        }
        Ok(())
    }

    /// Where Claude Code keeps `session`'s transcript.
    fn transcript(&self, session: &Session) -> PathBuf {
        let directory: String = session
            .cwd
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let name: String = session
            .session
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        self.sources
            .projects
            .join(directory)
            .join(format!("{name}.jsonl"))
    }

    // -----------------------------------------------------------------------------------------
    // Context
    // -----------------------------------------------------------------------------------------

    /// The context of each live session, one with a `pid`, above the instance's
    /// `context_handover`, once per session. The session is named by the repository it works on
    /// ([`Sources::repository`]), else by its name: conductor's line reads `context: conductor`
    /// either way.
    fn context(&self, sessions: &[Session], lines: &mut Vec<String>) -> Result<()> {
        let file = self.state(CONTEXT_REPORTED);
        for session in sessions.iter().filter(|session| session.pid.is_some()) {
            let Some(tokens) = context_tokens(&self.transcript(session)) else {
                continue;
            };
            if tokens > self.settings.context_handover && !holds_line(&file, &session.session)? {
                // A session named after one of the instance's roles (conductor, conductor-dev) is
                // labelled by that name: conductor-dev works in conductor's directory, so the
                // directory would name it "conductor" and make conductor hand itself over. Every
                // other session is labelled by its directory.
                let role_named = crate::config::active()
                    .instance
                    .roles
                    .iter()
                    .any(|role| role.role == session.name);
                let named = if role_named {
                    session.name.clone()
                } else {
                    self.sources
                        .repository(&session.cwd)
                        .unwrap_or_else(|| session.name.clone())
                };
                lines.push(format!(
                    "context: {} {}k tokens (session {})",
                    one_line(&named),
                    tokens / 1000,
                    session.id
                ));
                append_line(&file, &session.session)?;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Main's CI
    // -----------------------------------------------------------------------------------------

    /// Each workflow whose newest `main` run that decided something turned it red or green again
    /// since the kept state, then keeps it, for each repository of [`Watch::repositories`]. Each
    /// repository's run list is run once; one that does not answer is named on `notes`, and the
    /// others are read. Without a complete snapshot no run list is read, and that is named on
    /// `notes` once, until one completes.
    fn ci(&mut self, lines: &mut Vec<String>, notes: &mut dyn Write) -> Result<()> {
        let repositories = match self.repositories() {
            Ok(repositories) => {
                self.answered(SNAPSHOT);
                repositories
            }
            Err(error) => {
                self.failed(SNAPSHOT, &error, notes);
                return Ok(());
            }
        };
        let read = each(&repositories, WIDTH, |repository| {
            Ok(newest_runs(&self.sources, repository))
        })?;
        let mut fresh = Vec::new();
        for (repository, answer) in repositories.iter().zip(read) {
            let source = format!("ci {repository}");
            match answer {
                Ok(runs) => {
                    self.answered(&source);
                    fresh.extend(runs);
                }
                Err(error) => self.failed(&source, &error, notes),
            }
        }
        if fresh.is_empty() {
            return Ok(());
        }
        let file = self.state(CI_STATE);
        let mut kept: BTreeMap<(String, String), Latest> = read_optional(&file)?
            .unwrap_or_default()
            .lines()
            .filter_map(Latest::parse)
            .map(|latest| (latest.key(), latest))
            .collect();
        let compare = !kept.is_empty();
        for latest in fresh {
            let key = latest.key();
            match kept.get(&key) {
                Some(before) if compare => {
                    if later(&before.at, &latest.at) {
                        continue;
                    }
                    if red(&latest.conclusion) && !red(&before.conclusion) {
                        lines.push(format!(
                            "CI red on main: {} / {} (run {})",
                            latest.repository, latest.workflow, latest.run
                        ));
                    } else if latest.conclusion == GREEN && red(&before.conclusion) {
                        lines.push(format!(
                            "CI green again on main: {} / {}",
                            latest.repository, latest.workflow
                        ));
                    }
                    kept.insert(key, latest);
                }
                _ => {
                    kept.insert(key, latest);
                }
            }
        }
        let state: String = kept
            .values()
            .map(|latest| format!("{}\n", latest.line()))
            .collect();
        replace(&file, &state)
    }

    /// The repositories whose `main` CI a pass reads: [`Sources::repositories`], or, with a
    /// [`Sources::store`], the Active ones of [`repository::activity`] over the store's newest
    /// complete snapshot, by name. The store is opened for the read and closed after it.
    ///
    /// # Errors
    ///
    /// There is no store, it does not open, or it holds no complete snapshot yet.
    fn repositories(&self) -> Result<Vec<String>> {
        let Some(state) = &self.sources.store else {
            return Ok(self.sources.repositories.clone());
        };
        let what = "no snapshot yet; main CI is read of no repository";
        let store = state::open_existing(Some(state)).context(what)?;
        let complete = SnapshotStorage::list(&store)
            .iter()
            .any(|snapshot| snapshot.state == SnapshotState::Complete);
        if !complete {
            bail!(
                "{what}: no complete snapshot in the store under {}",
                state.display()
            );
        }
        Ok(repository::activity(&store)
            .into_iter()
            .filter(|row| row.activity == Activity::Active)
            .map(|row| row.repository.0)
            .collect())
    }

    // -----------------------------------------------------------------------------------------
    // Disk
    // -----------------------------------------------------------------------------------------

    /// Free disk under the instance's `disk_low`, once per fall: the next report waits until
    /// free disk was back at its `disk_clear`.
    fn disk(&mut self, lines: &mut Vec<String>, notes: &mut dyn Write) -> Result<()> {
        let free = match self.free_bytes() {
            Ok(free) => {
                self.answered("disk");
                free.div_ceil(1 << 30)
            }
            Err(error) => {
                self.failed("disk", &error, notes);
                return Ok(());
            }
        };
        let marker = self.state(DISK_LOW);
        if free < self.settings.disk_low && !marker.exists() {
            lines.push(format!("disk low: {free}G free on /"));
            fs::write(&marker, "").with_context(|| format!("write {}", marker.display()))?;
        } else if free >= self.settings.disk_clear {
            match fs::remove_file(&marker) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).with_context(|| format!("remove {}", marker.display()));
                }
            }
        }
        Ok(())
    }

    fn free_bytes(&self) -> Result<u64> {
        let shown = command_line(&self.sources.disk);
        let (program, arguments) = self
            .sources
            .disk
            .split_first()
            .context("no free-disk command is given")?;
        let output = collect::run(Command::new(program).args(arguments), self.sources.bound)
            .with_context(|| format!("read free disk `{shown}`"))?;
        if !output.status.success() {
            bail!("`{shown}` exited {:?}", output.status.code());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .last()
            .and_then(|line| line.trim().parse::<u64>().ok())
            .ok_or_else(|| anyhow!("`{shown}` printed no byte count"))
    }
}

/// The newest `main` run of each workflow of `repository` that decided something: completed,
/// green or red ([`decided`]). The run list is run once, through [`collect::run`].
fn newest_runs(sources: &Sources, repository: &str) -> Result<Vec<Latest>> {
    let fill = [
        ("organization", sources.organization.as_str()),
        ("repository", repository),
    ];
    let command = filled(&sources.runs, &fill);
    let shown = command.join(" ");
    let what = format!("main CI of {repository} not read");
    let (program, arguments) = command
        .split_first()
        .with_context(|| format!("{what}: no run-list command is given"))?;
    let output = collect::run(Command::new(program).args(arguments), sources.bound)
        .with_context(|| format!("{what}: `{shown}`"))?;
    if !output.status.success() {
        let status = output.status.code().map_or_else(
            || "was ended by a signal".to_owned(),
            |code| format!("exited {code}"),
        );
        let stderr: String = one_line(&String::from_utf8_lossy(&output.stderr))
            .chars()
            .take(EXCERPT)
            .collect();
        if stderr.is_empty() {
            bail!("{what}: `{shown}` {status}");
        }
        bail!("{what}: `{shown}` {status}: {stderr}");
    }
    let runs: Vec<Run> = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("{what}: `{shown}` printed no run list"))?;
    let mut newest: BTreeMap<String, Run> = BTreeMap::new();
    for run in runs {
        let conclusion = run.conclusion.as_deref().unwrap_or_default();
        if run.status.as_deref() != Some("completed") || !decided(conclusion) {
            continue;
        }
        if newest
            .get(&run.workflow_name)
            .is_none_or(|known| !later(&known.created_at, &run.created_at))
        {
            newest.insert(run.workflow_name.clone(), run);
        }
    }
    Ok(newest
        .into_values()
        .map(|run| Latest {
            repository: repository.to_owned(),
            workflow: one_line(&run.workflow_name).replace('|', "/"),
            run: run.database_id,
            conclusion: run.conclusion.unwrap_or_default(),
            at: run.created_at,
        })
        .collect())
}

/// Whether the instant `a` is later than `b`, both RFC 3339; as text when either is not one.
fn later(a: &str, b: &str) -> bool {
    match (
        OffsetDateTime::parse(a, &Rfc3339),
        OffsetDateTime::parse(b, &Rfc3339),
    ) {
        (Ok(a), Ok(b)) => a > b,
        _ => a > b,
    }
}

impl Sources {
    /// Whether a session working in `cwd` is watched: in the checkouts root, the managed trees or
    /// the records, when given.
    fn watches(&self, cwd: &str) -> bool {
        let mut places = vec![self.root.as_path(), self.trees.as_path()];
        places.extend(self.records.as_deref());
        watched(cwd, &places)
    }

    /// The repository a session working in `cwd` works on, by the checkouts root and the managed
    /// trees ([`repository_of`]).
    fn repository(&self, cwd: &str) -> Option<String> {
        repository_of(cwd, &[self.root.as_path(), self.trees.as_path()])
    }
}

/// Whether `cwd` lies in one of `places`: an absolute path free of `..`, compared by component.
fn watched(cwd: &str, places: &[&Path]) -> bool {
    let cwd = Path::new(cwd);
    cwd.is_absolute()
        && !cwd.components().any(|part| part == Component::ParentDir)
        && places.iter().any(|place| cwd.starts_with(place))
}

/// The first component of `cwd` below the deepest of `places` that holds it; none for one of
/// `places` itself, or a directory none holds.
fn repository_of(cwd: &str, places: &[&Path]) -> Option<String> {
    let cwd = Path::new(cwd);
    places
        .iter()
        .filter_map(|place| {
            let below = cwd.strip_prefix(place).ok()?;
            Some((place.components().count(), below))
        })
        .max_by_key(|(depth, _)| *depth)
        .and_then(|(_, below)| match below.components().next() {
            Some(Component::Normal(name)) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
}

/// Whether a line of `recent` names the session `id`, or its `pid` as a word.
fn explained(recent: &[String], id: &str, pid: Option<u64>) -> bool {
    let pid = pid.map(|pid| pid.to_string());
    recent
        .iter()
        .any(|line| line.contains(id) || pid.as_deref().is_some_and(|pid| holds_word(line, pid)))
}

/// Whether `word` occurs in `line` with no letter, digit or `_` on either side.
fn holds_word(line: &str, word: &str) -> bool {
    let constituent = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    line.match_indices(word).any(|(at, _)| {
        !constituent(line[..at].chars().next_back())
            && !constituent(line[at + word.len()..].chars().next())
    })
}

/// The text of the newest `"error":"rate_limit"` line later than `since` among the last
/// [`TAIL`] lines of `transcript`; none when there is none or the file cannot be read.
fn rate_limit(transcript: &Path, since: OffsetDateTime) -> Option<String> {
    let mut lines = Backward::open(transcript).ok()?;
    for _ in 0..TAIL {
        let line = lines.next_line().ok()??;
        let Ok(record) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        if record["error"] != "rate_limit" {
            continue;
        }
        let at = record["timestamp"]
            .as_str()
            .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok());
        if at.is_none_or(|at| at <= since) {
            continue;
        }
        let text = record["message"]["content"][0]["text"]
            .as_str()
            .unwrap_or("usage limit");
        return Some(one_line(text));
    }
    None
}

/// The context of the newest line of `transcript` that carries `cache_read_input_tokens`: its
/// input, cache-read and cache-creation tokens.
fn context_tokens(transcript: &Path) -> Option<u64> {
    let mut lines = Backward::open(transcript).ok()?;
    while let Some(line) = lines.next_line().ok()? {
        if !contains(&line, b"\"cache_read_input_tokens\"") {
            continue;
        }
        let record: Value = serde_json::from_slice(&line).ok()?;
        let usage = &record["message"]["usage"];
        return Some(
            [
                "input_tokens",
                "cache_read_input_tokens",
                "cache_creation_input_tokens",
            ]
            .iter()
            .map(|key| usage[*key].as_u64().unwrap_or(0))
            .sum(),
        );
    }
    None
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The lines of a file from its end, read [`CHUNK`] bytes at a time, so a long transcript is
/// not read whole. Empty lines are skipped.
struct Backward {
    file: File,
    /// The bytes before `at` are not read yet.
    at: u64,
    /// The start of the line the read bytes begin in, not complete yet.
    partial: Vec<u8>,
    /// Complete lines, the last of the file last.
    ready: Vec<Vec<u8>>,
}

impl Backward {
    fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let at = file.metadata()?.len();
        Ok(Self {
            file,
            at,
            partial: Vec::new(),
            ready: Vec::new(),
        })
    }

    fn next_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        loop {
            if let Some(line) = self.ready.pop() {
                if line.is_empty() {
                    continue;
                }
                return Ok(Some(line));
            }
            if self.at == 0 {
                let line = std::mem::take(&mut self.partial);
                return Ok((!line.is_empty()).then_some(line));
            }
            let size = CHUNK.min(self.at);
            self.at -= size;
            let mut chunk = vec![0; usize::try_from(size).unwrap_or(usize::MAX)];
            self.file.seek(SeekFrom::Start(self.at))?;
            self.file.read_exact(&mut chunk)?;
            chunk.append(&mut self.partial);
            let mut parts = chunk.split(|byte| *byte == b'\n');
            self.partial = parts.next().unwrap_or_default().to_vec();
            self.ready = parts.map(<[u8]>::to_vec).collect();
        }
    }
}

/// Each month from `since`'s through `now`'s, as `YYYY-MM`, UTC: the names of their dispatch
/// logs.
fn months(since: OffsetDateTime, now: OffsetDateTime) -> Vec<String> {
    let since = since.to_offset(UtcOffset::UTC);
    let now = now.to_offset(UtcOffset::UTC);
    let (mut year, mut month) = (since.year(), u8::from(since.month()));
    let last = (now.year(), u8::from(now.month()));
    let mut months = Vec::new();
    while (year, month) <= last {
        months.push(format!("{year:04}-{month:02}"));
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }
    months
}

/// `at` as `YYYY-MM-DD`, UTC.
fn day(at: OffsetDateTime) -> String {
    let at = at.to_offset(UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}",
        at.year(),
        u8::from(at.month()),
        at.day()
    )
}

/// What `file` holds, or `None` when it is not there.
fn read_optional(file: &Path) -> Result<Option<String>> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", file.display())),
    }
}

/// Whether `file` holds the line `line`; a file that is not there holds none.
fn holds_line(file: &Path, line: &str) -> Result<bool> {
    Ok(read_optional(file)?.is_some_and(|text| text.lines().any(|kept| kept == line)))
}

fn append_line(file: &Path, line: &str) -> Result<()> {
    let mut opened = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .with_context(|| format!("open {}", file.display()))?;
    writeln!(opened, "{line}").with_context(|| format!("write {}", file.display()))
}

/// Writes `text` to `file` through a sibling file and a rename, so a reader never sees half.
fn replace(file: &Path, text: &str) -> Result<()> {
    let mut next = file.as_os_str().to_owned();
    next.push(".next");
    let next = PathBuf::from(next);
    fs::write(&next, text).with_context(|| format!("write {}", next.display()))?;
    fs::rename(&next, file).with_context(|| format!("replace {}", file.display()))
}

/// `text` on one line: each line break or tab a space.
fn one_line(text: &str) -> String {
    text.split(['\n', '\r', '\t'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn command_line(command: &[OsString]) -> String {
    command
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    use super::{holds_word, later, months, repository_of, watched};

    #[test]
    fn the_dispatch_logs_read_are_each_month_from_since_through_now() {
        let at = |text: &str| OffsetDateTime::parse(text, &Rfc3339).expect("an instant");
        assert_eq!(
            months(at("2026-10-07T11:00:00Z"), at("2026-10-07T12:00:00Z")),
            ["2026-10"]
        );
        assert_eq!(
            months(at("2026-11-30T23:50:00Z"), at("2027-02-01T00:05:00+01:00")),
            ["2026-11", "2026-12", "2027-01"]
        );
    }

    #[test]
    fn a_pid_is_a_word_only_between_non_word_characters() {
        assert!(holds_word("stop pid 2102 now", "2102"));
        assert!(holds_word("2102", "2102"));
        assert!(holds_word("\"pid\":2102}", "2102"));
        assert!(holds_word("load 21020 and pid 2102", "2102"));
        assert!(!holds_word("load 21020", "2102"));
        assert!(!holds_word("x12102", "2102"));
        assert!(!holds_word("pid_2102", "2102"));
    }

    #[test]
    fn a_working_directory_is_watched_in_the_instances_places_only() {
        let places = [Path::new("/h/work"), Path::new("/h/.trees/org")];
        for inside in ["/h/work", "/h/work/conductor", "/h/.trees/org/ess/tree"] {
            assert!(watched(inside, &places), "{inside}");
        }
        for outside in [
            "/h",
            "/h/work-old/x",
            "/h/work/../elsewhere",
            "/h/.trees/other/x",
            "work/conductor",
        ] {
            assert!(!watched(outside, &places), "{outside}");
        }
    }

    #[test]
    fn a_session_works_on_the_first_directory_below_the_deepest_place_holding_it() {
        let places = [Path::new("/h/work"), Path::new("/h/work/.trees")];
        for (cwd, repository) in [
            ("/h/work/ess", Some("ess")),
            ("/h/work/ess/crates/x", Some("ess")),
            ("/h/work/.trees/loom/tree-1", Some("loom")),
            ("/h/work", None),
            ("/h/work/.trees", None),
            ("/h/elsewhere/x", None),
        ] {
            assert_eq!(repository_of(cwd, &places).as_deref(), repository, "{cwd}");
        }
    }

    #[test]
    fn a_later_instant_is_compared_as_an_instant() {
        assert!(later("2026-10-07T11:00:00Z", "2026-10-07T10:00:00Z"));
        assert!(later(
            "2026-10-07T11:00:00+00:00",
            "2026-10-07T12:30:00+02:00"
        ));
        assert!(!later("2026-10-07T10:00:00Z", "2026-10-07T10:00:00Z"));
    }
}
