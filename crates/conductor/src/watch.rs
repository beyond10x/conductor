//! `conductor watch run`: wake conductor on the first change (`story:watch-command`).
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `watch-run` and its input `conductor.observation.WatchRun`. Like `dashboard
//! serve`, it is no placed command of the `cli:` block: it writes no store record. It replaces
//! the stopgap loop conductor ran with Bash `run_in_background`, and keeps its probes, its lines
//! and its state files.
//!
//! It watches the instance the process runs ([`config::active`]): its checkouts, its records,
//! its first `github` owner and its `gitlab` sources ([`Watch::with_gitlab`]), its cadence
//! (`cadence.watch` and `cadence.ci`, which `--every` and `--ci-every` override;
//! [`Options::of`]) and its thresholds (`thresholds.context_handover`,
//! `disk_low`, `disk_clear`, `disk_admit`, `gate_stop`, `gate_resume` and `silent_session`;
//! [`Settings::of`]), and names it in the
//! usage-limit notification's title. Without a config file that is the built-in instance.
//!
//! It runs cheap probes, not snapshots. A pass, every [`Options::every`]:
//!
//! | probe | source ([`Sources`]) | line |
//! |---|---|---|
//! | sessions | the session list (`claude agents --json`); only the entries whose working directory lies in the instance's checkouts root or managed trees, or in its records when a config file names them, so no other session is ever named | `session exited (blocked): <name> <id>`, `session gone: <name> <id>`, `conductors: <n> live sessions named <name>: <id> <id>…` |
//! | usage limit | the last [`TAIL`] lines of each such session's transcript | `usage limit: <text>; <n> sessions: <names>` |
//! | context | the newest line with a usage in the transcript of each such session that has a `pid` | `context: <repo> <n>k tokens (session <id>)` |
//! | silent controllers | the modification times of the transcript of each such session that is a controller, and of its sub-agents' transcripts | `session silent: <name> <id> <minutes>m` |
//! | `main` CI, on the first pass and then every [`Options::ci_every`] | the newest completed `main` run per workflow of each repository the newest complete snapshot holds Active ([`crate::repository::activity`]); for a project of a `gitlab` source, the newest pipeline of its default branch that decided something, the workflow `pipeline` (`story:watch-gitlab-ci`) | `CI red on main: <repo> / <workflow> (run <id>)`, `CI green again on main: <repo> / <workflow>` |
//! | review, with each `main` CI read | the pull requests that wait for review in the newest complete snapshot, counted per repository as the board counts them ([`crate::status::waiting_in`]), against `controllers.max_open_reviews` ([`Settings::max_open_reviews`], `0` no limit) | `review full: <repo> <k>/<n>` once when a repository reaches its limit, `review open: <repo>` once when it falls under it (`story:review-limit`) |
//! | Gates hooks, with each `main` CI read | the repositories the board names `stale gates hooks` in the newest complete snapshot ([`crate::status::stale_gates_hooks`]) | `gates hooks stale: <n> checkouts` once per new snapshot that finds any (`story:gates-hooks-fresh`) |
//! | Workspace file, with each `main` CI read | the repositories the newest complete snapshot records as `NotInWorkspace` ([`crate::status::unlisted_repositories`]) | `workspace stale: <repo>`, one line each, once per new snapshot (`story:snapshot-unlisted-repository`) |
//! | free disk | `statvfs` on `/` ([`crate::disk::space`]) | `disk low: <n>G free on /`, `disk admit: <n>G free on / (builds wait)`, `disk gate stop: <n>G free on / (gates stop)` |
//! | usage | the usage readings of the complete snapshots of the store under the state directory ([`crate::collect::usage::Forecast`], [`Watch::with_usage`]) | `usage: pool runs out ~<hour>, next reset <minute>`, once per change of the forecast hour (`usage.hour`; `story:usage-probe`) |
//! | escalated decision requests | the decision requests in state `Escalated` (class O or H) of the store under the state directory ([`crate::status::unanswered`], [`Watch::with_decisions`]) | `escalated <O\|H> <request-id> <question>` in the first pass that finds one; `unanswered <O\|H> <request-id> <minutes>` once, in the first pass [`UNANSWERED_AFTER`] (30 minutes) or more after that one (`escalated.seen`; `story:decisions-notify`), and once more in the first pass at or after the deadline its escalation recorded, `<minutes>` counted from the pass that printed its escalation (`story:decisions-page`); `answered <request-id> <choice>` once, in the first pass after a printed request was answered, from the Decisions page or by conductor, its decision's choice on one line, and nothing for one withdrawn (`story:decisions-batch-answer`) |
//!
//! `<id>` is the first eight characters of a session's `sessionId`. A session is reported:
//! - **exited** when it turns `blocked` without a `pid`, which is how the list keeps a session
//!   stopped at the usage limit; **gone** when it leaves the list, unless it was already
//!   exited. Either only when no line of the dispatch log written since the kept list was taken
//!   or in the last [`EXPLAINED_WITHIN`], whichever is earlier, names its id, or its pid as a
//!   word, or is of a dispatch (has the `id` of one) a line of which names its id, however old:
//!   conductor's `sent` line names the session in `session`, and the `done` line it writes
//!   before it stops the session names only the dispatch. A pid in an older line links no
//!   dispatch, since another process may have had it. The log is read from the month before
//!   that instant's, which holds the `sent` line of any dispatch shorter than a month. Conductor
//!   itself stopped such a session; and each session id once (`exited.seen`)
//!   until the list shows it with a `pid` again, so its next exit is reported once more. A start
//!   is no change.
//! - **conductors** when two or more live sessions (with a `pid`) carry the session name of the
//!   instance's conductor role ([`Settings::conductor`]), by their ids in order: a session
//!   resumed by hand without the instance's settings runs no hook and no guard
//!   (`story:watch-two-conductors`). Once per set of ids (`conductors.reported`), which is
//!   forgotten once fewer than two are live.
//! - **usage limit** for a `"error":"rate_limit"` line of the last [`LIMIT_WITHIN`], once per
//!   limit text per [`LIMIT_AGAIN`] from its last report, with a desktop notification to the
//!   operator through [`Sources::notify`], titled `<instance>: usage limit`, because conductor is
//!   stopped by the same limit.
//! - **context** above the instance's `context_handover` tokens, once per session
//!   (`story:controller-context`). `<repo>` is the directory the session works in under the
//!   managed trees or the checkouts root, else the session's name; conductor's line reads
//!   `context: conductor` either way.
//! - **silent** when a live controller (a `pid`, its list `state` `working` or `waiting`, and not
//!   named as one of the instance's roles) and every one of its sub-agents have written nothing
//!   to their transcripts for longer than the instance's `silent_session`, by the newest
//!   modification time among them (`story:watch-silent-controller`). Once per silence
//!   (`silent.reported`, `<sessionId> <newest write>` per line): it is reported again only after
//!   it has written and fallen silent anew. A session without a transcript is not read.
//! - **CI** when a workflow's newest `main` run that decided something (completed, `success` or
//!   one of [`RED`], as the github collector maps them) is not older than the one kept and turns
//!   it red (one of [`RED`] after another conclusion), or green again (`success` after one of
//!   [`RED`]): by its conclusion, so a failed run re-run to success is reported with the same run
//!   id. Every other conclusion (`cancelled`, `skipped`, empty, `neutral`, `action_required`,
//!   `stale`, and one GitHub adds) decides nothing; it, and a workflow missing from the list,
//!   keep the last run as it was. `ci.state` keeps the conclusion as GitHub printed it. While
//!   the store under the state directory holds no complete snapshot, no run list is read.
//! - **disk** under each of the instance's floors, once per fall (`story:watch-disk-floors`):
//!   `disk low` under `disk_low` GiB, its next report waiting until free disk was back at
//!   `disk_clear` GiB; `disk admit` under `disk_admit`, until it was back at `disk_admit`; and
//!   `disk gate stop` under `gate_stop`, until it was back at `gate_resume`. One pass reports
//!   each floor it crossed, in that order. A reading more than [`DISK_JUMP`] from the
//!   last pass's is read again before it is compared, and the second reading stands
//!   (`story:watch-disk-reading`): one odd reading is not reported.
//! - **reclaim** once per fall under `disk_admit`, in the pass that reports `disk admit`
//!   (`story:disk-reclaim`, `story:disk-escalation`): the watch runs [`crate::gc::reclaim`] to
//!   the instance's floor ([`crate::gc::reclaim::floor_bytes`]: `disk_admit` plus a margin) and
//!   reports `disk reclaimed: <n>G` when it reached it, else `disk floor not reached: <n>G short`
//!   with the GiB still lacking by a reading taken after it, so conductor raises it to the
//!   operator. A reclaim that fails is named on standard error, as a source that does not answer.
//!
//! An empty or unreadable session list skips the five session probes for that pass. A source
//! that does not answer is named once on standard error, `conductor watch: <error>`, until it
//! answers again; the watch goes on. A CI pass runs each repository's run list once, and a
//! repository whose list does not answer is such a source of its own, named in its line; a
//! store without a complete snapshot is one too.
//!
//! [`Watch::run`] exits after the first pass that printed a line, with every line of that pass;
//! with [`Options::follow`] it prints and goes on, for the harness `Monitor` tool. Its state is
//! kept under `<state>/watch/`: `sessions.json` (the kept list and the instant it was taken),
//! `exited.seen`, `conductors.reported` (the ids last reported, on one line), `silent.reported`,
//! `review.full` (the repositories last reported full, one per line), `gates-hooks.snapshot`
//! (the id of the snapshot the Gates hooks probe read last), `escalated.seen` (each escalated
//! request printed, when, whether its 30-minute unanswered line was, and whether its line at the
//! deadline was), `usage.hour` (the forecast hour the
//! usage line was last printed for),
//! `limit.reported` (`<instant> <text>`, the last report of each limit text), `disk-low`,
//! `disk-admit`, `gate-stop`, `disk.free` (the bytes free the last pass read), and, in the
//! formats of the stopgap, `limit.new` (`<text>\t<name>`), `limit.seen` (`YYYY-MM-DD <text>`),
//! `context-reported` (a `sessionId` per line) and `ci.state`
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
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use conductor_model::behaviour::{DecisionRequestStorage, DecisionStorage, SnapshotStorage};
use conductor_model::config::{Instance, Source};
use conductor_model::decision::RequestId;
use conductor_model::direction::Activity;
use conductor_model::observation::{RunConclusion, SnapshotState};

use crate::cli::WatchRunArgs;
use crate::collect::gitlab::{self, Forge, PIPELINE, Project};
use crate::collect::repositories::{WIDTH, each, filled, words};
use crate::collect::{self, BOUND, Origins};
use crate::config::{self, Active};
use crate::gc::reclaim::ReclaimReport;
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

/// How far, in bytes, a free-disk reading may lie from the previous pass's before it is read
/// again (`story:watch-disk-reading`): 20 GiB.
pub const DISK_JUMP: u64 = 20 << 30;

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
const DISK_ADMIT: &str = "disk-admit";
const GATE_STOP: &str = "gate-stop";
const DISK_FREE: &str = "disk.free";
const CONDUCTORS_REPORTED: &str = "conductors.reported";
const SILENT_REPORTED: &str = "silent.reported";
const REVIEW_FULL: &str = "review.full";

/// The id of the last complete snapshot the Gates hooks probe read.
const GATES_HOOKS_READ: &str = "gates-hooks.snapshot";

/// The id of the last complete snapshot the workspace probe read (`story:snapshot-unlisted-repository`).
const WORKSPACE_READ: &str = "workspace.snapshot";

/// The forecast hour the usage line was last printed for (`story:usage-probe`).
const USAGE_HOUR: &str = "usage.hour";

/// The escalated requests the watch has printed, one per line: `<request-id>\t<instant of the
/// pass that printed its escalation, RFC 3339>\t<1 once its unanswered line was printed, else 0>`
/// (`story:decisions-notify`)`\t<1 once its line at the deadline was printed, else 0>`
/// (`story:decisions-page`; a line without it reads 0).
const ESCALATED_SEEN: &str = "escalated.seen";

/// How long an escalated request waits from the pass that printed its escalation before it is
/// printed unanswered (`story:decisions-notify`).
pub const UNANSWERED_AFTER: Duration = Duration::from_secs(30 * 60);

/// The states of the session list in which a live controller is expected to write to its
/// transcripts.
const LIVE: [&str; 2] = ["working", "waiting"];

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
    /// Every GitHub owner of the instance, in its order, each once; a repository's run list is
    /// read under each in turn until one answers. Empty: [`Sources::organization`] only.
    pub owners: Vec<String>,
    /// The state directory whose store names the repositories whose `main` CI is read: the
    /// Active ones of its newest complete snapshot ([`crate::repository::activity`]), read on
    /// each CI pass. `None`: [`Sources::repositories`].
    pub store: Option<PathBuf>,
    /// The repositories whose `main` CI is read when [`Sources::store`] is `None`.
    pub repositories: Vec<String>,
    /// Prints one repository's `main` runs as one JSON array of objects with `databaseId`,
    /// `workflowName`, `status`, `conclusion` and `createdAt`; `{repository}` is filled in.
    pub runs: Vec<String>,
    /// The path on whose file system free disk is read: `/`.
    pub disk: PathBuf,
    /// Reads the bytes free on the file system holding [`Sources::disk`]: the `available` of
    /// [`crate::disk::space`], or the readings [`crate::gc::reclaim::free_bytes`] is pointed at.
    pub free_bytes: fn(&Path) -> Result<u64>,
    /// The program and arguments of the desktop notifier; a title and a body are added.
    pub notify: Vec<OsString>,
    /// The current time.
    pub clock: fn() -> OffsetDateTime,
    /// How long the session list and one run list may take before they are stopped.
    pub bound: Duration,
}

impl Sources {
    /// The sources of the instance `active`, for the home directory `home` and the state
    /// directory `state`: `claude agents --json`; the instance's checkouts root and managed
    /// trees, and its records when a config file names it; `~/.claude/projects`; the dispatch log
    /// under the records when a config file names them, else under `conductor/` of the checkouts
    /// root, as before a config file existed; the owner of the instance's first `github` source,
    /// whose run lists `gh` prints for the Active repositories of the store under `state` (none
    /// for an instance without a `github` or a `gitlab` source; a `gitlab` source's are read
    /// through [`Watch::with_gitlab`]); the free bytes `statvfs` reads on `/`;
    /// `notify-send -u critical`; the system clock; and [`BOUND`].
    #[must_use]
    pub fn new(home: PathBuf, active: &Active, state: PathBuf) -> Self {
        let instance = &active.instance;
        let root = PathBuf::from(&instance.checkouts.root);
        let records = PathBuf::from(&instance.records);
        let owner = instance.sources.iter().find_map(|source| match source {
            Source::GitHub(github) => Some(github.owner.clone()),
            Source::Local(_) | Source::GitLab(_) => None,
        });
        let gitlab = instance
            .sources
            .iter()
            .any(|source| matches!(source, Source::GitLab(_)));
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
            store: (owner.is_some() || gitlab).then_some(state),
            organization: owner.unwrap_or_default(),
            owners: Origins::of(instance).owners,
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
            disk: PathBuf::from("/"),
            free_bytes: crate::gc::reclaim::free_bytes,
            notify: ["notify-send", "-u", "critical"]
                .map(OsString::from)
                .to_vec(),
            clock: OffsetDateTime::now_utc,
            bound: BOUND,
        }
    }
}

/// What the instance sets for the watch besides its sources: its name, its thresholds and its
/// conductor role's session name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The instance's name, which the usage-limit notification's title names.
    pub instance: String,
    /// The key of the instance's fall of free disk in the host directory: its session prefix, its
    /// name without one (`story:host-fall-per-instance`).
    pub fall_key: String,
    /// Tokens of a session's context above which it is reported: `thresholds.context_handover`.
    pub context_handover: u64,
    /// Free GiB on `/` under which it is reported: `thresholds.disk_low`.
    pub disk_low: u64,
    /// Free GiB on `/` from which a further fall is reported again: `thresholds.disk_clear`.
    pub disk_clear: u64,
    /// Free GiB on `/` under which conductor starts no build, reported as `disk admit`, and from
    /// which a further fall is reported again: `thresholds.disk_admit`.
    pub disk_admit: u64,
    /// Free GiB on `/` under which the full-gate watchdog stops a gate, reported as `disk gate
    /// stop`: `thresholds.gate_stop`.
    pub gate_stop: u64,
    /// Free GiB on `/` from which a further fall under [`Settings::gate_stop`] is reported
    /// again: `thresholds.gate_resume`.
    pub gate_resume: u64,
    /// How long a live controller, and each of its sub-agents, may write nothing to its
    /// transcript before it is reported silent: `thresholds.silent_session`.
    pub silent_session: Duration,
    /// The full session name of the instance's conductor role (`<session_prefix>-conductor`, or
    /// `conductor` without a prefix), two live sessions of which are reported; `None` for an
    /// instance without a conductor role.
    pub conductor: Option<String>,
    /// How many pull requests of one repository may wait for review before it is reported full:
    /// `controllers.max_open_reviews`; `0` is no limit (`story:review-limit`).
    pub max_open_reviews: u64,
}

impl Settings {
    /// The name, the thresholds and the conductor role's session name of `instance`.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        let count = |value: i64| u64::try_from(value).unwrap_or(0);
        let thresholds = &instance.thresholds;
        Self {
            instance: instance.name.0.clone(),
            fall_key: crate::host::fall_key(instance),
            context_handover: count(thresholds.context_handover.0),
            disk_low: count(thresholds.disk_low.0),
            disk_clear: count(thresholds.disk_clear.0),
            disk_admit: count(thresholds.disk_admit.0),
            gate_stop: count(thresholds.gate_stop.0),
            gate_resume: count(thresholds.gate_resume.0),
            silent_session: Duration::from_secs(
                config::seconds(&thresholds.silent_session)
                    .unwrap_or(config::SILENT_SESSION_MINUTES * 60),
            ),
            // The director's session, whose session name two live sessions must not carry: its
            // role under its name or its old name `conductor` (story:role-name-aliases).
            conductor: config::director(instance).map(|role| {
                role.session_name
                    .clone()
                    .unwrap_or_else(|| config::session_name(instance, &role.role))
                    .0
            }),
            max_open_reviews: count(instance.controllers.max_open_reviews),
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
    let host = crate::host::dir(state)?;
    let state = state::dir(state)?;
    let mut watch = Watch::new(
        Sources::new(home, active, state.clone()),
        state.join("watch"),
    )
    .with_settings(Settings::of(&active.instance))
    .with_gitlab(Forge::all(&active.instance))
    .with_usage(state.clone())
    .with_decisions(state.clone())
    .with_reclaim(state, crate::gc::reclaim::reclaim)
    .with_host(host);
    watch.run(&options, &mut io::stdout().lock(), &mut io::stderr().lock())?;
    Ok(ExitCode::SUCCESS)
}

/// The watch over its [`Sources`], keeping its state in one directory.
#[derive(Debug)]
pub struct Watch {
    sources: Sources,
    settings: Settings,
    /// The `gitlab` sources whose projects' default-branch pipelines are read as `main` CI.
    gitlab: Vec<Forge>,
    dir: PathBuf,
    /// The sources named on standard error that have not answered since.
    failing: BTreeSet<String>,
    /// The state directory a reclaim run is recorded under, and the reclaim run once per fall
    /// under `disk_admit`; none: no reclaim.
    reclaim: Option<(PathBuf, Reclaimer)>,
    /// The host directory whose fall of free disk an instance's pass ends
    /// (`story:instance-isolation`); none: the pass ends none.
    host: Option<PathBuf>,
    /// The state directory whose store's usage readings the usage probe reads; none: no usage
    /// line (`story:usage-probe`).
    usage: Option<PathBuf>,
    /// The state directory whose store's escalated decision requests the decisions probe reads;
    /// none: no `escalated` or `unanswered` line (`story:decisions-notify`).
    decisions: Option<PathBuf>,
}

/// A reclaim run to a floor, recorded under a state directory: [`crate::gc::reclaim::reclaim`],
/// or a test's fake.
pub type Reclaimer = fn(&Path, u64) -> Result<ReclaimReport>;

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
    /// Its `state` alone, else `-`: `working`, `waiting`, `idle`, `blocked` and so on.
    #[serde(default)]
    phase: String,
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
            gitlab: Vec::new(),
            dir,
            failing: BTreeSet::new(),
            reclaim: None,
            host: None,
            usage: None,
            decisions: None,
        }
    }

    /// This watch printing the `escalated` and `unanswered` lines of the decision requests of the
    /// store under the state directory `state` (`story:decisions-notify`).
    #[must_use]
    pub fn with_decisions(mut self, state: PathBuf) -> Self {
        self.decisions = Some(state);
        self
    }

    /// This watch running `reclaim` once per fall under `disk_admit`, to the instance's floor
    /// ([`crate::gc::reclaim::floor_bytes`]), recording it under the state directory `state`.
    #[must_use]
    pub fn with_reclaim(mut self, state: PathBuf, reclaim: Reclaimer) -> Self {
        self.reclaim = Some((state, reclaim));
        self
    }

    /// This watch ending, once free disk is back at `disk_clear`, the fall its instance recorded
    /// an operator item for in the host directory `host` ([`crate::host::clear_fall`]).
    #[must_use]
    pub fn with_host(mut self, host: PathBuf) -> Self {
        self.host = Some(host);
        self
    }

    /// This watch printing the usage line of the store under the state directory `state`
    /// (`story:usage-probe`).
    #[must_use]
    pub fn with_usage(mut self, state: PathBuf) -> Self {
        self.usage = Some(state);
        self
    }

    /// This watch on `settings`.
    #[must_use]
    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    /// This watch reading the `main` CI of each project of `forges` (`story:watch-gitlab-ci`): a
    /// repository a CI pass reads that is a project of one of them, not archived, is read from
    /// its forge ([`newest_pipeline`]), not through [`Sources::runs`]; and without a GitHub owner
    /// ([`Sources::organization`] empty) no other repository's run list is read.
    #[must_use]
    pub fn with_gitlab(mut self, forges: Vec<Forge>) -> Self {
        self.gitlab = forges;
        self
    }

    /// Passes every `options.every` (`main`'s CI on the first and then every
    /// `options.ci_every`), writing each change as a line to `out`, until the first pass that
    /// found one; with `options.follow`, for ever. A source that does not answer is named on
    /// `notes`. Each probe that reads the store opens it as a reader
    /// ([`crate::store::open_reader`]), which never takes a tree's writer lock, and drops it
    /// within the pass: nothing is held across a sleep
    /// (`story:store-readers-hold-no-writer-lock`).
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
                    self.conductors(&sessions, &mut lines)?;
                    self.limits(now, &sessions, &mut lines, notes)?;
                    self.context(&sessions, &mut lines)?;
                    self.silent(now, &sessions, &mut lines)?;
                }
            }
            Err(error) => self.failed("sessions", &error, notes),
        }
        if ci {
            self.ci(&mut lines, notes)?;
            self.reviews(&mut lines)?;
            self.gates_hooks(&mut lines)?;
            self.workspace_stale(&mut lines)?;
        }
        self.disk(&mut lines, notes)?;
        self.usage(&mut lines)?;
        self.decisions(now, &mut lines)?;
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
        let active = config::active();
        let instances = active.all();
        let mut sessions = Vec::new();
        for entry in &entries {
            let text = |key: &str| entry[key].as_str().filter(|text| !text.is_empty());
            let Some(cwd) = text("cwd").filter(|cwd| self.sources.watches(cwd)) else {
                continue;
            };
            let Some(session) = text("sessionId").or_else(|| text("id")) else {
                continue;
            };
            // Another instance's session is not this watch's, wherever it works: its name and
            // directory say whose it is (story:instance-session-names).
            if let Some(name) = text("name")
                && collect::session_instance(&instances, name, Some(Path::new(cwd)))
                    .is_some_and(|owner| owner.name != active.instance.name)
            {
                continue;
            }
            sessions.push(Session {
                id: session.chars().take(8).collect(),
                session: session.to_owned(),
                name: one_line(text("name").unwrap_or("unnamed")),
                pid: entry["pid"].as_u64(),
                state: text("status")
                    .or_else(|| text("state"))
                    .unwrap_or("-")
                    .to_owned(),
                phase: text("state").unwrap_or("-").to_owned(),
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

    /// The dispatch log read from the month before the month of `since` through the month of
    /// `now`: every line read, and the lines written from `since` on, by their `at`, else
    /// `sent_at`, else `decided_at`. The month before is enough for the line that names a
    /// session: it is its dispatch's `sent` line, which no dispatch outlives by a month.
    fn dispatches_since(&self, since: OffsetDateTime, now: OffsetDateTime) -> Dispatches {
        let since_utc = since.to_offset(UtcOffset::UTC);
        let earlier = since_utc
            .date()
            .replace_day(1)
            .ok()
            .and_then(time::Date::previous_day)
            .map_or(since_utc, |day| day.midnight().assume_utc());
        let mut log = Dispatches::default();
        for month in months(earlier, now) {
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
                let line = DispatchLine {
                    id: record.get("id").and_then(Value::as_str).map(str::to_owned),
                    text: line.to_owned(),
                };
                if written.is_some_and(|at| at >= since) {
                    log.recent.push(line.clone());
                }
                log.read.push(line);
            }
        }
        log
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

    /// Two or more live sessions, ones with a `pid`, named as the instance's conductor role
    /// (`story:watch-two-conductors`): one line naming each session id, once per set of ids
    /// (`conductors.reported`), which is forgotten once fewer than two are live.
    fn conductors(&self, sessions: &[Session], lines: &mut Vec<String>) -> Result<()> {
        let Some(name) = &self.settings.conductor else {
            return Ok(());
        };
        let ids: Vec<&str> = sessions
            .iter()
            .filter(|session| session.pid.is_some() && session.name == *name)
            .map(|session| session.id.as_str())
            .collect();
        let file = self.state(CONDUCTORS_REPORTED);
        let reported = read_optional(&file)?.unwrap_or_default();
        if ids.len() < 2 {
            if !reported.is_empty() {
                replace(&file, "")?;
            }
            return Ok(());
        }
        let set = format!("{}\n", ids.join(" "));
        if reported != set {
            lines.push(format!(
                "conductors: {} live sessions named {}: {}",
                ids.len(),
                one_line(name),
                ids.join(" ")
            ));
            replace(&file, &set)?;
        }
        Ok(())
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
                // A role's session is matched by its session name (`<session_prefix>-<role>`, or
                // the role without a prefix) and labelled by the role, so conductor's line reads
                // `context: conductor` whatever the prefix (story:instance-session-names).
                let named = if let Some(role) = role_named(&session.name) {
                    role
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
    // Silent controllers
    // -----------------------------------------------------------------------------------------

    /// Each live controller, one with a `pid` the list shows `working` or `waiting` and not named
    /// as one of the instance's roles, whose transcript and every one of its sub-agents' have
    /// written nothing for longer than the instance's `silent_session` at `now`
    /// (`story:watch-silent-controller`). Once per silence: `silent.reported` keeps, per session,
    /// the newest write its report was of, and forgets it once the session writes again or
    /// leaves the list. A session without a transcript is not read.
    fn silent(
        &self,
        now: OffsetDateTime,
        sessions: &[Session],
        lines: &mut Vec<String>,
    ) -> Result<()> {
        let file = self.state(SILENT_REPORTED);
        let before = read_optional(&file)?.unwrap_or_default();
        let reported: BTreeMap<&str, &str> = before
            .lines()
            .filter_map(|line| line.split_once(' '))
            .collect();
        let now = SystemTime::from(now);
        let mut kept = String::new();
        for session in sessions {
            if role_named(&session.name).is_some() {
                continue;
            }
            let Some(newest) = newest_write(&self.transcript(session)) else {
                continue;
            };
            let written = newest
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
                .to_string();
            if reported.get(session.session.as_str()) == Some(&written.as_str()) {
                kept.push_str(&format!("{} {written}\n", session.session));
                continue;
            }
            let silence = now.duration_since(newest).unwrap_or_default();
            let live = session.pid.is_some() && LIVE.contains(&session.phase.as_str());
            if live && silence > self.settings.silent_session {
                lines.push(format!(
                    "session silent: {} {} {}m",
                    session.name,
                    session.id,
                    silence.as_secs() / 60
                ));
                kept.push_str(&format!("{} {written}\n", session.session));
            }
        }
        if kept != before {
            replace(&file, &kept)?;
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
        let projects = self.gitlab_projects(notes);
        let github = self.gitlab.is_empty() || !self.sources.organization.is_empty();
        let forges = &self.gitlab;
        let sources = &self.sources;
        let read = each(&repositories, WIDTH, |repository| {
            Ok(match projects.get(repository) {
                Some((forge, project)) => {
                    Some(newest_pipeline(&forges[*forge], project, repository))
                }
                None if github => Some(newest_runs(sources, repository)),
                None => None,
            })
        })?;
        let mut fresh = Vec::new();
        for (repository, answer) in repositories.iter().zip(read) {
            let source = format!("ci {repository}");
            match answer {
                None => {}
                Some(Ok(runs)) => {
                    self.answered(&source);
                    fresh.extend(runs);
                }
                Some(Err(error)) => self.failed(&source, &error, notes),
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

    /// The projects of [`Watch::gitlab`]'s forges that are not archived, by name, each with the
    /// place of its forge. A forge whose project list does not answer is named on `notes`, as the
    /// source `ci the gitlab group <group> on <host>`, until it answers again; its repositories
    /// are read as no forge's.
    fn gitlab_projects(&mut self, notes: &mut dyn Write) -> BTreeMap<String, (usize, Project)> {
        let mut projects = BTreeMap::new();
        let forges = self.gitlab.clone();
        for (place, forge) in forges.iter().enumerate() {
            let source = format!("ci {}", forge.named());
            match forge.projects() {
                Ok(listed) => {
                    self.answered(&source);
                    for project in listed.into_iter().filter(|project| !project.archived) {
                        projects
                            .entry(project.name.clone())
                            .or_insert((place, project));
                    }
                }
                Err(error) => self.failed(&source, &error, notes),
            }
        }
        projects
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
        let store = crate::store::open_reader(state).context(what)?;
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
    // Review
    // -----------------------------------------------------------------------------------------

    /// Each repository whose pull requests waiting for review (as the board counts them,
    /// [`crate::status::waiting_in`]) in the store's newest complete snapshot reached
    /// [`Settings::max_open_reviews`], once, `review full: <repo> <k>/<n>`; and each one so
    /// reported that fell under it, once, `review open: <repo>` (`story:review-limit`). The
    /// repositories reported full are kept in `review.full`. Without a limit (`0`) nothing is
    /// reported and that file is emptied; without a store or a complete snapshot, which the CI
    /// probe names, nothing is read or changed.
    fn reviews(&self, lines: &mut Vec<String>) -> Result<()> {
        let Some(state) = &self.sources.store else {
            return Ok(());
        };
        let file = self.state(REVIEW_FULL);
        let kept: BTreeSet<String> = read_optional(&file)?
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_owned)
            .collect();
        let limit = self.settings.max_open_reviews;
        if limit == 0 {
            if !kept.is_empty() {
                replace(&file, "")?;
            }
            return Ok(());
        }
        let Ok(store) = crate::store::open_reader(state) else {
            return Ok(());
        };
        let snapshots = SnapshotStorage::list(&store);
        let Some(held) = crate::status::newest_complete(&snapshots, None) else {
            return Ok(());
        };
        let full = crate::status::review_full(
            &crate::status::waiting_in(&store, held),
            i64::try_from(limit).unwrap_or(i64::MAX),
        );
        for (repository, waiting) in &full {
            if !kept.contains(repository) {
                lines.push(format!("review full: {repository} {waiting}/{limit}"));
            }
        }
        for repository in &kept {
            if !full.contains_key(repository) {
                lines.push(format!("review open: {repository}"));
            }
        }
        if full.keys().ne(kept.iter()) {
            let text: String = full.keys().map(|name| format!("{name}\n")).collect();
            replace(&file, &text)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Gates hooks
    // -----------------------------------------------------------------------------------------

    /// `gates hooks stale: <n> checkouts` once per new complete snapshot of the store in which
    /// the board names `n` repositories `stale gates hooks` ([`crate::status::stale_gates_hooks`],
    /// `story:gates-hooks-fresh`), and nothing for one that names none. The snapshot read last is
    /// kept in `gates-hooks.snapshot`; without a store or a complete snapshot, which the CI probe
    /// names, nothing is read or changed.
    fn gates_hooks(&self, lines: &mut Vec<String>) -> Result<()> {
        let Some(state) = &self.sources.store else {
            return Ok(());
        };
        let Ok(store) = crate::store::open_reader(state) else {
            return Ok(());
        };
        let snapshots = SnapshotStorage::list(&store);
        let Some(held) = crate::status::newest_complete(&snapshots, None) else {
            return Ok(());
        };
        let id = &held.data.snapshot_id.0.0;
        let file = self.state(GATES_HOOKS_READ);
        if read_optional(&file)?.is_some_and(|kept| kept.trim() == id.as_str()) {
            return Ok(());
        }
        let stale = crate::status::stale_gates_hooks(&store, held);
        if stale > 0 {
            lines.push(format!("gates hooks stale: {stale} checkouts"));
        }
        replace(&file, &format!("{id}\n"))
    }

    // -----------------------------------------------------------------------------------------
    // Workspace
    // -----------------------------------------------------------------------------------------

    /// `workspace stale: <repo>` for each repository the newest complete snapshot holds as
    /// `NotInWorkspace` (`story:snapshot-unlisted-repository`), once per new snapshot. The
    /// snapshot read last is kept in `workspace.snapshot`; without a store or a complete
    /// snapshot nothing is read or changed.
    fn workspace_stale(&self, lines: &mut Vec<String>) -> Result<()> {
        let Some(state) = &self.sources.store else {
            return Ok(());
        };
        let Ok(store) = crate::store::open_reader(state) else {
            return Ok(());
        };
        let snapshots = SnapshotStorage::list(&store);
        let Some(held) = crate::status::newest_complete(&snapshots, None) else {
            return Ok(());
        };
        let id = &held.data.snapshot_id.0.0;
        let file = self.state(WORKSPACE_READ);
        if read_optional(&file)?.is_some_and(|kept| kept.trim() == id.as_str()) {
            return Ok(());
        }
        for repository in crate::status::unlisted_repositories(&store, held) {
            lines.push(format!("workspace stale: {repository}"));
        }
        replace(&file, &format!("{id}\n"))
    }

    // -----------------------------------------------------------------------------------------
    // Usage
    // -----------------------------------------------------------------------------------------

    /// The usage line of the store under [`Watch::with_usage`]'s state directory, once per change
    /// of the forecast hour (`story:usage-probe`): nothing without such a store or a reading in
    /// a complete snapshot of it; otherwise the line when the hour differs from the one kept in
    /// `usage.hour`, which then keeps it.
    fn usage(&self, lines: &mut Vec<String>) -> Result<()> {
        let Some(state) = &self.usage else {
            return Ok(());
        };
        let Ok(store) = crate::store::open_reader(state) else {
            return Ok(());
        };
        let Some(forecast) = crate::collect::usage::Forecast::of(&store) else {
            return Ok(());
        };
        let hour = forecast.hour();
        let file = self.state(USAGE_HOUR);
        if read_optional(&file)?.is_some_and(|kept| kept.trim() == hour) {
            return Ok(());
        }
        lines.push(forecast.line());
        replace(&file, &format!("{hour}\n"))
    }

    // -----------------------------------------------------------------------------------------
    // Escalated decision requests
    // -----------------------------------------------------------------------------------------

    /// The decision requests escalated as class O or H and not answered or withdrawn
    /// ([`crate::status::unanswered`], `story:decisions-notify`): `escalated <O|H> <request-id>
    /// <question>` in the first pass that finds one, and `unanswered <O|H> <request-id>
    /// <minutes>` once, in the first pass [`UNANSWERED_AFTER`] or more after it, and once in the
    /// first pass at or after the deadline its escalation recorded (`story:decisions-page`); one
    /// line when both fall in the same pass. A request that leaves `Escalated` is forgotten, after
    /// `answered <request-id> <choice>` when it was answered (`story:decisions-batch-answer`).
    /// Without a store nothing is printed.
    fn decisions(&self, now: OffsetDateTime, lines: &mut Vec<String>) -> Result<()> {
        let Some(state) = &self.decisions else {
            return Ok(());
        };
        let Ok(store) = crate::store::open_reader(state) else {
            return Ok(());
        };
        let items = crate::status::unanswered(&store);
        let file = self.state(ESCALATED_SEEN);
        let before = read_optional(&file)?.unwrap_or_default();
        let kept: BTreeMap<&str, (&str, bool, bool)> = before
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let id = fields.next()?;
                let seen = fields.next()?;
                let reported = fields.next()? == "1";
                // A line written before deadlines were recorded has no fourth field.
                let at_deadline = fields.next() == Some("1");
                Some((id, (seen, reported, at_deadline)))
            })
            .collect();
        let after = i64::try_from(UNANSWERED_AFTER.as_secs() / 60).unwrap_or(i64::MAX);
        let mut next = String::new();
        for item in &items {
            let id = one_line(&item.request_id);
            let (seen, reported, at_deadline) = match kept.get(id.as_str()) {
                Some(&(seen, reported, at_deadline)) => (seen.to_owned(), reported, at_deadline),
                None => {
                    lines.push(format!(
                        "escalated {} {id} {}",
                        item.class,
                        one_line(&item.question)
                    ));
                    (now.format(&Rfc3339)?, false, false)
                }
            };
            let waited = OffsetDateTime::parse(&seen, &Rfc3339)
                .ok()
                .map(|at| (now - at).whole_minutes());
            let due = !reported && waited.is_some_and(|minutes| minutes >= after);
            // The deadline its escalation recorded (`story:decisions-page`): once, at or after it.
            let deadline_due = !at_deadline
                && item
                    .deadline
                    .as_deref()
                    .and_then(|deadline| OffsetDateTime::parse(deadline, &Rfc3339).ok())
                    .is_some_and(|deadline| now >= deadline);
            if let (true, Some(minutes)) = (due || deadline_due, waited) {
                lines.push(format!("unanswered {} {id} {minutes}", item.class));
            }
            let flag = |set: bool| if set { "1" } else { "0" };
            next.push_str(&format!(
                "{id}\t{seen}\t{}\t{}\n",
                flag(reported || due),
                flag(at_deadline || deadline_due)
            ));
        }
        // A printed request that left `Escalated` is forgotten; one that left it answered is
        // printed `answered <request-id> <choice>` first, once, so conductor relays the answer
        // (`story:decisions-batch-answer`). A withdrawn one prints nothing.
        for (gone, _) in kept
            .iter()
            .filter(|(id, _)| !items.iter().any(|item| one_line(&item.request_id) == **id))
        {
            let request = DecisionRequestStorage::get(&store, &RequestId((*gone).to_owned()));
            let Some(request) = request.filter(|request| {
                request.state == conductor_model::decision::DecisionRequestState::Answered
            }) else {
                continue;
            };
            let choice = request
                .data
                .decision_id
                .as_ref()
                .and_then(|decision| DecisionStorage::get(&store, decision))
                .map_or_else(String::new, |decision| one_line(&decision.data.choice));
            lines.push(format!("answered {gone} {choice}").trim_end().to_owned());
        }
        if next != before {
            replace(&file, &next)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Disk
    // -----------------------------------------------------------------------------------------

    /// Free disk under each of the instance's floors, once per fall (`story:watch-disk-floors`):
    /// under `disk_low` the next report waits until free disk was back at its `disk_clear`,
    /// under `disk_admit` until it was back at `disk_admit`, and under `gate_stop` until it was
    /// back at `gate_resume`; each floor keeps its own marker. A reading more than [`DISK_JUMP`]
    /// from the one kept in `disk.free` is read again, and the second reading stands and is kept.
    fn disk(&mut self, lines: &mut Vec<String>, notes: &mut dyn Write) -> Result<()> {
        let kept = self.state(DISK_FREE);
        let previous = fs::read_to_string(&kept)
            .ok()
            .and_then(|text| text.trim().parse::<u64>().ok());
        let reading = self.free_bytes().and_then(|first| {
            if previous.is_some_and(|previous| first.abs_diff(previous) > DISK_JUMP) {
                self.free_bytes()
            } else {
                Ok(first)
            }
        });
        let bytes = match reading {
            Ok(bytes) => {
                self.answered("disk");
                bytes
            }
            Err(error) => {
                self.failed("disk", &error, notes);
                return Ok(());
            }
        };
        if previous != Some(bytes) {
            fs::write(&kept, format!("{bytes}\n"))
                .with_context(|| format!("write {}", kept.display()))?;
        }
        let free = bytes.div_ceil(1 << 30);
        let floors = [
            (
                DISK_LOW,
                self.settings.disk_low,
                self.settings.disk_clear,
                format!("disk low: {free}G free on /"),
            ),
            (
                DISK_ADMIT,
                self.settings.disk_admit,
                self.settings.disk_admit,
                format!("disk admit: {free}G free on / (builds wait)"),
            ),
            (
                GATE_STOP,
                self.settings.gate_stop,
                self.settings.gate_resume,
                format!("disk gate stop: {free}G free on / (gates stop)"),
            ),
        ];
        let mut fell_under_admit = false;
        for (file, floor, clear, line) in floors {
            let marker = self.state(file);
            if free < floor && !marker.exists() {
                lines.push(line);
                fs::write(&marker, "").with_context(|| format!("write {}", marker.display()))?;
                fell_under_admit |= file == DISK_ADMIT;
            } else if free >= clear {
                match fs::remove_file(&marker) {
                    Ok(()) => {}
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(error).with_context(|| format!("remove {}", marker.display()));
                    }
                }
            }
        }
        if fell_under_admit {
            self.reclaim(bytes, lines, notes);
        }
        // Free disk back at `disk_clear` ends a fall `conductor build run` recorded an operator
        // item for, which is withdrawn (`story:disk-item-per-fall`).
        if let (Some(state), Some(host)) = (
            self.reclaim.as_ref().map(|(state, _)| state.clone()),
            self.host.clone(),
        ) {
            let clear = self
                .settings
                .disk_clear
                .saturating_mul(1 << 30)
                .max(crate::gc::reclaim::floor_bytes(self.settings.disk_admit));
            let instance = self.settings.instance.clone();
            let key = self.settings.fall_key.clone();
            match crate::host::clear_fall(&host, &state, &key, &instance, bytes, clear) {
                Ok(_) => self.answered("disk item"),
                Err(error) => {
                    let error = error.context("withdraw the operator item of a fall that is over");
                    self.failed("disk item", &error, notes);
                }
            }
        }
        Ok(())
    }

    /// Runs the reclaim, if any, to the instance's floor, after a fall under `disk_admit` read as
    /// `bytes` free: `disk reclaimed: <n>G` (the GiB it freed, rounded down) when it reached the
    /// floor, else `disk floor not reached: <n>G short` (rounded up), by a reading taken after it
    /// or, when that fails, by `bytes` and what it freed.
    fn reclaim(&mut self, bytes: u64, lines: &mut Vec<String>, notes: &mut dyn Write) {
        let Some((state, reclaim)) = self.reclaim.clone() else {
            return;
        };
        let need = crate::gc::reclaim::floor_bytes(self.settings.disk_admit);
        match reclaim(&state, need) {
            Ok(report) => {
                self.answered("reclaim");
                if report.reached {
                    lines.push(format!("disk reclaimed: {}G", report.reclaimed_bytes >> 30));
                } else {
                    let free = self
                        .free_bytes()
                        .unwrap_or_else(|_| bytes.saturating_add(report.reclaimed_bytes));
                    let short = need.saturating_sub(free).max(1).div_ceil(1 << 30);
                    lines.push(format!("disk floor not reached: {short}G short"));
                }
            }
            Err(error) => {
                let error = error.context("reclaim disk under disk_admit");
                self.failed("reclaim", &error, notes);
            }
        }
    }

    fn free_bytes(&self) -> Result<u64> {
        (self.sources.free_bytes)(&self.sources.disk)
            .with_context(|| format!("read free disk on {}", self.sources.disk.display()))
    }
}

/// The newest `main` run of each workflow of `repository` that decided something: completed,
/// green or red ([`decided`]). The run list is run once, through [`collect::run`].
///
/// A repository carries no owner in the store, so its run list is asked of each of
/// [`Sources::owners`] in turn, and the first owner that answers is the repository's; when none
/// answers, the first owner's failure is the answer.
fn newest_runs(sources: &Sources, repository: &str) -> Result<Vec<Latest>> {
    let owners: &[String] = if sources.owners.is_empty() {
        std::slice::from_ref(&sources.organization)
    } else {
        &sources.owners
    };
    let mut first = None;
    for owner in owners {
        match newest_runs_of(sources, owner, repository) {
            Ok(runs) => return Ok(runs),
            Err(error) => {
                first.get_or_insert(error);
            }
        }
    }
    Err(first.unwrap_or_else(|| anyhow!("main CI of {repository} not read: no GitHub owner")))
}

/// [`newest_runs`] of `repository` under the one GitHub owner `owner`.
fn newest_runs_of(sources: &Sources, owner: &str, repository: &str) -> Result<Vec<Latest>> {
    let fill = [("organization", owner), ("repository", repository)];
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

/// One pipeline, as GitLab's `pipelines` answers it.
#[derive(Deserialize)]
struct Pipeline {
    id: i64,
    status: String,
    created_at: String,
}

/// The newest pipeline of `project`'s default branch, of the latest 20, that decided something:
/// a `Success` or a `Failure` by the `gitlab` collector's mapping ([`gitlab::conclusion`]), kept
/// as `success` or `failure`, the GitHub conclusions [`GREEN`] and [`RED`] compare; recorded as
/// the workflow [`PIPELINE`] of `repository`. Read with `glab api --hostname <host>` through the
/// forge ([`Forge::get`]).
fn newest_pipeline(forge: &Forge, project: &Project, repository: &str) -> Result<Vec<Latest>> {
    let what = format!("main CI of {repository} not read");
    let pipelines: Vec<Pipeline> = forge.get(
        &what,
        &format!(
            "projects/{}/pipelines?ref={}&per_page=20",
            project.id,
            gitlab::encoded(&project.default_branch)
        ),
    )?;
    let mut newest: Option<(Pipeline, &'static str)> = None;
    for pipeline in pipelines {
        let conclusion = match gitlab::conclusion(&pipeline.status) {
            Some(RunConclusion::Success) => GREEN,
            Some(RunConclusion::Failure) => RED[0],
            _ => continue,
        };
        if newest
            .as_ref()
            .is_none_or(|(known, _)| !later(&known.created_at, &pipeline.created_at))
        {
            newest = Some((pipeline, conclusion));
        }
    }
    Ok(newest
        .into_iter()
        .map(|(pipeline, conclusion)| Latest {
            repository: repository.to_owned(),
            workflow: PIPELINE.to_owned(),
            run: pipeline.id,
            conclusion: conclusion.to_owned(),
            at: pipeline.created_at,
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

/// The dispatch log as a session probe reads it ([`Watch::dispatches_since`]).
#[derive(Debug, Default)]
struct Dispatches {
    /// The lines written since the probe's instant.
    recent: Vec<DispatchLine>,
    /// Every line read, `recent` among them.
    read: Vec<DispatchLine>,
}

/// One line of the dispatch log: its `id`, the dispatch it is of, and its text.
#[derive(Debug, Clone)]
struct DispatchLine {
    id: Option<String>,
    text: String,
}

/// Whether the dispatch log explains the exit of the session `id` with `pid`: a recent line
/// names its id, or its `pid` as a word; or a recent line is of a dispatch (has its `id`) one of
/// whose lines read, however old, names the session id (as a `sent` line's `session` names the
/// session a later `done` line of the same dispatch stops). A pid links no dispatch: an older
/// line naming it may name another process that had it.
fn explained(log: &Dispatches, id: &str, pid: Option<u64>) -> bool {
    let pid = pid.map(|pid| pid.to_string());
    if log.recent.iter().any(|line| {
        line.text.contains(id)
            || pid
                .as_deref()
                .is_some_and(|pid| holds_word(&line.text, pid))
    }) {
        return true;
    }
    let dispatches: BTreeSet<&str> = log
        .read
        .iter()
        .filter(|line| line.text.contains(id))
        .filter_map(|line| line.id.as_deref())
        .collect();
    log.recent.iter().any(|line| {
        line.id
            .as_deref()
            .is_some_and(|dispatch| dispatches.contains(dispatch))
    })
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

/// The role of the instance the process runs whose session name is `name` (`conductor`,
/// `conductor-dev`, by their full names), if any: such a session is no controller.
fn role_named(name: &str) -> Option<String> {
    config::active()
        .instance
        .roles
        .iter()
        .find(|role| {
            role.session_name
                .as_ref()
                .is_some_and(|session_name| session_name.0 == name)
        })
        .map(|role| role.role.clone())
}

/// When a session last wrote: the newest modification time of its `transcript` and of every
/// `*.jsonl` at any depth under `<transcript without .jsonl>/subagents/`, where its sub-agents
/// keep theirs; `None` when its transcript cannot be read.
fn newest_write(transcript: &Path) -> Option<SystemTime> {
    let mut newest = fs::metadata(transcript).ok()?.modified().ok()?;
    let mut directories = vec![transcript.with_extension("").join("subagents")];
    while let Some(directory) = directories.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                directories.push(path);
            } else if kind.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "jsonl")
                && let Some(modified) = entry.metadata().ok().and_then(|meta| meta.modified().ok())
            {
                newest = newest.max(modified);
            }
        }
    }
    Some(newest)
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
