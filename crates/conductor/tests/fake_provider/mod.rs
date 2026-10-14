//! A confinement provider that starts nothing, for the cases of `story:session-envelope-seam`.
//!
//! A case runs a `conductor` command with these providers in place of the host's: the test binary
//! runs itself again as a child ([`command`]), and its `child` case parses the command line and
//! runs it through [`Cli::run_with`] over [`Fakes`] ([`Child::exit`]), with the case's `HOME`,
//! `PATH` and working directory, as the built binary would run it. Nothing reaches a real
//! `systemd-run`, `systemctl` or `claude` launch.
//!
//! The fakes read how to behave from files in the case's fake directory, and append one JSON line
//! per call to `calls` there:
//!
//! | file | effect |
//! |---|---|
//! | `<name>.unavailable` | the provider `<name>` is not available; the file's text is the reason |
//! | `providers` | the providers `all` answers, one name per line (default: `systemd`, `none`) |
//! | `start.fails`, `prompt.fails`, `stop.fails` | that call fails with the file's text |
//! | `measure.json` | fields that replace those of the full box `measure` answers otherwise |
//!
//! The full box: the provider `systemd`, the cgroup `/fake/<session name>`, each limit the
//! request sets read back as set (`memory_max` in bytes), and every path the request's places
//! name, joined by `:`. The `none` fake measures the session unboxed. `prompt` gives the session
//! the id `fake-<session name>`.

#![allow(dead_code)]

use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio};

use anyhow::{Result, bail};
use clap::Parser as _;
use conductor_cli::cli::Cli;
use conductor_cli::confine::{Launch, Provider, Providers, Said};
use conductor_cli::{NOT_IMPLEMENTED_EXIT, NotImplemented};
use conductor_model::config::{ConfinementProvider, ProviderAvailability};
use conductor_model::observation::EnvelopeProvider;
use conductor_model::primitives::Timestamp;
use conductor_model::session::{EnvelopeMeasurement, SessionRequest, StartedSession};
use serde_json::{Value, json};

/// The variable that holds the fake directory of a child run.
pub const FAKE: &str = "CONDUCTOR_TEST_FAKE_PROVIDER";

/// The variable that holds a child run's command line, as a JSON list after `conductor`.
pub const ARGS: &str = "CONDUCTOR_TEST_CHILD_ARGS";

/// The providers of a case: each a [`Fake`] over the case's fake directory.
#[derive(Debug, Clone)]
pub struct Fakes {
    pub dir: PathBuf,
}

impl Providers for Fakes {
    fn provider(&self, name: ConfinementProvider) -> Box<dyn Provider> {
        let name = match name {
            ConfinementProvider::Systemd => "systemd",
            ConfinementProvider::None => "none",
        };
        Box::new(Fake::new(&self.dir, name))
    }

    fn unboxed(&self) -> Box<dyn Provider> {
        Box::new(Fake::new(&self.dir, "none"))
    }

    fn all(&self) -> Vec<Box<dyn Provider>> {
        let names = fs::read_to_string(self.dir.join("providers"))
            .unwrap_or_else(|_| "systemd\nnone\n".to_owned());
        names
            .lines()
            .filter(|name| !name.trim().is_empty())
            .map(|name| Box::new(Fake::new(&self.dir, name.trim())) as Box<dyn Provider>)
            .collect()
    }
}

/// One fake provider; see the module documentation.
#[derive(Debug, Clone)]
pub struct Fake {
    dir: PathBuf,
    name: String,
}

impl Fake {
    fn new(dir: &Path, name: &str) -> Self {
        Self {
            dir: dir.to_owned(),
            name: name.to_owned(),
        }
    }

    fn log(&self, call: Value) {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("calls"))
            .expect("open the fake's call log");
        writeln!(file, "{call}").expect("log a call");
    }

    fn fails(&self, call: &str) -> Result<()> {
        match fs::read_to_string(self.dir.join(format!("{call}.fails"))) {
            Ok(why) => bail!("{}", why.trim()),
            Err(_) => Ok(()),
        }
    }
}

/// `request` as the call log writes it.
pub fn request_json(request: &SessionRequest) -> Value {
    json!({
        "role": request.role,
        "session_name": request.session_name.0,
        "cwd": request.cwd,
        "memory_max": request.limits.memory_max.as_ref().map(|size| size.0),
        "cpu_weight": request.limits.cpu_weight,
        "tasks_max": request.limits.tasks_max,
        "writable": request.writable.iter().map(|place| json!({
            "place": place.place.0, "paths": place.paths,
        })).collect::<Vec<_>>(),
        "forge_token_env": request.forge_token_env,
    })
}

impl Provider for Fake {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn available(&mut self) -> ProviderAvailability {
        let reason = fs::read_to_string(self.dir.join(format!("{}.unavailable", self.name)))
            .ok()
            .map(|why| why.trim().to_owned());
        ProviderAvailability {
            provider: self.name.clone(),
            available: reason.is_none(),
            reason,
        }
    }

    fn start(&mut self, launch: &Launch) -> Result<StartedSession> {
        self.log(json!({
            "call": "start", "provider": self.name, "request": request_json(&launch.request),
            "program": launch.program, "args": launch.args,
        }));
        self.fails("start")?;
        Ok(StartedSession {
            role: launch.request.role.clone(),
            session_name: launch.request.session_name.clone(),
            provider: self.name.clone(),
            unit: (self.name != "none")
                .then(|| format!("conductor-{}.service", launch.request.session_name.0)),
            session_id: None,
        })
    }

    fn prompt(&mut self, session: &mut StartedSession, prompt: &str) -> Result<Said> {
        self.log(json!({
            "call": "prompt", "provider": self.name, "session": session.session_name.0,
            "prompt": prompt,
        }));
        self.fails("prompt")?;
        session.session_id = Some(format!("fake-{}", session.session_name.0));
        Ok(Said::default())
    }

    fn stop(&mut self, session: &StartedSession, request: &SessionRequest) -> Result<()> {
        self.log(json!({
            "call": "stop", "provider": self.name, "session": session.session_name.0,
            "session_id": session.session_id, "cwd": request.cwd,
        }));
        self.fails("stop")
    }

    fn resume(&mut self, session: &StartedSession, _launch: &Launch) -> Result<StartedSession> {
        self.log(json!({"call": "resume", "provider": self.name}));
        Ok(session.clone())
    }

    fn measure(
        &mut self,
        session: &StartedSession,
        request: &SessionRequest,
    ) -> Result<EnvelopeMeasurement> {
        self.log(json!({"call": "measure", "provider": self.name}));
        let mut measurement = EnvelopeMeasurement {
            session: session.clone(),
            measured_at: Timestamp("2026-10-14T12:00:00Z".to_owned()),
            provider: Some(EnvelopeProvider::Systemd),
            cgroup: Some(format!("/fake/{}", session.session_name.0)),
            memory_max: request.limits.memory_max.as_ref().map(|size| size.0 << 30),
            cpu_weight: request.limits.cpu_weight,
            tasks_max: request.limits.tasks_max,
            places: request.writable.clone(),
            writable: Some(
                request
                    .writable
                    .iter()
                    .flat_map(|place| place.paths.iter().cloned())
                    .collect::<Vec<_>>()
                    .join(":"),
            ),
            fence_missing: None,
            unboxed: None,
        };
        if self.name == "none" {
            measurement.provider = None;
            measurement.cgroup = None;
            measurement.memory_max = None;
            measurement.cpu_weight = None;
            measurement.tasks_max = None;
            measurement.writable = None;
            measurement.unboxed = Some("the fake none sets up no box".to_owned());
        }
        if let Ok(text) = fs::read_to_string(self.dir.join("measure.json")) {
            let fields: Value = serde_json::from_str(&text).expect("measure.json is JSON");
            for (key, value) in fields.as_object().expect("measure.json is an object") {
                let text = value.as_str().map(str::to_owned);
                match key.as_str() {
                    "memory_max" => measurement.memory_max = value.as_i64(),
                    "cpu_weight" => measurement.cpu_weight = value.as_i64(),
                    "tasks_max" => measurement.tasks_max = value.as_i64(),
                    "writable" => measurement.writable = text,
                    "fence_missing" => measurement.fence_missing = text,
                    "unboxed" => measurement.unboxed = text,
                    other => panic!("measure.json: no field {other}"),
                }
            }
        }
        Ok(measurement)
    }
}

/// The calls the fakes logged in `dir`, in order.
pub fn calls(dir: &Path) -> Vec<Value> {
    fs::read_to_string(dir.join("calls"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("a logged call"))
        .collect()
}

/// The names of the calls the fakes logged in `dir`, in order.
pub fn call_names(dir: &Path) -> Vec<String> {
    calls(dir)
        .iter()
        .map(|call| call["call"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// The child run of a test binary, when this process is one.
pub struct Child {
    args: Vec<String>,
    fake: PathBuf,
}

impl Child {
    /// This process's child run, when [`ARGS`] and [`FAKE`] say it is one.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let args = env::var(ARGS).ok()?;
        let fake = env::var_os(FAKE)?;
        Some(Self {
            args: serde_json::from_str(&args).expect("the child's arguments are a JSON list"),
            fake: PathBuf::from(fake),
        })
    }

    /// The config file the command line names with `--config`, else `CONDUCTOR_CONFIG`.
    #[must_use]
    pub fn config_file(&self) -> Option<PathBuf> {
        self.args
            .iter()
            .position(|arg| arg == "--config")
            .and_then(|at| self.args.get(at + 1))
            .map(PathBuf::from)
            .or_else(|| env::var_os("CONDUCTOR_CONFIG").map(PathBuf::from))
    }

    /// Runs the command line over [`Fakes`] and ends this process with its exit status, as the
    /// binary's `main` does: a failure on one line of standard error, exit 1, or 2 for a handler
    /// not written yet.
    pub fn exit(self) -> ! {
        let mut argv = vec!["conductor".to_owned()];
        argv.extend(self.args);
        let code = match Cli::try_parse_from(argv) {
            Err(error) => {
                let _ = error.print();
                2
            }
            Ok(cli) => match cli.run_with(&Fakes { dir: self.fake }) {
                Ok(code) if code == ExitCode::SUCCESS => 0,
                Ok(_) => 1,
                Err(error) => {
                    eprintln!("conductor: {error:#}");
                    if error.is::<NotImplemented>() {
                        i32::from(NOT_IMPLEMENTED_EXIT)
                    } else {
                        1
                    }
                }
            },
        };
        let _ = std::io::stdout().flush();
        std::process::exit(code)
    }
}

/// The command that runs `conductor <args>` over the fakes of `fake`: this test binary's `child`
/// case, with `HOME` naming `home` and neither config variable set. The case sets the working
/// directory, `PATH` and a config file as it would on the binary's command.
pub fn command(home: &Path, fake: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env::current_exe().expect("this test binary"));
    command
        .args([
            "--exact",
            "child",
            "--nocapture",
            "--test-threads",
            "1",
            "-q",
        ])
        .env("HOME", home)
        .env_remove("CONDUCTOR_CONFIG")
        .env_remove("CONDUCTOR_INSTANCE")
        .env(FAKE, fake)
        .env(
            ARGS,
            serde_json::to_string(args).expect("the arguments as JSON"),
        )
        .stdin(Stdio::null());
    command
}

/// [`command`] with the environment cleared first: only `HOME` and the child's own variables
/// are set; the case sets `PATH`.
pub fn command_in_clear_env(home: &Path, fake: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env::current_exe().expect("this test binary"));
    command
        .args([
            "--exact",
            "child",
            "--nocapture",
            "--test-threads",
            "1",
            "-q",
        ])
        .env_clear()
        .env("HOME", home)
        .env(FAKE, fake)
        .env(
            ARGS,
            serde_json::to_string(args).expect("the arguments as JSON"),
        )
        .stdin(Stdio::null());
    command
}

/// `output` with the test harness's own header (`running 1 test`) taken off its standard output,
/// so what is left is what the command printed.
#[must_use]
pub fn without_harness(mut output: Output) -> Output {
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let kept: Vec<&str> = text
        .lines()
        .skip_while(|line| line.trim().is_empty() || line.starts_with("running "))
        .collect();
    output.stdout = if kept.is_empty() {
        Vec::new()
    } else {
        format!("{}\n", kept.join("\n")).into_bytes()
    };
    output
}
