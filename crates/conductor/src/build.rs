//! `conductor build run --reserve <size> [--repository <repo>] -- <command> [args…]`
//! (`story:build-run`): disk for a build is a reservation the binary grants, not a message.
//!
//! 1. **Reserve.** Under the host ledger's lock ([`host::Ledger`]) the run first releases every
//!    grant whose process group is gone, then records its request (`RequestDisk`) and counts what
//!    the disk lacks for it: the bytes asked, less free disk, less every live reservation, less
//!    the floor (`thresholds.disk_admit` and the margin `conductor host reclaim` keeps,
//!    [`crate::gc::reclaim::floor_bytes`]). Free disk is read as the watch and reclaim read it,
//!    through [`crate::gc::reclaim::free_bytes`] on `thresholds.disk_path` ([`Sources::free_bytes`]).
//!    Requests are answered in arrival order (`story:build-run-queue`): while a request made
//!    before it still waits ([`host::Ledger::ahead`]), a run waits behind it, even when it alone
//!    would fit, and names its position in the queue on standard error once it has waited
//!    `thresholds.watchdog_every`. The head waits for live reservations at most
//!    [`Settings::wait`], then is refused; one whose shortfall they do not cover is answered at
//!    once. When every request ahead lacks more than the live reservations hold together, so each
//!    can only be refused, a run that fits beside the floor and the live reservations is granted
//!    past them ([`host::Ledger::ahead_bytes`], `story:build-run-backfill`).
//! 2. **Reclaim.** When it does not fit, one reclaim run frees what needs no decision
//!    ([`crate::gc::reclaim::reclaim`]), and the run counts again. The reclaim never runs under
//!    the ledger's lock: the run lets it go, reclaims, takes it again and counts from the start,
//!    so the queue never waits on one run's walk of the disk; the requests behind it stay behind
//!    it meanwhile. One reclaim walks the disk at a time (`host.reclaim.lock` in the host
//!    directory, `host/` beside the config file, which every instance shares), and a run that finds one started within [`RECLAIM_REUSE`] (`host.reclaim`: when
//!    it started and ended, what it was asked to reach, freed and left, or why it failed) reuses
//!    its result instead of walking again ([`shared_reclaim`]). When that run ended and freed
//!    nothing, the head takes no reclaim lock and walks nothing: it is answered at once, naming
//!    that run ([`barren_reclaim`]). The answer (`ReserveDisk`) is
//!    granted, granted after reclaim, or refused with the shortfall: the command never runs, the
//!    shortfall and what is left are printed on standard error as the operator item names them
//!    (`/var/tmp` entries as a count and a size, the home as `~`), one operator item is recorded
//!    for the fall of free disk ([`host::escalate`]) unless only scratch and caches are left
//!    ([`host::needs_person`], `story:reclaim-scratch-class-c`), and the exit is 2. The fall, and
//!    its item, end when free disk is back at `thresholds.disk_clear`: each run reads it first
//!    ([`host::Ledger::clear_fall`]), not when a reservation fits.
//! 3. **Run under the watchdog.** The command runs in its own process group (`StartRun`). Every
//!    `thresholds.watchdog_every` a watchdog reads free disk: under `thresholds.gate_stop` it
//!    stops the group (`SIGSTOP`) and runs reclaim; at or over `thresholds.gate_resume` it
//!    continues it (`SIGCONT`). It measures the run's `target/` (the working directory's, when
//!    that is a Cargo project or holds one), else the working directory, and keeps the largest.
//! 4. **Release.** When the command ends, its peak is recorded (`RecordPeak`) and the grant
//!    released (`ReleaseGrant`: `Exited`, or `Killed` by a signal), and `build run` exits with the
//!    command's status, 128 + the signal for a killed one.
//! 5. **Stop** (`story:build-run-cancel-group`). SIGTERM, SIGINT or SIGHUP to `build run` itself
//!    ([`catch`]) never leaves a build behind it: while the command runs, the signal is forwarded
//!    to its whole process group (and SIGCONT after it, for a group the watchdog stopped); a group
//!    with a process left after [`GRACE`] is sent SIGKILL; the run is reaped, its peak recorded and
//!    its grant released as `Killed`. A request still waiting in the queue is withdrawn
//!    (`WithdrawDiskRequest`), and one granted whose command has not started is released. Either
//!    way `build run` exits with 128 + the signal it received.
//! 6. **Session-bound** (`story:build-run-session-bound`). The request records the harness session
//!    `build run` is a command of, from [`SESSION_VARIABLE`]. Each pass of a waiting run reads the
//!    session list ([`Sources::sessions`], `claude agents --json`) when a waiting request names a
//!    session, without the ledger's lock, and withdraws each such request whose session the list
//!    no longer shows running (`WithdrawDiskRequest`, `SessionStopped`). The run whose request it
//!    was says `withdrawn: session <id> stopped` and exits 2; its command never ran. A list that
//!    does not read withdraws nothing. The list shows no paused session apart from a working one,
//!    so a session paused on a usage limit keeps its requests until it stops.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::Instance;
use conductor_model::host::{GrantId, ReleaseReason};
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use serde_json::{Value, json};
use time::OffsetDateTime;

use crate::cli::BuildRunArgs;
use crate::gc::reclaim::ReclaimReport;
use crate::host::{self, Answer, Ledger, Shortfall, gib};

/// One GiB.
const GIB: u64 = 1 << 30;

/// The signals that stop `build run`, each forwarded to its command's process group.
pub const STOPPING: [Signal; 3] = [Signal::SIGTERM, Signal::SIGINT, Signal::SIGHUP];

/// How long the command's process group has to end after the forwarded signal before it is sent
/// SIGKILL.
pub const GRACE: Duration = Duration::from_secs(10);

/// How often a run that waits, in the queue or on its command, looks for a signal it received.
const SIGNAL_POLL: Duration = Duration::from_millis(20);

/// The signal this process received of [`STOPPING`], 0 while none, once [`catch`] installed its
/// handlers.
static RECEIVED: OnceLock<Arc<AtomicUsize>> = OnceLock::new();

/// Installs the handlers that note SIGTERM, SIGINT and SIGHUP instead of ending this process, so
/// [`run`] stops its build first. A command `build run` starts gets the default handlers back
/// when it executes.
///
/// # Errors
///
/// A handler cannot be installed.
pub fn catch() -> Result<()> {
    let received = RECEIVED.get_or_init(|| Arc::new(AtomicUsize::new(0)));
    for signal in STOPPING {
        let number = signal as i32;
        signal_hook::flag::register_usize(
            number,
            Arc::clone(received),
            usize::try_from(number).unwrap_or(0),
        )
        .with_context(|| format!("build run: catch {signal}"))?;
    }
    Ok(())
}

/// The signal of [`STOPPING`] this process received, if [`catch`] installed its handlers and one
/// arrived.
fn received() -> Option<Signal> {
    let number = RECEIVED.get()?.load(Ordering::SeqCst);
    Signal::try_from(i32::try_from(number).ok()?)
        .ok()
        .filter(|signal| STOPPING.contains(signal))
}

/// Sleeps `pause`, waking early once a signal was received ([`received`]).
fn pause_unless_received(pause: Duration) {
    let until = Instant::now() + pause;
    while received().is_none() {
        let now = Instant::now();
        if now >= until {
            return;
        }
        thread::sleep((until - now).min(SIGNAL_POLL));
    }
}

/// 128 + `signal`, the exit of a run `signal` stopped.
fn signal_code(signal: Signal) -> u8 {
    u8::try_from(128 + signal as i32).unwrap_or(1)
}

/// How often the watchdog reads free disk when `thresholds.watchdog_every` does not read.
const EVERY: Duration = Duration::from_secs(10);

/// How long one walk over the directories a run writes may take before its reading is dropped
/// (`story:reserve-from-peak`). The walk has its own thread, so the watchdog never waits for it.
const WALK_BOUND: Duration = Duration::from_secs(60);

/// The variable the harness sets for the commands of a session, naming it: Claude Code's
/// (`story:build-run-session-bound`). `build run` records it on its request.
pub const SESSION_VARIABLE: &str = "CLAUDE_CODE_SESSION_ID";

/// How long the session list ([`Sources::sessions`]) may take before it counts as unread.
const SESSIONS_BOUND: Duration = Duration::from_secs(20);

/// What the instance sets for a run: its name and its thresholds, in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The instance the grant is taken for.
    pub instance: String,
    /// The key of the instance's fall of free disk in the host directory: its session prefix, its
    /// name without one (`story:host-fall-per-instance`).
    pub fall_key: String,
    /// Free bytes no reservation may take: `thresholds.disk_admit` and reclaim's margin
    /// ([`crate::gc::reclaim::floor_bytes`]).
    pub floor: u64,
    /// Free bytes from which a fall of free disk is over and its operator item withdrawn:
    /// `thresholds.disk_clear`, never under [`Settings::floor`] ([`host::clear_fall`]).
    pub clear: u64,
    /// Free bytes under which the run is stopped: `thresholds.gate_stop`.
    pub gate_stop: u64,
    /// Free bytes from which a stopped run continues: `thresholds.gate_resume`.
    pub gate_resume: u64,
    /// How long a reservation waits for live reservations to release when they alone hold the
    /// space it lacks: such a shortfall ends when their runs end, and is not the operator's.
    pub wait: Duration,
    /// The CPU cores of the host.
    pub cores: u64,
    /// The cores left to the desktop: `thresholds.cpu_reserve`.
    pub cpu_reserve: u64,
}

impl Settings {
    /// The settings of `instance`.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        let bytes = |gib: i64| u64::try_from(gib).unwrap_or(0).saturating_mul(GIB);
        let thresholds = &instance.thresholds;
        Self {
            instance: instance.name.0.clone(),
            fall_key: crate::host::fall_key(instance),
            floor: crate::gc::reclaim::floor_bytes(
                u64::try_from(thresholds.disk_admit.0).unwrap_or(0),
            ),
            clear: bytes(thresholds.disk_clear.0),
            gate_stop: bytes(thresholds.gate_stop.0),
            gate_resume: bytes(thresholds.gate_resume.0),
            wait: WAIT_FOR_RESERVATIONS,
            cores: std::thread::available_parallelism().map_or(1, |cores| cores.get() as u64),
            cpu_reserve: u64::try_from(thresholds.cpu_reserve).unwrap_or(0),
        }
    }
}

/// How long a reservation waits for live reservations to release: an hour, a long gate's length.
pub const WAIT_FOR_RESERVATIONS: Duration = Duration::from_secs(3600);

/// How long the result of a reclaim run is reused by the next run that needs one, instead of
/// walking the disk again: five minutes. Several runs short in a row then walk the disk once, not
/// once each; a walk of a full disk takes minutes.
pub const RECLAIM_REUSE: Duration = Duration::from_secs(5 * 60);

/// The lock one reclaim run holds while it walks the disk, in the host directory.
const RECLAIM_LOCK: &str = "host.reclaim.lock";

/// The last reclaim run of the host, in the host directory: when it started and ended, the bytes
/// it was asked to reach, whether it reached them, what it freed and left, or why it failed.
const RECLAIM_RECORD: &str = "host.reclaim";

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
    /// The home directory, written `~` in an operator item: `HOME`.
    pub home: Option<PathBuf>,
    /// `/var/tmp`, whose entries an operator item names only as a count and a total size.
    pub var_tmp: PathBuf,
    /// The instance's own scratch, `<cache>/scratch`: reclaim deletes stale entries there, so what
    /// is left there needs no person.
    pub scratch: PathBuf,
    /// Writes one line of standard error, given the state directory: [`stderr`]. A case gives its
    /// own.
    pub say: fn(&Path, &str),
    /// Reads a variable of `build run`'s own environment, which its command inherits.
    pub env: fn(&str) -> Option<String>,
    /// The program and arguments that print the harness's session list as one JSON array:
    /// `claude agents --json`, as the `sessions` collector reads it. A session is running while
    /// an entry with a `pid` names it by `id` or `sessionId`; the list keeps one that exited
    /// without one. Empty: no list is read, and no request is withdrawn for its session.
    pub sessions: Vec<OsString>,
    /// The bytes under the directories `paths`, or none when the walk takes longer than the
    /// bound: [`walk`]. A case gives a slow or a failing one.
    pub walk: fn(&[PathBuf], Duration) -> Option<u64>,
}

/// Writes `line` to standard error.
pub fn stderr(_state: &Path, line: &str) {
    let _ = writeln!(std::io::stderr().lock(), "{line}");
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
            home: std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|home| home.is_absolute()),
            var_tmp: PathBuf::from("/var/tmp"),
            scratch: Path::new(&instance.cache).join(crate::dashboard::SCRATCH),
            say: stderr,
            env: |name| std::env::var(name).ok(),
            sessions: ["claude", "agents", "--json"].map(OsString::from).to_vec(),
            walk,
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
    /// Directories measured besides the run's own (`--measure`).
    pub measure: Vec<PathBuf>,
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
        /// The growth measured of what it writes; none when no measurement of it completed.
        peak: Option<u64>,
        /// What a reclaim run freed before it was granted.
        reclaimed: u64,
        /// How often the watchdog stopped it.
        paused: u32,
        /// How often the watchdog continued it.
        resumed: u32,
    },
    /// A signal stopped `build run` before its command ran ([`STOPPING`]): the request, still
    /// waiting in the queue, was withdrawn, or the grant was released.
    Stopped {
        /// The grant.
        grant: GrantId,
        /// The signal received.
        signal: Signal,
    },
    /// The request, still waiting in the queue, was withdrawn because the session that made it
    /// is no longer running (`story:build-run-session-bound`): the command never ran.
    Withdrawn {
        /// The withdrawn grant.
        grant: GrantId,
        /// The session that stopped.
        session: String,
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
    /// The exit status of `build run`: the command's, or 2 when it was refused or its session
    /// stopped while it waited.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Ran { code, .. } => *code,
            Self::Stopped { signal, .. } => signal_code(*signal),
            Self::Refused { .. } | Self::Withdrawn { .. } => 2,
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

/// Runs `request` over the ledger in the host directory `host` (`story:instance-isolation`), for
/// the instance whose state directory is `state`: its reclaim and its operator item are that
/// instance's.
///
/// # Errors
///
/// The command is empty, the ledger does not open or keep a move, free disk does not read, or
/// the command does not start. A failing reclaim is named on standard error and counted as
/// freeing nothing.
pub fn run(
    state: &Path,
    host: &Path,
    settings: &Settings,
    sources: &Sources,
    request: &Request,
) -> Result<Outcome> {
    let Some((program, _)) = request.command.split_first() else {
        bail!("build run: no command: write it after `--`");
    };
    let command = request.command.join(" ");
    let at = host::now((sources.clock)());

    let mut ledger = Ledger::lock(host)?;
    ledger.sweep(&at)?;
    // Free disk back at `disk_clear` ends the fall: its operator item is withdrawn.
    let clear = settings.clear.max(settings.floor);
    if let Err(error) = ledger.clear_fall(
        state,
        &settings.fall_key,
        &settings.instance,
        free_bytes(sources)?,
        clear,
    ) {
        (sources.say)(
            state,
            &format!("conductor: build run: the fall's operator item was not withdrawn: {error:#}"),
        );
    }
    // The harness session this run is a command of (`story:build-run-session-bound`).
    let session = (sources.env)(SESSION_VARIABLE).filter(|id| !id.trim().is_empty());
    let grant = ledger.request_of(
        &settings.instance,
        request.repository.as_deref(),
        request.session_name.as_deref(),
        session.as_deref(),
        request.reserve,
        &command,
        &at,
    )?;
    // Held from the request to the release, so a sweep closes this grant only once this run is gone.
    let run_lock = host::hold_run(host, &grant)?;
    let lacking = |ledger: &Ledger| -> Result<u64> {
        let free = free_bytes(sources)?;
        Ok(shortfall(
            request.reserve,
            free,
            ledger.reserved(Some(&grant))?,
            settings.floor,
        ))
    };
    let say = |line: &str| (sources.say)(state, line);
    let mut short;
    let mut reclaimed = 0;
    let mut left = Vec::new();
    let mut reclaim_ran = false;
    // The queue (`story:build-run-queue`): the head is answered, and a request made later never
    // takes the space an earlier one waits for. The head waits for live reservations at most
    // `settings.wait` (`held`); since nothing behind it is granted meanwhile, they only shrink. A
    // request behind heads that can only be refused is granted when it fits (backfill).
    let mut held = Duration::ZERO;
    let mut queued = Duration::ZERO;
    let mut told = None;
    let mut unread_told = false;
    let pause = sources.every.max(Duration::from_millis(10));
    loop {
        // A signal while the request waits withdraws it: it leaves the queue, and the command
        // never runs.
        if let Some(signal) = received() {
            ledger.withdraw(&grant, &host::now((sources.clock)()))?;
            drop(ledger);
            host::end_run(host, &grant, run_lock);
            say(&format!(
                "conductor: build run: {signal}: the request was withdrawn from the queue; the \
                 command did not run (grant {})",
                grant.0
            ));
            return Ok(Outcome::Stopped { grant, signal });
        }
        // A ledger pass withdrew the request: the session that made it stopped.
        if let Some(session) = ledger.stopped_session(&grant)? {
            drop(ledger);
            host::end_run(host, &grant, run_lock);
            say(&format!(
                "conductor: build run: withdrawn: session {session} stopped; the command did not \
                 run (grant {})",
                grant.0
            ));
            return Ok(Outcome::Withdrawn { grant, session });
        }
        let before = ledger.ahead_bytes(&grant)?;
        let ahead = before.len();
        if ahead == 0 {
            short = lacking(&ledger)?;
            if short > 0 && !reclaim_ran {
                reclaim_ran = true;
                // A reclaim run that ended a moment ago and freed nothing is not walked again:
                // the head is answered now, naming that run (`story:build-run-backfill`).
                if let Some(barren) = barren_reclaim(sources, host) {
                    say(&format!(
                        "conductor: build run: reclaim: the reclaim run started {}s ago freed \
                         nothing; no reclaim walks the disk again until it is {}s old",
                        barren.age,
                        RECLAIM_REUSE.as_secs()
                    ));
                    left = barren.left;
                    continue;
                }
                // Reclaim frees until this many bytes are free: where the request fits beside the
                // floor and every live reservation.
                let fits = settings
                    .floor
                    .saturating_add(ledger.reserved(Some(&grant))?)
                    .saturating_add(request.reserve);
                // Never under the ledger's lock: every other run would wait on this walk. The
                // requests behind this one stay behind it meanwhile, as its request is held.
                drop(ledger);
                let report = reclaim(sources, state, host, fits);
                reclaimed = report.reclaimed_bytes;
                left = report.left;
                ledger = Ledger::lock(host)?;
                ledger.sweep(&host::now((sources.clock)()))?;
                // Count again from the start, under the lock.
                continue;
            }
            // A shortfall live reservations cover is a wait: they release as their runs end. Wait
            // for them, without the lock, rather than refuse; any other is answered now.
            let holding = ledger.reserved(Some(&grant))?;
            if short == 0 || short > holding || held >= settings.wait {
                break;
            }
            if held.is_zero() {
                say(&format!(
                    "conductor: build run: waiting: {}G short while live reservations hold {}G; \
                     they release as their runs end",
                    short.div_ceil(GIB),
                    holding.div_ceil(GIB)
                ));
            }
            held += pause;
        } else {
            // Backfill (`story:build-run-backfill`): when every request ahead lacks more than the
            // live reservations hold together, each can only be refused, so one that fits beside
            // the floor and the live reservations is granted past them. While any one ahead lacks
            // no more than they hold, it waits for them, and nothing behind it is granted.
            let free = free_bytes(sources)?;
            let holding = ledger.reserved(Some(&grant))?;
            let refused_only = before
                .iter()
                .all(|&bytes| shortfall(bytes, free, holding, settings.floor) > holding);
            if refused_only && shortfall(request.reserve, free, holding, settings.floor) == 0 {
                short = 0;
                say(&format!(
                    "conductor: build run: granted past {ahead} request(s) ahead, each short of \
                     more than the live reservations hold"
                ));
                break;
            }
        }
        // Waiting longer than `thresholds.watchdog_every` names the position, and again when it
        // moves.
        if queued >= sources.every && told != Some(ahead) {
            say(&format!(
                "conductor: build run: queued: position {} in the queue of disk requests, {ahead} \
                 ahead; waited {}s",
                ahead + 1,
                queued.as_secs()
            ));
            told = Some(ahead);
        }
        // The ledger pass (`story:build-run-session-bound`): the requests waiting for a session
        // the session list shows stopped are withdrawn. The list is read without the lock, and
        // only a request that waited before it was read is withdrawn.
        let bound = ledger.session_bound()?;
        drop(ledger);
        let stopped = stopped_sessions(sources, state, &bound, &mut unread_told);
        if stopped.is_empty() {
            pause_unless_received(pause);
            queued += pause;
        }
        ledger = Ledger::lock(host)?;
        let at = host::now((sources.clock)());
        ledger.sweep(&at)?;
        for (stopped, session) in &stopped {
            if ledger.withdraw_stopped(stopped, &at)? && *stopped != grant {
                say(&format!(
                    "conductor: build run: withdrew grant {}: session {session} stopped",
                    stopped.0
                ));
            }
        }
    }
    // Only a shortfall live reservations do not cover needs a person.
    let beyond_reservations = short > ledger.reserved(Some(&grant))?;
    let answer = ledger.reserve(&grant, short, reclaimed)?;
    if answer == Answer::Refused {
        // The operator item goes into the instance's store, under `host.lock`, which keeps one
        // item per fall. Its writer lock is never waited for under `host.lock`
        // (`story:store-readers-hold-no-writer-lock`): while another writer holds it, the run lets
        // `host.lock` go, waits, and takes it again.
        let _ledger = if beyond_reservations && crate::store::tree_held(state).unwrap_or(false) {
            drop(ledger);
            if let Err(error) = crate::store::wait_for_tree(state) {
                say(&format!(
                    "conductor: build run: wait for the store's writer lock: {error}"
                ));
            }
            Ledger::lock(host)?
        } else {
            ledger
        };
        let escalate = |shortfall: &Shortfall<'_>| {
            if !beyond_reservations {
                return Ok(None);
            }
            if !host::needs_person(shortfall) {
                say(
                    "conductor: build run: no operator item: only scratch and caches are left, \
                     which reclaim deletes once each is stale and nothing holds it",
                );
                return Ok(None);
            }
            host::escalate(host, state, shortfall)
        };
        let shortfall = Shortfall {
            instance: &settings.instance,
            key: &settings.fall_key,
            repository: request.repository.as_deref(),
            command: &command,
            bytes: request.reserve,
            shortfall: short,
            left: &left,
            disk: &sources.disk,
            home: sources.home.as_deref(),
            var_tmp: &sources.var_tmp,
            scratch: Some(&sources.scratch),
            at: &at,
        };
        let item = escalate(&shortfall).unwrap_or_else(|error| {
            (sources.say)(
                state,
                &format!("conductor: build run: the operator item was not recorded: {error:#}"),
            );
            None
        });
        host::end_run(host, &grant, run_lock);
        say_refused(
            sources,
            state,
            &grant,
            &shortfall,
            reclaimed,
            item.as_deref(),
        );
        return Ok(Outcome::Refused {
            grant,
            bytes: request.reserve,
            shortfall: short,
            reclaimed,
            left,
            item,
        });
    }
    drop(ledger);
    // Measured before the command starts, so a run's growth counts what a fast command wrote too.
    let paths = measured(request, sources);
    let start = (sources.walk)(&paths, WALK_BOUND);
    // A signal before the command starts: the granted reservation is released, and nothing runs.
    if let Some(signal) = received() {
        host::release(
            host,
            &grant,
            Some(ReleaseReason::Killed),
            &host::now((sources.clock)()),
        )?;
        host::end_run(host, &grant, run_lock);
        say(&format!(
            "conductor: build run: {signal}: the grant was released; the command did not run \
             (grant {})",
            grant.0
        ));
        return Ok(Outcome::Stopped { grant, signal });
    }

    let mut command = yielding(&request.command);
    command.current_dir(&request.cwd).process_group(0);
    for (name, value) in job_variables(settings, sources) {
        command.env(name, value);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let at = host::now((sources.clock)());
            host::release(host, &grant, Some(ReleaseReason::Exited), &at)?;
            return Err(error).with_context(|| format!("build run: start {program:?}"));
        }
    };
    let group = i64::from(child.id());
    if let Err(error) = host::start_run(host, &grant, group, &host::now((sources.clock)())) {
        signal(group, Signal::SIGKILL);
        let _ = child.wait();
        let at = host::now((sources.clock)());
        let _ = host::release(host, &grant, Some(ReleaseReason::Killed), &at);
        return Err(error);
    }

    let done = Arc::new(AtomicBool::new(false));
    let watchdog = {
        let done = Arc::clone(&done);
        let watch = Watchdog {
            grant: grant.clone(),
            start,
            state: state.to_owned(),
            host: host.to_owned(),
            sources: sources.clone(),
            settings: settings.clone(),
            group,
            measured: paths.clone(),
        };
        thread::spawn(move || watch.run(&done))
    };
    let (status, stopped_by) = supervise(&mut child, group);
    done.store(true, Ordering::SeqCst);
    let watched = watchdog
        .join()
        .map_err(|_| anyhow!("build run: the disk watchdog panicked"))?;
    let status = status.context("build run: wait for the command")?;
    if watched.paused > 0 {
        // A red run during a pause is recognised as one (story:build-run-pause-visible).
        (sources.say)(
            state,
            &format!(
                "conductor: build run: paused {} times for {}s during this run; a failure may \
                 come from the pause",
                watched.paused,
                watched.paused_for.as_secs()
            ),
        );
    }

    let (code, killed) = match (stopped_by, status.code(), status.signal()) {
        (Some(signal), _, _) => {
            say(&format!(
                "conductor: build run: {signal}: the command's process group {group} was stopped"
            ));
            (signal_code(signal), true)
        }
        (None, Some(code), _) => (u8::try_from(code & 0xff).unwrap_or(1), false),
        (None, None, Some(signal)) => (u8::try_from(128 + (signal & 0x7f)).unwrap_or(1), true),
        (None, None, None) => (1, true),
    };
    // The run's growth: what it wrote beyond the size it started from, by the largest complete
    // reading. With no reading of the start or of the end it is unknown, and none is recorded.
    let last = (sources.walk)(&paths, WALK_BOUND);
    let peak = watched
        .start
        .zip([watched.peak, last].into_iter().flatten().max())
        .map(|(start, peak)| peak.saturating_sub(start));
    if let Some(peak) = peak
        && let Err(error) = host::record_run_of(host, &grant, peak, watched.paused)
    {
        (sources.say)(
            state,
            &format!("conductor: build run: the peak was not recorded: {error:#}"),
        );
    }
    let reason = if killed {
        ReleaseReason::Killed
    } else {
        ReleaseReason::Exited
    };
    if let Err(error) = host::release(host, &grant, Some(reason), &host::now((sources.clock)())) {
        (sources.say)(
            state,
            &format!("conductor: build run: the grant was not released: {error:#}"),
        );
    }
    host::end_run(host, &grant, run_lock);
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

/// The command at the lowest priority: `nice -n 19`, and `ionice -c3` (idle I/O class) when the
/// host has one. Both exec the command, so the process group and its leader stay the command's.
fn yielding(command: &[String]) -> Command {
    let mut argv: Vec<&str> = vec!["nice", "-n", "19"];
    if on_path("ionice") {
        argv.extend(["ionice", "-c3"]);
    }
    argv.extend(command.iter().map(String::as_str));
    let mut wrapped = Command::new(argv[0]);
    wrapped.args(&argv[1..]);
    wrapped
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

/// The variables that keep the command's builds off the desktop's last cores: `CARGO_BUILD_JOBS`
/// and `MAKEFLAGS=-j` of the cores less `thresholds.cpu_reserve` (at least one), each unless the
/// environment of `build run` already holds it (`story:build-run-spare-cores`).
fn job_variables(settings: &Settings, sources: &Sources) -> Vec<(&'static str, String)> {
    let jobs = settings.cores.saturating_sub(settings.cpu_reserve).max(1);
    [
        ("CARGO_BUILD_JOBS", jobs.to_string()),
        ("MAKEFLAGS", format!("-j{jobs}")),
    ]
    .into_iter()
    .filter(|(name, _)| (sources.env)(name).is_none())
    .collect()
}

/// Waits for the command `child`, the leader of the process group `group`, to end. A signal
/// `build run` receives meanwhile ([`received`]) is forwarded to the whole group, with SIGCONT
/// after it; once the leader has ended, the wait goes on until no process of the group is left,
/// and a group with one left [`GRACE`] after the signal is sent SIGKILL. Answers the leader's
/// status and the signal that stopped the run, if one did.
fn supervise(
    child: &mut std::process::Child,
    group: i64,
) -> (std::io::Result<std::process::ExitStatus>, Option<Signal>) {
    let mut forwarded: Option<(Signal, Instant)> = None;
    let mut killed = false;
    let mut status = None;
    loop {
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(exited)) => status = Some(Ok(exited)),
                Ok(None) => {}
                Err(error) => status = Some(Err(error)),
            }
        }
        if forwarded.is_none()
            && let Some(received) = received()
        {
            signal(group, received);
            signal(group, Signal::SIGCONT);
            forwarded = Some((received, Instant::now()));
        }
        match (&status, forwarded) {
            (Some(_), None) => break,
            (Some(_), Some(_)) if !host::alive(group) => break,
            _ => {}
        }
        if let Some((_, since)) = forwarded
            && !killed
            && since.elapsed() >= GRACE
        {
            signal(group, Signal::SIGKILL);
            killed = true;
        }
        thread::sleep(SIGNAL_POLL);
    }
    let status = status.unwrap_or_else(|| child.wait());
    (status, forwarded.map(|(signal, _)| signal))
}

/// `conductor build run`: [`run`] for the active instance, in the working directory, over the
/// host directory ([`host::dir`]), with the handlers of [`catch`] installed. Exits with the command's status, 2 when it was refused
/// ([`run`] names the shortfall on standard error), or 128 + the signal that stopped it.
///
/// # Errors
///
/// `--reserve` or the command is missing, a handler cannot be installed, or [`run`] fails.
pub fn build_run(state: Option<&Path>, args: &BuildRunArgs) -> Result<ExitCode> {
    const COMMAND: &str = "build run";
    catch()?;
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
        measure: args.measure.iter().map(PathBuf::from).collect(),
        cwd: std::env::current_dir().context("build run: read the working directory")?,
    };
    let host = host::dir(state)?;
    let outcome = run(
        &crate::state::dir(state)?,
        &host,
        &Settings::of(instance),
        &Sources::of(instance),
        &request,
    )?;
    Ok(ExitCode::from(outcome.exit_code()))
}

/// The requests of `bound` (waiting, each with the session that made it) whose session the
/// session list ([`Sources::sessions`]) no longer shows running (`story:build-run-session-bound`).
/// None while `bound` is empty, since then no list is read, and none when the list does not read:
/// a request is withdrawn only for a session the list shows stopped. An unread list is named on
/// standard error once per run (`told`).
fn stopped_sessions(
    sources: &Sources,
    state: &Path,
    bound: &[(GrantId, String)],
    told: &mut bool,
) -> Vec<(GrantId, String)> {
    if bound.is_empty() || sources.sessions.is_empty() {
        return Vec::new();
    }
    match running_sessions(&sources.sessions) {
        Ok(running) => bound
            .iter()
            .filter(|(_, session)| !running.contains(session))
            .cloned()
            .collect(),
        Err(error) => {
            if !*told {
                *told = true;
                (sources.say)(
                    state,
                    &format!(
                        "conductor: build run: the session list did not read, so no request is \
                         withdrawn for its session: {error:#}"
                    ),
                );
            }
            Vec::new()
        }
    }
}

/// The ids of the sessions `list` prints running: each entry with a `pid`, by its `id` and its
/// `sessionId`.
fn running_sessions(list: &[OsString]) -> Result<HashSet<String>> {
    let shown = list
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let (program, arguments) = list.split_first().context("no session-list command")?;
    let output = crate::collect::run(Command::new(program).args(arguments), SESSIONS_BOUND)
        .with_context(|| format!("`{shown}`"))?;
    if !output.status.success() {
        bail!("`{shown}` exited {}", output.status);
    }
    let Value::Array(entries) = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("`{shown}` printed no JSON"))?
    else {
        bail!("`{shown}` printed no JSON array");
    };
    Ok(entries
        .iter()
        .filter(|entry| !entry["pid"].is_null())
        .flat_map(|entry| [entry["id"].as_str(), entry["sessionId"].as_str()])
        .flatten()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Names a refusal on standard error: the size, the shortfall and what reclaim freed, what reclaim
/// left as the operator item names it ([`host::left_redacted`]: the `/var/tmp` entries as a count
/// and a total size, the home directory as `~`), and the operator item when this refusal recorded
/// it (`story:build-run-stderr-redacted`).
fn say_refused(
    sources: &Sources,
    state: &Path,
    grant: &GrantId,
    shortfall: &Shortfall<'_>,
    reclaimed: u64,
    item: Option<&str>,
) {
    let say = |line: &str| (sources.say)(state, line);
    say(&format!(
        "conductor: build run: refused: {}G asked, {}G short after reclaim freed {}G; the command \
         did not run (grant {})",
        gib(shortfall.bytes),
        gib(shortfall.shortfall),
        gib(reclaimed),
        grant.0
    ));
    for (path, bytes) in host::left_redacted(shortfall) {
        say(&format!("  left: {path} ({}G)", gib(bytes)));
    }
    if let Some(item) = item {
        say(&format!(
            "  operator item {item}: `conductor decision hands-todo` lists it"
        ));
    }
}

/// What the watchdog saw of one run.
#[derive(Debug, Default)]
struct Watched {
    /// The size measured when the run started; none when that walk did not complete.
    start: Option<u64>,
    /// The largest complete reading since; none while there is none.
    peak: Option<u64>,
    paused: u32,
    resumed: u32,
    /// How long the run was stopped, over all its pauses.
    paused_for: Duration,
}

/// The disk watchdog of one run.
struct Watchdog {
    grant: GrantId,
    start: Option<u64>,
    state: PathBuf,
    /// The host directory.
    host: PathBuf,
    sources: Sources,
    settings: Settings,
    group: i64,
    /// What the run writes: the directories measured.
    measured: Vec<PathBuf>,
}

impl Watchdog {
    /// Reads free disk and measures the run every [`Sources::every`] until `done`.
    fn run(&self, done: &Arc<AtomicBool>) -> Watched {
        let start = self.start;
        let mut watched = Watched {
            start,
            peak: None,
            ..Watched::default()
        };
        // The walks run on a thread of their own, each bounded by [`WALK_BOUND`]; the watchdog
        // reads the last complete one, and a walk that ran out of time leaves the reading as it
        // was. The thread ends after the walk under way once the run is over.
        let reading = Arc::new(Mutex::new(None::<u64>));
        {
            let (reading, sources, paths) = (
                Arc::clone(&reading),
                self.sources.clone(),
                self.measured.clone(),
            );
            let done = Arc::clone(done);
            thread::spawn(move || {
                while !done.load(Ordering::SeqCst) {
                    if let Some(bytes) = (sources.walk)(&paths, WALK_BOUND)
                        && let Ok(mut held) = reading.lock()
                    {
                        *held = Some(bytes);
                    }
                    wait(&done, sources.every);
                }
            });
        }
        let mut recorded = 0u64;
        let mut stopped: Option<Instant> = None;
        while !wait(done, self.sources.every) {
            match free_bytes(&self.sources) {
                Ok(free) if stopped.is_none() && free < self.settings.gate_stop => {
                    signal(self.group, Signal::SIGSTOP);
                    stopped = Some(Instant::now());
                    watched.paused += 1;
                    (self.sources.say)(
                        &self.state,
                        &format!(
                            "conductor: build run: paused: {}G free is under gate_stop {}G; \
                         reclaiming",
                            free / GIB,
                            gib(self.settings.gate_stop)
                        ),
                    );
                    let need = self.settings.gate_resume;
                    reclaim(&self.sources, &self.state, &self.host, need);
                }
                Ok(free) if stopped.is_some() && free >= self.settings.gate_resume => {
                    signal(self.group, Signal::SIGCONT);
                    if let Some(since) = stopped.take() {
                        watched.paused_for += since.elapsed();
                    }
                    watched.resumed += 1;
                    (self.sources.say)(
                        &self.state,
                        &format!(
                            "conductor: build run: resumed: {}G free is over gate_resume {}G",
                            free / GIB,
                            gib(self.settings.gate_resume)
                        ),
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    (self.sources.say)(&self.state, &format!("conductor: build run: {error:#}"))
                }
            }
            let read = reading.lock().ok().and_then(|held| *held);
            watched.peak = watched.peak.max(read);
            // The growth so far, recorded as it passes each further gibibyte, so the ledger counts
            // only what this run has not written yet.
            let grown = watched
                .start
                .zip(watched.peak)
                .map_or(0, |(start, peak)| peak.saturating_sub(start));
            if grown >= recorded.saturating_add(GIB) {
                match host::record_peak_now(&self.host, &self.grant, grown, watched.paused) {
                    Ok(true) => recorded = grown,
                    Ok(false) => {}
                    Err(error) => {
                        (self.sources.say)(&self.state, &format!("conductor: build run: {error:#}"))
                    }
                }
            }
        }
        if let Some(since) = stopped {
            signal(self.group, Signal::SIGCONT);
            watched.paused_for += since.elapsed();
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

/// One reclaim for `need` bytes, shared by the host ([`shared_reclaim`]); a failing one is named
/// on standard error and frees nothing. The caller holds no ledger lock.
fn reclaim(sources: &Sources, state: &Path, host: &Path, need: u64) -> ReclaimReport {
    shared_reclaim(sources, state, host, need).unwrap_or_else(|error| {
        (sources.say)(state, &format!("conductor: build run: reclaim: {error:#}"));
        ReclaimReport {
            reclaimed_bytes: 0,
            steps: Vec::new(),
            left: Vec::new(),
            reached: false,
        }
    })
}

/// A reclaim run that ended and freed nothing ([`barren_reclaim`]).
struct Barren {
    /// Seconds since it started.
    age: u64,
    /// What it left, by path and size.
    left: Vec<(String, u64)>,
}

/// The last reclaim run of the host (`host.reclaim` in the host directory `host`, where
/// [`shared_reclaim`] writes it), when it started within [`RECLAIM_REUSE`], ended without failing
/// and freed nothing: a head that does not fit is answered at once rather than wait for the next
/// walk (`story:build-run-backfill`). Read without the reclaim lock: the record is replaced whole,
/// and a walk under way has not ended.
fn barren_reclaim(sources: &Sources, host: &Path) -> Option<Barren> {
    let text = fs::read_to_string(host.join(RECLAIM_RECORD)).ok()?;
    let last = serde_json::from_str::<Value>(&text).ok()?;
    let started = last["started"].as_i64()?;
    let ended = last["finished"].as_i64().is_some();
    if !ended || last["error"].is_string() || last["reclaimed"].as_u64()? != 0 {
        return None;
    }
    let now = (sources.clock)().unix_timestamp();
    let age = u64::try_from(now.saturating_sub(started)).ok()?;
    if age >= RECLAIM_REUSE.as_secs() {
        return None;
    }
    let left = last["left"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| Some((row[0].as_str()?.to_owned(), row[1].as_u64()?)))
        .collect();
    Some(Barren { age, left })
}

/// A reclaim for `need` bytes, one at a time on the host: waits while another run walks the disk
/// (`host.reclaim.lock` in the host directory `host`), then reuses the last run's result when it started within
/// [`RECLAIM_REUSE`] and ended: its failure, or what it left and nothing freed by this run, when
/// it did not reach what it was asked (it freed all it could) or was asked at least `need`.
/// Otherwise it records its start in `host.reclaim`, runs [`Sources::reclaim`] and records its
/// end and result, or its failure, there. A run that started and never ended died in its walk,
/// and is not reused.
///
/// # Errors
///
/// The lock or the record cannot be written, the reclaim fails, or the reused one failed.
fn shared_reclaim(
    sources: &Sources,
    state: &Path,
    host: &Path,
    need: u64,
) -> Result<ReclaimReport> {
    fs::create_dir_all(host).with_context(|| format!("create {}", host.display()))?;
    let path = host.join(RECLAIM_LOCK);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    lock.lock()
        .with_context(|| format!("lock {}", path.display()))?;
    let record = host.join(RECLAIM_RECORD);
    let now = (sources.clock)().unix_timestamp();
    if let Some(last) = fs::read_to_string(&record)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    {
        let started = last["started"].as_i64();
        let ended = last["finished"].as_i64().is_some();
        let age = started.and_then(|started| u64::try_from(now.saturating_sub(started)).ok());
        let recent = age.is_some_and(|age| age < RECLAIM_REUSE.as_secs());
        let asked = last["need"].as_u64().unwrap_or(0);
        let reached = last["reached"].as_bool().unwrap_or(false);
        if recent && ended {
            let age = age.unwrap_or(0);
            if let Some(error) = last["error"].as_str() {
                bail!(
                    "the reclaim run started {age}s ago failed, and the next runs once it is {}s \
                     old: {error}",
                    RECLAIM_REUSE.as_secs()
                );
            }
            if !reached || asked >= need {
                (sources.say)(
                    state,
                    &format!(
                        "conductor: build run: reclaim: the run started {age}s ago is reused; \
                         one reclaim walks the disk every {}s at most",
                        RECLAIM_REUSE.as_secs()
                    ),
                );
                let left = last["left"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|row| Some((row[0].as_str()?.to_owned(), row[1].as_u64()?)))
                    .collect();
                return Ok(ReclaimReport {
                    reclaimed_bytes: 0,
                    steps: Vec::new(),
                    left,
                    reached,
                });
            }
        }
    }
    let write = |value: &Value| -> Result<()> {
        let next = record.with_extension("next");
        fs::write(&next, value.to_string()).with_context(|| format!("write {}", next.display()))?;
        fs::rename(&next, &record).with_context(|| format!("replace {}", record.display()))
    };
    write(&json!({"started": now, "finished": null, "need": need}))?;
    let ran = (sources.reclaim)(state, need);
    let finished = (sources.clock)().unix_timestamp();
    match &ran {
        Ok(report) => write(&json!({
            "started": now,
            "finished": finished,
            "need": need,
            "reached": report.reached,
            "reclaimed": report.reclaimed_bytes,
            "left": report
                .left
                .iter()
                .map(|(path, bytes)| json!([path, bytes]))
                .collect::<Vec<_>>(),
        }))?,
        Err(error) => write(&json!({
            "started": now,
            "finished": finished,
            "need": need,
            "error": format!("{error:#}"),
        }))?,
    }
    drop(lock);
    ran
}

/// What a run writes: its `target/` when its working directory is a Cargo project or holds one,
/// else the working directory itself; each `--measure` directory; and `CARGO_TARGET_DIR` when the
/// environment of the command sets it. A relative path is relative to the working directory.
fn measured(request: &Request, sources: &Sources) -> Vec<PathBuf> {
    let cwd = &request.cwd;
    let target = cwd.join("target");
    let own = if target.exists() || cwd.join("Cargo.toml").exists() {
        target
    } else {
        cwd.clone()
    };
    let mut paths = vec![own];
    let named = request
        .measure
        .iter()
        .cloned()
        .chain((sources.env)("CARGO_TARGET_DIR").map(PathBuf::from))
        .filter(|path| !path.as_os_str().is_empty());
    for path in named {
        let path = cwd.join(path);
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

/// The bytes under `paths`, each inode once, or none when the walk takes longer than `bound`. A
/// path that does not exist holds nothing.
#[must_use]
pub fn walk(paths: &[PathBuf], bound: Duration) -> Option<u64> {
    let there: Vec<PathBuf> = paths.iter().filter(|path| path.exists()).cloned().collect();
    if there.is_empty() {
        return Some(0);
    }
    crate::disk::tree_size(&there, Some(Instant::now() + bound))
        .ok()
        .map(|size| size.allocated)
}
