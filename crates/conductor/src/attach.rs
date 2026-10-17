//! `conductor session attach` and `conductor session list` (`story:session-attach-command`,
//! `story:session-attach-watch`, `story:session-attach-refuses-sessions`): a session the systemd
//! provider started runs its harness, whichever it is, in a tmux server of its own on
//! `<state>/sessions/<name>.tmux` ([`systemd::TMUX_CONF`]); attach runs `tmux attach` on it in
//! the operator's terminal, read-only under `--watch`. A session started before the tmux
//! launcher has only its typescript: `--watch` follows that ([`follow`]) until its unit ends or
//! the operator presses `ctrl-]` then `d`; a read-write attach to it is refused.
//!
//! Both commands read every instance of the config file, from any directory, or the one
//! `--instance` names, or only the `--state-dir` given. Nothing here names a harness.

use std::collections::BTreeSet;
use std::env;
use std::fs::{self, File};
use std::io::{self, IsTerminal as _, Read as _, Seek as _, SeekFrom, Write};
use std::os::fd::AsFd as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, bail};
use conductor_model::config::{Instance, SessionName};
use conductor_model::session::{
    AttachMode, AttachSession, AttachableSession, ListSessions, SessionAttachRefusal,
    SessionAttachRefused,
};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::termios::{self, SetArg, Termios};
use serde_json::json;

use crate::cli::{AttachArgs, ListSessionsArgs};
use crate::config::{self, Environment};
use crate::confine::systemd::{self, SYSTEMCTL, TMUX};

/// The variable the provider sets in every process of a session, its session's name
/// ([`systemd::TMUX_CONF`]); attach refuses to run where it is set.
pub const MARKER: &str = "CONDUCTOR_SESSION";

/// How much of a typescript [`follow`] replays before it follows: its last 64 KiB.
pub const REPLAY: u64 = 64 * 1024;

/// A session whose screen changed within this many seconds is shown busy.
const BUSY_SECONDS: i64 = 10;

/// The detach key's first byte, `ctrl-]`; `d` follows it.
const DETACH_PREFIX: u8 = 0x1d;

/// One instance's sessions directory, where its sessions' sockets and typescripts are.
struct Place {
    instance: Instance,
    sessions: PathBuf,
}

/// Answers a refusal of `session attach`: one line naming its error, and exit 1.
fn refused(answer: &SessionAttachRefused) -> Result<ExitCode> {
    let refusal = match answer.refusal {
        SessionAttachRefusal::InsideSession => "inside-session",
        SessionAttachRefusal::UnknownSession => "unknown-session",
        SessionAttachRefusal::NotAttachable => "not-attachable",
        SessionAttachRefusal::Ambiguous => "ambiguous",
    };
    bail!(
        "session attach: refused: {refusal}: session {:?}: {}",
        answer.name,
        answer.detail
    )
}

/// The places to look in: the `--state-dir` given alone, else every instance of the config
/// file, or the one `instance` names.
fn places(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    instance: Option<&str>,
) -> Result<Vec<Place>> {
    if let Some(dir) = state {
        return Ok(vec![Place {
            instance: config::active().instance.clone(),
            sessions: dir.join("sessions"),
        }]);
    }
    let environment = Environment::from_process()?;
    let loaded = config::load(config_flag, &environment)?;
    let names: Vec<String> = loaded
        .config
        .instances
        .iter()
        .map(|each| each.name.0.clone())
        .collect();
    let places: Vec<Place> = loaded
        .config
        .instances
        .into_iter()
        .filter(|each| instance.is_none_or(|name| each.name.0 == name))
        .map(|each| Place {
            sessions: PathBuf::from(&each.state).join("sessions"),
            instance: each,
        })
        .collect();
    if places.is_empty() {
        bail!(
            "no instance {:?}: the config file names {}",
            instance.unwrap_or_default(),
            names.join(", ")
        );
    }
    Ok(places)
}

/// The names of the sessions `sessions` holds a socket or a typescript for.
fn names(sessions: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    let entries = match fs::read_dir(sessions) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(names),
        Err(error) => return Err(error).with_context(|| format!("read {}", sessions.display())),
    };
    for entry in entries {
        let file = entry?.file_name().to_string_lossy().into_owned();
        if let Some(name) = file
            .strip_suffix(".tmux")
            .or_else(|| file.strip_suffix(".typescript"))
        {
            names.insert(name.to_owned());
        }
    }
    Ok(names)
}

/// Seconds since the epoch, now.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// Whether the unit of the session `name` runs: `systemctl --user show -p ActiveState --value`
/// answers `active`. No `systemctl` is no unit.
fn unit_active(name: &SessionName) -> bool {
    Command::new(SYSTEMCTL)
        .args(["--user", "show", "-p", "ActiveState", "--value"])
        .arg(systemd::unit_name(name))
        .output()
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "active")
}

/// The live session `name` of `place`, and how attach reaches it: its tmux server answers
/// (`Attach`), or its unit runs and keeps a typescript, from before the tmux launcher (`Watch`).
fn find(place: &Place, name: &str) -> Option<AttachableSession> {
    let session = SessionName(name.to_owned());
    let row = |mode, directory, idle_seconds| AttachableSession {
        instance: place.instance.name.0.clone(),
        name: name.to_owned(),
        mode,
        directory,
        idle_seconds,
    };
    if place.sessions.join(format!("{name}.tmux")).exists()
        && let Ok(text) = systemd::tmux_call(
            &place.sessions,
            &session,
            &[
                "display-message",
                "-p",
                "-t",
                &systemd::target(&session),
                "#{pane_current_path}\t#{window_activity}",
            ],
        )
    {
        let mut fields = text.trim_end().splitn(2, '\t');
        let directory = fields
            .next()
            .filter(|dir| !dir.is_empty())
            .map(str::to_owned);
        let idle = fields
            .next()
            .and_then(|at| at.parse::<i64>().ok())
            .map(|at| (now() - at).max(0));
        return Some(row(AttachMode::Attach, directory, idle));
    }
    let typescript = place.sessions.join(format!("{name}.typescript"));
    if typescript.exists() && unit_active(&session) {
        let idle = fs::metadata(&typescript)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
            .map(|at| (now() - i64::try_from(at.as_secs()).unwrap_or(0)).max(0));
        return Some(row(AttachMode::Watch, None, idle));
    }
    None
}

/// The live sessions of `places`, in the order of the places, each by name.
fn rows(places: &[Place]) -> Result<Vec<AttachableSession>> {
    let mut rows = Vec::new();
    for place in places {
        for name in names(&place.sessions)? {
            rows.extend(find(place, &name));
        }
    }
    Ok(rows)
}

/// The text line of one row: name, mode, repository (the last component of the harness's
/// directory, `-` when unknown), busy or idle, and the seconds since the screen last changed.
#[must_use]
pub fn line(row: &AttachableSession) -> String {
    let mode = match row.mode {
        AttachMode::Attach => "attach",
        AttachMode::Watch => "watch",
    };
    let repository = row
        .directory
        .as_deref()
        .and_then(|dir| Path::new(dir).file_name())
        .map_or_else(
            || "-".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
    let (state, idle) = row.idle_seconds.map_or(("-", "-".to_owned()), |idle| {
        (
            if idle < BUSY_SECONDS { "busy" } else { "idle" },
            format!("{idle}s"),
        )
    });
    format!("{} {mode} {repository} {state} {idle}", row.name)
}

/// `conductor session list`: one line per live session of every instance ([`line`]), under a
/// `<instance>:` line when there are several, or the rows as JSON.
///
/// # Errors
///
/// The config file does not load, `--instance` names no instance, a sessions directory does not
/// read, or `--format` is neither `text` nor `json`.
pub fn list(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &ListSessionsArgs,
) -> Result<ExitCode> {
    let input = ListSessions {
        instance: args.instance.clone(),
        format: args.format.clone(),
    };
    let places = places(config_flag, state, input.instance.as_deref()).context("session list")?;
    let rows = rows(&places).context("session list")?;
    let mut out = io::stdout().lock();
    match input.format.as_deref().unwrap_or("text") {
        "text" => {
            let mut current = None;
            for row in &rows {
                if places.len() > 1 && current != Some(&row.instance) {
                    writeln!(out, "{}:", row.instance)?;
                    current = Some(&row.instance);
                }
                writeln!(out, "{}", line(row))?;
            }
        }
        "json" => {
            let rows: Vec<_> = rows
                .iter()
                .map(|row| {
                    json!({
                        "instance": row.instance,
                        "name": row.name,
                        "mode": match row.mode {
                            AttachMode::Attach => "Attach",
                            AttachMode::Watch => "Watch",
                        },
                        "directory": row.directory,
                        "idle_seconds": row.idle_seconds,
                    })
                })
                .collect();
            writeln!(out, "{}", serde_json::Value::Array(rows))?;
        }
        other => bail!("session list: --format {other:?}: text or json"),
    }
    Ok(ExitCode::SUCCESS)
}

/// `conductor session attach`: refuses inside a session, finds the session by `--name` or
/// `--role` in every instance (or `--instance`'s), and attaches the terminal to its tmux, or
/// follows its typescript under `--watch` when it was started before the tmux launcher.
///
/// # Errors
///
/// The config file does not load, `--instance` names no instance, tmux does not start, the
/// typescript does not read, or a refusal.
pub fn attach(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &AttachArgs,
) -> Result<ExitCode> {
    let input = AttachSession {
        name: args.name.clone(),
        role: args.role.clone(),
        instance: args.instance.clone(),
        watch: args.watch,
    };
    let asked = input
        .name
        .clone()
        .or_else(|| input.role.clone())
        .unwrap_or_default();
    let refuse = |refusal, detail: String| {
        refused(&SessionAttachRefused {
            name: asked.clone(),
            refusal,
            detail,
        })
    };
    if env::var_os(MARKER).is_some_and(|value| !value.is_empty()) {
        return refuse(
            SessionAttachRefusal::InsideSession,
            format!(
                "{MARKER} is set: this runs inside a session, and only the operator's own \
                 terminal attaches"
            ),
        );
    }
    if input.name.is_none() && input.role.is_none() {
        return crate::not_implemented("session attach without --name or --role (the picker)");
    }
    let watch = input.watch.unwrap_or(false);
    let places = places(config_flag, state, input.instance.as_deref()).context("session attach")?;
    let mut found = Vec::new();
    for place in &places {
        let by_role = input
            .role
            .as_deref()
            .map(|role| config::session_name(&place.instance, role).0);
        let name = match (&input.name, &by_role) {
            (Some(name), Some(by_role)) if name != by_role => {
                return refuse(
                    SessionAttachRefusal::Ambiguous,
                    format!(
                        "--name {name} and --role {} (the session {by_role} of the instance {}) \
                         name different sessions",
                        input.role.as_deref().unwrap_or_default(),
                        place.instance.name.0
                    ),
                );
            }
            (Some(name), _) => name.clone(),
            (None, Some(by_role)) => by_role.clone(),
            (None, None) => unreachable!("a name or a role was checked above"),
        };
        if let Some(row) = find(place, &name) {
            found.push((place, row));
        }
    }
    let (place, row) = match found.len() {
        0 => {
            return refuse(
                SessionAttachRefusal::UnknownSession,
                format!(
                    "no live session of that name in {}; `conductor session list` lists them",
                    places
                        .iter()
                        .map(|place| place.instance.name.0.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
        1 => found.remove(0),
        _ => {
            return refuse(
                SessionAttachRefusal::Ambiguous,
                format!(
                    "the instances {} each hold a live session {}; choose one with --instance",
                    found
                        .iter()
                        .map(|(_, row)| row.instance.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    found[0].1.name
                ),
            );
        }
    };
    match (&row.mode, watch) {
        (AttachMode::Attach, _) => tmux_attach(&place.sessions, &row.name, watch),
        (AttachMode::Watch, true) => {
            let path = place.sessions.join(format!("{}.typescript", row.name));
            let mut file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
            let session = SessionName(row.name.clone());
            watch_typescript(&mut file, || unit_active(&session))
        }
        (AttachMode::Watch, false) => refuse(
            SessionAttachRefusal::NotAttachable,
            "it runs under the launcher before tmux; restart the session to attach to it \
             (`conductor session stop` then `start`, or the controller's restart); `--watch` \
             follows its typescript now"
                .to_owned(),
        ),
    }
}

/// Runs `tmux attach` on the session `name`'s server in this terminal, read-only and without
/// resizing the window under `watch`; answers tmux's exit. `TMUX` is removed, so the operator
/// may attach from inside a tmux of his own.
fn tmux_attach(sessions: &Path, name: &str, watch: bool) -> Result<ExitCode> {
    let session = SessionName(name.to_owned());
    eprintln!(
        "conductor: attaching to {name}{}; detach with ctrl-] then d",
        if watch { " read-only" } else { "" }
    );
    let mut command = Command::new(TMUX);
    command
        .current_dir(sessions)
        .args(["-S", &format!("{name}.tmux"), "attach-session"])
        .args(["-t", &format!("={}", session.0)])
        .env_remove("TMUX")
        .env_remove("TMUX_PANE");
    if watch {
        command.args(["-f", "read-only,ignore-size"]);
    }
    let status = command
        .status()
        .with_context(|| format!("session attach: run `{TMUX} attach-session`"))?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .map_or(ExitCode::FAILURE, ExitCode::from))
}

/// The terminal's mode before raw mode, restored when dropped, on every way out of the watch.
struct Raw {
    saved: Termios,
}

impl Raw {
    /// Puts standard input's terminal in raw mode.
    fn enter() -> Result<Self> {
        let stdin = io::stdin();
        let saved = termios::tcgetattr(stdin.as_fd()).context("read the terminal's mode")?;
        let mut raw = saved.clone();
        termios::cfmakeraw(&mut raw);
        termios::tcsetattr(stdin.as_fd(), SetArg::TCSANOW, &raw)
            .context("put the terminal in raw mode")?;
        Ok(Self { saved })
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        let _ = termios::tcsetattr(io::stdin().as_fd(), SetArg::TCSANOW, &self.saved);
    }
}

/// `--watch` on a session before the tmux launcher: [`follow`]s its typescript to standard
/// output. On a terminal it is in raw mode and `ctrl-]` then `d` detaches; nothing typed goes
/// anywhere. `SIGINT`, `SIGTERM` and `SIGHUP` end it the same way, the terminal restored.
fn watch_typescript(file: &mut File, alive: impl FnMut() -> bool) -> Result<ExitCode> {
    let stop = Arc::new(AtomicBool::new(false));
    for signal in [
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
    ] {
        signal_hook::flag::register(signal, Arc::clone(&stop)).context("watch for signals")?;
    }
    let terminal = io::stdin().is_terminal();
    let raw = if terminal { Some(Raw::enter()?) } else { None };
    if terminal {
        eprint!("conductor: following the typescript; detach with ctrl-] then d\r\n");
    }
    let mut keys = Detach::default();
    let detach = |wait: Duration| {
        if stop.load(Ordering::Relaxed) {
            return true;
        }
        if !terminal {
            std::thread::sleep(wait);
            return false;
        }
        let stdin = io::stdin();
        let mut fds = [PollFd::new(stdin.as_fd(), PollFlags::POLLIN)];
        let millis = u16::try_from(wait.as_millis()).unwrap_or(u16::MAX);
        if poll(&mut fds, PollTimeout::from(millis)).unwrap_or(0) == 0 {
            return false;
        }
        let mut buffer = [0_u8; 64];
        match stdin.lock().read(&mut buffer) {
            Ok(0) | Err(_) => true,
            Ok(read) => keys.feed(&buffer[..read]),
        }
    };
    let mut out = io::stdout().lock();
    let detached = follow(file, &mut out, alive, detach)?;
    drop(raw);
    if !detached {
        writeln!(out, "\r\nconductor: the session ended")?;
    }
    Ok(ExitCode::SUCCESS)
}

/// The detach key's state: `ctrl-]` seen, waiting for `d`.
#[derive(Debug, Default)]
pub struct Detach {
    prefix: bool,
}

impl Detach {
    /// Reads typed bytes; true once `ctrl-]` then `d` was typed. Every byte is dropped.
    pub fn feed(&mut self, bytes: &[u8]) -> bool {
        for &byte in bytes {
            if self.prefix && byte == b'd' {
                return true;
            }
            self.prefix = byte == DETACH_PREFIX;
        }
        false
    }
}

/// Writes the last [`REPLAY`] bytes of `file` to `out`, then what is appended to it, until
/// `detach` (asked with how long it may wait for a key) answers true, which answers true, or
/// `alive`, asked about once a second, answers false, which writes what is left and answers
/// false.
///
/// # Errors
///
/// `file` does not read or `out` does not take the bytes.
pub fn follow(
    file: &mut File,
    out: &mut dyn Write,
    mut alive: impl FnMut() -> bool,
    mut detach: impl FnMut(Duration) -> bool,
) -> Result<bool> {
    let length = file.metadata().context("read the typescript")?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(REPLAY)))
        .context("seek in the typescript")?;
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut checked = Instant::now();
    loop {
        let read = file.read(&mut buffer).context("read the typescript")?;
        if read > 0 {
            out.write_all(&buffer[..read])?;
            out.flush()?;
            continue;
        }
        if detach(Duration::from_millis(200)) {
            return Ok(true);
        }
        if checked.elapsed() >= Duration::from_secs(1) {
            checked = Instant::now();
            if !alive() {
                io::copy(file, out).context("read the typescript")?;
                out.flush()?;
                return Ok(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Detach;

    /// `ctrl-]` then `d` detaches, across reads too; `d` alone or after another key does not.
    #[test]
    fn ctrl_bracket_then_d_detaches() {
        let mut keys = Detach::default();
        assert!(!keys.feed(b"d"));
        assert!(!keys.feed(b"x\x1dxd"));
        assert!(keys.feed(b"ab\x1dd"));
        let mut keys = Detach::default();
        assert!(!keys.feed(b"\x1d"));
        assert!(keys.feed(b"d"));
    }
}
