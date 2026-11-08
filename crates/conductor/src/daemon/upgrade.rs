//! Upgrading an instance's binary (`story:instance-upgrade`, design architecture-next § 4, § 10):
//! `POST /v1/instances/{name}/upgrade {"tag": "v<x.y.z>"}` on the daemon, and
//! `conductor instance upgrade`, its client.
//!
//! [`Daemon::upgrade`] takes one running instance from the binary its runtime runs to the
//! release `tag`, in four steps, each one `Reconciliation` of action `Upgrade` with the step first
//! in its detail:
//! - `fetch`: the release's archive for this host's target, verified against its `SHA256SUMS` and
//!   unpacked ([`Releases::fetch`]; [`Github`] is `conductor upgrade`'s download);
//! - `install`: the binary copied to `<daemon dir>/binaries/<version>/conductor`
//!   ([`BINARIES_DIR`]), which the runtime of a local placement runs; a remote placement waits for
//!   its substrate;
//! - `doctor`: the new binary's `conductor doctor`, with the runtime's environment and directory,
//!   before the switch; a failure is `RolledBack` and the runtime runs on, untouched;
//! - `restart`: the runtime stopped and started on the new binary, which becomes the instance's
//!   desired binary, the one it ran kept as its previous ([`super::reconcile::BINARIES_FILE`]).
//!
//! The instance is then `Upgrading` until [`Daemon::settle`] sees its runtime answer (`up`,
//! `Succeeded`), or puts the previous binary back and starts it (`rollback`, `RolledBack`): when
//! the runtime does not answer within `daemon.upgrade_timeout` (reason `Cadence`), or stops
//! running (reason `LostRuntime`). An upgrade is an operator's, the request's actor; a rollback
//! is the daemon's own.
//!
//! The daemon's API thread hands a request to the reconciling loop with [`post`], over the
//! channel [`listen`] opened; the loop answers it with [`Daemon::answer`].

use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::Instance;
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use conductor_model::operator::Actor;
use serde_json::{Value, json};

use super::reconcile::{Binary, Daemon, Done, Pinned, Started, Upgrading};
use super::units::Units;
use crate::cli::InstanceUpgradeArgs;
use crate::confine::systemd::Service;

/// Where the daemon installs each release's binary, under its directory: `<version>/conductor`.
pub const BINARIES_DIR: &str = "binaries";

/// Where a release is fetched and unpacked, under the daemon's directory, removed afterwards.
const WORK_DIR: &str = "upgrade";

/// How long the API waits for the loop to answer an upgrade.
const ANSWER_WAIT: Duration = Duration::from_secs(900);

/// How often `conductor instance upgrade` looks at `GET /v1/instances` while one upgrades.
const POLL: Duration = Duration::from_secs(1);

/// What fetches a release and runs a binary's doctor: [`Github`], or a test's fake.
pub trait Releases {
    /// Downloads the release `tag`'s archive for this host's target into `work`, verifies it,
    /// unpacks it and runs its binary's `--version`; answers the binary's path.
    ///
    /// # Errors
    ///
    /// The release, its archive or its sums cannot be read, the sum differs, or the binary does
    /// not run.
    fn fetch(&mut self, tag: &str, work: &Path) -> Result<PathBuf>;

    /// Runs `<binary> --config <file> doctor` in the environment and directory of `service`,
    /// the runtime it is to run.
    ///
    /// # Errors
    ///
    /// The doctor does not run or exits other than 0; the error says what it printed.
    fn doctor(&mut self, binary: &Path, service: &Service) -> Result<()>;
}

/// The releases of the repository this binary was built from, as `conductor upgrade` reads them
/// ([`crate::upgrade`]).
#[derive(Debug, Default, Clone, Copy)]
pub struct Github;

impl Releases for Github {
    fn fetch(&mut self, tag: &str, work: &Path) -> Result<PathBuf> {
        crate::upgrade::fetch_tag(tag, work).map_err(|refused| anyhow!("{refused}"))
    }

    fn doctor(&mut self, binary: &Path, service: &Service) -> Result<()> {
        let config = service
            .command
            .iter()
            .skip_while(|word| *word != "--config")
            .nth(1)
            .cloned()
            .unwrap_or_default();
        let mut command = Command::new(binary);
        command
            .args(["--config", &config, "doctor"])
            .current_dir(&service.cwd)
            .stdin(Stdio::null());
        for variable in &service.env {
            if let Some((name, value)) = variable.split_once('=') {
                command.env(name, value);
            }
        }
        let output = command
            .output()
            .with_context(|| format!("run {} doctor", binary.display()))?;
        if output.status.success() {
            return Ok(());
        }
        let said = [&output.stderr, &output.stdout]
            .iter()
            .map(|bytes| String::from_utf8_lossy(bytes).trim().to_owned())
            .find(|text| !text.is_empty())
            .unwrap_or_default();
        bail!(
            "{} doctor exited {}: {}",
            binary.display(),
            output.status,
            said.lines().next().unwrap_or_default()
        )
    }
}

/// What an upgrade that was not refused came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upgraded {
    /// The runtime runs on the new binary and has not answered yet.
    Upgrading {
        /// The new binary's version.
        version: String,
    },
    /// The new binary was not switched to, or was switched back; the runtime is on the previous.
    RolledBack {
        /// Why.
        detail: String,
    },
}

/// Why an upgrade was refused; nothing changed but what [`Refusal::Refused`] says was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `conductor.instance.UnknownInstance`: the config names no instance of this name.
    UnknownInstance(String),
    /// `conductor.instance.InstanceStateConflict`: the instance's runtime does not run, or it is
    /// upgrading already.
    InstanceStateConflict(String),
    /// The release was not fetched or its binary not installed; the step is recorded `Refused`.
    Refused {
        /// What failed.
        detail: String,
    },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownInstance(name) => {
                write!(
                    formatter,
                    "UnknownInstance: the config names no instance {name}"
                )
            }
            Self::InstanceStateConflict(name) => write!(
                formatter,
                "InstanceStateConflict: {name} is not running, or is upgrading already; only a \
                 running instance is upgraded"
            ),
            Self::Refused { detail } => write!(formatter, "refused: {detail}"),
        }
    }
}

/// The version a tag names: `v1.2.0` and `1.2.0` are `1.2.0`.
fn version_of(tag: &str) -> &str {
    tag.trim().trim_start_matches('v')
}

fn unix(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

impl<U: Units> Daemon<U> {
    /// `daemon.upgrade_timeout` in seconds, [`crate::config::UPGRADE_TIMEOUT`] when it does not read as seconds.
    #[must_use]
    pub fn upgrade_timeout(&self) -> u64 {
        crate::config::seconds(&self.config().daemon.upgrade_timeout)
            .unwrap_or(crate::config::UPGRADE_TIMEOUT)
    }

    /// Upgrades the instance `name` to the release `tag` for `actor` at `now`, as the module
    /// documentation says.
    ///
    /// # Errors
    ///
    /// A [`Refusal`]; a store that does not take a record or a runtime record that cannot be
    /// written is `Refused` too, naming it.
    pub fn upgrade(
        &mut self,
        name: &str,
        tag: &str,
        actor: &Actor,
        now: SystemTime,
    ) -> Result<Upgraded, Refusal> {
        let failed = |error: anyhow::Error| Refusal::Refused {
            detail: format!("{error:#}"),
        };
        let Some(instance) = self
            .config()
            .instances
            .iter()
            .find(|instance| instance.name.0 == name)
            .cloned()
        else {
            return Err(Refusal::UnknownInstance(name.to_owned()));
        };
        let held = self.started.get(name).cloned();
        let upgrading = self
            .pins
            .get(name)
            .is_some_and(|pinned| pinned.upgrading.is_some());
        let running = match &held {
            Some(held) if !upgrading => self.units.active(&held.unit).map_err(failed)?,
            _ => false,
        };
        let Some(held) = held.filter(|_| running) else {
            return Err(Refusal::InstanceStateConflict(name.to_owned()));
        };
        let version = version_of(tag).to_owned();
        let reason = ReconcileReason::OperatorCommand;
        let step = |daemon: &mut Self, result: ReconcileResult, detail: String| {
            daemon
                .put(
                    name,
                    ReconcileAction::Upgrade,
                    reason,
                    result,
                    Some(detail),
                    actor,
                )
                .map_err(failed)
        };

        let work = self.dir.join(WORK_DIR).join(&version);
        let _ = fs::remove_dir_all(&work);
        let fetched = fs::create_dir_all(&work)
            .with_context(|| format!("create {}", work.display()))
            .and_then(|()| self.releases.fetch(tag, &work));
        let fetched = match fetched {
            Ok(fetched) => fetched,
            Err(error) => {
                let _ = fs::remove_dir_all(&work);
                let detail = format!("fetch: {tag}: {error:#}");
                step(self, ReconcileResult::Refused, detail.clone())?;
                return Err(Refusal::Refused { detail });
            }
        };
        step(
            self,
            ReconcileResult::Succeeded,
            format!("fetch: {tag}: verified and unpacked"),
        )?;

        let installed = install(&fetched, &self.dir.join(BINARIES_DIR).join(&version));
        let _ = fs::remove_dir_all(&work);
        let path = match installed {
            Ok(path) => path,
            Err(error) => {
                let detail = format!("install: {error:#}");
                step(self, ReconcileResult::Refused, detail.clone())?;
                return Err(Refusal::Refused { detail });
            }
        };
        step(
            self,
            ReconcileResult::Succeeded,
            format!(
                "install: {} for placement {}",
                path.display(),
                instance.placement.0
            ),
        )?;
        let new = Binary { path, version };

        let token = crate::dashboard::mint()
            .context("mint the runtime's token")
            .map_err(failed)?;
        let service = self.service(&instance, &new, &token);
        if let Err(error) = self.releases.doctor(&new.path, &service) {
            let detail = format!("doctor: {error:#}; {name} stays on {}", held.version);
            step(self, ReconcileResult::RolledBack, detail.clone())?;
            return Ok(Upgraded::RolledBack { detail });
        }
        step(
            self,
            ReconcileResult::Succeeded,
            format!("doctor: {} doctor passed", new.version),
        )?;

        let previous = Binary {
            path: PathBuf::from(&held.binary),
            version: held.version.clone(),
        };
        let deadline = unix(now) + self.upgrade_timeout();
        self.pins.insert(
            name.to_owned(),
            Pinned {
                desired: new.clone(),
                previous: Some(previous.clone()),
                upgrading: Some(Upgrading {
                    version: new.version.clone(),
                    deadline,
                }),
            },
        );
        let restarted = self
            .units
            .stop(&held.unit)
            .and_then(|()| self.units.start(&service));
        match restarted {
            Ok(()) => {
                let at = crate::records::now().map_err(failed)?;
                self.started.insert(
                    name.to_owned(),
                    Started {
                        unit: held.unit.clone(),
                        placement: held.placement.clone(),
                        binary: new.path.display().to_string(),
                        version: new.version.clone(),
                        last_seen: Some(at.0),
                        token: Some(token.clone()),
                    },
                );
                self.running.insert(name.to_owned(), true);
                step(
                    self,
                    ReconcileResult::Succeeded,
                    format!(
                        "restart: on {}, {} kept to roll back to unless it answers within {} s",
                        new.version,
                        previous.version,
                        self.upgrade_timeout()
                    ),
                )?;
                self.save().map_err(failed)?;
                Ok(Upgraded::Upgrading {
                    version: new.version,
                })
            }
            Err(error) => {
                step(
                    self,
                    ReconcileResult::Refused,
                    format!("restart: {error:#}"),
                )?;
                let done = self
                    .roll_back(&instance, reason, "its restart was refused")
                    .map_err(failed)?;
                self.save().map_err(failed)?;
                Ok(Upgraded::RolledBack {
                    detail: done.detail.unwrap_or_default(),
                })
            }
        }
    }

    /// Ends each upgrade whose runtime answered, or that is to roll back at `now`, as the module
    /// documentation says; answers what it did, each recorded.
    ///
    /// # Errors
    ///
    /// The service manager cannot be asked, a reconciliation cannot be recorded, or the runtime
    /// record cannot be written.
    pub fn settle(&mut self, now: SystemTime) -> Result<Vec<Done>> {
        let mut done = Vec::new();
        let upgrading: Vec<(String, Upgrading)> = self
            .pins
            .iter()
            .filter_map(|(name, pinned)| Some((name.clone(), pinned.upgrading.clone()?)))
            .collect();
        for (name, upgrade) in upgrading {
            let Some(instance) = self
                .config()
                .instances
                .iter()
                .find(|instance| instance.name.0 == name)
                .cloned()
            else {
                continue;
            };
            let unit = self.started.get(&name).map(|held| held.unit.clone());
            let running = match &unit {
                Some(unit) => self.units.active(unit)?,
                None => false,
            };
            if running
                && unit
                    .as_deref()
                    .is_some_and(|unit| self.units.answers(unit).unwrap_or(false))
            {
                if let Some(pinned) = self.pins.get_mut(&name) {
                    pinned.upgrading = None;
                }
                done.push(self.put(
                    &name,
                    ReconcileAction::Upgrade,
                    ReconcileReason::OperatorCommand,
                    ReconcileResult::Succeeded,
                    Some(format!("up: {name} answers on {}", upgrade.version)),
                    &crate::actor::daemon(),
                )?);
            } else if !running {
                done.push(self.roll_back(
                    &instance,
                    ReconcileReason::LostRuntime,
                    "its runtime stopped running",
                )?);
            } else if unix(now) >= upgrade.deadline {
                let why = format!(
                    "it did not answer within {} s (daemon.upgrade_timeout)",
                    self.upgrade_timeout()
                );
                done.push(self.roll_back(&instance, ReconcileReason::Cadence, &why)?);
            }
        }
        if !done.is_empty() {
            self.save()?;
        }
        Ok(done)
    }

    /// Puts `instance`'s previous binary back as its desired one and starts its runtime on it,
    /// recorded `RolledBack` for `why`; with no previous binary recorded, ends the upgrade and
    /// leaves the runtime to the next pass.
    fn roll_back(
        &mut self,
        instance: &Instance,
        reason: ReconcileReason,
        why: &str,
    ) -> Result<Done> {
        let name = instance.name.0.clone();
        let pinned = self.pins.get(&name).cloned();
        let upgraded = pinned
            .as_ref()
            .and_then(|pinned| pinned.upgrading.as_ref())
            .map_or_else(String::new, |upgrading| upgrading.version.clone());
        let Some(previous) = pinned.and_then(|pinned| pinned.previous) else {
            if let Some(pinned) = self.pins.get_mut(&name) {
                pinned.upgrading = None;
            }
            return self.put(
                &name,
                ReconcileAction::Upgrade,
                reason,
                ReconcileResult::Refused,
                Some(format!(
                    "rollback: {upgraded}: {why}; no previous binary is recorded"
                )),
                &crate::actor::daemon(),
            );
        };
        self.pins.insert(
            name.clone(),
            Pinned {
                desired: previous.clone(),
                previous: None,
                upgrading: None,
            },
        );
        let token = crate::dashboard::mint().context("mint the runtime's token")?;
        let service = self.service(instance, &previous, &token);
        let unit = super::reconcile::unit_name(&name);
        let outcome = (|| {
            if self.units.active(&unit)? {
                self.units.stop(&unit)?;
            }
            self.units.start(&service)
        })();
        let (result, detail) = match outcome {
            Ok(()) => {
                let at = crate::records::now()?;
                self.started.insert(
                    name.clone(),
                    Started {
                        unit,
                        placement: instance.placement.0.clone(),
                        binary: previous.path.display().to_string(),
                        version: previous.version.clone(),
                        last_seen: Some(at.0),
                        token: Some(token.clone()),
                    },
                );
                self.running.insert(name.clone(), true);
                (
                    ReconcileResult::RolledBack,
                    format!("rollback: {upgraded}: {why}; back on {}", previous.version),
                )
            }
            Err(error) => {
                self.started.remove(&name);
                self.running.insert(name.clone(), false);
                (
                    ReconcileResult::Refused,
                    format!(
                        "rollback: {upgraded}: {why}; starting {} was refused: {error:#}",
                        previous.version
                    ),
                )
            }
        };
        self.put(
            &name,
            ReconcileAction::Upgrade,
            reason,
            result,
            Some(detail),
            &crate::actor::daemon(),
        )
    }

    /// Answers one request [`post`] handed over: the upgrade, then its reply.
    pub fn answer(&mut self, request: Request) {
        let reply = match self.upgrade(
            &request.name,
            &request.tag,
            &request.actor,
            SystemTime::now(),
        ) {
            Ok(Upgraded::Upgrading { version }) => (
                "202 Accepted",
                json!({"name": request.name, "state": "Upgrading", "binary_version": version}),
            ),
            Ok(Upgraded::RolledBack { detail }) => (
                "200 OK",
                json!({"name": request.name, "result": "RolledBack", "detail": detail}),
            ),
            Err(refusal) => {
                let status = match refusal {
                    Refusal::UnknownInstance(_) => "404 Not Found",
                    Refusal::InstanceStateConflict(_) => "409 Conflict",
                    Refusal::Refused { .. } => "502 Bad Gateway",
                };
                (status, json!({"error": refusal.to_string()}))
            }
        };
        let _ = request.reply.send(reply);
    }
}

/// Copies `fetched` to `<dir>/conductor`, executable, through a file beside it; answers that path.
fn install(fetched: &Path, dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join("conductor");
    let next = dir.join(".conductor.next");
    fs::copy(fetched, &next).with_context(|| format!("write {}", next.display()))?;
    fs::set_permissions(&next, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("make {} executable", next.display()))?;
    fs::rename(&next, &path).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// One `POST /v1/instances/{name}/upgrade`, handed from the API thread to the loop.
#[derive(Debug)]
pub struct Request {
    /// The instance.
    pub name: String,
    /// The release, `v<x.y.z>`.
    pub tag: String,
    /// Who asks.
    pub actor: Actor,
    /// Where the loop sends the answer: the status line and the JSON body.
    pub reply: Sender<(&'static str, Value)>,
}

/// Where [`post`] hands a request: the sender of the channel [`listen`] opened last.
static QUEUE: Mutex<Option<Sender<Request>>> = Mutex::new(None);

/// Opens the channel [`post`] hands requests over; the loop reads the answer's end.
pub fn listen() -> Receiver<Request> {
    let (sender, receiver) = mpsc::channel();
    if let Ok(mut queue) = QUEUE.lock() {
        *queue = Some(sender);
    }
    receiver
}

/// The instance an upgrade route names: `/v1/instances/{name}/upgrade`.
#[must_use]
pub fn route(path: &str) -> Option<&str> {
    let name = path
        .strip_prefix("/v1/instances/")?
        .strip_suffix("/upgrade")?;
    (!name.is_empty() && !name.contains('/')).then_some(name)
}

/// `POST /v1/instances/{name}/upgrade` with `body`, `{"tag": "v<x.y.z>"}`: hands it to the loop
/// and answers what the loop answered, `400` for a body without a tag and `503` when no loop
/// listens. `actor` is the operator the global API admitted for the write ([`super::api`]).
#[must_use]
pub fn post(path: &str, body: &[u8], actor: Actor) -> (&'static str, Value) {
    let Some(name) = route(path) else {
        return (
            "404 Not Found",
            json!({"error": format!("no route {path}")}),
        );
    };
    let tag = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|body| body.get("tag")?.as_str().map(str::to_owned))
        .filter(|tag| !tag.trim().is_empty());
    let Some(tag) = tag else {
        return (
            "400 Bad Request",
            json!({"error": "the body names no tag: write {\"tag\": \"v<x.y.z>\"}"}),
        );
    };
    let (reply, answer) = mpsc::channel();
    let request = Request {
        name: name.to_owned(),
        tag,
        actor,
        reply,
    };
    let sent = QUEUE
        .lock()
        .ok()
        .and_then(|queue| queue.as_ref().map(|queue| queue.send(request).is_ok()));
    if sent != Some(true) {
        return (
            "503 Service Unavailable",
            json!({"error": "the daemon's loop takes no upgrade"}),
        );
    }
    answer.recv_timeout(ANSWER_WAIT).unwrap_or_else(|_| {
        (
            "504 Gateway Timeout",
            json!({"error": "the daemon's loop did not answer the upgrade"}),
        )
    })
}

/// `conductor instance upgrade --instance <name> --tag <v> [--all]`: asks the daemon of the config
/// file (`127.0.0.1:<daemon.port>`) to upgrade one instance, or with `--all` each instance of the
/// file in its order, one at a time: each `POST /v1/instances/{name}/upgrade`, then
/// `GET /v1/instances` until it no longer upgrades. It prints one line per instance upgraded and
/// stops, exit 1, at the first rollback or refusal.
///
/// # Errors
///
/// No `--tag`; both or neither of `--instance` and `--all`; the config does not load; the daemon does not
/// answer; an upgrade is rolled back or refused.
pub fn instance_upgrade(
    config_flag: Option<&Path>,
    args: &InstanceUpgradeArgs,
) -> Result<ExitCode> {
    const WHAT: &str = "instance upgrade";
    let all = args.all == Some(true);
    let Some(tag) = args.tag.as_deref().filter(|tag| !tag.trim().is_empty()) else {
        bail!("{WHAT}: give --tag v<x.y.z>, the release to install");
    };
    let environment = crate::config::Environment::from_process().context(WHAT)?;
    let loaded = crate::config::load(config_flag, &environment).context(WHAT)?;
    let config = loaded.config;
    let names: Vec<String> = match (&args.instance, all) {
        (Some(_), true) => bail!("{WHAT}: --instance and --all are given; give one"),
        (None, false) => bail!("{WHAT}: give --instance <name> or --all"),
        (Some(name), false) => vec![name.clone()],
        (None, true) => config
            .instances
            .iter()
            .map(|instance| instance.name.0.clone())
            .collect(),
    };
    let port = u16::try_from(config.daemon.port).context("instance upgrade: daemon.port")?;
    let timeout = crate::config::seconds(&config.daemon.upgrade_timeout)
        .unwrap_or(crate::config::UPGRADE_TIMEOUT);
    let version = version_of(tag).to_owned();
    // The daemon takes a write from a local client with the token it keeps in its directory.
    let token = environment
        .home()
        .ok()
        .and_then(|home| super::api::kept_token(&home.join(super::DIR)));
    for name in names {
        let (status, body) = request(
            token.as_deref(),
            port,
            "POST",
            &format!("/v1/instances/{name}/upgrade"),
            Some(&json!({"tag": tag}).to_string()),
        )
        .with_context(|| format!("{WHAT}: {name}"))?;
        match status {
            202 => {}
            200 => bail!(
                "{WHAT}: {name}: rolled back: {}",
                body["detail"].as_str().unwrap_or_default()
            ),
            _ => bail!(
                "{WHAT}: {name}: refused: {}",
                body["error"]
                    .as_str()
                    .unwrap_or("the daemon answered no reason")
            ),
        }
        let row = wait_upgraded(port, &name, timeout).with_context(|| format!("{WHAT}: {name}"))?;
        let now_on = row["binary_version"].as_str().unwrap_or("unknown");
        if now_on != version {
            bail!("{WHAT}: {name}: rolled back to {now_on}: {version} did not come up");
        }
        println!("{name}: upgraded to {version}");
    }
    Ok(ExitCode::SUCCESS)
}

/// `name`'s row of `GET /v1/instances` once it no longer upgrades, within the timeout and a
/// minute.
fn wait_upgraded(port: u16, name: &str, timeout: u64) -> Result<Value> {
    let bound = Duration::from_secs(timeout + 60);
    let started = std::time::Instant::now();
    loop {
        let (_, rows) = request(None, port, "GET", super::api::INSTANCES, None)?;
        let row = rows
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["name"] == name))
            .cloned()
            .ok_or_else(|| anyhow!("GET {} lists no {name}", super::api::INSTANCES))?;
        if row["state"] != "Upgrading" {
            return Ok(row);
        }
        if started.elapsed() > bound {
            bail!("still upgrading after {} s", bound.as_secs());
        }
        thread::sleep(POLL);
    }
}

/// One HTTP/1.1 request to `127.0.0.1:port`; answers the status code and the JSON body.
fn request(
    token: Option<&str>,
    port: u16,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<(u16, Value)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("connect to the daemon on 127.0.0.1:{port}"))?;
    let body = body.unwrap_or_default();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
         {}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        token.map_or_else(String::new, |token| format!(
            "Origin: http://127.0.0.1:{port}\r\n{}: {token}\r\n",
            crate::dashboard::TOKEN_HEADER
        )),
        body.len()
    )?;
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let status = line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| anyhow!("the daemon answered {:?}", line.trim()))?;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
    }
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    let body = serde_json::from_str(&text).unwrap_or(Value::Null);
    Ok((status, body))
}
