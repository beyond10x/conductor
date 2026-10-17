//! The `systemd` provider (`story:session-limits-systemd`): each session in a `systemd-run
//! --user` unit inside its instance's slice, with the way the spike found working
//! (`docs/analysis/session-box-spike.md`, way (a)).
//!
//! - The instance's slice is `conductor_<session_prefix>.slice`, or `conductor.slice` without a
//!   prefix ([`slice_name`]). systemd reads each `-` of a slice's name as one step down the
//!   hierarchy, so no slice name has one: a prefix's own `-` is written `\x2d`, and every
//!   instance's slice is a child of the user manager's root slice, a sibling of every other
//!   instance's. Before a unit starts in it, `systemctl --user set-property --runtime <slice>`
//!   sets the instance's `confinement.limits` on it: `MemoryMax`, `CPUWeight` and `TasksMax`,
//!   each reset (`infinity`, or an empty `CPUWeight=`) where the instance sets none.
//! - [`Provider::start`] runs the session in the unit `conductor_<session name>.service`
//!   ([`unit_name`]), once no unit of that name is left ([`Systemd::wait_gone`]): `systemd-run
//!   --user --unit <unit> --slice <slice> --collect`, `-p
//!   MemoryMax=`, `-p CPUWeight=` and `-p TasksMax=` for each limit the role sets, its output to
//!   nothing, in the session's start directory, with `TERM`, `SHELL=/bin/bash`, this process's
//!   `PATH`, the launch's variables and, by name only, each `forge_token_env` variable this
//!   process has. The unit runs a tmux server of the session's own as its main process, `tmux -D
//!   -S <state>/sessions/<session name>.tmux -f <session name>.tmux.conf` ([`TMUX_CONF`]): it
//!   holds the PTY of the session's one pane, which runs the harness command, `<program>
//!   --session-id <id> <arguments>`, whichever harness the role names, and copies its output to
//!   the typescript `<session name>.typescript`. The id is chosen here. `conductor session
//!   attach` attaches a terminal to that server (`crate::attach`).
//! - [`Provider::prompt`] waits until the session list shows the session live, then types the
//!   prompt into the pane as one paste through tmux ([`type_text`]: `set-buffer`, `paste-buffer -p`, bracketed
//!   when the harness asked for it), and Enter.
//! - [`Provider::stop`] is `systemctl --user stop <unit>`, and nothing when systemd has no unit of
//!   that name loaded (`systemctl --user show -p LoadState -p ActiveState <unit>` reads
//!   `LoadState=not-found`): the session ended already. [`Provider::resume`] starts a new unit
//!   of the same name running `claude --resume <id> <arguments>`, as `start` does once the old
//!   unit is gone: while it is still stopping the provider waits for it, up to [`GONE_WAIT`], and
//!   resets it once it is inactive or failed so systemd drops it; a unit that still runs is
//!   refused, since two sessions would write one transcript.
//! - [`Provider::measure`] asks systemd for the unit's cgroup (`systemctl --user show -p
//!   ControlGroup --value <unit>`) and reads `memory.max`, `cpu.weight` and `pids.max` there,
//!   each absent where the file reads `max` or is not there. The provider sets up no write fence
//!   yet, so the measurement is a partial box: `writable` absent and `fence_missing` the reason.
//! - [`Provider::available`] reads the user manager's delegated controllers from
//!   `<cgroup root>/user.slice/user-<uid>.slice/user@<uid>.service/cgroup.controllers`
//!   ([`controllers_file`]), not `systemctl --user show`, which reports `Delegate=no` where the
//!   controllers are delegated all the same; it is available when `cpu`, `memory` and `pids` are
//!   listed. `io` is not asked for.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{Limits, ProviderAvailability, SessionName};
use conductor_model::observation::EnvelopeProvider;
use conductor_model::session::{EnvelopeMeasurement, SessionRequest, StartedSession};

use super::{BOUND, Launch, Provider, Said, exited, live_session, now};
use crate::collect;

/// The cgroup root of this host.
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// The program that starts a unit, found on `PATH`.
pub const SYSTEMD_RUN: &str = "systemd-run";

/// The program that sets a slice's limits, stops a unit and reads its cgroup, found on `PATH`.
pub const SYSTEMCTL: &str = "systemctl";

/// The controllers the user manager must delegate for a box.
pub const DELEGATED: [&str; 3] = ["cpu", "memory", "pids"];

/// The program that runs a session's server and types into it, found on `PATH`.
pub const TMUX: &str = "tmux";

/// The configuration of a session's tmux server, `<state>/sessions/<name>.tmux.conf`, which
/// creates the session: `tmux -D` runs the server as the unit's main process and takes no
/// command, so the session is made here. The server ends with its one session (`exit-empty`,
/// which `-D` turns off); panes run `/bin/sh`, not the operator's login shell; the prefix is
/// `ctrl-]`, so `ctrl-]` then `d` detaches and `ctrl-b` stays the harness's; no status line, so
/// the harness has the whole screen; the window is 160 by 50; the pane runs the harness command
/// `CONDUCTOR_HARNESS` and copies its output to the typescript `CONDUCTOR_TYPESCRIPT`. The
/// session's name is `CONDUCTOR_SESSION`, which also marks every process of the session as one
/// (`story:session-attach-refuses-sessions`). Nothing here names a harness.
pub const TMUX_CONF: &str = r#"set -s exit-empty on
set -s escape-time 10
set -g default-shell /bin/sh
set -g status off
set -g prefix 'C-]'
unbind C-b
bind 'C-]' send-prefix
new-session -d -s "$CONDUCTOR_SESSION" -x 160 -y 50 'exec /bin/sh -c "$CONDUCTOR_HARNESS"'
pipe-pane -o -t "=$CONDUCTOR_SESSION:" 'exec cat >> "$CONDUCTOR_TYPESCRIPT"'
"#;

/// What the pane runs before the harness: tmux, built with systemd, moves each pane it spawns
/// into a `tmux-spawn-<uuid>.scope` of its own, out of the unit's cgroup and its limits; the pane
/// writes itself back into the cgroup of its parent, the tmux server, the unit's main process.
/// Ends with `; `, the harness command follows.
pub const MOVE_BACK: &str =
    r#"p=$(sed -n 's/^0:://p' /proc/$PPID/cgroup); echo $$ > "/sys/fs/cgroup$p/cgroup.procs"; "#;

/// The record's reason for a box with no write fence.
pub const NO_FENCE: &str = "the systemd provider sets up no write fence yet";

/// How long the TUI is given between the pasted prompt and Enter.
const PASTE_SETTLE: Duration = Duration::from_millis(500);

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

/// The user manager's `cgroup.controllers` of the user `uid` under `root`.
#[must_use]
pub fn controllers_file(root: &Path, uid: u32) -> PathBuf {
    root.join(format!(
        "user.slice/user-{uid}.slice/user@{uid}.service/cgroup.controllers"
    ))
}

/// The user id of this process.
fn uid() -> Result<u32> {
    Ok(fs::metadata("/proc/self")
        .context("read the user id of this process from /proc/self")?
        .uid())
}

/// A new session id, as the kernel makes a random UUID.
fn session_id() -> Result<String> {
    Ok(fs::read_to_string("/proc/sys/kernel/random/uuid")
        .context("make a session id from /proc/sys/kernel/random/uuid")?
        .trim()
        .to_owned())
}

/// `word` quoted for a POSIX shell: in single quotes, a single quote written `'\''`.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
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

/// A file of the session `name` under `state`: `<state>/sessions/<name>.<what>`.
fn session_file(state: &Path, name: &SessionName, what: &str) -> PathBuf {
    state.join("sessions").join(format!("{}.{what}", name.0))
}

/// One limit file of a cgroup: its number, or `None` where it reads `max` or is not there.
fn limit(dir: &Path, file: &str) -> Result<Option<i64>> {
    let path = dir.join(file);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    match text.trim() {
        "max" => Ok(None),
        value => value
            .parse()
            .map(Some)
            .with_context(|| format!("{} reads {value:?}, no number", path.display())),
    }
}

/// The `systemd` provider; see the module documentation.
#[derive(Debug, Clone)]
pub struct Systemd {
    /// The cgroup root the provider reads, [`CGROUP_ROOT`] on this host.
    cgroup_root: PathBuf,
    /// The launch [`Provider::start`] kept for [`Provider::prompt`].
    launch: Option<Launch>,
}

impl Systemd {
    /// The provider reading the cgroups under `cgroup_root`.
    #[must_use]
    pub fn new(cgroup_root: PathBuf) -> Self {
        Self {
            cgroup_root,
            launch: None,
        }
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
    fn unit_state(unit: &str) -> Result<(String, String)> {
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
            let (load, active) = Self::unit_state(unit)?;
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

    /// Starts the unit of `launch`'s session in its instance's slice, once no unit of its name
    /// is left, running `claude <harness>` through the launcher, and answers the unit's name.
    fn run_unit(launch: &Launch, harness: &[String]) -> Result<String> {
        let request = &launch.request;
        let unit = unit_name(&request.session_name);
        Self::wait_gone(&unit)?;
        let slice = slice_name(
            launch
                .session_prefix
                .as_ref()
                .map(|prefix| prefix.0.as_str()),
        );
        Self::set_slice(&slice, &launch.instance_limits)?;
        let name = &request.session_name;
        let socket = session_file(&launch.state, name, "tmux");
        let conf = session_file(&launch.state, name, "tmux.conf");
        let typescript = session_file(&launch.state, name, "typescript");
        let dir = socket.parent().unwrap_or(&launch.state);
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        // A socket a server killed without cleanup left behind is replaced by the new server.
        match fs::remove_file(&socket) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("remove {}", socket.display()));
            }
        }
        fs::write(&conf, TMUX_CONF).with_context(|| format!("write {}", conf.display()))?;

        let mut command = String::from(MOVE_BACK);
        command.push_str("exec ");
        let words: Vec<String> = std::iter::once(launch.program.as_str())
            .chain(harness.iter().map(String::as_str))
            .map(quoted)
            .collect();
        command.push_str(&words.join(" "));

        let mut args: Vec<String> = ["--user", "--unit", &unit, "--slice", &slice, "--collect"]
            .map(str::to_owned)
            .to_vec();
        let Limits {
            memory_max,
            cpu_weight,
            tasks_max,
        } = &request.limits;
        let properties = [
            memory_max
                .as_ref()
                .map(|size| format!("MemoryMax={}G", size.0)),
            cpu_weight.map(|weight| format!("CPUWeight={weight}")),
            tasks_max.map(|tasks| format!("TasksMax={tasks}")),
            Some("StandardOutput=null".to_owned()),
        ];
        for property in properties.into_iter().flatten() {
            args.extend(["-p".to_owned(), property]);
        }
        args.push(format!("--working-directory={}", request.cwd));
        let mut set = vec![
            "TERM=xterm-256color".to_owned(),
            "SHELL=/bin/bash".to_owned(),
            format!("CONDUCTOR_SESSION={}", name.0),
            format!("CONDUCTOR_TYPESCRIPT={}", typescript.display()),
            format!("CONDUCTOR_HARNESS={command}"),
        ];
        if let Some(path) = std::env::var_os("PATH") {
            set.push(format!("PATH={}", path.to_string_lossy()));
        }
        set.extend(
            launch
                .env
                .iter()
                .map(|(name, value)| format!("{name}={value}")),
        );
        set.extend(
            request
                .forge_token_env
                .iter()
                .filter(|name| std::env::var_os(name).is_some())
                .cloned(),
        );
        for variable in set {
            args.extend(["-E".to_owned(), variable]);
        }
        args.extend([
            TMUX.to_owned(),
            "-D".to_owned(),
            "-S".to_owned(),
            socket.display().to_string(),
            "-f".to_owned(),
            conf.display().to_string(),
        ]);
        call(SYSTEMD_RUN, &args).with_context(|| format!("start the unit {unit}"))?;
        Ok(unit)
    }

    /// The unit `session` runs in.
    fn unit_of(session: &StartedSession) -> String {
        session
            .unit
            .clone()
            .unwrap_or_else(|| unit_name(&session.session_name))
    }
}

/// Runs `tmux -S <name>.tmux <args>` on the session `name`'s server, from the directory of its
/// socket, so a long state path does not exceed a socket's path limit; within [`BOUND`].
pub fn tmux_call(sessions: &Path, name: &SessionName, args: &[&str]) -> Result<String> {
    let socket = format!("{}.tmux", name.0);
    let shown = format!("{TMUX} -S {socket} {}", args.join(" "));
    let output = collect::run(
        Command::new(TMUX)
            .current_dir(sessions)
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
/// (`set-buffer`, then `paste-buffer -p`, bracketed when the harness asked for it), and, after
/// [`PASTE_SETTLE`], a carriage return pasted the same way unbracketed. Not `send-keys`: tmux
/// refuses it ("client is read-only") while a `--watch` client is attached.
///
/// # Errors
///
/// A tmux call fails.
pub fn type_text(sessions: &Path, name: &SessionName, text: &str) -> Result<()> {
    let pane = target(name);
    tmux_call(
        sessions,
        name,
        &["set-buffer", "-b", "conductor", "--", text],
    )?;
    tmux_call(
        sessions,
        name,
        &["paste-buffer", "-d", "-p", "-b", "conductor", "-t", &pane],
    )?;
    thread::sleep(PASTE_SETTLE);
    tmux_call(
        sessions,
        name,
        &["set-buffer", "-b", "conductor", "--", "\r"],
    )?;
    tmux_call(
        sessions,
        name,
        &["paste-buffer", "-d", "-b", "conductor", "-t", &pane],
    )?;
    Ok(())
}

/// The tmux target of the session `name`'s one pane: the session of exactly that name.
#[must_use]
pub fn target(name: &SessionName) -> String {
    format!("={}:", name.0)
}

impl Provider for Systemd {
    fn name(&self) -> String {
        "systemd".to_owned()
    }

    fn available(&mut self) -> ProviderAvailability {
        let reason = (|| -> Result<Option<String>> {
            let file = controllers_file(&self.cgroup_root, uid()?);
            let text = fs::read_to_string(&file).with_context(|| {
                format!(
                    "the user manager's delegated controllers cannot be read from {}",
                    file.display()
                )
            })?;
            let listed: Vec<&str> = text.split_whitespace().collect();
            let missing: Vec<&str> = DELEGATED
                .into_iter()
                .filter(|controller| !listed.contains(controller))
                .collect();
            Ok((!missing.is_empty()).then(|| {
                format!(
                    "the user manager does not delegate {}: {} lists {:?}",
                    missing.join(", "),
                    file.display(),
                    text.trim()
                )
            }))
        })()
        .unwrap_or_else(|error| Some(format!("{error:#}")));
        ProviderAvailability {
            provider: self.name(),
            available: reason.is_none(),
            reason,
        }
    }

    fn start(&mut self, launch: &Launch) -> Result<StartedSession> {
        let id = session_id()?;
        let mut harness = vec!["--session-id".to_owned(), id.clone()];
        harness.extend(launch.args.iter().cloned());
        let unit = Self::run_unit(launch, &harness)?;
        self.launch = Some(launch.clone());
        Ok(StartedSession {
            role: launch.request.role.clone(),
            session_name: launch.request.session_name.clone(),
            provider: self.name(),
            unit: Some(unit),
            session_id: Some(id),
        })
    }

    fn prompt(&mut self, session: &mut StartedSession, prompt: &str) -> Result<Said> {
        let launch = self
            .launch
            .as_ref()
            .ok_or_else(|| anyhow!("no session was started to prompt"))?;
        let name = &session.session_name;
        let started = Instant::now();
        while live_session(&name.0)?.is_none() {
            if started.elapsed() >= BOUND {
                bail!(
                    "the session {} was not listed live within {} s of its unit's start",
                    name.0,
                    BOUND.as_secs()
                );
            }
            thread::sleep(Duration::from_millis(200));
        }
        type_text(&launch.state.join("sessions"), name, prompt)?;
        Ok(Said::default())
    }

    fn stop(&mut self, session: &StartedSession, _request: &SessionRequest) -> Result<()> {
        let unit = Self::unit_of(session);
        let (load, _) = Self::unit_state(&unit)?;
        if load == "not-found" {
            return Ok(());
        }
        call(SYSTEMCTL, &["--user", "stop", &unit].map(str::to_owned))
            .with_context(|| format!("stop the unit {unit}"))?;
        Ok(())
    }

    fn resume(&mut self, session: &StartedSession, launch: &Launch) -> Result<StartedSession> {
        let id = session
            .session_id
            .as_deref()
            .ok_or_else(|| anyhow!("the session {} has no id", session.session_name.0))?;
        let mut harness = vec!["--resume".to_owned(), id.to_owned()];
        harness.extend(launch.args.iter().cloned());
        let unit = Self::run_unit(launch, &harness)?;
        self.launch = Some(launch.clone());
        Ok(StartedSession {
            provider: self.name(),
            unit: Some(unit),
            ..session.clone()
        })
    }

    fn measure(
        &mut self,
        session: &StartedSession,
        request: &SessionRequest,
    ) -> Result<EnvelopeMeasurement> {
        let unit = Self::unit_of(session);
        let cgroup = call(
            SYSTEMCTL,
            &["--user", "show", "-p", "ControlGroup", "--value", &unit].map(str::to_owned),
        )
        .with_context(|| format!("read the cgroup of the unit {unit}"))?
        .trim()
        .to_owned();
        if cgroup.is_empty() {
            bail!("the unit {unit} has no cgroup: it is not running");
        }
        let dir = self.cgroup_root.join(cgroup.trim_start_matches('/'));
        Ok(EnvelopeMeasurement {
            session: session.clone(),
            measured_at: now(),
            provider: Some(EnvelopeProvider::Systemd),
            memory_max: limit(&dir, "memory.max")?,
            cpu_weight: limit(&dir, "cpu.weight")?,
            tasks_max: limit(&dir, "pids.max")?,
            cgroup: Some(cgroup),
            places: request.writable.clone(),
            writable: None,
            fence_missing: Some(NO_FENCE.to_owned()),
            unboxed: None,
        })
    }
}
