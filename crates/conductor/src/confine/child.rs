//! The `child` provider (`story:box-session-without-bg`): a session that runs as a plain child
//! process, for a box that refuses `socket(AF_UNIX)`.
//!
//! `claude --bg` starts a background service that listens on a pathname Unix socket, and its
//! `stop`, `rm`, `attach` and `agents` talk to that service. A substrate box refuses
//! `socket(AF_UNIX)` (it allows a Unix socket only through `socketpair`), so no `--bg` session
//! starts in one. This provider needs no socket: it starts the harness headless,
//!
//! ```text
//! claude -p --input-format stream-json --output-format stream-json --verbose
//!   --replay-user-messages --session-id <id> <the arguments `--bg` gets>
//! ```
//!
//! with the arguments [`super::background_args`] gives a `--bg` launch, in its own process group,
//! its standard input a FIFO it holds open for reading and writing itself (so it never reads an
//! end of input), its standard output and error files. `<id>` is a UUID conductor chooses, the
//! session's id and the name of its transcript under `CLAUDE_CONFIG_DIR`, as a `--bg` session's
//! is. Every file of a session is in the registry ([`registry`], `conductor-sessions` in Claude
//! Code's state directory, the one `CLAUDE_CONFIG_DIR` names), named by the session's id:
//!
//! | file | holds |
//! |---|---|
//! | `<id>.json` | the entry: name, id, working directory, pid, start time, exit code once ended |
//! | `<id>.in` | the FIFO the session reads its messages from, one stream-json line each |
//! | `<id>.out`, `<id>.err` | what the harness writes; `.out` is its stream-json output |
//! | `<id>.lock` | the lock a delivery holds while it writes, so two messages never interleave |
//!
//! The first prompt and every later delivery are one line `{"type":"user","message":{"role":
//! "user","content":<text>}}` written to the FIFO ([`feed`]). A session is live while its pid
//! is a process that is not a zombie and whose command line names its id ([`alive`]); the
//! process that started it reaps it while it runs and records its exit code. [`append`] adds
//! the registry's sessions to the Claude session list (`claude agents --json`) in the list's own
//! shape, so every reader of that list (the sessions collector, the watch, spawn, the run loop,
//! the guard, the dashboard) sees them; `status` is `busy` from a message replayed to the
//! output until the turn's `result`, `idle` after it. A stop ([`stop`]) sends `SIGTERM` to the
//! session's process group, `SIGKILL` after [`STOP_WAIT`](super::STOP_WAIT), and removes its
//! files; a resume starts `claude -p --resume <id>` the same way, with no prompt.
//!
//! The provider is chosen by `confinement.provider: child`, and in place of `none` wherever this
//! process may not create a Unix socket ([`unix_sockets_refused`]), as in a substrate box.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{ErrorKind, Read as _, Seek as _, SeekFrom, Write as _};
use std::os::fd::AsFd as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::session::{EnvelopeMeasurement, SessionRequest, StartedSession};
use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg, OFlag};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::signal::{Signal, kill, killpg};
use nix::sys::stat::Mode;
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{BOUND, Launch, Provider, ProviderAvailability, STOP_WAIT, Said, Unboxed};

/// The provider's name, as `confinement.provider` and a session's record write it.
pub const NAME: &str = "child";

/// The registry's directory in Claude Code's state directory.
pub const REGISTRY: &str = "conductor-sessions";

/// The reason the `child` provider's record gives: it measures no box of its own.
pub const CHILD_UNBOXED: &str = "confinement.provider is child, which runs the session as a child \
                                 process in the box of the process that starts it and measures \
                                 no box of its own";

/// How long a resumed session must keep running to count as started: a resume of an id the
/// harness does not know ends at once.
const RESUME_GRACE: Duration = Duration::from_millis(1500);

/// The most of a session's output read from its end to find its state.
const TAIL: u64 = 256 * 1024;

/// The registry for the home directory `home`: [`REGISTRY`] in the directory `CLAUDE_CONFIG_DIR`
/// names when it is set, else in `<home>/.claude`.
#[must_use]
pub fn registry(home: &Path) -> PathBuf {
    std::env::var_os(crate::placement::CLAUDE_CONFIG_VARIABLE)
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .unwrap_or_else(|| home.join(".claude"))
        .join(REGISTRY)
}

/// The registry of this process's home directory (`HOME`), `None` without an absolute one.
#[must_use]
pub fn registry_here() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .map(|home| registry(&home))
}

/// Whether this process may not create a Unix socket: `socket(AF_UNIX)` is refused as a
/// permission (`EACCES`, `EPERM`) or an unsupported family (`EAFNOSUPPORT`), as a substrate box
/// and a unit with `RestrictAddressFamilies` refuse it. Asked once per process; the socket made to
/// ask is closed at once and never bound.
#[must_use]
pub fn unix_sockets_refused() -> bool {
    static REFUSED: OnceLock<bool> = OnceLock::new();
    *REFUSED.get_or_init(|| match std::os::unix::net::UnixDatagram::unbound() {
        Ok(_) => false,
        Err(error) => {
            error.kind() == ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(Errno::EAFNOSUPPORT as i32)
                || error.raw_os_error() == Some(Errno::EPERM as i32)
        }
    })
}

/// One session's entry in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The session's name, `-n`.
    pub name: String,
    /// Its id, `--session-id`.
    pub session_id: String,
    /// The directory it runs in.
    pub cwd: String,
    /// The pid of its harness process, which leads its process group.
    pub pid: Option<u32>,
    /// When it was started, in milliseconds since the epoch.
    pub started_at: u64,
    /// How it ended, once the process that started it saw it end.
    pub exit_code: Option<i32>,
}

impl Entry {
    /// Its pid while it lives ([`alive`]).
    #[must_use]
    pub fn live_pid(&self) -> Option<u32> {
        self.pid.filter(|pid| alive(*pid, &self.session_id))
    }
}

/// The files of the session `id` in the registry `dir`.
struct Files {
    entry: PathBuf,
    input: PathBuf,
    output: PathBuf,
    errors: PathBuf,
    lock: PathBuf,
}

impl Files {
    fn of(dir: &Path, id: &str) -> Self {
        Self {
            entry: dir.join(format!("{id}.json")),
            input: dir.join(format!("{id}.in")),
            output: dir.join(format!("{id}.out")),
            errors: dir.join(format!("{id}.err")),
            lock: dir.join(format!("{id}.lock")),
        }
    }
}

/// Whether `pid` is a live process of the session `id`: it exists, is no zombie, and, where
/// `/proc` shows it, its command line names `id`, so a pid the system gave another process since
/// does not count.
#[must_use]
pub fn alive(pid: u32, id: &str) -> bool {
    let Ok(raw) = i32::try_from(pid) else {
        return false;
    };
    match kill(Pid::from_raw(raw), None) {
        Ok(()) | Err(Errno::EPERM) => {}
        Err(_) => return false,
    }
    let proc = PathBuf::from(format!("/proc/{pid}"));
    if let Ok(stat) = fs::read_to_string(proc.join("stat"))
        && let Some(state) = stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().next())
        && state == "Z"
    {
        return false;
    }
    match fs::read(proc.join("cmdline")) {
        Ok(cmdline) => cmdline
            .split(|byte| *byte == 0)
            .any(|word| word == id.as_bytes()),
        Err(_) => true,
    }
}

/// Every entry of the registry `dir`, oldest first; an entry that does not read is left out.
#[must_use]
pub fn entries(dir: &Path) -> Vec<Entry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("json")))
        .filter_map(|path| fs::read(&path).ok())
        .filter_map(|text| serde_json::from_slice(&text).ok())
        .collect();
    entries.sort_by(|a, b| (a.started_at, &a.session_id).cmp(&(b.started_at, &b.session_id)));
    entries
}

/// The live entry named `name` in the registry `dir`, if any.
#[must_use]
pub fn live_named(dir: &Path, name: &str) -> Option<Entry> {
    entries(dir)
        .into_iter()
        .rev()
        .find(|entry| entry.name == name && entry.live_pid().is_some())
}

/// `entry` as the Claude session list prints one: `id` and `sessionId` its id, `name`, `cwd`,
/// `pid` while it lives, `status` from its output ([`status`]), `startedAt`, and `kind`
/// [`NAME`].
fn listed(dir: &Path, entry: &Entry) -> Value {
    let pid = entry.live_pid();
    json!({
        "id": entry.session_id,
        "sessionId": entry.session_id,
        "name": entry.name,
        "cwd": entry.cwd,
        "pid": pid,
        "status": if pid.is_some() { status(&Files::of(dir, &entry.session_id).output) } else { "exited" },
        "startedAt": entry.started_at,
        "kind": NAME,
    })
}

/// `busy` while the last message the session took has no `result` after it in its output, else
/// `idle`.
fn status(output: &Path) -> &'static str {
    let Ok(mut file) = File::open(output) else {
        return "idle";
    };
    let length = file.metadata().map_or(0, |meta| meta.len());
    if file
        .seek(SeekFrom::Start(length.saturating_sub(TAIL)))
        .is_err()
    {
        return "idle";
    }
    let mut tail = Vec::new();
    if file.read_to_end(&mut tail).is_err() {
        return "idle";
    }
    String::from_utf8_lossy(&tail)
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|line| match line["type"].as_str() {
            Some("result") => Some("idle"),
            Some("user") => Some("busy"),
            _ => None,
        })
        .unwrap_or("idle")
}

/// Appends the sessions of the registry under `home` to `entries`, a Claude session list that
/// `program` printed, when `program` is the harness, `claude` ([`super::CLAUDE`]); a list another
/// program printed is left as it is.
pub fn append_under(home: &Path, program: &OsStr, entries: &mut Vec<Value>) {
    if Path::new(program).file_name() != Some(OsStr::new(super::CLAUDE)) {
        return;
    }
    let dir = registry(home);
    entries.extend(self::entries(&dir).iter().map(|entry| listed(&dir, entry)));
}

/// [`append_under`] the home directory `HOME` names.
pub fn append(program: &OsStr, entries: &mut Vec<Value>) {
    if let Some(home) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
    {
        append_under(&home, program, entries);
    }
}

/// What [`feed`] did with a text for a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fed {
    /// No live session of that name runs as a child.
    NoSession,
    /// The whole message is in the session's input.
    Written,
    /// Nothing reached the session, and why.
    Unwritten(String),
    /// Part of the message reached the session, and why the rest did not.
    Partial(String),
}

/// One stream-json input line: a user message of `text`.
#[must_use]
pub fn message(text: &str) -> Vec<u8> {
    let mut line = json!({"type": "user", "message": {"role": "user", "content": text}})
        .to_string()
        .into_bytes();
    line.push(b'\n');
    line
}

/// Writes `text` as one message ([`message`]) into the input of the live child session `name`,
/// within `bound`.
#[must_use]
pub fn feed(name: &str, text: &str, bound: Duration) -> Fed {
    let Some(dir) = registry_here() else {
        return Fed::NoSession;
    };
    let Some(entry) = live_named(&dir, name) else {
        return Fed::NoSession;
    };
    write_message(&Files::of(&dir, &entry.session_id), &message(text), bound)
}

/// Writes `line` into the FIFO of `files` within `bound`, holding the session's lock.
fn write_message(files: &Files, line: &[u8], bound: Duration) -> Fed {
    let started = Instant::now();
    // Non-blocking: with no reader the open fails at once (`ENXIO`), and a full pipe is waited
    // for within the bound, not forever.
    let fifo = match File::options()
        .write(true)
        .custom_flags(OFlag::O_NONBLOCK.bits())
        .open(&files.input)
    {
        Ok(fifo) => fifo,
        Err(error) => {
            return Fed::Unwritten(format!(
                "its input {} does not open: {error}",
                files.input.display()
            ));
        }
    };
    let lock = match File::options()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&files.lock)
        .map_err(|error| error.to_string())
        .and_then(|file| {
            Flock::lock(file, FlockArg::LockExclusive).map_err(|(_, errno)| errno.to_string())
        }) {
        Ok(lock) => lock,
        Err(error) => {
            return Fed::Unwritten(format!(
                "its lock {} is not taken: {error}",
                files.lock.display()
            ));
        }
    };
    let mut written = 0;
    let mut fifo = fifo;
    while written < line.len() {
        match fifo.write(&line[written..]) {
            Ok(0) => break,
            Ok(count) => written += count,
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                let left = bound.saturating_sub(started.elapsed());
                if left.is_zero() {
                    break;
                }
                let wait = PollTimeout::try_from(left.min(Duration::from_millis(500)))
                    .unwrap_or(PollTimeout::MAX);
                let _ = poll(&mut [PollFd::new(fifo.as_fd(), PollFlags::POLLOUT)], wait);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => {
                drop(lock);
                return unfinished(written, line.len(), &error.to_string());
            }
        }
    }
    drop(lock);
    if written == line.len() {
        Fed::Written
    } else {
        unfinished(
            written,
            line.len(),
            &format!("the session read no more within {} s", bound.as_secs()),
        )
    }
}

/// A message `written` of whose `total` bytes reached the session, for `why`.
fn unfinished(written: usize, total: usize, why: &str) -> Fed {
    if written == 0 {
        Fed::Unwritten(why.to_owned())
    } else {
        Fed::Partial(format!("{written} of {total} bytes reached it: {why}"))
    }
}

/// Starts the session `id` for `launch` as a child process ([`NAME`]): `--session-id <id>`, or
/// `--resume <id>` when `resume`; writes its entry; and, given a `prompt`, feeds it and waits
/// within [`BOUND`] for the harness to say it started (its `init` line). A resumed session counts
/// as started when it still runs after [`RESUME_GRACE`].
fn run(launch: &Launch, id: &str, resume: bool, prompt: Option<&str>) -> Result<()> {
    let dir = registry_here().context("HOME is not an absolute path")?;
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    let files = Files::of(&dir, id);
    match fs::remove_file(&files.input) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("remove {}", files.input.display()));
        }
    }
    nix::unistd::mkfifo(&files.input, Mode::from_bits_truncate(0o600))
        .with_context(|| format!("make the FIFO {}", files.input.display()))?;
    // Read and write: the open does not wait for a writer, and the session holds a writer
    // itself, so it reads no end of input when a delivery closes its own.
    let input = File::options()
        .read(true)
        .write(true)
        .open(&files.input)
        .with_context(|| format!("open {}", files.input.display()))?;
    let output = File::options()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&files.output)
        .with_context(|| format!("open {}", files.output.display()))?;
    let errors = File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&files.errors)
        .with_context(|| format!("open {}", files.errors.display()))?;
    let mut prefix = vec![
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--replay-user-messages",
    ];
    prefix.extend(if resume {
        ["--resume", id]
    } else {
        ["--session-id", id]
    });
    let args = super::background_args(&prefix, &launch.args);
    let mut command = Unboxed::command(launch, &args);
    command
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(errors))
        .process_group(0);
    let mut child = command
        .spawn()
        .with_context(|| format!("start `{}`", launch.program))?;
    drop(command);
    let pid = child.id();
    let entry = Entry {
        name: launch.request.session_name.0.clone(),
        session_id: id.to_owned(),
        cwd: launch.request.cwd.clone(),
        pid: Some(pid),
        started_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            }),
        exit_code: None,
    };
    if let Err(error) = write_entry(&files.entry, &entry) {
        let _ = killpg(
            Pid::from_raw(i32::try_from(pid).unwrap_or(0)),
            Signal::SIGKILL,
        );
        let _ = child.wait();
        return Err(error);
    }
    // While this process runs it reaps the session and records how it ended; a session that
    // outlives it is reaped by whoever adopts it, and read live or not by its pid.
    let reaped = files.entry.clone();
    let reaper = id.to_owned();
    thread::spawn(move || {
        if let Ok(status) = child.wait() {
            let _ = ended(&reaped, &reaper, pid, status.code());
        }
    });
    let started = Instant::now();
    if let Some(prompt) = prompt {
        match write_message(&files, &message(prompt), BOUND) {
            Fed::Written => {}
            Fed::NoSession => bail!("the session took no prompt"),
            Fed::Unwritten(why) | Fed::Partial(why) => {
                bail!(
                    "the prompt did not reach the session: {why}{}",
                    said(&files)
                )
            }
        }
    }
    loop {
        if !alive(pid, id) {
            bail!("the session ended as it started{}", said(&files));
        }
        if prompt.is_none() && started.elapsed() >= RESUME_GRACE {
            return Ok(());
        }
        if prompt.is_some() && initialised(&files.output, id) {
            return Ok(());
        }
        if started.elapsed() >= BOUND {
            bail!(
                "the session did not say it started within {} s; it runs as pid {pid}",
                BOUND.as_secs()
            );
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Whether the output `output` holds the harness's `init` line for the session `id`.
fn initialised(output: &Path, id: &str) -> bool {
    fs::read_to_string(output).is_ok_and(|text| {
        text.lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|line| {
                line["type"] == "system"
                    && line["subtype"] == "init"
                    && line["session_id"].as_str() == Some(id)
            })
    })
}

/// What the session wrote to standard error, on one line after `: `, or nothing.
fn said(files: &Files) -> String {
    let text = fs::read_to_string(&files.errors).unwrap_or_default();
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        String::new()
    } else {
        format!(": {}", lines.join("; "))
    }
}

/// Writes `entry` to `file`, through a file beside it renamed into place.
fn write_entry(file: &Path, entry: &Entry) -> Result<()> {
    let written = file.with_extension(format!("json.{}", std::process::id()));
    let mut text = serde_json::to_vec_pretty(entry).context("render the session's entry")?;
    text.push(b'\n');
    fs::write(&written, text).with_context(|| format!("write {}", written.display()))?;
    fs::rename(&written, file).with_context(|| format!("write {}", file.display()))
}

/// Records in `file` that the session `id`, run as `pid`, ended with `code`; an entry that is
/// gone, or names another pid by now, is left as it is.
fn ended(file: &Path, id: &str, pid: u32, code: Option<i32>) -> Result<()> {
    let text = fs::read(file).with_context(|| format!("read {}", file.display()))?;
    let mut entry: Entry = serde_json::from_slice(&text)
        .with_context(|| format!("{} is no session entry", file.display()))?;
    if entry.session_id != id || entry.pid != Some(pid) {
        return Ok(());
    }
    entry.pid = None;
    entry.exit_code = code;
    write_entry(file, &entry)
}

/// Stops the child session `id`: `SIGTERM` to its process group, `SIGKILL` after
/// [`STOP_WAIT`], then removes its files. A session that is not live has its files removed.
///
/// # Errors
///
/// The registry is not found, or the session still lives after `SIGKILL`.
pub fn stop(id: &str) -> Result<()> {
    let dir = registry_here().context("HOME is not an absolute path")?;
    let files = Files::of(&dir, id);
    let entry: Option<Entry> = fs::read(&files.entry)
        .ok()
        .and_then(|text| serde_json::from_slice(&text).ok());
    if let Some(pid) = entry.as_ref().and_then(Entry::live_pid) {
        let group = Pid::from_raw(i32::try_from(pid).context("the pid is out of range")?);
        for (signal, wait) in [
            (Signal::SIGTERM, STOP_WAIT),
            (Signal::SIGKILL, Duration::from_secs(5)),
        ] {
            if killpg(group, signal).is_err() {
                let _ = kill(group, signal);
            }
            let started = Instant::now();
            while alive(pid, id) && started.elapsed() < wait {
                thread::sleep(Duration::from_millis(50));
            }
            if !alive(pid, id) {
                break;
            }
        }
        if alive(pid, id) {
            bail!("the child session {id} (pid {pid}) still runs after SIGKILL");
        }
    }
    for file in [
        &files.entry,
        &files.input,
        &files.output,
        &files.errors,
        &files.lock,
    ] {
        match fs::remove_file(file) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("remove {}", file.display()));
            }
        }
    }
    Ok(())
}

/// The `child` provider; see the module documentation.
#[derive(Debug, Clone, Default)]
pub struct Child {
    /// The launch [`Provider::start`] kept for [`Provider::prompt`].
    launch: Option<Launch>,
}

impl Provider for Child {
    fn name(&self) -> String {
        NAME.to_owned()
    }

    /// Always: a pipe, a FIFO and a child process need no socket and no service.
    fn available(&mut self) -> ProviderAvailability {
        ProviderAvailability {
            provider: self.name(),
            available: true,
            reason: None,
        }
    }

    fn start(&mut self, launch: &Launch) -> Result<StartedSession> {
        self.launch = Some(launch.clone());
        Ok(StartedSession {
            role: launch.request.role.clone(),
            session_name: launch.request.session_name.clone(),
            provider: self.name(),
            unit: None,
            session_id: Some(eventlog_core::new_event_id()),
        })
    }

    fn prompt(&mut self, session: &mut StartedSession, prompt: &str) -> Result<Said> {
        let launch = self
            .launch
            .as_ref()
            .ok_or_else(|| anyhow!("no session was started to prompt"))?;
        let id = session
            .session_id
            .clone()
            .ok_or_else(|| anyhow!("the session {} has no id", session.session_name.0))?;
        run(launch, &id, false, Some(prompt))?;
        Ok(Said::default())
    }

    fn stop(&mut self, session: &StartedSession, _request: &SessionRequest) -> Result<()> {
        let id = session
            .session_id
            .as_deref()
            .ok_or_else(|| anyhow!("the session {} has no id", session.session_name.0))?;
        stop(id)
    }

    fn resume(&mut self, session: &StartedSession, launch: &Launch) -> Result<StartedSession> {
        let id = session
            .session_id
            .as_deref()
            .ok_or_else(|| anyhow!("the session {} has no id", session.session_name.0))?;
        run(launch, id, true, None)?;
        Ok(StartedSession {
            provider: self.name(),
            unit: None,
            ..session.clone()
        })
    }

    fn measure(
        &mut self,
        session: &StartedSession,
        request: &SessionRequest,
    ) -> Result<EnvelopeMeasurement> {
        Ok(super::unboxed(session, request, CHILD_UNBOXED))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_one_stream_json_line_of_the_text() {
        let line = message("line one\nline \"two\"");
        assert_eq!(line.last(), Some(&b'\n'));
        assert_eq!(line.iter().filter(|byte| **byte == b'\n').count(), 1);
        let value: Value = serde_json::from_slice(&line).expect("JSON");
        assert_eq!(
            value,
            json!({"type": "user", "message": {"role": "user", "content": "line one\nline \"two\""}})
        );
    }

    #[test]
    fn the_status_is_busy_from_a_taken_message_to_its_result() {
        let dir = std::env::temp_dir().join(format!("child-status-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a directory");
        let output = dir.join("out");
        fs::write(&output, "").expect("write");
        assert_eq!(status(&output), "idle");
        fs::write(
            &output,
            "{\"type\":\"system\",\"subtype\":\"init\"}\n{\"type\":\"user\"}\n{\"type\":\"assistant\"}\n",
        )
        .expect("write");
        assert_eq!(status(&output), "busy");
        fs::write(
            &output,
            "{\"type\":\"user\"}\n{\"type\":\"result\"}\n{\"type\":\"system\",\"subtype\":\"post_turn_summary\"}\n",
        )
        .expect("write");
        assert_eq!(status(&output), "idle");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pid_that_is_no_process_of_the_session_is_not_alive() {
        // This process lives, and its command line names no such id.
        assert!(!alive(std::process::id(), "no-such-session-id"));
        assert!(!alive(u32::MAX, "x"));
    }

    #[test]
    fn a_list_another_program_printed_gets_no_child_session() {
        let mut entries = vec![json!({"id": "a"})];
        append_under(
            Path::new("/nonexistent"),
            OsStr::new("printf"),
            &mut entries,
        );
        assert_eq!(entries, vec![json!({"id": "a"})]);
    }
}
