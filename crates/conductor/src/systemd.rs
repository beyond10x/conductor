//! The service manager and the terminal server conductor drives on this host: the user units
//! of `conductor run`, of conductord's instance runtimes and host roles ([`Service`],
//! [`start_service`], [`stop_unit`], [`unit_state`]), their slices ([`slice_name`],
//! [`HOST_SLICE`]), and a session's tmux server ([`TMUX_CONF`], [`tmux_call`], [`type_text`]).
//!
//! Conductor confines no session itself: every placement is a substrate box
//! (`crate::placement`), and conductor's own `systemd` session provider is gone. What is left
//! here is no box: a unit name, a slice and its limits for the units conductor runs outside any
//! box, and the typing into a tmux server `conductor session attach` and `deliver` reach.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use conductor_model::config::{Limits, SessionName};

use crate::collect;
use crate::confine::{BOUND, exited};

/// The program that starts a unit, found on `PATH`.
pub const SYSTEMD_RUN: &str = "systemd-run";

/// The program that sets a slice's limits, stops a unit and reads its cgroup, found on `PATH`.
pub const SYSTEMCTL: &str = "systemctl";

/// The program that runs a session's server and types into it, found on `PATH`.
pub const TMUX: &str = "tmux";

/// The configuration of a session's tmux server, `<state>/sessions/<name>.tmux.conf`, which
/// creates the session: `tmux -D` runs the server as the unit's main process and takes no
/// command, so the session is made here, when the first client connects (`-D` runs the
/// configuration then), and its pane starts in `CONDUCTOR_CWD`, the request's working
/// directory, so the first client's own directory (the liveness check runs from
/// `<state>/sessions/`) is never the pane's. The server ends with its one session (`exit-empty`,
/// which `-D` turns off); panes run `/bin/sh`, not the operator's login shell; the prefix is
/// `ctrl-]`, so `ctrl-]` then `d` detaches and `ctrl-b` stays the harness's; no status line, so
/// the harness has the whole screen; the window is 160 by 50; the pane runs the harness command
/// `CONDUCTOR_HARNESS` and copies its output to the terminal log `CONDUCTOR_TERMINAL_LOG` (the old
/// variable `CONDUCTOR_TYPESCRIPT` when that is unset, for one release). The
/// session's name is `CONDUCTOR_SESSION`, which also marks every process of the session as one
/// (`story:session-attach-refuses-sessions`). Nothing here names a harness.
pub const TMUX_CONF: &str = r#"set -s exit-empty on
set -s escape-time 10
set -g default-shell /bin/sh
set -g status off
set -g prefix 'C-]'
unbind C-b
bind 'C-]' send-prefix
new-session -d -s "$CONDUCTOR_SESSION" -c "$CONDUCTOR_CWD" -x 160 -y 50 'exec /bin/sh -c "$CONDUCTOR_HARNESS"'
pipe-pane -o -t "=$CONDUCTOR_SESSION:" 'exec cat >> "${CONDUCTOR_TERMINAL_LOG:-$CONDUCTOR_TYPESCRIPT}"'
"#;

/// How long the TUI is given between the pasted prompt and Enter.
pub const PASTE_SETTLE: Duration = Duration::from_millis(500);

/// How long a unit of a session's name may take to be gone before a start or resume refuses.
pub const GONE_WAIT: Duration = Duration::from_secs(30);

/// The unit a session runs in: `conductor_<session name>.service`, every character but letters,
/// digits, `:`, `_`, `.` and `-` written `\xNN` as `systemd-escape` writes it. A `-` in a
/// service's name sets no cgroup hierarchy; the unit sits in the slice `--slice` names.
#[must_use]
pub fn unit_name(name: &SessionName) -> String {
    let mut unit = String::from("conductor_");
    for byte in name.0.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'.' | b'-') {
            unit.push(char::from(byte));
        } else {
            unit.push_str(&format!("\\x{byte:02x}"));
        }
    }
    unit.push_str(".service");
    unit
}

/// The slice of an instance: `conductor_<prefix>.slice`, or `conductor.slice` without one; every
/// character of the prefix but letters and digits, its `-` above all, written `\xNN` as
/// `systemd-escape` writes it. No name has a `-`, which systemd would read as nesting: each
/// instance's slice is a sibling of every other's.
#[must_use]
pub fn slice_name(prefix: Option<&str>) -> String {
    prefix.map_or_else(
        || "conductor.slice".to_owned(),
        |prefix| {
            let mut slice = String::from("conductor_");
            for byte in prefix.bytes() {
                if byte.is_ascii_alphanumeric() {
                    slice.push(char::from(byte));
                } else {
                    slice.push_str(&format!("\\x{byte:02x}"));
                }
            }
            slice.push_str(".slice");
            slice
        },
    )
}

/// The slice of the host roles' sessions, the supervisor's and the teacher's
/// (`story:host-roles-by-daemon`, decision H2): a sibling of every instance's slice.
pub const HOST_SLICE: &str = "conductor_host.slice";

/// The slice a session of the role `role` runs in: [`HOST_SLICE`] for a host role
/// ([`crate::daemon::host_roles::is_host_role`]), else its instance's ([`slice_name`] of
/// `prefix`).
#[must_use]
pub fn session_slice(role: &str, prefix: Option<&str>) -> String {
    if crate::daemon::host_roles::is_host_role(role) {
        HOST_SLICE.to_owned()
    } else {
        slice_name(prefix)
    }
}

/// Runs `program <args>` within [`BOUND`] and fails naming how it ended when it did not succeed;
/// answers its standard output.
fn call(program: &str, args: &[String]) -> Result<String> {
    let shown = format!("{program} {}", args.join(" "));
    let output = collect::run(Command::new(program).args(args), BOUND)
        .with_context(|| format!("run `{shown}`"))?;
    if !output.status.success() {
        bail!("`{shown}` {}", exited(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Sets the instance's limits on its slice.
fn set_slice(slice: &str, limits: &Limits) -> Result<()> {
    let mut args = ["--user", "set-property", "--runtime", slice]
        .map(str::to_owned)
        .to_vec();
    args.extend([
        format!(
            "MemoryMax={}",
            limits
                .memory_max
                .as_ref()
                .map_or_else(|| "infinity".to_owned(), |size| format!("{}G", size.0))
        ),
        format!(
            "CPUWeight={}",
            limits
                .cpu_weight
                .map_or_else(String::new, |weight| weight.to_string())
        ),
        format!(
            "TasksMax={}",
            limits
                .tasks_max
                .map_or_else(|| "infinity".to_owned(), |tasks| tasks.to_string())
        ),
    ]);
    call(SYSTEMCTL, &args).with_context(|| format!("set the limits of the slice {slice}"))?;
    Ok(())
}

/// What systemd says of `unit`: its `LoadState` and its `ActiveState`.
fn read_unit_state(unit: &str) -> Result<(String, String)> {
    let text = call(
        SYSTEMCTL,
        &[
            "--user",
            "show",
            "-p",
            "LoadState",
            "-p",
            "ActiveState",
            unit,
        ]
        .map(str::to_owned),
    )
    .with_context(|| format!("read the state of the unit {unit}"))?;
    let value = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .unwrap_or_default()
            .trim()
            .to_owned()
    };
    Ok((value("LoadState"), value("ActiveState")))
}

/// Waits until systemd has no unit `unit` loaded, within [`GONE_WAIT`]: one that is stopping
/// is waited for, one that is inactive or failed is reset (`systemctl --user reset-failed`)
/// so systemd drops it. Fails at once on a unit that runs, and at the bound on one that is
/// still there.
fn wait_gone(unit: &str) -> Result<()> {
    let started = Instant::now();
    loop {
        let (load, active) = read_unit_state(unit)?;
        if load == "not-found" || load.is_empty() {
            return Ok(());
        }
        match active.as_str() {
            "active" | "activating" | "reloading" | "refreshing" => bail!(
                "the unit {unit} still runs (ActiveState={active}): stop it before a session \
                 starts under its name"
            ),
            "inactive" | "failed" => {
                call(
                    SYSTEMCTL,
                    &["--user", "reset-failed", unit].map(str::to_owned),
                )
                .with_context(|| format!("reset the unit {unit}"))?;
            }
            _ => {}
        }
        if started.elapsed() >= GONE_WAIT {
            bail!(
                "the unit {unit} is still there (LoadState={load}, ActiveState={active}) \
                 after {} s",
                GONE_WAIT.as_secs()
            );
        }
        thread::sleep(Duration::from_millis(200));
    }
}

/// A user service that is no session: `conductor run` as `conductor_<instance>-run.service`
/// (`story:run-daemon`), started through the same `systemd-run --user` path a session's unit is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// The unit's name.
    pub unit: String,
    /// The slice it runs in, the instance's ([`slice_name`]).
    pub slice: String,
    /// The instance's `confinement.limits`, set on the slice first as a session's start does.
    pub limits: Limits,
    /// Its working directory.
    pub cwd: PathBuf,
    /// `-p` properties beside `StandardOutput` (systemd's journal keeps what it prints).
    pub properties: Vec<String>,
    /// Variables set for it, `NAME=value`, or `NAME` alone to pass this process's value.
    pub env: Vec<String>,
    /// The program and its arguments.
    pub command: Vec<String>,
}

/// Starts `service` once no unit of its name is left ([`wait_gone`]), in its slice with
/// the instance's limits set on it: `systemd-run --user --unit <unit> --slice <slice> --collect -p
/// <property>… --working-directory=<cwd> -E <variable>… <command>`.
///
/// # Errors
///
/// A unit of that name still runs, the slice's limits cannot be set, or `systemd-run` fails.
pub fn start_service(service: &Service) -> Result<()> {
    wait_gone(&service.unit)?;
    set_slice(&service.slice, &service.limits)?;
    let mut args: Vec<String> = [
        "--user",
        "--unit",
        &service.unit,
        "--slice",
        &service.slice,
        "--collect",
    ]
    .map(str::to_owned)
    .to_vec();
    for property in &service.properties {
        args.extend(["-p".to_owned(), property.clone()]);
    }
    args.push(format!("--working-directory={}", service.cwd.display()));
    for variable in &service.env {
        args.extend(["-E".to_owned(), variable.clone()]);
    }
    args.extend(service.command.iter().cloned());
    call(SYSTEMD_RUN, &args).with_context(|| format!("start the unit {}", service.unit))?;
    Ok(())
}

/// What systemd says of `unit`: its `LoadState` and its `ActiveState` (`systemctl --user show`).
///
/// # Errors
///
/// `systemctl` fails.
pub fn unit_state(unit: &str) -> Result<(String, String)> {
    read_unit_state(unit)
}

/// Stops `unit` (`systemctl --user stop`).
///
/// # Errors
///
/// `systemctl` fails.
pub fn stop_unit(unit: &str) -> Result<()> {
    call(SYSTEMCTL, &["--user", "stop", unit].map(str::to_owned))
        .with_context(|| format!("stop the unit {unit}"))?;
    Ok(())
}

/// Runs `tmux -S <name>.tmux <args>` on the session `name`'s server, from the directory of its
/// socket, so a long state path does not exceed a socket's path limit; within [`BOUND`].
pub fn tmux_call(sessions: &Path, name: &SessionName, args: &[&str]) -> Result<String> {
    tmux_on(sessions, &name.0, args)
}

/// Runs `tmux -S <socket>.tmux <args>` from `dir`, the directory of that socket; within
/// [`BOUND`]. [`tmux_call`] for a server whose socket is not named after its session.
pub fn tmux_on(dir: &Path, socket: &str, args: &[&str]) -> Result<String> {
    let socket = format!("{socket}.tmux");
    let shown = format!("{TMUX} -S {socket} {}", args.join(" "));
    let output = collect::run(
        Command::new(TMUX)
            .current_dir(dir)
            .args(["-S", &socket])
            .args(args)
            .env_remove("TMUX"),
        BOUND,
    )
    .with_context(|| format!("run `{shown}`"))?;
    if !output.status.success() {
        bail!("`{shown}` {}", exited(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Types `text` into the session `name`'s pane, then Enter: `text` as one paste
/// (`load-buffer` from `<sessions>/<name>.paste`, written for the call and removed after it,
/// then `paste-buffer -p`, bracketed when the harness asked for it), and, after
/// [`PASTE_SETTLE`], a carriage return pasted the same way unbracketed. Not `set-buffer` with
/// the text as an argument: tmux refuses a command over its 16 KB limit ("command too long"),
/// and a charter is longer. Not `send-keys`: tmux refuses it ("client is read-only") while a
/// `--watch` client is attached.
///
/// # Errors
///
/// The paste file cannot be written, or a tmux call fails.
pub fn type_text(sessions: &Path, name: &SessionName, text: &str) -> Result<()> {
    type_into(sessions, &name.0, name, text)
}

/// [`type_text`] into the pane of the tmux session `name` on the server whose socket is
/// `<dir>/<socket>.tmux`; the paste file is `<dir>/<socket>.paste`.
///
/// # Errors
///
/// The paste file cannot be written, or a tmux call fails.
pub fn type_into(dir: &Path, socket: &str, name: &SessionName, text: &str) -> Result<()> {
    let pane = target(name);
    let paste = dir.join(format!("{socket}.paste"));
    fs::write(&paste, text).with_context(|| format!("write {}", paste.display()))?;
    let loaded = tmux_on(
        dir,
        socket,
        &[
            "load-buffer",
            "-b",
            "conductor",
            &paste.display().to_string(),
        ],
    );
    let _ = fs::remove_file(&paste);
    loaded?;
    tmux_on(
        dir,
        socket,
        &["paste-buffer", "-d", "-p", "-b", "conductor", "-t", &pane],
    )?;
    thread::sleep(PASTE_SETTLE);
    tmux_on(dir, socket, &["set-buffer", "-b", "conductor", "--", "\r"])?;
    tmux_on(
        dir,
        socket,
        &["paste-buffer", "-d", "-b", "conductor", "-t", &pane],
    )?;
    Ok(())
}

/// The tmux target of the session `name`'s one pane: the session of exactly that name.
#[must_use]
pub fn target(name: &SessionName) -> String {
    format!("={}:", name.0)
}
