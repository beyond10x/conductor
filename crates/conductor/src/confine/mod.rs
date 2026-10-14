//! How conductor starts, prompts, stops, resumes and measures a session in the box its role
//! requests (`story:session-envelope-seam`): the one path every session start takes.
//!
//! A [`Provider`] is the whole session surface: [`Provider::available`] probes the host,
//! [`Provider::start`] starts a session from a [`Launch`] (a `conductor.session.SessionRequest`
//! of absolute paths, limits and the role's `forge_token_env` names, beside the harness command)
//! and answers its identity (`conductor.session.StartedSession`), [`Provider::prompt`] delivers
//! the first prompt or charter, [`Provider::stop`] and [`Provider::resume`] end and restart it,
//! and [`Provider::measure`] reads the box back at its boundary
//! (`conductor.session.EnvelopeMeasurement`). [`Providers`] names the provider of each
//! `confinement.provider`; [`Host`] is this host's, which the command line runs with.
//!
//! The `none` provider ([`Unboxed`]) is today's commands: its `start` launches nothing and keeps
//! the launch, its `prompt` runs `claude --bg <arguments> <prompt>` in the start directory (one
//! argv, the prompt one argument, never through a shell), `stop` is `claude stop <id>`, a wait for
//! the session list to show it gone, and `claude rm <id>`, `resume` is `claude --bg --resume <id>
//! <arguments>`, and `measure` measures nothing: its record says the session runs unboxed. The
//! `systemd` provider is not written yet; until it is, [`Host`] reports it not available.
//!
//! [`choose`] reads `confinement`: under `strict`, a provider that is not available starts
//! nothing (`ProviderUnavailable`); under `report`, the session starts unboxed through `none`
//! and its record names why. [`Chosen::start`] then starts, prompts and measures; a box short of
//! the request ([`shortfall`]) is stopped again under `strict` (`MeasurementShort`) and recorded
//! as it is under `report`. The measurement of a started session is written to
//! `<state>/sessions/<session name>.envelope.json` ([`record_path`]), which the sessions
//! collector reads into the session's `envelope_*` fields ([`read_measurement`]).

pub mod places;

use std::fs::{self, File};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{
    Confinement, ConfinementMode, ConfinementProvider, Limits, ProviderAvailability, SessionName,
};
use conductor_model::observation::EnvelopeProvider;
use conductor_model::primitives::Timestamp;
use conductor_model::session::{
    EnvelopeMeasurement, PlaceResolved, SessionRequest, StartedSession,
};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::collect;

/// The harness program the `none` provider runs, found on `PATH`.
pub const CLAUDE: &str = "claude";

/// How long the session list, a launch and a stop may each take before they are stopped.
pub const BOUND: Duration = Duration::from_secs(120);

/// How long a stopped session may stay in the session list with a pid.
const STOP_WAIT: Duration = Duration::from_secs(30);

/// A session to start: the box it asks for, and the harness command it runs in that box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The box: role, session name, start directory, limits, writable places, network, forge
    /// token variables.
    pub request: SessionRequest,
    /// The harness program, `claude`.
    pub program: String,
    /// Its arguments before the first prompt, such as `-n <name> --model <model>`.
    pub args: Vec<String>,
    /// Variables set for the session beside the inherited environment.
    pub env: Vec<(String, String)>,
}

/// What the harness wrote when the first prompt was delivered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Said {
    /// Its standard output.
    pub stdout: Vec<u8>,
    /// Its standard error.
    pub stderr: Vec<u8>,
}

/// The whole session surface of one confinement provider; see the module documentation.
pub trait Provider {
    /// The provider's name as `confinement.provider` writes it.
    fn name(&self) -> String;

    /// Whether this provider can set up a box on this host, and why not.
    fn available(&mut self) -> ProviderAvailability;

    /// Starts a session for `launch` and answers its identity.
    ///
    /// # Errors
    ///
    /// The session did not start.
    fn start(&mut self, launch: &Launch) -> Result<StartedSession>;

    /// Delivers `prompt`, the first prompt or charter, to `session`, filling its id once the
    /// harness printed one.
    ///
    /// # Errors
    ///
    /// The prompt was not delivered; the text says how the harness ended.
    fn prompt(&mut self, session: &mut StartedSession, prompt: &str) -> Result<Said>;

    /// Stops `session`, started for `request`.
    ///
    /// # Errors
    ///
    /// The session was not stopped.
    fn stop(&mut self, session: &StartedSession, request: &SessionRequest) -> Result<()>;

    /// Starts `session` again from its transcript, for `launch`.
    ///
    /// # Errors
    ///
    /// It did not start.
    fn resume(&mut self, session: &StartedSession, launch: &Launch) -> Result<StartedSession>;

    /// The box `session` runs in, read back at its boundary, for `request`.
    ///
    /// # Errors
    ///
    /// The box could not be read.
    fn measure(
        &mut self,
        session: &StartedSession,
        request: &SessionRequest,
    ) -> Result<EnvelopeMeasurement>;
}

/// The providers a command may start a session through.
pub trait Providers {
    /// The provider `confinement.provider` names.
    fn provider(&self, name: ConfinementProvider) -> Box<dyn Provider>;

    /// The provider whose session runs unboxed, `none`: what `report` falls back to.
    fn unboxed(&self) -> Box<dyn Provider>;

    /// Every provider, in the order `conductor doctor` prints them.
    fn all(&self) -> Vec<Box<dyn Provider>>;
}

/// This host's providers: `systemd`, which is not written yet, and `none`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Host;

impl Providers for Host {
    fn provider(&self, name: ConfinementProvider) -> Box<dyn Provider> {
        match name {
            ConfinementProvider::Systemd => Box::new(NotWritten("systemd")),
            ConfinementProvider::None => Box::new(Unboxed::default()),
        }
    }

    fn unboxed(&self) -> Box<dyn Provider> {
        Box::new(Unboxed::default())
    }

    fn all(&self) -> Vec<Box<dyn Provider>> {
        vec![self.provider(ConfinementProvider::Systemd), self.unboxed()]
    }
}

/// The availability of every provider of `providers`, in order: `conductor doctor`'s rows.
#[must_use]
pub fn availability(providers: &dyn Providers) -> Vec<ProviderAvailability> {
    providers
        .all()
        .iter_mut()
        .map(|provider| provider.available())
        .collect()
}

/// A provider conductor does not have yet: never available.
struct NotWritten(&'static str);

impl NotWritten {
    fn reason(&self) -> String {
        format!("conductor does not set up a {} box yet", self.0)
    }
}

impl Provider for NotWritten {
    fn name(&self) -> String {
        self.0.to_owned()
    }

    fn available(&mut self) -> ProviderAvailability {
        ProviderAvailability {
            provider: self.name(),
            available: false,
            reason: Some(self.reason()),
        }
    }

    fn start(&mut self, _launch: &Launch) -> Result<StartedSession> {
        bail!("{}", self.reason())
    }

    fn prompt(&mut self, _session: &mut StartedSession, _prompt: &str) -> Result<Said> {
        bail!("{}", self.reason())
    }

    fn stop(&mut self, _session: &StartedSession, _request: &SessionRequest) -> Result<()> {
        bail!("{}", self.reason())
    }

    fn resume(&mut self, _session: &StartedSession, _launch: &Launch) -> Result<StartedSession> {
        bail!("{}", self.reason())
    }

    fn measure(
        &mut self,
        _session: &StartedSession,
        _request: &SessionRequest,
    ) -> Result<EnvelopeMeasurement> {
        bail!("{}", self.reason())
    }
}

/// The reason the `none` provider's record gives: it sets up no box.
pub const NONE_UNBOXED: &str = "confinement.provider is none, which sets up no box";

/// The `none` provider: the harness's own commands, unboxed; see the module documentation.
#[derive(Debug, Clone, Default)]
pub struct Unboxed {
    /// The launch [`Provider::start`] kept for [`Provider::prompt`].
    launch: Option<Launch>,
}

impl Unboxed {
    /// `claude <args>` with the launch's working directory and variables.
    fn command(launch: &Launch, args: &[String]) -> Command {
        let mut command = Command::new(&launch.program);
        command.args(args).current_dir(&launch.request.cwd);
        for (name, value) in &launch.env {
            command.env(name, value);
        }
        command
    }

    /// Runs one `claude` call in `cwd` within [`BOUND`] and answers an error naming how it
    /// ended when it did not succeed.
    fn call(cwd: &str, args: &[&str]) -> Result<()> {
        let shown = format!("{CLAUDE} {}", args.join(" "));
        let output = collect::run(Command::new(CLAUDE).args(args).current_dir(cwd), BOUND)
            .with_context(|| format!("run `{shown}`"))?;
        if !output.status.success() {
            bail!("`{shown}` {}", exited(&output));
        }
        Ok(())
    }
}

impl Provider for Unboxed {
    fn name(&self) -> String {
        "none".to_owned()
    }

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
            session_id: None,
        })
    }

    fn prompt(&mut self, session: &mut StartedSession, prompt: &str) -> Result<Said> {
        let launch = self
            .launch
            .as_ref()
            .ok_or_else(|| anyhow!("no session was started to prompt"))?;
        let mut args = vec!["--bg".to_owned()];
        args.extend(launch.args.iter().cloned());
        args.push(prompt.to_owned());
        let output = run_launch(&mut Self::command(launch, &args))?;
        if !output.status.success() {
            bail!("{}", exited(&output));
        }
        session.session_id = Some(crate::spawn::session_id(&String::from_utf8_lossy(
            &output.stdout,
        )));
        Ok(Said {
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn stop(&mut self, session: &StartedSession, request: &SessionRequest) -> Result<()> {
        let id = session
            .session_id
            .as_deref()
            .ok_or_else(|| anyhow!("the session {} has no id", session.session_name.0))?;
        Self::call(&request.cwd, &["stop", id])?;
        let started = Instant::now();
        while live_session(&session.session_name.0)?.is_some() {
            if started.elapsed() >= STOP_WAIT {
                bail!(
                    "the stopped session {id} still has a pid after {} s",
                    STOP_WAIT.as_secs()
                );
            }
            thread::sleep(Duration::from_millis(200));
        }
        Self::call(&request.cwd, &["rm", id])
    }

    fn resume(&mut self, session: &StartedSession, launch: &Launch) -> Result<StartedSession> {
        let id = session
            .session_id
            .as_deref()
            .ok_or_else(|| anyhow!("the session {} has no id", session.session_name.0))?;
        let mut args = vec!["--bg".to_owned(), "--resume".to_owned(), id.to_owned()];
        args.extend(launch.args.iter().cloned());
        let output = run_launch(&mut Self::command(launch, &args))?;
        if !output.status.success() {
            bail!("{}", exited(&output));
        }
        Ok(session.clone())
    }

    fn measure(
        &mut self,
        session: &StartedSession,
        request: &SessionRequest,
    ) -> Result<EnvelopeMeasurement> {
        Ok(unboxed(session, request, NONE_UNBOXED))
    }
}

/// The record of a session that runs unboxed for `reason`: every measured field absent.
#[must_use]
pub fn unboxed(
    session: &StartedSession,
    request: &SessionRequest,
    reason: &str,
) -> EnvelopeMeasurement {
    EnvelopeMeasurement {
        session: session.clone(),
        measured_at: now(),
        provider: None,
        cgroup: None,
        memory_max: None,
        cpu_weight: None,
        tasks_max: None,
        places: request.writable.clone(),
        writable: None,
        fence_missing: None,
        unboxed: Some(reason.to_owned()),
    }
}

/// `$TMPDIR`, or `/tmp` when it is not set: the `scratch` place.
#[must_use]
pub fn tmpdir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .filter(|dir| !dir.is_empty())
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}

/// The time now, as a record writes it.
fn now() -> Timestamp {
    Timestamp(
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default(),
    )
}

/// Why a session started nothing, or was stopped again (`conductor.session.SessionStartRefusal`
/// and `conductor.dispatch.SpawnRefusal` share these three).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `strict`, and the provider is not available: nothing started.
    ProviderUnavailable(String),
    /// The provider's start, or the delivery of the first prompt, failed. `session` is the
    /// session the start answered, when the prompt failed after it.
    LaunchFailed {
        /// How it failed.
        detail: String,
        /// The session started before the prompt failed, if any.
        session: Option<StartedSession>,
    },
    /// `strict`, and the measured box is short of the request: the session was stopped.
    MeasurementShort(String),
}

/// The provider a session starts through, chosen by [`choose`].
pub struct Chosen {
    provider: Box<dyn Provider>,
    mode: ConfinementMode,
    /// Why the session runs unboxed though another provider was asked for.
    unboxed: Option<String>,
}

impl std::fmt::Debug for Chosen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chosen")
            .field("provider", &self.provider.name())
            .field("mode", &self.mode)
            .field("unboxed", &self.unboxed)
            .finish()
    }
}

/// A session [`Chosen::start`] started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    /// Its identity.
    pub session: StartedSession,
    /// What the harness wrote when it was prompted.
    pub said: Said,
    /// The box measured, as written to its record.
    pub measurement: EnvelopeMeasurement,
}

/// The provider `confinement` asks for, when it is available; under `report` the `none` provider
/// in its place when it is not, the reason kept for the record.
///
/// # Errors
///
/// `strict`, and the provider is not available ([`Refusal::ProviderUnavailable`]).
pub fn choose(providers: &dyn Providers, confinement: &Confinement) -> Result<Chosen, Refusal> {
    let mut provider = providers.provider(confinement.provider);
    let availability = provider.available();
    if availability.available {
        return Ok(Chosen {
            provider,
            mode: confinement.mode,
            unboxed: None,
        });
    }
    let reason = format!(
        "the provider {} is not available: {}",
        availability.provider,
        availability
            .reason
            .as_deref()
            .unwrap_or("it gave no reason")
    );
    match confinement.mode {
        ConfinementMode::Strict => Err(Refusal::ProviderUnavailable(reason)),
        ConfinementMode::Report => Ok(Chosen {
            provider: providers.unboxed(),
            mode: confinement.mode,
            unboxed: Some(reason),
        }),
    }
}

impl Chosen {
    /// The name of the provider the session starts through.
    #[must_use]
    pub fn name(&self) -> String {
        self.provider.name()
    }

    /// Starts `launch` through the chosen provider, delivers `prompt`, measures the box, and
    /// writes the measurement under `state` ([`record_path`]). Under `strict` a box short of the
    /// request is stopped and nothing is written.
    ///
    /// # Errors
    ///
    /// [`Refusal::LaunchFailed`] or [`Refusal::MeasurementShort`]; a record that cannot be
    /// written is a launch failure too, naming the session that runs.
    pub fn start(
        mut self,
        launch: &Launch,
        prompt: &str,
        state: &Path,
    ) -> Result<Started, Refusal> {
        let request = &launch.request;
        let mut session = self
            .provider
            .start(launch)
            .map_err(|error| Refusal::LaunchFailed {
                detail: format!("{error:#}"),
                session: None,
            })?;
        let said = match self.provider.prompt(&mut session, prompt) {
            Ok(said) => said,
            Err(error) => {
                return Err(Refusal::LaunchFailed {
                    detail: format!("{error:#}"),
                    session: Some(session),
                });
            }
        };
        let mut measurement = match self.provider.measure(&session, request) {
            Ok(measurement) => measurement,
            Err(error) => unboxed(
                &session,
                request,
                &format!("the box could not be measured: {error:#}"),
            ),
        };
        if let Some(reason) = self.unboxed.take() {
            measurement = unboxed(&session, request, &reason);
        }
        measurement.session = session.clone();
        if self.mode == ConfinementMode::Strict
            && let Some(short) = shortfall(&measurement, request)
        {
            let stopped = match self.provider.stop(&session, request) {
                Ok(()) => "the session was stopped".to_owned(),
                Err(error) => format!("stopping the session failed: {error:#}"),
            };
            return Err(Refusal::MeasurementShort(format!("{short}; {stopped}")));
        }
        write_measurement(state, &measurement).map_err(|error| Refusal::LaunchFailed {
            detail: format!(
                "the session {} started, and its measurement was not written: {error:#}",
                session.session_name.0
            ),
            session: Some(session.clone()),
        })?;
        Ok(Started {
            session,
            said,
            measurement,
        })
    }
}

/// How `measurement` falls short of `request`, or `None` when it does not: a session that runs
/// unboxed; a limit the request sets that reads back absent or different (`memory_max` compared
/// in bytes); a box with no write fence; or a fence that lets the session write a path none of
/// the request's places names.
#[must_use]
pub fn shortfall(measurement: &EnvelopeMeasurement, request: &SessionRequest) -> Option<String> {
    if let Some(reason) = &measurement.unboxed {
        return Some(format!("the session runs unboxed: {reason}"));
    }
    let Limits {
        memory_max,
        cpu_weight,
        tasks_max,
    } = &request.limits;
    let asked = [
        (
            "memory_max",
            memory_max.as_ref().map(|size| size.0 << 30),
            measurement.memory_max,
        ),
        ("cpu_weight", *cpu_weight, measurement.cpu_weight),
        ("tasks_max", *tasks_max, measurement.tasks_max),
    ];
    for (key, asked, read) in asked {
        if let Some(asked) = asked
            && read != Some(asked)
        {
            let read = read.map_or_else(|| "no limit".to_owned(), |read| read.to_string());
            return Some(format!("{key} reads back {read}, not {asked}"));
        }
    }
    let Some(writable) = &measurement.writable else {
        return Some(format!(
            "the box has no write fence: {}",
            measurement
                .fence_missing
                .as_deref()
                .unwrap_or("the provider gave no reason")
        ));
    };
    let named: Vec<&str> = request
        .writable
        .iter()
        .flat_map(|place| place.paths.iter().map(String::as_str))
        .collect();
    writable
        .split(':')
        .filter(|path| !path.is_empty())
        .find(|path| !named.contains(path))
        .map(|path| format!("the box lets the session write {path}, which no place names"))
}

/// Where the measurement of the session `name` is kept: `<state>/sessions/<name>.envelope.json`.
#[must_use]
pub fn record_path(state: &Path, name: &SessionName) -> PathBuf {
    state
        .join("sessions")
        .join(format!("{}.envelope.json", name.0))
}

/// Writes `measurement` to its [`record_path`], through a file beside it renamed into place.
///
/// # Errors
///
/// The directory or the file cannot be written.
pub fn write_measurement(state: &Path, measurement: &EnvelopeMeasurement) -> Result<()> {
    // A grouped repository's controller is named `<prefix>-<group>/<repo>`: its record lies one
    // directory down, and the file written first lies beside it.
    let file = record_path(state, &measurement.session.session_name);
    let dir = file.parent().unwrap_or(state);
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let written = dir.join(format!(
        ".{}.{}",
        file.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        std::process::id()
    ));
    let mut text = serde_json::to_vec_pretty(&measurement_json(measurement))
        .context("render the measurement")?;
    text.push(b'\n');
    fs::write(&written, text).with_context(|| format!("write {}", written.display()))?;
    fs::rename(&written, &file).with_context(|| format!("write {}", file.display()))
}

/// Reads the measurement of the session `name` under `state`; `None` when there is none.
///
/// # Errors
///
/// The record is there and cannot be read, or is no measurement.
pub fn read_measurement(state: &Path, name: &SessionName) -> Result<Option<EnvelopeMeasurement>> {
    let file = record_path(state, name);
    let text = match fs::read(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("read {}", file.display())),
    };
    let value: Value =
        serde_json::from_slice(&text).with_context(|| format!("{} is no JSON", file.display()))?;
    measurement_of(&value)
        .map(Some)
        .with_context(|| format!("{} is no measurement", file.display()))
}

/// Removes the measurement of the session `name`, if there is one.
///
/// # Errors
///
/// It is there and cannot be removed.
pub fn remove_measurement(state: &Path, name: &SessionName) -> Result<()> {
    let file = record_path(state, name);
    match fs::remove_file(&file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", file.display())),
    }
}

/// `provider` as the record writes it.
fn provider_word(provider: EnvelopeProvider) -> &'static str {
    match provider {
        EnvelopeProvider::Systemd => "systemd",
    }
}

/// The record's JSON: each field of `conductor.session.EnvelopeMeasurement` under its own name.
fn measurement_json(measurement: &EnvelopeMeasurement) -> Value {
    let EnvelopeMeasurement {
        session,
        measured_at,
        provider,
        cgroup,
        memory_max,
        cpu_weight,
        tasks_max,
        places,
        writable,
        fence_missing,
        unboxed,
    } = measurement;
    json!({
        "session": {
            "role": session.role,
            "session_name": session.session_name.0,
            "provider": session.provider,
            "unit": session.unit,
            "session_id": session.session_id,
        },
        "measured_at": measured_at.0,
        "provider": provider.map(provider_word),
        "cgroup": cgroup,
        "memory_max": memory_max,
        "cpu_weight": cpu_weight,
        "tasks_max": tasks_max,
        "places": places
            .iter()
            .map(|place| json!({"place": place.place.0, "paths": place.paths}))
            .collect::<Vec<_>>(),
        "writable": writable,
        "fence_missing": fence_missing,
        "unboxed": unboxed,
    })
}

/// The measurement a record's JSON holds.
fn measurement_of(value: &Value) -> Result<EnvelopeMeasurement> {
    let text = |value: &Value, key: &str| -> Result<String> {
        value[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("`{key}` is not text"))
    };
    let optional_text = |value: &Value, key: &str| value[key].as_str().map(str::to_owned);
    let session = &value["session"];
    let provider = match value["provider"].as_str() {
        None => None,
        Some("systemd") => Some(EnvelopeProvider::Systemd),
        Some(other) => bail!("`provider` {other:?} is no provider"),
    };
    let places = value["places"]
        .as_array()
        .ok_or_else(|| anyhow!("`places` is not a list"))?
        .iter()
        .map(|place| {
            Ok(PlaceResolved {
                place: conductor_model::config::Writable(text(place, "place")?),
                paths: place["paths"]
                    .as_array()
                    .ok_or_else(|| anyhow!("`paths` is not a list"))?
                    .iter()
                    .map(|path| {
                        path.as_str()
                            .map(str::to_owned)
                            .ok_or_else(|| anyhow!("a path is not text"))
                    })
                    .collect::<Result<_>>()?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(EnvelopeMeasurement {
        session: StartedSession {
            role: text(session, "role")?,
            session_name: SessionName(text(session, "session_name")?),
            provider: text(session, "provider")?,
            unit: optional_text(session, "unit"),
            session_id: optional_text(session, "session_id"),
        },
        measured_at: Timestamp(text(value, "measured_at")?),
        provider,
        cgroup: optional_text(value, "cgroup"),
        memory_max: value["memory_max"].as_i64(),
        cpu_weight: value["cpu_weight"].as_i64(),
        tasks_max: value["tasks_max"].as_i64(),
        places,
        writable: optional_text(value, "writable"),
        fence_missing: optional_text(value, "fence_missing"),
        unboxed: optional_text(value, "unboxed"),
    })
}

/// The id of a live session the Claude session list (`claude agents --json`) names `name`: an
/// entry with a `pid`, as the sessions collector counts one live, `id` else `sessionId`.
///
/// # Errors
///
/// The list cannot be run or read.
pub fn live_session(name: &str) -> Result<Option<String>> {
    let shown = format!("{CLAUDE} agents --json");
    let output = collect::run(Command::new(CLAUDE).args(["agents", "--json"]), BOUND)
        .with_context(|| format!("read the Claude session list `{shown}`"))?;
    if !output.status.success() {
        bail!("`{shown}` {}", exited(&output));
    }
    let Value::Array(entries) = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("`{shown}` printed no JSON"))?
    else {
        bail!("`{shown}` printed no JSON array");
    };
    Ok(entries
        .iter()
        .find(|entry| !entry["pid"].is_null() && entry["name"].as_str() == Some(name))
        .map(|entry| {
            entry["id"]
                .as_str()
                .or_else(|| entry["sessionId"].as_str())
                .unwrap_or("with no id")
                .to_owned()
        }))
}

/// Runs the launch `command` to its end within [`BOUND`], with no standard input, and answers its
/// exit status and output. Its standard output and error go to files of this process's own, not
/// to pipes: `claude --bg` leaves a session running, which may keep what it inherited open, and a
/// pipe would not reach its end before that session does. The files are removed afterwards. A
/// launch still running after the bound is killed, and the answer is an error.
fn run_launch(command: &mut Command) -> Result<Output> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let base = format!("conductor-spawn.{}.{stamp}", std::process::id());
    let files = ["out", "err"].map(|what| std::env::temp_dir().join(format!("{base}.{what}")));
    let ran = (|| -> Result<Output> {
        let mut streams = Vec::with_capacity(files.len());
        for file in &files {
            streams.push(
                File::options()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(file)
                    .with_context(|| format!("create {}", file.display()))?,
            );
        }
        let stderr = streams.pop().map(Stdio::from).unwrap_or_else(Stdio::null);
        let stdout = streams.pop().map(Stdio::from).unwrap_or_else(Stdio::null);
        let mut child = command
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .with_context(|| format!("start `{CLAUDE}`"))?;
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child
                .try_wait()
                .with_context(|| format!("wait for `{CLAUDE}`"))?
            {
                break status;
            }
            if started.elapsed() >= BOUND {
                let _ = child.kill();
                let _ = child.wait();
                bail!(
                    "gave no answer within {} s and was stopped",
                    BOUND.as_secs()
                );
            }
            thread::sleep(Duration::from_millis(20));
        };
        let [out, err] = &files;
        Ok(Output {
            status,
            stdout: fs::read(out).with_context(|| format!("read {}", out.display()))?,
            stderr: fs::read(err).with_context(|| format!("read {}", err.display()))?,
        })
    })();
    for file in &files {
        let _ = fs::remove_file(file);
    }
    ran
}

/// How a program that did not succeed ended, with what it wrote to standard error on one line.
#[must_use]
pub fn exited(output: &Output) -> String {
    let said: Vec<&str> = std::str::from_utf8(&output.stderr)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let how = output.status.code().map_or_else(
        || "was ended by a signal".to_owned(),
        |code| format!("exited {code}"),
    );
    if said.is_empty() {
        how
    } else {
        format!("{how}: {}", said.join("; "))
    }
}

/// Writes `said` through: the harness's standard error to this process's, its standard output
/// to this process's.
///
/// # Errors
///
/// Either cannot be written.
pub fn pass_through(said: &Said) -> Result<()> {
    std::io::stderr()
        .lock()
        .write_all(&said.stderr)
        .context("write what the harness said to standard error")?;
    std::io::stdout()
        .lock()
        .write_all(&said.stdout)
        .context("write what the harness printed to standard output")
}
