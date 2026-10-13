//! `conductor build run --reserve <size> [--repository <repo>] -- <command> [args…]`
//! (`story:build-run`): disk for a build is a reservation the binary grants, not a message.
//!
//! 1. **Reserve.** Under the host ledger's lock ([`host::Ledger`]) the run first releases every
//!    grant whose process group is gone, then records its request (`RequestDisk`) and counts what
//!    the disk lacks for it: the bytes asked, less free disk, less every live reservation, less
//!    the floor (`thresholds.disk_admit` and the margin `conductor host reclaim` keeps,
//!    [`crate::gc::reclaim::floor_bytes`]). Free disk is read as the watch and reclaim read it,
//!    through [`crate::gc::reclaim::free_bytes`] on `thresholds.disk_path` ([`Sources::free_bytes`]).
//! 2. **Reclaim.** When it does not fit, one reclaim run frees what needs no decision
//!    ([`crate::gc::reclaim::reclaim`]), and the run counts again. The answer (`ReserveDisk`) is
//!    granted, granted after reclaim, or refused with the shortfall: the command never runs, the
//!    shortfall and what is left are printed on standard error, one operator item is recorded for
//!    the fall ([`host::escalate`]), and the exit is 2.
//! 3. **Run under the watchdog.** The command runs in its own process group (`StartRun`). Every
//!    `thresholds.watchdog_every` a watchdog reads free disk: under `thresholds.gate_stop` it
//!    stops the group (`SIGSTOP`) and runs reclaim; at or over `thresholds.gate_resume` it
//!    continues it (`SIGCONT`). It measures the run's `target/` (the working directory's, when
//!    that is a Cargo project or holds one), else the working directory, and keeps the largest.
//! 4. **Release.** When the command ends, its peak is recorded (`RecordPeak`) and the grant
//!    released (`ReleaseGrant`: `Exited`, or `Killed` by a signal), and `build run` exits with the
//!    command's status, 128 + the signal for a killed one.

use std::io::Write as _;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::Instance;
use conductor_model::host::{GrantId, ReleaseReason};
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use time::OffsetDateTime;

use crate::cli::BuildRunArgs;
use crate::gc::reclaim::ReclaimReport;
use crate::host::{self, Answer, Ledger, Shortfall, gib};

/// One GiB.
const GIB: u64 = 1 << 30;

/// How often the watchdog reads free disk when `thresholds.watchdog_every` does not read.
const EVERY: Duration = Duration::from_secs(10);

/// What the instance sets for a run: its name and its thresholds, in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The instance the grant is taken for.
    pub instance: String,
    /// Free bytes no reservation may take: `thresholds.disk_admit` and reclaim's margin
    /// ([`crate::gc::reclaim::floor_bytes`]).
    pub floor: u64,
    /// Free bytes under which the run is stopped: `thresholds.gate_stop`.
    pub gate_stop: u64,
    /// Free bytes from which a stopped run continues: `thresholds.gate_resume`.
    pub gate_resume: u64,
    /// How long a reservation waits for live reservations to release when they alone hold the
    /// space it lacks: such a shortfall ends when their runs end, and is not the operator's.
    pub wait: Duration,
}

impl Settings {
    /// The settings of `instance`.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        let bytes = |gib: i64| u64::try_from(gib).unwrap_or(0).saturating_mul(GIB);
        let thresholds = &instance.thresholds;
        Self {
            instance: instance.name.0.clone(),
            floor: crate::gc::reclaim::floor_bytes(
                u64::try_from(thresholds.disk_admit.0).unwrap_or(0),
            ),
            gate_stop: bytes(thresholds.gate_stop.0),
            gate_resume: bytes(thresholds.gate_resume.0),
            wait: WAIT_FOR_RESERVATIONS,
        }
    }
}

/// How long a reservation waits for live reservations to release: an hour, a long gate's length.
pub const WAIT_FOR_RESERVATIONS: Duration = Duration::from_secs(3600);

/// Where a run reads what it decides by. A case gives its own.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The path on whose file system free disk is read: `thresholds.disk_path`.
    pub disk: PathBuf,
    /// Reads the bytes free on the file system holding [`Sources::disk`]: the `available` of
    /// [`crate::disk::space`], as the watch reads it.
    pub free_bytes: fn(&Path) -> Result<u64>,
    /// Frees what needs no decision, for the state directory, until the bytes asked are free.
    pub reclaim: fn(&Path, u64) -> Result<ReclaimReport>,
    /// The current time.
    pub clock: fn() -> OffsetDateTime,
    /// How often the watchdog reads free disk and measures the run.
    pub every: Duration,
}

impl Sources {
    /// The sources of `instance`: `statvfs` on its `thresholds.disk_path`, the reclaim of
    /// `conductor host reclaim`, the system clock, and `thresholds.watchdog_every`.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        let thresholds = &instance.thresholds;
        Self {
            disk: PathBuf::from(&thresholds.disk_path),
            free_bytes: crate::gc::reclaim::free_bytes,
            reclaim: crate::gc::reclaim::reclaim,
            clock: OffsetDateTime::now_utc,
            every: crate::config::seconds(&thresholds.watchdog_every)
                .filter(|seconds| *seconds > 0)
                .map_or(EVERY, Duration::from_secs),
        }
    }
}

/// One run asked for.
#[derive(Debug, Clone)]
pub struct Request {
    /// The bytes reserved: the command's expected peak.
    pub reserve: u64,
    /// The repository the command builds.
    pub repository: Option<String>,
    /// The session that runs it, when known.
    pub session_name: Option<String>,
    /// The command and its arguments.
    pub command: Vec<String>,
    /// Where it runs.
    pub cwd: PathBuf,
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The reservation was granted and the command ran.
    Ran {
        /// The grant.
        grant: GrantId,
        /// The command's exit status; 128 + the signal when it was killed.
        code: u8,
        /// Whether a signal ended it.
        killed: bool,
        /// The largest size measured of what it writes.
        peak: u64,
        /// What a reclaim run freed before it was granted.
        reclaimed: u64,
        /// How often the watchdog stopped it.
        paused: u32,
        /// How often the watchdog continued it.
        resumed: u32,
    },
    /// Reclaim could not reach the size: the command never ran.
    Refused {
        /// The refused grant.
        grant: GrantId,
        /// The bytes asked for.
        bytes: u64,
        /// The bytes the disk still lacks.
        shortfall: u64,
        /// What the reclaim run freed.
        reclaimed: u64,
        /// What it left, by path and size.
        left: Vec<(String, u64)>,
        /// The operator item recorded for this fall; none when the fall had one already.
        item: Option<String>,
    },
}

impl Outcome {
    /// The exit status of `build run`: the command's, or 2 when it was refused.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Ran { code, .. } => *code,
            Self::Refused { .. } => 2,
        }
    }
}

/// Reads a size in GiB: digits, optionally followed by `G`, as the config file writes one.
///
/// # Errors
///
/// Anything else.
pub fn gibibytes(value: &str) -> Result<i64, String> {
    let digits = value.strip_suffix('G').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "{value:?} is no size in GiB: write whole GiB, such as 8G"
        ));
    }
    digits
        .parse()
        .map_err(|error| format!("{value:?} is no size in GiB: {error}"))
}

/// The bytes the disk lacks for `bytes`, with `free` bytes free, `reserved` held by live grants
/// and `floor` kept free.
#[must_use]
pub fn shortfall(bytes: u64, free: u64, reserved: u64, floor: u64) -> u64 {
    let available = free.saturating_sub(floor).saturating_sub(reserved);
    bytes.saturating_sub(available)
}

/// Runs `request` over the ledger in the state directory `state`.
///
/// # Errors
///
/// The command is empty, the ledger does not open or keep a move, free disk does not read, or
/// the command does not start. A failing reclaim is named on standard error and counted as
/// freeing nothing.
pub fn run(
    state: &Path,
    settings: &Settings,
    sources: &Sources,
    request: &Request,
) -> Result<Outcome> {
    let Some((program, arguments)) = request.command.split_first() else {
        bail!("build run: no command: write it after `--`");
    };
    let command = request.command.join(" ");
    let at = host::now((sources.clock)());

    let mut ledger = Ledger::lock(state)?;
    ledger.sweep(&at)?;
    let grant = ledger.request(
        &settings.instance,
        request.repository.as_deref(),
        request.session_name.as_deref(),
        request.reserve,
        &command,
        &at,
    )?;
    // Held from the request to the release, so a sweep closes this grant only once this run is gone.
    let run_lock = host::hold_run(state, &grant)?;
    let lacking = |ledger: &Ledger| -> Result<u64> {
        let free = free_bytes(sources)?;
        Ok(shortfall(
            request.reserve,
            free,
            ledger.reserved(Some(&grant))?,
            settings.floor,
        ))
    };
    let mut short = lacking(&ledger)?;
    let mut reclaimed = 0;
    let mut left = Vec::new();
    if short > 0 {
        // Reclaim frees until this many bytes are free: where the request fits beside the floor
        // and every live reservation.
        let fits = settings
            .floor
            .saturating_add(ledger.reserved(Some(&grant))?)
            .saturating_add(request.reserve);
        let report = reclaim(sources, state, fits);
        reclaimed = report.reclaimed_bytes;
        left = report.left;
        short = lacking(&ledger)?;
    }
    // A shortfall live reservations cover is a wait: they release as their runs end. Wait for
    // them, without the lock, rather than refuse.
    let mut waited = Duration::ZERO;
    let pause = sources.every.max(Duration::from_millis(10));
    while short > 0 && short <= ledger.reserved(Some(&grant))? && waited < settings.wait {
        if waited.is_zero() {
            eprintln!(
                "conductor: build run: waiting: {}G short while live reservations hold {}G; \
                 they release as their runs end",
                short.div_ceil(GIB),
                ledger.reserved(Some(&grant))?.div_ceil(GIB)
            );
        }
        drop(ledger);
        thread::sleep(pause);
        waited += pause;
        ledger = Ledger::lock(state)?;
        ledger.sweep(&host::now((sources.clock)()))?;
        short = lacking(&ledger)?;
    }
    // Only a shortfall live reservations do not cover needs a person.
    let beyond_reservations = short > ledger.reserved(Some(&grant))?;
    let answer = ledger.reserve(&grant, short, reclaimed)?;
    if answer == Answer::Refused {
        let escalate = |shortfall: &Shortfall<'_>| {
            if beyond_reservations {
                host::escalate(state, shortfall)
            } else {
                Ok(None)
            }
        };
        let item = escalate(&Shortfall {
            instance: &settings.instance,
            repository: request.repository.as_deref(),
            command: &command,
            bytes: request.reserve,
            shortfall: short,
            left: &left,
            disk: &sources.disk,
            at: &at,
        })
        .unwrap_or_else(|error| {
            eprintln!("conductor: build run: the operator item was not recorded: {error:#}");
            None
        });
        host::end_run(state, &grant, run_lock);
        return Ok(Outcome::Refused {
            grant,
            bytes: request.reserve,
            shortfall: short,
            reclaimed,
            left,
            item,
        });
    }
    host::fall_over(state)?;
    drop(ledger);
    // Measured before the command starts, so a run's growth counts what a fast command wrote too.
    let start = size(&measured(&request.cwd), sources.every).unwrap_or(0);

    let mut child = match Command::new(program)
        .args(arguments)
        .current_dir(&request.cwd)
        .process_group(0)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let at = host::now((sources.clock)());
            host::release(state, &grant, Some(ReleaseReason::Exited), &at)?;
            return Err(error).with_context(|| format!("build run: start {program:?}"));
        }
    };
    let group = i64::from(child.id());
    if let Err(error) = host::start_run(state, &grant, group, &host::now((sources.clock)())) {
        signal(group, Signal::SIGKILL);
        let _ = child.wait();
        let at = host::now((sources.clock)());
        let _ = host::release(state, &grant, Some(ReleaseReason::Killed), &at);
        return Err(error);
    }

    let done = Arc::new(AtomicBool::new(false));
    let watchdog = {
        let done = Arc::clone(&done);
        let watch = Watchdog {
            grant: grant.clone(),
            start,
            state: state.to_owned(),
            sources: sources.clone(),
            settings: settings.clone(),
            group,
            measured: measured(&request.cwd),
        };
        thread::spawn(move || watch.run(&done))
    };
    let status = child.wait();
    done.store(true, Ordering::SeqCst);
    let watched = watchdog
        .join()
        .map_err(|_| anyhow!("build run: the disk watchdog panicked"))?;
    let status = status.context("build run: wait for the command")?;

    let (code, killed) = match (status.code(), status.signal()) {
        (Some(code), _) => (u8::try_from(code & 0xff).unwrap_or(1), false),
        (None, Some(signal)) => (u8::try_from(128 + (signal & 0x7f)).unwrap_or(1), true),
        (None, None) => (1, true),
    };
    // The run's growth: what it wrote beyond the size it started from.
    let peak = watched
        .peak
        .max(size(&measured(&request.cwd), sources.every).unwrap_or(0))
        .saturating_sub(watched.start);
    if let Err(error) = host::record_peak_of(state, &grant, peak) {
        eprintln!("conductor: build run: the peak was not recorded: {error:#}");
    }
    let reason = if killed {
        ReleaseReason::Killed
    } else {
        ReleaseReason::Exited
    };
    if let Err(error) = host::release(state, &grant, Some(reason), &host::now((sources.clock)())) {
        eprintln!("conductor: build run: the grant was not released: {error:#}");
    }
    host::end_run(state, &grant, run_lock);
    Ok(Outcome::Ran {
        grant,
        code,
        killed,
        peak,
        reclaimed,
        paused: watched.paused,
        resumed: watched.resumed,
    })
}

/// `conductor build run`: [`run`] for the active instance, in the working directory. Exits with
/// the command's status, or 2 naming the shortfall on standard error when it was refused.
///
/// # Errors
///
/// `--reserve` or the command is missing, or [`run`] fails.
pub fn build_run(state: Option<&Path>, args: &BuildRunArgs) -> Result<ExitCode> {
    const COMMAND: &str = "build run";
    let reserve = args
        .reserve
        .ok_or_else(|| anyhow!("{COMMAND}: --reserve is missing"))?;
    let reserve = u64::try_from(reserve)
        .map_err(|_| anyhow!("{COMMAND}: --reserve {reserve} is no size"))?
        .saturating_mul(GIB);
    let instance = &crate::config::active().instance;
    let request = Request {
        reserve,
        repository: args.repository.clone(),
        session_name: None,
        command: args.command.clone(),
        cwd: std::env::current_dir().context("build run: read the working directory")?,
    };
    let outcome = run(
        &crate::state::dir(state)?,
        &Settings::of(instance),
        &Sources::of(instance),
        &request,
    )?;
    if let Outcome::Refused {
        grant,
        bytes,
        shortfall,
        reclaimed,
        left,
        item,
    } = &outcome
    {
        let mut err = std::io::stderr().lock();
        let _ = writeln!(
            err,
            "conductor: {COMMAND}: refused: {}G asked, {}G short after reclaim freed {}G; the \
             command did not run (grant {})",
            gib(*bytes),
            gib(*shortfall),
            gib(*reclaimed),
            grant.0
        );
        for (path, bytes) in left {
            let _ = writeln!(err, "  left: {path} ({}G)", gib(*bytes));
        }
        if let Some(item) = item {
            let _ = writeln!(
                err,
                "  operator item {item}: `conductor decision hands-todo` lists it"
            );
        }
    }
    Ok(ExitCode::from(outcome.exit_code()))
}

/// What the watchdog saw of one run.
#[derive(Debug, Default)]
struct Watched {
    /// The size measured when the run started.
    start: u64,
    peak: u64,
    paused: u32,
    resumed: u32,
}

/// The disk watchdog of one run.
struct Watchdog {
    grant: GrantId,
    start: u64,
    state: PathBuf,
    sources: Sources,
    settings: Settings,
    group: i64,
    measured: PathBuf,
}

impl Watchdog {
    /// Reads free disk and measures the run every [`Sources::every`] until `done`.
    fn run(&self, done: &AtomicBool) -> Watched {
        let start = self.start;
        let mut watched = Watched {
            start,
            peak: start,
            ..Watched::default()
        };
        let mut recorded = 0u64;
        let mut stopped = false;
        while !wait(done, self.sources.every) {
            match free_bytes(&self.sources) {
                Ok(free) if !stopped && free < self.settings.gate_stop => {
                    signal(self.group, Signal::SIGSTOP);
                    stopped = true;
                    watched.paused += 1;
                    eprintln!(
                        "conductor: build run: paused: {}G free is under gate_stop {}G; \
                         reclaiming",
                        free / GIB,
                        gib(self.settings.gate_stop)
                    );
                    let need = self.settings.gate_resume;
                    reclaim(&self.sources, &self.state, need);
                }
                Ok(free) if stopped && free >= self.settings.gate_resume => {
                    signal(self.group, Signal::SIGCONT);
                    stopped = false;
                    watched.resumed += 1;
                    eprintln!(
                        "conductor: build run: resumed: {}G free is over gate_resume {}G",
                        free / GIB,
                        gib(self.settings.gate_resume)
                    );
                }
                Ok(_) => {}
                Err(error) => eprintln!("conductor: build run: {error:#}"),
            }
            if let Some(bytes) = size(&self.measured, self.sources.every) {
                watched.peak = watched.peak.max(bytes);
            }
            // The growth so far, recorded as it passes each further gibibyte, so the ledger counts
            // only what this run has not written yet.
            let grown = watched.peak.saturating_sub(watched.start);
            if grown >= recorded.saturating_add(GIB) {
                match host::record_peak_now(&self.state, &self.grant, grown) {
                    Ok(true) => recorded = grown,
                    Ok(false) => {}
                    Err(error) => eprintln!("conductor: build run: {error:#}"),
                }
            }
        }
        if stopped {
            signal(self.group, Signal::SIGCONT);
        }
        watched
    }
}

/// Sleeps `every`, waking early when `done`; answers whether it is.
fn wait(done: &AtomicBool, every: Duration) -> bool {
    let until = Instant::now() + every;
    loop {
        if done.load(Ordering::SeqCst) {
            return true;
        }
        let now = Instant::now();
        if now >= until {
            return false;
        }
        thread::sleep((until - now).min(Duration::from_millis(20)));
    }
}

/// Sends `signal` to the process group `group`; a group that is gone is not an error.
fn signal(group: i64, signal: Signal) {
    if let Ok(group) = i32::try_from(group) {
        let _ = killpg(Pid::from_raw(group), signal);
    }
}

/// Free disk, as [`Sources::free_bytes`] reads it.
fn free_bytes(sources: &Sources) -> Result<u64> {
    (sources.free_bytes)(&sources.disk)
        .with_context(|| format!("read free disk on {}", sources.disk.display()))
}

/// One reclaim run for `need` bytes; a failing one is named on standard error and frees nothing.
fn reclaim(sources: &Sources, state: &Path, need: u64) -> ReclaimReport {
    (sources.reclaim)(state, need).unwrap_or_else(|error| {
        eprintln!("conductor: build run: reclaim: {error:#}");
        ReclaimReport {
            reclaimed_bytes: 0,
            steps: Vec::new(),
            left: Vec::new(),
            reached: false,
        }
    })
}

/// What a run in `cwd` writes: its `target/` when `cwd` is a Cargo project or holds one, else
/// `cwd` itself.
fn measured(cwd: &Path) -> PathBuf {
    let target = cwd.join("target");
    if target.exists() || cwd.join("Cargo.toml").exists() {
        target
    } else {
        cwd.to_owned()
    }
}

/// The bytes under `path`, or none when the walk takes longer than `bound`.
fn size(path: &Path, bound: Duration) -> Option<u64> {
    if !path.exists() {
        return Some(0);
    }
    crate::disk::tree_size(
        &[path.to_owned()],
        Some(Instant::now() + bound.max(Duration::from_secs(1))),
    )
    .ok()
    .map(|size| size.allocated)
}
