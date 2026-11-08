//! `conductor tui`, and `conductor` with no arguments at a terminal (`story:tui-readonly`, design
//! architecture-next § 5.3): a full-screen view over the daemon's global API, read-only.
//!
//! Three panes, each from one route of the daemon the global `--daemon` names (else `CONDUCTOR_DAEMON`, else
//! `127.0.0.1:<daemon.port>` of the config file, [`daemon_url`]):
//! - the instances: `GET /v1/instances` with each one's state, its open decisions counted from the
//!   federated `GET /v1/decisions`, and its KPI flags from the federated `GET /v1/kpis` (how many
//!   measured KPIs are off their target); an instance the federated views answer `unreachable` for
//!   says so;
//! - the sessions of the selected instance: the federated `GET /v1/sessions`, its rows, each with
//!   its role, state, context and current work (its dispatch's goal);
//! - the settings of the selected instance: `GET /v1/instances/{name}/settings`, proxied to the
//!   instance, its fields read-only.
//!
//! `Enter` attaches to the selected session, `w` watches it read-only and `l` follows its terminal
//! log. When the daemon is on this host, the terminal is handed to `conductor session attach`
//! (`--watch` for `w`) and to the follow of [`crate::attach::follow_log`] for `l`; `ctrl-]` then `d`
//! comes back, the session untouched. Against a daemon elsewhere the UI asks
//! `GET /v1/instances/{name}/agents/{agent}/attach` through the daemon's proxy and shows its answer:
//! a PTY stream over HTTP is not served yet (`501`). Nothing here reads the store or a session's
//! files of another host. Rendering is [`view::draw`], testable on ratatui's test backend.

pub mod client;
pub mod view;

use std::io::IsTerminal as _;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use serde_json::Value;

use crate::config::{self, Environment};
use client::Client;

/// The variable that names the daemon a command talks to, when `--daemon` does not (design § 4).
pub const DAEMON_VARIABLE: &str = crate::remote::VARIABLE;

/// The daemon's port when no config file names one (`daemon.port`'s default).
const DEFAULT_PORT: i64 = 7300;

/// How often the panes are read again while no key is pressed.
const REFRESH: Duration = Duration::from_secs(5);

/// The pane the arrow keys move in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// The instances.
    Instances,
    /// The sessions of the selected instance.
    Sessions,
}

/// One instance as the instances pane shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRow {
    /// Its name.
    pub name: String,
    /// Its state, as the daemon spells it, or `unreachable`.
    pub state: String,
    /// Its open decision requests, when it answered.
    pub decisions: Option<usize>,
    /// Its KPI flag: `K ok`, `K<n> !` for `n` off target, `K -` when none is measured.
    pub kpis: String,
}

/// One session as the sessions pane shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    /// Its full name, the agent's name on the instance API.
    pub name: String,
    /// Its role.
    pub role: String,
    /// Its state.
    pub state: String,
    /// The context it has used, or `-` when the row carries none.
    pub context: String,
    /// What it works on: its dispatch's goal, else its repository, else `-`.
    pub work: String,
}

/// What a key asks of the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing more than the screen's own change.
    Nothing,
    /// Leave the UI.
    Quit,
    /// Read every pane again.
    Refresh,
    /// Another instance is selected: read its settings.
    Selected,
    /// Hand the terminal to the selected session.
    Enter(Mode),
}

/// How the terminal is handed to a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `Enter`: attach, typing reaches the session.
    Attach,
    /// `w`: attach read-only.
    Watch,
    /// `l`: follow its terminal log.
    Log,
}

impl Mode {
    const fn verb(self) -> &'static str {
        match self {
            Self::Attach => "attach",
            Self::Watch => "watch",
            Self::Log => "log",
        }
    }
}

/// Everything the UI shows.
#[derive(Debug, Clone)]
pub struct Screen {
    /// The daemon's origin.
    pub daemon: String,
    /// The instances pane.
    pub instances: Vec<InstanceRow>,
    /// The federated sessions' rows, every instance's.
    pub all_sessions: Vec<Value>,
    /// The settings of the selected instance: each field's path and value.
    pub settings: Vec<(String, String)>,
    /// The selected instance's index.
    pub selected: usize,
    /// The selected session's index in [`Screen::sessions`].
    pub session: usize,
    /// The pane the arrow keys move in.
    pub focus: Pane,
    /// The status line: the last refusal or failure, else empty.
    pub status: String,
}

impl Screen {
    /// An empty screen for the daemon at `daemon`.
    #[must_use]
    pub fn new(daemon: String) -> Self {
        Self {
            daemon,
            instances: Vec::new(),
            all_sessions: Vec::new(),
            settings: Vec::new(),
            selected: 0,
            session: 0,
            focus: Pane::Instances,
            status: String::new(),
        }
    }

    /// The selected instance's name.
    #[must_use]
    pub fn instance(&self) -> Option<&str> {
        self.instances
            .get(self.selected)
            .map(|row| row.name.as_str())
    }

    /// The sessions of the selected instance.
    #[must_use]
    pub fn sessions(&self) -> Vec<SessionRow> {
        let Some(name) = self.instance() else {
            return Vec::new();
        };
        self.all_sessions
            .iter()
            .filter(|row| row["instance"] == name && row["state"] != "unreachable")
            .map(session_row)
            .collect()
    }

    /// The selected session.
    #[must_use]
    pub fn selected_session(&self) -> Option<SessionRow> {
        self.sessions().into_iter().nth(self.session)
    }

    /// Reads the instances, decisions, KPIs and sessions from `client`, keeping the selection by
    /// name; a route that fails is named on the status line and leaves its pane as it was.
    pub fn load(&mut self, client: &Client) {
        let kept = self.instance().map(str::to_owned);
        let mut failed = Vec::new();
        let mut read = |path: &str| match client.json(path) {
            Ok(value) => Some(value),
            Err(error) => {
                failed.push(format!("{error:#}"));
                None
            }
        };
        let instances = read(crate::daemon::api::INSTANCES);
        let decisions = read("/v1/decisions").unwrap_or(Value::Null);
        let kpis = read("/v1/kpis").unwrap_or(Value::Null);
        let sessions = read("/v1/sessions");
        if let Some(instances) = instances {
            self.instances = instance_rows(&instances, &decisions, &kpis);
        }
        if let Some(sessions) = sessions {
            self.all_sessions = rows(&sessions).to_vec();
        }
        self.selected = kept
            .and_then(|name| self.instances.iter().position(|row| row.name == name))
            .unwrap_or(0);
        self.clamp();
        self.load_settings(client);
        if !failed.is_empty() {
            self.status = failed.join("; ");
        }
    }

    /// Reads the selected instance's settings from `client`.
    pub fn load_settings(&mut self, client: &Client) {
        self.settings.clear();
        let Some(name) = self.instance().map(str::to_owned) else {
            return;
        };
        match client.json(&format!("/v1/instances/{name}/settings")) {
            Ok(page) => {
                self.settings = page["fields"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|field| {
                        Some((field["path"].as_str()?.to_owned(), text(&field["value"])))
                    })
                    .collect();
            }
            Err(error) => self.status = format!("settings of {name}: {error:#}"),
        }
    }

    fn clamp(&mut self) {
        self.selected = self.selected.min(self.instances.len().saturating_sub(1));
        self.session = self.session.min(self.sessions().len().saturating_sub(1));
    }

    /// Takes the key `code` with `modifiers`.
    pub fn key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Action {
        match code {
            KeyCode::Char('c' | 'd') if modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Tab | KeyCode::Left | KeyCode::Right | KeyCode::Char('h') => {
                self.focus = match self.focus {
                    Pane::Instances => Pane::Sessions,
                    Pane::Sessions => Pane::Instances,
                };
                Action::Nothing
            }
            KeyCode::Up | KeyCode::Char('k') => self.step(false),
            KeyCode::Down | KeyCode::Char('j') => self.step(true),
            KeyCode::Enter => self.enter(Mode::Attach),
            KeyCode::Char('w') => self.enter(Mode::Watch),
            KeyCode::Char('l') => self.enter(Mode::Log),
            _ => Action::Nothing,
        }
    }

    fn step(&mut self, down: bool) -> Action {
        let move_in = |at: &mut usize, len: usize| {
            *at = if down {
                (*at + 1).min(len.saturating_sub(1))
            } else {
                at.saturating_sub(1)
            };
        };
        match self.focus {
            Pane::Instances => {
                let before = self.selected;
                move_in(&mut self.selected, self.instances.len());
                if before == self.selected {
                    return Action::Nothing;
                }
                self.session = 0;
                Action::Selected
            }
            Pane::Sessions => {
                let len = self.sessions().len();
                move_in(&mut self.session, len);
                Action::Nothing
            }
        }
    }

    fn enter(&mut self, mode: Mode) -> Action {
        if self.selected_session().is_none() {
            self.status = format!("{}: no session is selected", mode.verb());
            return Action::Nothing;
        }
        Action::Enter(mode)
    }
}

/// The rows of a federated view's answer.
fn rows(view: &Value) -> &[Value] {
    view["rows"].as_array().map_or(&[], Vec::as_slice)
}

/// A JSON value as one short text: a string as it is, `-` for null, anything else as JSON.
fn text(value: &Value) -> String {
    match value {
        Value::Null => "-".to_owned(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The instances pane's rows: each instance `instances` lists, with its count of open requests in
/// `decisions` and its KPI flag from `kpis`; one the federated views answer unreachable for is
/// `unreachable`.
#[must_use]
pub fn instance_rows(instances: &Value, decisions: &Value, kpis: &Value) -> Vec<InstanceRow> {
    let of = |view: &Value, name: &str| -> Vec<Value> {
        rows(view)
            .iter()
            .filter(|row| row["instance"] == name)
            .cloned()
            .collect()
    };
    instances
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|instance| {
            let name = instance["name"].as_str()?.to_owned();
            let asked = of(decisions, &name);
            let measured = of(kpis, &name);
            let unreachable = asked
                .iter()
                .chain(&measured)
                .any(|row| row["state"] == "unreachable");
            let state = if unreachable {
                "unreachable".to_owned()
            } else {
                instance["state"]
                    .as_str()
                    .map_or_else(|| "?".to_owned(), str::to_ascii_lowercase)
            };
            let decisions = (!unreachable).then_some(asked.len());
            Some(InstanceRow {
                name,
                state,
                decisions,
                kpis: if unreachable {
                    "K ?".to_owned()
                } else {
                    kpi_flag(&measured)
                },
            })
        })
        .collect()
}

/// `K ok`, `K<n> !` for `n` KPIs off their target, `K -` when none of `rows` is measured against
/// a target.
fn kpi_flag(rows: &[Value]) -> String {
    let mut measured = 0;
    let mut off = 0;
    for row in rows {
        let (Some(value), Some(target)) = (row["value"].as_f64(), row["target"].as_f64()) else {
            continue;
        };
        measured += 1;
        let higher = row["direction"] == "higher";
        if (higher && value < target) || (!higher && value > target) {
            off += 1;
        }
    }
    match (measured, off) {
        (0, _) => "K -".to_owned(),
        (_, 0) => "K ok".to_owned(),
        (_, off) => format!("K{off} !"),
    }
}

/// A federated session row as the sessions pane shows it.
fn session_row(row: &Value) -> SessionRow {
    let state = match (&row["state"], &row["status"]) {
        (Value::String(state), Value::String(status)) if state != status => {
            format!("{state} {status}")
        }
        (Value::String(state), _) => state.clone(),
        (_, status) => text(status),
    };
    let context = ["context", "context_used"]
        .iter()
        .map(|key| &row[*key])
        .find(|value| !value.is_null())
        .map_or_else(|| "-".to_owned(), text);
    let work = [&row["dispatch"]["goal"], &row["repository"]]
        .into_iter()
        .find(|value| value.as_str().is_some_and(|text| !text.is_empty()))
        .map_or_else(|| "-".to_owned(), text);
    SessionRow {
        name: text(&row["name"]),
        role: text(&row["role"]),
        state,
        context,
        work,
    }
}

/// Where the terminal goes for `mode` on the session `name` of `instance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handoff {
    /// The daemon is on this host: hand the terminal to the session here.
    Local {
        /// The session's instance.
        instance: String,
        /// The session's full name.
        name: String,
        /// How.
        mode: Mode,
    },
    /// The daemon is elsewhere: its answer to the attach route, shown on the status line.
    Refused(String),
}

/// Where `mode` on the session `name` of `instance` goes, through `client`: [`Handoff::Local`]
/// when the daemon is on this host, else the daemon's answer to the attach route
/// ([`remote_attach`]).
#[must_use]
pub fn handoff(client: &Client, instance: &str, name: &str, mode: Mode) -> Handoff {
    if client.local() {
        return Handoff::Local {
            instance: instance.to_owned(),
            name: name.to_owned(),
            mode,
        };
    }
    Handoff::Refused(remote_attach(client, instance, name, mode))
}

/// Asks `GET /v1/instances/{instance}/agents/{name}/attach` of the daemon at `client`, through its
/// proxy (decision U1), and answers the line the status shows: a PTY stream over HTTP is not
/// served yet, so the answer is its refusal, or the failure to reach it.
#[must_use]
pub fn remote_attach(client: &Client, instance: &str, name: &str, mode: Mode) -> String {
    let path = format!("/v1/instances/{instance}/agents/{name}/attach");
    let verb = mode.verb();
    match client.get(&path) {
        Ok((status, answer)) => format!(
            "{verb} {name}: the daemon at {} answered {status}{}",
            client.origin(),
            client::reason(&answer)
        ),
        Err(error) => format!("{verb} {name}: {error:#}"),
    }
}

/// The daemon `flag` names, else `variable` (`CONDUCTOR_DAEMON`), else `127.0.0.1:<port>`.
#[must_use]
pub fn daemon_url(flag: Option<&str>, variable: Option<&str>, port: Option<i64>) -> String {
    flag.or(variable)
        .filter(|url| !url.trim().is_empty())
        .map_or_else(
            || format!("http://127.0.0.1:{}", port.unwrap_or(DEFAULT_PORT)),
            str::to_owned,
        )
}

/// `conductor tui`, as the module documentation says.
///
/// # Errors
///
/// `--daemon` names no `http://` or `https://` origin, or standard input or output is no
/// terminal.
pub fn tui(config_flag: Option<&Path>, daemon: Option<&str>) -> Result<ExitCode> {
    const WHAT: &str = "tui";
    // The config file only gives the local daemon's port and the views' timeout; one that does not
    // load leaves both their defaults.
    let loaded = Environment::from_process().ok().and_then(|environment| {
        let home = environment.home().ok()?.to_owned();
        let (file, _) = config::locate(config_flag, &environment).ok()?;
        let text = std::fs::read_to_string(environment.cwd.join(file)).ok()?;
        config::parse(&text, &home).ok()
    });
    let variable = std::env::var(DAEMON_VARIABLE).ok();
    let url = daemon_url(
        daemon,
        variable.as_deref(),
        loaded.as_ref().map(|loaded| loaded.daemon.port),
    );
    // A federated view waits up to daemon.timeout for its instances; the UI waits that and more.
    let timeout = Duration::from_secs(
        loaded
            .as_ref()
            .and_then(|loaded| config::seconds(&loaded.daemon.timeout))
            .unwrap_or(config::FEDERATED_TIMEOUT)
            * 2
            + 1,
    );
    let client = Client::parse(&url, timeout).context(WHAT)?;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!(
            "{WHAT}: needs a terminal on standard input and output; `conductor status` prints \
             the instances without one"
        );
    }
    let mut screen = Screen::new(client.origin());
    screen.load(&client);
    let mut terminal = ratatui::init();
    let mut read_at = Instant::now();
    let answer = loop {
        if let Err(error) = terminal.draw(|frame| view::draw(frame, &screen)) {
            break Err(error).context("tui: draw");
        }
        let pressed = match event::poll(Duration::from_millis(250)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => Some(key),
                Ok(_) => None,
                Err(error) => break Err(error).context("tui: read a key"),
            },
            Ok(false) => None,
            Err(error) => break Err(error).context("tui: read a key"),
        };
        let action = pressed.map_or(Action::Nothing, |key| screen.key(key.code, key.modifiers));
        match action {
            Action::Quit => break Ok(ExitCode::SUCCESS),
            Action::Refresh => {
                screen.status.clear();
                screen.load(&client);
                read_at = Instant::now();
            }
            Action::Selected => screen.load_settings(&client),
            Action::Enter(mode) => {
                let Some((instance, session)) = screen
                    .instance()
                    .map(str::to_owned)
                    .zip(screen.selected_session())
                else {
                    continue;
                };
                match handoff(&client, &instance, &session.name, mode) {
                    Handoff::Refused(line) => screen.status = line,
                    Handoff::Local {
                        instance,
                        name,
                        mode,
                    } => {
                        ratatui::restore();
                        screen.status = local(config_flag, &instance, &name, mode);
                        terminal = ratatui::init();
                        let _ = terminal.clear();
                        screen.load(&client);
                        read_at = Instant::now();
                    }
                }
            }
            Action::Nothing if read_at.elapsed() >= REFRESH => {
                screen.load(&client);
                read_at = Instant::now();
            }
            Action::Nothing => {}
        }
    };
    ratatui::restore();
    answer
}

/// Hands this terminal to the session `name` of `instance` on this host for `mode`, and answers
/// the status line once it comes back: empty, or the refusal `session attach` printed.
fn local(config_flag: Option<&Path>, instance: &str, name: &str, mode: Mode) -> String {
    if mode == Mode::Log {
        return match crate::attach::follow_log(config_flag, instance, name) {
            Ok(_) => String::new(),
            Err(error) => format!("log {name}: {error:#}"),
        };
    }
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(error) => return format!("{} {name}: {error}", mode.verb()),
    };
    let mut command = Command::new(program);
    if let Some(file) = config_flag {
        command.arg("--config").arg(file);
    }
    command.args(["session", "attach", "--instance", instance, "--name", name]);
    if mode == Mode::Watch {
        command.arg("--watch");
    }
    let ran = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(std::process::Child::wait_with_output);
    match ran {
        Ok(output) if output.status.success() => String::new(),
        Ok(output) => String::from_utf8_lossy(&output.stderr)
            .lines()
            .last()
            .unwrap_or("session attach failed")
            .to_owned(),
        Err(error) => format!("{} {name}: {error}", mode.verb()),
    }
}

#[cfg(test)]
mod tests;
