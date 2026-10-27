//! `conductor run` (`story:run-daemon`, docs/design/conductor-run.md § 3): the instance's loop as
//! a program, with no model call.
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `run` and its input `conductor.session.RunDaemon`. It is one process per
//! instance, which holds `<state>/run/lock` while it runs, so a second one refuses. It keeps
//! nothing in memory that is not in the store, the watch's state files or its own files under
//! `<state>/run/` ([`RUN_DIR`]): a restart of the process loses nothing.
//!
//! **Ticks.** A watch tick, every `cadence.watch` from the end of the last: the watch pass
//! ([`Watch::pass`], reading `main`'s CI every `cadence.ci`), then what earlier ticks left waiting
//! ([`handlers::pending`]: sessions a usage limit stopped, hand-overs asked for, deliveries kept
//! for a controller's next start), then a handler per line ([`handlers::handle`]), then the
//! mailbox ([`mailbox::read`]). A cycle tick, every `cadence.cycle`, first at the start
//! ([`dispatch::cycle_tick`]): `snapshot start-snapshot`, `snapshot publish-board` and `records
//! check`, each a run of this binary, then dispatch v1, and one `cycle` line. A daily tick, once
//! a day at `cadence.daily` UTC ([`dispatch::daily_tick`]): `metric compute` for the previous UTC
//! day, the brief `<records>/REPORT.md`, then the records committed and pushed through the bot
//! route ([`commit_records`], `story:records-committed-by-daemon`). Each tick ends recorded with
//! `RecordRunTick`: its kind,
//! its start, the watch lines and mailbox files it acted on, and the event of each dispatch log
//! line written while it ran, in order; each line the daemon writes itself carries the tick's id
//! (`tick_id`). A handler that fails is named on standard error, `conductor run: <what>: <why>`,
//! and the tick goes on; a tick that cannot be recorded fails the run.
//!
//! `--once` runs one watch tick and one cycle tick and exits, 1 when either could not be
//! recorded, else 0. `--once --commit-records` runs [`commit_records`] alone for today's UTC day,
//! with no tick and without the daemon's lock (git's own lock keeps two commits apart): it prints
//! what it did and exits 0, or 1 when the commit was refused. `--install` and `--uninstall` start
//! and stop the user unit
//! `conductor_<instance>-run.service` ([`unit`]). The design's `conductor run install` is a flag:
//! the binding refuses a root command that is also a group.
//!
//! The daemon acts through [`Seams`]: the confinement providers sessions start and stop through,
//! the [`Deliverer`] that puts a text in front of a session (`conductor session deliver`,
//! [`ShellDeliverer`]), the [`Headroom`] reader ([`Ais`]), `decide::handle_request`
//! (`story:decide-model-adapter`, [`handle_request`]), the binary the cycle's steps run, and `gc`.
//! A test gives its own.

pub mod dispatch;
pub mod handlers;
pub mod mailbox;
pub mod records;
pub mod unit;

pub use records::{Committed, Route, commit_records};

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    ControllerStorage, DispatchStorage, Generated, RunTickStorage, TryContext, unmet_context,
};
use conductor_model::config::{Instance, SessionName};
use conductor_model::dispatch::{
    ControllerState, DispatchLogEvent, DispatchLogLine, DispatchState,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::session::obligations::{RecordRunTickBehavior, RunTicksQuery};
use conductor_model::session::{
    RecordRunTick, RecordRunTickOutcome, RunTickSnapshot, TickId, TickKind,
};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{GcArgs, RecordRunTickArgs, RunArgs, SpawnArgs, StopControllerArgs, ViewArgs};
use crate::confine::Providers;
use crate::session::{Deliverer, ShellDeliverer};
use crate::store::Store;
use crate::store::run::{kind_name, kind_named, state_name};
use crate::watch::{Options, Watch};
use crate::{collect, config};

/// The daemon's own directory under the state directory.
pub const RUN_DIR: &str = "run";

/// The lock a running daemon holds, in [`RUN_DIR`].
const LOCK: &str = "lock";

/// When the last watch pass read `main`'s CI, the last cycle ran (RFC 3339 each) and the UTC day
/// the last daily tick was for (`YYYY-MM-DD`), in [`RUN_DIR`].
const CI_LAST: &str = "ci.last";
const CYCLE_LAST: &str = "cycle.last";
const DAILY_LAST: &str = "daily.last";

/// The cycle's cadence when the instance's `cadence.cycle` does not read: the built-in two hours.
const CYCLE_EVERY: u64 = 2 * 3600;

/// How long `ais status` may take.
const AIS_BOUND: Duration = Duration::from_secs(60);

/// Whether the account the sessions use has headroom again after a usage limit
/// (docs/design/conductor-run.md § 3). [`Ais`] is the real one; a test gives its own.
pub trait Headroom {
    /// Whether it has.
    ///
    /// # Errors
    ///
    /// The answer could not be read.
    fn headroom(&self) -> Result<bool>;
}

/// The real [`Headroom`]: `ais status --json --provider claude`, within a minute, read by
/// [`headroom_of`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Ais;

impl Headroom for Ais {
    fn headroom(&self) -> Result<bool> {
        let shown = "ais status --json --provider claude";
        let output = collect::run(
            Command::new("ais").args(["status", "--json", "--provider", "claude"]),
            AIS_BOUND,
        )
        .with_context(|| format!("run `{shown}`"))?;
        if !output.status.success() {
            bail!("`{shown}` {}", crate::confine::exited(&output));
        }
        let status: Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("`{shown}` printed no JSON"))?;
        Ok(headroom_of(&status))
    }
}

/// Whether `status`, what `ais status --json` prints, shows headroom: its active account has not
/// reached its limit and none of its usage windows is full. No active account is no headroom.
#[must_use]
pub fn headroom_of(status: &Value) -> bool {
    status["accounts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|account| account["active"] == true)
        .is_some_and(|account| {
            let usage = &account["usage"];
            usage["limit_reached"] != true
                && usage["windows"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .all(|window| {
                        window["used_percent"]
                            .as_f64()
                            .is_none_or(|used| used < 100.0)
                    })
        })
}

/// `decide::handle_request` (`story:decide-model-adapter`): handles the `[DECISION-REQUEST]`
/// message it is given whole, in the store under the state directory: raises it with the rule
/// that classified it, answers and records class C, escalates class O and H; answers whether the
/// request was decided, so its `[DECISION]` line is to be delivered. [`handle_request`] is the
/// real one; with none, a request is raised and escalated as class O ([`mailbox`]).
pub type HandleRequest = fn(Option<&Path>, &str) -> Result<bool>;

/// The variable that chooses the `Decider` adapter of [`handle_request`]: `fake`, which answers
/// the request's recommendation, for tests and dry runs; anything else, or none, is `claude -p`
/// on the instance's model ([`crate::decide::model::ClaudePrint`]).
pub const DECIDER_VARIABLE: &str = "CONDUCTOR_DECIDER";

/// [`HandleRequest`] over [`crate::decide::handle_request`], with the decider
/// [`DECIDER_VARIABLE`] chooses: answers `true` when the request was decided, `false` when it was
/// escalated.
///
/// # Errors
///
/// The request does not read, the instance names no model for the decider, or
/// [`crate::decide::handle_request`] fails.
pub fn handle_request(state: Option<&Path>, text: &str) -> Result<bool> {
    use crate::decide::model::{ClaudePrint, Decider, Fake};
    let decider: Box<dyn Decider> = if std::env::var(DECIDER_VARIABLE).as_deref() == Ok("fake") {
        let request = crate::decide::parse_request(text)?;
        Box::new(Fake::answering(
            &request.recommended,
            "the fake decider answers the request's recommendation",
        ))
    } else {
        Box::new(ClaudePrint::for_instance(&config::active().instance)?)
    };
    Ok(matches!(
        crate::decide::handle_request(state, text, decider.as_ref())?,
        crate::decide::Handled::Decided { .. }
    ))
}

/// `gc` ([`crate::gc::gc`]), or a test's stand-in.
pub type Cleaner = fn(&GcArgs) -> Result<ExitCode>;

/// What the daemon acts through; see the module documentation.
pub struct Seams<'a> {
    /// The providers sessions start, stop and resume through.
    pub providers: &'a dyn Providers,
    /// What puts a text in front of a session.
    pub deliverer: &'a dyn Deliverer,
    /// Whether usage has headroom.
    pub headroom: &'a dyn Headroom,
    /// `decide::handle_request` ([`handle_request`]), or none: a request is escalated as class O.
    pub handle_request: Option<HandleRequest>,
    /// The `conductor` binary the cycle's and the daily tick's steps run.
    pub program: PathBuf,
    /// `gc`.
    pub gc: Cleaner,
}

/// What a tick did, as it is recorded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    /// The tick's id.
    pub tick_id: String,
    /// The watch lines and mailbox files it acted on.
    pub lines_handled: i64,
    /// The dispatch log lines written while it ran, by event, in order.
    pub actions: Vec<DispatchLogEvent>,
}

/// `conductor run`, as the module documentation says.
///
/// # Errors
///
/// More than one of `--once`, `--install` and `--uninstall` is given, `--commit-records` is given
/// without `--once`, another daemon holds the lock, the state directory cannot be written, or a
/// tick cannot be recorded.
pub fn run(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &RunArgs,
    providers: &dyn Providers,
) -> Result<ExitCode> {
    let given = [
        ("--once", args.once),
        ("--install", args.install),
        ("--uninstall", args.uninstall),
    ];
    let modes: Vec<&str> = given
        .iter()
        .filter(|(_, set)| *set == Some(true))
        .map(|(flag, _)| *flag)
        .collect();
    if modes.len() > 1 {
        bail!(
            "run: {} are given; give at most one of --once, --install and --uninstall",
            modes.join(" and ")
        );
    }
    if args.commit_records == Some(true) {
        if args.once != Some(true) {
            bail!("run: --commit-records runs with --once; give both");
        }
        return commit_alone(config_flag, state);
    }
    if args.install == Some(true) {
        return unit::install(config_flag, state);
    }
    if args.uninstall == Some(true) {
        return unit::uninstall();
    }
    let deliverer = ShellDeliverer::current(config_flag, state)?;
    let seams = Seams {
        providers,
        deliverer: &deliverer,
        headroom: &Ais,
        handle_request: Some(handle_request),
        program: std::env::current_exe().context("run: read the path of the conductor binary")?,
        gc: crate::gc::gc,
    };
    run_with(config_flag, state, args.once == Some(true), &seams)
}

/// [`run`] over `seams`: with `once`, one watch tick and one cycle tick; else the loop.
///
/// # Errors
///
/// Another daemon holds the lock, the state directory cannot be written, or a tick cannot be
/// recorded.
pub fn run_with(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    once: bool,
    seams: &Seams<'_>,
) -> Result<ExitCode> {
    let dir = run_dir(state)?;
    let _lock = lock(&dir)?;
    if once {
        let watched = watch_tick(config_flag, state, seams);
        let cycled = cycle_tick(config_flag, state, seams);
        let mut failed = false;
        for (what, tick) in [("watch tick", watched), ("cycle tick", cycled)] {
            if let Err(error) = tick {
                eprintln!("conductor run: {what}: {error:#}");
                failed = true;
            }
        }
        return Ok(if failed {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        });
    }
    let instance = &config::active().instance;
    let every = Duration::from_secs(
        config::seconds(&instance.cadence.watch).unwrap_or(config::WATCH_EVERY),
    );
    let cycle_every =
        Duration::from_secs(config::seconds(&instance.cadence.cycle).unwrap_or(CYCLE_EVERY));
    loop {
        if let Err(error) = watch_tick(config_flag, state, seams) {
            eprintln!("conductor run: watch tick: {error:#}");
        }
        let now = OffsetDateTime::now_utc();
        if due(&dir.join(CYCLE_LAST), now, cycle_every) {
            stamp(&dir.join(CYCLE_LAST), now)?;
            if let Err(error) = cycle_tick(config_flag, state, seams) {
                eprintln!("conductor run: cycle tick: {error:#}");
            }
        }
        if let Some(day) = daily_due(&dir, now, &instance.cadence.daily.0) {
            write_file(&dir.join(DAILY_LAST), &format!("{}\n", today(now)))?;
            if let Err(error) = dispatch::daily_tick(config_flag, state, &day, seams) {
                eprintln!("conductor run: daily tick: {error:#}");
            }
        }
        thread::sleep(every);
    }
}

/// A watch tick: the watch pass, then [`handle_lines`] over its lines.
///
/// # Errors
///
/// The watch cannot be built or its state written, or the tick cannot be recorded.
pub fn watch_tick(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    seams: &Seams<'_>,
) -> Result<Summary> {
    let dir = run_dir(state)?;
    let instance = &config::active().instance;
    let ci_every = Options::of(
        instance,
        &crate::cli::WatchRunArgs {
            every: None,
            ci_every: None,
            follow: None,
        },
    )
    .ci_every;
    let now = OffsetDateTime::now_utc();
    let ci = due(&dir.join(CI_LAST), now, ci_every);
    if ci {
        stamp(&dir.join(CI_LAST), now)?;
    }
    let mut watch = Watch::of_instance(state)?;
    let lines = watch.pass(ci, &mut io::stderr().lock())?;
    for line in &lines {
        println!("{line}");
    }
    handle_lines(config_flag, state, &lines, seams)
}

/// A watch tick over `lines` in place of a pass's: what earlier ticks left waiting
/// ([`handlers::pending`]), a handler per line ([`handlers::handle`]) and the mailbox
/// ([`mailbox::read`]); recorded as a `watch` tick.
///
/// # Errors
///
/// The state directory cannot be written, or the tick cannot be recorded.
pub fn handle_lines(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    lines: &[String],
    seams: &Seams<'_>,
) -> Result<Summary> {
    let mut tick = Tick::start(config_flag, state, TickKind::Watch, seams)?;
    handlers::pending(&mut tick);
    handlers::handle(&mut tick, lines);
    mailbox::read(&mut tick);
    tick.finish()
}

/// A cycle tick ([`dispatch::cycle_tick`]).
///
/// # Errors
///
/// What [`dispatch::cycle_tick`] answers.
pub fn cycle_tick(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    seams: &Seams<'_>,
) -> Result<Summary> {
    dispatch::cycle_tick(config_flag, state, seams)
}

/// A daily tick for the UTC day `day` ([`dispatch::daily_tick`]).
///
/// # Errors
///
/// What [`dispatch::daily_tick`] answers.
pub fn daily_tick(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    day: &str,
    seams: &Seams<'_>,
) -> Result<Summary> {
    dispatch::daily_tick(config_flag, state, day, seams)
}

/// `run --once --commit-records`: [`commit_records`] for today's UTC day, its line appended with
/// no tick; prints `records committed <commit>: <message>` (and `, pushed origin main`) or
/// `records: nothing to commit`, or names the refusal on standard error and exits 1.
///
/// # Errors
///
/// The path of this binary cannot be read, or the line not appended.
fn commit_alone(config_flag: Option<&Path>, state: Option<&Path>) -> Result<ExitCode> {
    let program = std::env::current_exe().context("run: read the path of the conductor binary")?;
    let route = Route::new(&program, config_flag, state);
    let records = PathBuf::from(&config::active().instance.records);
    let committed = commit_records(&records, &route, &today(OffsetDateTime::now_utc()));
    if let Some(line) = committed.line()? {
        crate::records::append_dispatch(&line)?;
    }
    let mut out = io::stdout().lock();
    match committed {
        Committed::Nothing => {
            writeln!(out, "records: nothing to commit").context("write to standard output")?;
            Ok(ExitCode::SUCCESS)
        }
        Committed::Committed {
            commit,
            message,
            pushed,
        } => {
            let pushed = if pushed { ", pushed origin main" } else { "" };
            writeln!(out, "records committed {commit}: {message}{pushed}")
                .context("write to standard output")?;
            Ok(ExitCode::SUCCESS)
        }
        Committed::Refused { reason, line, .. } => {
            eprintln!("conductor run: records commit refused: {reason}: {line}");
            Ok(ExitCode::FAILURE)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The tick
// ---------------------------------------------------------------------------------------------

/// One tick while it runs: what it was given, and what it did so far.
pub struct Tick<'s, 'a> {
    /// The global `--config`.
    pub config: Option<&'s Path>,
    /// The global `--state-dir`.
    pub state: Option<&'s Path>,
    /// `<state>/run/`.
    pub dir: PathBuf,
    /// What it acts through.
    pub seams: &'s Seams<'a>,
    /// Its id, which each line it writes carries.
    pub id: String,
    kind: TickKind,
    started_at: Timestamp,
    /// The watch lines and mailbox files it acted on.
    pub lines_handled: i64,
    /// The events of the dispatch log lines written while it ran.
    pub actions: Vec<DispatchLogEvent>,
}

impl<'s, 'a> Tick<'s, 'a> {
    /// A tick of `kind` starting now.
    ///
    /// # Errors
    ///
    /// The state directory cannot be told or written, or the clock not formatted.
    pub fn start(
        config_flag: Option<&'s Path>,
        state: Option<&'s Path>,
        kind: TickKind,
        seams: &'s Seams<'a>,
    ) -> Result<Self> {
        Ok(Self {
            config: config_flag,
            state,
            dir: run_dir(state)?,
            seams,
            id: eventlog_core::new_event_id(),
            kind,
            started_at: crate::records::now()?,
            lines_handled: 0,
            actions: Vec::new(),
        })
    }

    /// The instance the tick runs for.
    #[must_use]
    pub fn instance(&self) -> &'static Instance {
        &config::active().instance
    }

    /// A line of `event` about `repo` with the tick's id.
    ///
    /// # Errors
    ///
    /// The clock cannot be formatted.
    pub fn line(&self, event: DispatchLogEvent, repo: &str) -> Result<DispatchLogLine> {
        let mut line = crate::records::dispatch_line(event, repo)?;
        line.tick_id = Some(self.id.clone());
        Ok(line)
    }

    /// Appends `line` to the dispatch log and counts it.
    ///
    /// # Errors
    ///
    /// The line cannot be appended.
    pub fn write(&mut self, line: &DispatchLogLine) -> Result<()> {
        crate::records::append_dispatch(line)?;
        self.actions.push(line.event);
        Ok(())
    }

    /// Counts a line a command the tick ran wrote itself: a spawn's `sent`, a report's.
    pub fn wrote(&mut self, event: DispatchLogEvent) {
        self.actions.push(event);
    }

    /// Names on standard error what failed, and goes on.
    pub fn note(&self, what: &str, error: &anyhow::Error) {
        let _ = writeln!(io::stderr().lock(), "conductor run: {what}: {error:#}");
    }

    /// Records the tick in the store and answers what it did.
    ///
    /// # Errors
    ///
    /// The store does not open or keep the tick.
    pub fn finish(self) -> Result<Summary> {
        record_tick(
            self.state,
            &self.id,
            RecordRunTick {
                kind: self.kind,
                started_at: self.started_at,
                lines_handled: self.lines_handled,
                actions: self.actions.clone(),
            },
        )?;
        Ok(Summary {
            tick_id: self.id,
            lines_handled: self.lines_handled,
            actions: self.actions,
        })
    }

    /// Delivers `text` to the session `session`.
    ///
    /// # Errors
    ///
    /// What the deliverer answers.
    pub fn deliver(&self, session: &str, text: &str) -> Result<()> {
        self.seams.deliverer.deliver(session, text).map(|_| ())
    }

    /// The running controller of `repo`: its id and session name.
    ///
    /// # Errors
    ///
    /// The store does not open.
    pub fn running(&self, repo: &str) -> Result<Option<(String, String)>> {
        Ok(self
            .controllers()?
            .into_iter()
            .find(|held| held.data.repository.0 == repo && held.state == ControllerState::Running)
            .map(|held| (held.data.controller_id.0.0, held.data.session_name.0)))
    }

    /// Every controller the store holds.
    ///
    /// # Errors
    ///
    /// The store does not open, or a controller does not read.
    pub fn controllers(&self) -> Result<Vec<conductor_model::dispatch::ControllerSnapshot>> {
        let store = crate::state::open(self.state)?;
        let list = ControllerStorage::list(&store);
        store.check().context("read the controllers")?;
        Ok(list)
    }

    /// The repository the session `name` works on: the newest controller of that session name,
    /// else the name without the instance's session prefix.
    ///
    /// # Errors
    ///
    /// The store does not open.
    pub fn repo_of(&self, name: &str) -> Result<Option<String>> {
        if let Some(held) = self
            .controllers()?
            .into_iter()
            .rev()
            .find(|held| held.data.session_name.0 == name)
        {
            return Ok(Some(held.data.repository.0));
        }
        Ok(self
            .instance()
            .session_prefix
            .as_ref()
            .and_then(|prefix| name.strip_prefix(&format!("{}-", prefix.0)))
            .map(str::to_owned))
    }

    /// Whether `repo` has a dispatch sent, started or blocked.
    ///
    /// # Errors
    ///
    /// The store does not open.
    pub fn open_dispatch(&self, repo: &str) -> Result<bool> {
        let store = crate::state::open(self.state)?;
        Ok(DispatchStorage::list(&store).iter().any(|held| {
            held.data.repository.0 == repo
                && matches!(
                    held.state,
                    DispatchState::Sent | DispatchState::Started | DispatchState::Blocked
                )
        }))
    }

    /// The charter of `repo`: `<records>/charters/<repo>.md`.
    #[must_use]
    pub fn charter(&self, repo: &str) -> PathBuf {
        Path::new(&self.instance().records)
            .join("charters")
            .join(format!("{repo}.md"))
    }

    /// The checkout of `repo`: `<checkouts.root>/<repo>`.
    #[must_use]
    pub fn checkout(&self, repo: &str) -> PathBuf {
        Path::new(&self.instance().checkouts.root).join(repo)
    }

    /// Spawns `repo`'s controller from its charter (`conductor spawn`), within
    /// `controllers.max_working` running controllers; counts the `sent` line it writes. Answers
    /// why it did not, when it did not.
    ///
    /// # Errors
    ///
    /// The store does not open.
    pub fn spawn(&mut self, repo: &str) -> Result<Result<(), String>> {
        let working = self
            .controllers()?
            .iter()
            .filter(|held| held.state == ControllerState::Running)
            .count();
        let limit = self.instance().controllers.max_working;
        if i64::try_from(working).unwrap_or(i64::MAX) >= limit {
            return Ok(Err(format!("max_working {limit} reached")));
        }
        match crate::spawn::spawn(
            self.config,
            self.state,
            &SpawnArgs {
                repository: Some(repo.to_owned()),
            },
            self.seams.providers,
        ) {
            Ok(_) => {
                self.wrote(DispatchLogEvent::Sent);
                Ok(Ok(()))
            }
            Err(error) => Ok(Err(one_line(&format!("{error:#}")))),
        }
    }

    /// Stops `repo`'s controller, its session through its provider (`controller
    /// stop-controller`), sets `resume` as the charter's resume point when given, spawns it again
    /// and writes `controller-restarted` with the session, `reason` and the resume point; forgets
    /// what earlier ticks kept of the session.
    ///
    /// # Errors
    ///
    /// The controller is not stopped, the charter not written, the spawn refused, or the line not
    /// appended.
    pub fn restart(
        &mut self,
        repo: &str,
        session: &str,
        reason: &str,
        resume: Option<&Path>,
    ) -> Result<()> {
        let held = self.controllers()?.into_iter().find(|held| {
            held.data.repository.0 == repo
                && matches!(
                    held.state,
                    ControllerState::Running | ControllerState::Paused
                )
        });
        if let Some(held) = held {
            crate::controller::stop_controller(
                self.state,
                StopControllerArgs {
                    controller_id: Some(held.data.controller_id.0.0),
                    reason: Some(format!("conductor run: {reason}")),
                    input_json: None,
                },
                self.seams.providers,
            )
            .context("stop the controller")?;
        }
        if let Some(path) = resume {
            resume_point(&self.charter(repo), path)?;
        }
        self.spawn(repo)?
            .map_err(|why| anyhow!("spawn {repo} again: {why}"))?;
        handlers::forget(self, session);
        let mut line = self.line(DispatchLogEvent::ControllerRestarted, repo)?;
        line.session = Some(session.to_owned());
        line.reason = Some(reason.to_owned());
        line.path = resume.map(|path| path.display().to_string());
        self.write(&line)
    }
}

/// Sets `path` as the resume point of the charter `charter`: its `## Resume point` section, the
/// last of the file, holds the path alone; a charter without one gets it appended.
///
/// # Errors
///
/// The charter cannot be read or written.
pub fn resume_point(charter: &Path, path: &Path) -> Result<()> {
    const HEADING: &str = "## Resume point";
    let text = fs::read_to_string(charter)
        .with_context(|| format!("read the charter {}", charter.display()))?;
    let kept = match text.find(&format!("\n{HEADING}\n")) {
        Some(at) => &text[..at],
        None => text.trim_end(),
    };
    let next = format!("{}\n\n{HEADING}\n\n{}\n", kept.trim_end(), path.display());
    write_file(charter, &next)
}

/// Records a tick under the id `id`, as `RecordRunTick` orders.
fn record_tick(state: Option<&Path>, id: &str, input: RecordRunTick) -> Result<()> {
    let mut generated = Generated::new(TickPorts {
        store: crate::state::open(state)?,
        tick_id: TickId(Uuid(id.to_owned())),
    });
    let outcome = generated
        .record_run_tick(input)
        .map_err(|unmet| anyhow!("record the tick: {unmet}"))?;
    generated.ports.store.check().context("keep the tick")?;
    match outcome {
        RecordRunTickOutcome::Recorded { .. } => Ok(()),
        RecordRunTickOutcome::NegativeCount { error } => bail!(
            "refused: InvalidLineCount: a count of lines handled is never negative, and {} is",
            error.lines_handled
        ),
    }
}

/// `conductor session record-run-tick`: the command `conductor.session.RecordRunTick`. Prints the
/// tick's id.
///
/// # Errors
///
/// A flag is missing or does not read, `--input-json` is given
/// ([`NotImplemented`](crate::NotImplemented)), the store does not open or keep the tick, or the
/// count is negative (`InvalidLineCount`).
pub fn record_run_tick(state: Option<&Path>, args: &RecordRunTickArgs) -> Result<ExitCode> {
    const COMMAND: &str = "session record-run-tick";
    if args.input_json.is_some() {
        return crate::not_implemented("session record-run-tick --input-json");
    }
    let missing = |flag: &str| anyhow!("{COMMAND}: {flag} is required");
    let kind = args.kind.as_deref().ok_or_else(|| missing("--kind"))?;
    let kind =
        kind_named(kind).ok_or_else(|| anyhow!("{COMMAND}: --kind {kind:?} is no TickKind"))?;
    let started_at = Timestamp(
        args.started_at
            .clone()
            .ok_or_else(|| missing("--started-at"))?,
    );
    let lines_handled = args
        .lines_handled
        .ok_or_else(|| missing("--lines-handled"))?;
    let actions: Vec<Value> = serde_json::from_str(
        args.actions
            .as_deref()
            .ok_or_else(|| missing("--actions"))?,
    )
    .with_context(|| format!("{COMMAND}: --actions is no JSON array"))?;
    let actions = actions
        .iter()
        .map(|action| {
            action
                .as_str()
                .and_then(crate::records::dispatch_event_named)
                .ok_or_else(|| anyhow!("{COMMAND}: --actions holds {action}, no DispatchLogEvent"))
        })
        .collect::<Result<Vec<_>>>()?;
    let id = eventlog_core::new_event_id();
    record_tick(
        state,
        &id,
        RecordRunTick {
            kind,
            started_at,
            lines_handled,
            actions,
        },
    )
    .with_context(|| COMMAND.to_owned())?;
    writeln!(io::stdout().lock(), "{id}").context("write the tick's id")?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor session run-ticks`: the view `conductor.session.RunTicks`, over the existing store,
/// rendered as `--format` asks; `actions` as the dispatch log spells each event. A view creates
/// no store.
///
/// # Errors
///
/// There is no store, it does not open, the generated query refuses a row, or the rows cannot be
/// written.
pub fn run_ticks(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 6] = [
        "tick_id",
        "state",
        "kind",
        "started_at",
        "lines_handled",
        "actions",
    ];
    let generated = Generated::new(crate::state::open_reader(state)?);
    let rows = generated
        .run_ticks()
        .map_err(|unmet| anyhow!("session run-ticks: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.tick_id.0.0),
                Value::from(state_name(row.state)),
                Value::from(kind_name(row.kind)),
                Value::from(row.started_at.0),
                Value::from(row.lines_handled),
                Value::from(
                    row.actions
                        .iter()
                        .map(|event| crate::records::dispatch_event(*event))
                        .collect::<Vec<_>>(),
                ),
            ]
        })
        .collect();
    crate::store::show(&generated.ports, "run-ticks", &view.render(&COLUMNS, &rows))
}

/// The ports `RecordRunTick` runs over: the store, and the tick's id, chosen when the tick
/// started so each line it wrote names it.
struct TickPorts {
    store: Store,
    tick_id: TickId,
}

impl RunTickStorage for TickPorts {
    fn get(&self, identity: &TickId) -> Option<RunTickSnapshot> {
        RunTickStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: RunTickSnapshot) {
        RunTickStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &TickId) {
        RunTickStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<RunTickSnapshot> {
        RunTickStorage::list(&self.store)
    }
}

impl TryContext for TickPorts {
    fn try_generate_conductor_session_tick_id(&mut self) -> Result<TickId, UnmetObligation> {
        Ok(self.tick_id.clone())
    }

    fn try_generate_conductor_session_delivery_id(
        &mut self,
    ) -> Result<conductor_model::session::DeliveryId, UnmetObligation> {
        Err(unmet_context("conductor.session.DeliveryId"))
    }

    fn try_external(
        &mut self,
        _command: conductor_model::behaviour::ExternalCommand<'_>,
        _outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        Err(unmet_context("an external branch"))
    }

    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<conductor_model::direction::ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<conductor_model::dispatch::GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(
        &mut self,
    ) -> Result<conductor_model::dispatch::MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<conductor_model::dispatch::ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_host_grant_id(
        &mut self,
    ) -> Result<conductor_model::host::GrantId, UnmetObligation> {
        Err(unmet_context("conductor.host.GrantId"))
    }

    fn try_generate_conductor_host_reclaim_run_id(
        &mut self,
    ) -> Result<conductor_model::host::ReclaimRunId, UnmetObligation> {
        Err(unmet_context("conductor.host.ReclaimRunId"))
    }

    fn try_generate_conductor_observation_board_id(
        &mut self,
    ) -> Result<conductor_model::observation::BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<conductor_model::observation::ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<conductor_model::observation::SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }

    fn try_generate_conductor_tool_tool_call_id(
        &mut self,
    ) -> Result<conductor_model::tool::ToolCallId, UnmetObligation> {
        Err(unmet_context("conductor.tool.ToolCallId"))
    }
}

// ---------------------------------------------------------------------------------------------
// Files under `<state>/run/`
// ---------------------------------------------------------------------------------------------

/// `<state>/run/`, created.
///
/// # Errors
///
/// The state directory cannot be told, or the directory cannot be created.
pub fn run_dir(state: Option<&Path>) -> Result<PathBuf> {
    let dir = crate::state::dir(state)?.join(RUN_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    Ok(dir)
}

/// Takes the daemon's lock in `dir`, held while the answer is.
fn lock(dir: &Path) -> Result<File> {
    let path = dir.join(LOCK);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    if file.try_lock().is_err() {
        bail!(
            "run: another conductor run holds {}; one runs per instance",
            path.display()
        );
    }
    Ok(file)
}

/// Writes `text` to `path`, beside it first and then renamed into place.
///
/// # Errors
///
/// The file cannot be written.
pub fn write_file(path: &Path, text: &str) -> Result<()> {
    let beside = path.with_extension(format!(
        "{}.next",
        path.extension()
            .map(|extension| extension.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(&beside, text).with_context(|| format!("write {}", beside.display()))?;
    fs::rename(&beside, path).with_context(|| format!("replace {}", path.display()))
}

/// The JSON `dir/<name>` holds, `default` when it is not there or does not read.
#[must_use]
pub fn read_json(dir: &Path, name: &str, default: Value) -> Value {
    fs::read_to_string(dir.join(name))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(default)
}

/// Writes `value` as `dir/<name>`.
///
/// # Errors
///
/// The file cannot be written.
pub fn write_json(dir: &Path, name: &str, value: &Value) -> Result<()> {
    write_file(&dir.join(name), &format!("{value}\n"))
}

/// Whether the instant `file` holds is `every` or longer before `now`; a file that is not there
/// or does not read is due.
fn due(file: &Path, now: OffsetDateTime, every: Duration) -> bool {
    fs::read_to_string(file)
        .ok()
        .and_then(|text| OffsetDateTime::parse(text.trim(), &Rfc3339).ok())
        .is_none_or(|last| {
            now - last >= time::Duration::try_from(every).unwrap_or(time::Duration::ZERO)
        })
}

/// Writes `now` to `file`.
fn stamp(file: &Path, now: OffsetDateTime) -> Result<()> {
    write_file(
        file,
        &format!("{}\n", now.format(&Rfc3339).context("format the clock")?),
    )
}

/// The UTC day of `now`, `YYYY-MM-DD`.
fn today(now: OffsetDateTime) -> String {
    let date = now.to_offset(time::UtcOffset::UTC).date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

/// The day a daily tick is due for at `now`, when one is: the UTC day before today, once today's
/// `daily` (`HH:MM`, UTC) has passed and no daily tick ran today.
fn daily_due(dir: &Path, now: OffsetDateTime, daily: &str) -> Option<String> {
    let now = now.to_offset(time::UtcOffset::UTC);
    let (hour, minute) = daily.split_once(':')?;
    let at =
        now.replace_time(time::Time::from_hms(hour.parse().ok()?, minute.parse().ok()?, 0).ok()?);
    if now < at {
        return None;
    }
    let ran = fs::read_to_string(dir.join(DAILY_LAST)).unwrap_or_default();
    if ran.trim() == today(now) {
        return None;
    }
    Some(today(now - time::Duration::days(1)))
}

/// Nanoseconds since the epoch of `at`.
#[must_use]
pub fn nanos(at: SystemTime) -> u128 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos())
}

/// `text`'s first line.
#[must_use]
pub fn one_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_owned()
}

/// The session name the instance gives `repo`'s controller.
#[must_use]
pub fn session_of(repo: &str) -> SessionName {
    config::session_name(&config::active().instance, repo)
}
