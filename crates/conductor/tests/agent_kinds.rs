//! `story:agent-kinds`: the agent trait (`conductor_cli::agent::Kind`), the `claude-code` kind and
//! a second, run-to-exit kind registered by this test only.
//!
//! - The run-to-exit kind is a fake program that writes an events file and exits with a code: the
//!   runtime writes one `[REPORT]` line from the exit code and the file for each exit, an exit the
//!   kind classes as stopped becomes a new generation, a failure does not, and no interactive route
//!   is offered for it (`attach` and `deliver` answer `CapabilityMissing` by name).
//! - `claude-code` is the built-in kind: interactive, on `claude`; a role slot reads its kind from
//!   the effective settings, `claude-code` by default.
//! - `conductor agent start|stop --instance <i> --role <r>` go through the role's kind, a kind the
//!   binary does not implement is refused by name, `roles[].kind` takes any kind word, and
//!   `conductor doctor` names each configured kind's binary and version.
//!
//! Each case is a directory of its own in this test target's temporary directory.

mod active;
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use conductor_cli::agent::{
    self, AgentState, Capability, Context, ExitClass, Handle, Kind, PtyStream, Refused, Report,
    Route, Runtime, Slot,
};
use conductor_cli::config;
use conductor_cli::message;
use conductor_model::agent::{AgentKindName, AgentName};
use conductor_model::config::{InstanceName, Limits, PlacementName};
use conductor_model::dispatch::MessageKind;

/// A case directory of its own, removed when the case passes.
struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("agent-kinds")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("bin")).expect("create the case's bin");
        Self { dir }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.dir.join(name);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        fs::write(&path, text).expect("write a case file");
        path
    }

    fn script(&self, name: &str, text: &str) -> PathBuf {
        let path = self.write(name, text);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        path
    }
}

// -------------------------------------------------------------------------------------------
// The run-to-exit kind of this test.
// -------------------------------------------------------------------------------------------

/// The exit code the fake program's kind classes as stopped (resumable): a new generation.
const STOPPED: i32 = 3;

/// A run-to-exit kind: `start` spawns `program` with the events file to write as its argument;
/// `reports` answers its exit once it ended, with the code and the events file.
struct RunToExit {
    program: PathBuf,
    events: PathBuf,
    children: Mutex<BTreeMap<String, Child>>,
}

impl RunToExit {
    fn new(program: PathBuf, events: PathBuf) -> Self {
        Self {
            program,
            events,
            children: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Kind for RunToExit {
    fn name(&self) -> AgentKindName {
        AgentKindName("example-run-to-exit".to_owned())
    }

    fn binary(&self) -> String {
        self.program.display().to_string()
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::RunToExit]
    }

    fn start(&self, _context: &Context<'_>, slot: &Slot) -> Result<Handle, Refused> {
        let child = Command::new(&self.program)
            .arg(&self.events)
            .spawn()
            .map_err(|error| Refused::launch_failed(&slot.name, format!("{error}")))?;
        let id = child.id().to_string();
        self.children
            .lock()
            .expect("the children")
            .insert(slot.name.0.clone(), child);
        Ok(Handle {
            name: slot.name.clone(),
            kind: self.name(),
            id: Some(id),
        })
    }

    fn reports(&self, _context: &Context<'_>, handle: &Handle) -> Result<Vec<Report>, Refused> {
        let mut children = self.children.lock().expect("the children");
        let Some(child) = children.get_mut(&handle.name.0) else {
            return Ok(Vec::new());
        };
        match child
            .try_wait()
            .map_err(|error| Refused::Failed(error.into()))?
        {
            None => Ok(Vec::new()),
            Some(status) => {
                children.remove(&handle.name.0);
                let code = status.code().unwrap_or(-1);
                Ok(vec![Report::Exited {
                    code,
                    events: Some(self.events.clone()),
                    stopped: code == STOPPED,
                }])
            }
        }
    }

    fn stop(&self, _context: &Context<'_>, handle: &Handle) -> Result<(), Refused> {
        if let Some(mut child) = self
            .children
            .lock()
            .expect("the children")
            .remove(&handle.name.0)
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

/// The fake program: it appends one event to the events file it is given, then exits with the
/// code of the first line of `codes` it has not used yet (one line per generation), else 0.
fn fake_program(case: &Case, codes: &[i32]) -> (PathBuf, PathBuf) {
    let codes_file = case.write(
        "codes",
        &codes
            .iter()
            .map(|code| format!("{code}\n"))
            .collect::<String>(),
    );
    let used = case.dir.join("used");
    let program = case.script(
        "fake-run",
        &format!(
            "#!/bin/sh\n\
             n=$(cat '{used}' 2>/dev/null || echo 0)\n\
             n=$((n + 1))\n\
             echo $n > '{used}'\n\
             code=$(sed -n \"${{n}}p\" '{codes}')\n\
             printf '{{\"event\": \"run\", \"generation\": %s, \"summary\": \"unit worked\"}}\\n' $n >> \"$1\"\n\
             exit ${{code:-0}}\n",
            used = used.display(),
            codes = codes_file.display(),
        ),
    );
    (program, case.dir.join("events.jsonl"))
}

/// The report lines the runtime wrote, by agent.
type Lines = Arc<Mutex<Vec<(String, String)>>>;

fn runtime(kind: RunToExit) -> (Runtime, Lines) {
    let lines: Lines = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&lines);
    let mut runtime = Runtime::new(Box::new(move |agent: &AgentName, line: &str| {
        sink.lock()
            .expect("the lines")
            .push((agent.0.clone(), line.to_owned()));
        Ok(())
    }));
    runtime.register(Box::new(kind));
    (runtime, lines)
}

/// A worker slot on the run-to-exit kind.
fn worker_slot() -> Slot {
    Slot {
        name: AgentName("example-org-implementer".to_owned()),
        instance: InstanceName("example-org".to_owned()),
        role: "implementer".to_owned(),
        kind: AgentKindName("example-run-to-exit".to_owned()),
        placement: PlacementName("local".to_owned()),
        profile: None,
        limits: Limits {
            memory_max: None,
            cpu_weight: None,
            tasks_max: None,
        },
    }
}

/// Polls the runtime until the agent is stopped, or ten seconds pass.
fn poll_until_stopped(runtime: &mut Runtime, context: &Context<'_>, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        runtime.poll(context).expect("the runtime polls");
        if runtime
            .agent(name)
            .is_some_and(|agent| agent.state == AgentState::Stopped)
        {
            return;
        }
        assert!(Instant::now() < deadline, "the agent {name} never stopped");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn context(providers: &dyn conductor_cli::confine::Providers) -> Context<'_> {
    Context {
        config: None,
        state: None,
        providers,
    }
}

/// Asserts `line` is one § 6 report line naming the exit code and the events file.
fn assert_report(line: &str, verb: &str, code: i32, events: &Path) {
    let reading = message::read(line);
    assert_eq!(reading.kind, Some(MessageKind::Report), "{line}");
    assert!(reading.parsed, "the line parses as a report: {line}");
    assert!(
        line.starts_with("[REPORT example-org-implementer -] "),
        "{line}"
    );
    let rest = line.split_once("] ").expect("a verb").1;
    assert!(rest.starts_with(verb), "{line}");
    assert!(line.contains(&format!("exit {code}")), "{line}");
    assert!(line.contains(&events.display().to_string()), "{line}");
    assert!(line.contains("unit worked"), "{line}");
}

/// Exit 0: one `[REPORT … done` line from the code and the file, the agent stopped as `Exited`
/// with its code, still generation 1.
#[test]
fn a_run_to_exit_kind_reports_one_line_from_its_exit_code_and_events_file() {
    active::isolate();
    let case = Case::new("exit-zero");
    let (program, events) = fake_program(&case, &[0]);
    let (mut runtime, lines) = runtime(RunToExit::new(program, events.clone()));
    let host = conductor_cli::confine::Host::default();
    let context = context(&host);
    let started = runtime
        .start(&context, worker_slot())
        .expect("the worker starts");
    assert_eq!(started.state, AgentState::Running);
    assert_eq!(started.generation, 1);
    poll_until_stopped(&mut runtime, &context, "example-org-implementer");
    let agent = runtime.agent("example-org-implementer").expect("on record");
    assert_eq!(agent.generation, 1);
    assert_eq!(agent.exit_class, Some(ExitClass::Exited));
    assert_eq!(agent.exit_code, Some(0));
    let lines = lines.lock().expect("the lines").clone();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].0, "example-org-implementer");
    assert_report(&lines[0].1, "done", 0, &events);
}

/// An exit of the stopped class becomes a new generation: one line for it, the slot started
/// again as generation 2, which exits 0 and writes its own line.
#[test]
fn a_stopped_class_exit_becomes_a_new_generation() {
    active::isolate();
    let case = Case::new("exit-stopped");
    let (program, events) = fake_program(&case, &[STOPPED, 0]);
    let (mut runtime, lines) = runtime(RunToExit::new(program, events.clone()));
    let host = conductor_cli::confine::Host::default();
    let context = context(&host);
    runtime
        .start(&context, worker_slot())
        .expect("the worker starts");
    let deadline = Instant::now() + Duration::from_secs(10);
    while lines.lock().expect("the lines").len() < 2 {
        runtime.poll(&context).expect("the runtime polls");
        assert!(Instant::now() < deadline, "no second generation ended");
        std::thread::sleep(Duration::from_millis(20));
    }
    let agent = runtime.agent("example-org-implementer").expect("on record");
    assert_eq!(agent.generation, 2, "the stopped exit started generation 2");
    assert_eq!(agent.state, AgentState::Stopped);
    assert_eq!(agent.exit_code, Some(0));
    let lines = lines.lock().expect("the lines").clone();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_report(&lines[0].1, "blocked", STOPPED, &events);
    assert_report(&lines[1].1, "done", 0, &events);
}

/// A non-zero exit outside the stopped class is no new generation: one `failed` line, stopped.
#[test]
fn a_failed_exit_is_no_new_generation() {
    active::isolate();
    let case = Case::new("exit-failed");
    let (program, events) = fake_program(&case, &[1, 0]);
    let (mut runtime, lines) = runtime(RunToExit::new(program, events.clone()));
    let host = conductor_cli::confine::Host::default();
    let context = context(&host);
    runtime
        .start(&context, worker_slot())
        .expect("the worker starts");
    poll_until_stopped(&mut runtime, &context, "example-org-implementer");
    for _ in 0..5 {
        runtime.poll(&context).expect("the runtime polls");
    }
    let agent = runtime.agent("example-org-implementer").expect("on record");
    assert_eq!(agent.generation, 1);
    assert_eq!(agent.exit_code, Some(1));
    let lines = lines.lock().expect("the lines").clone();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_report(&lines[0].1, "failed", 1, &events);
}

/// No interactive route for a run-to-exit agent: its routes are `restart` only, and `attach` and
/// `deliver` answer the capability refusal by name.
#[test]
fn a_run_to_exit_agent_offers_no_interactive_route() {
    active::isolate();
    let case = Case::new("no-attach");
    let (program, events) = fake_program(&case, &[0]);
    let (mut runtime, _lines) = runtime(RunToExit::new(program, events));
    let host = conductor_cli::confine::Host::default();
    let context = context(&host);
    runtime
        .start(&context, worker_slot())
        .expect("the worker starts");
    assert_eq!(
        runtime
            .routes("example-org-implementer")
            .expect("the agent's routes"),
        vec![Route::Restart]
    );
    let refused: Result<PtyStream, Refused> =
        runtime.attach(&context, "example-org-implementer", false);
    let error = refused.expect_err("attach is refused").to_string();
    assert!(
        error.starts_with("refused: CapabilityMissing: agent example-org-implementer"),
        "{error}"
    );
    assert!(error.contains("Interactive"), "{error}");
    let error = runtime
        .deliver(&context, "example-org-implementer", "[DISPATCH a d-1] go")
        .expect_err("deliver is refused")
        .to_string();
    assert!(error.starts_with("refused: CapabilityMissing: "), "{error}");
    poll_until_stopped(&mut runtime, &context, "example-org-implementer");
}

/// A start on a kind the runtime does not implement is refused by name and records nothing; a
/// second start of a name on record is refused.
#[test]
fn an_unknown_kind_and_a_second_start_are_refused() {
    active::isolate();
    let case = Case::new("unknown-kind");
    let (program, events) = fake_program(&case, &[0]);
    let (mut runtime, _lines) = runtime(RunToExit::new(program, events));
    let host = conductor_cli::confine::Host::default();
    let context = context(&host);
    let mut slot = worker_slot();
    slot.kind = AgentKindName("example-absent".to_owned());
    let error = runtime
        .start(&context, slot)
        .map(|_| ())
        .expect_err("refused")
        .to_string();
    assert!(error.starts_with("refused: UnknownAgentKind: "), "{error}");
    assert!(runtime.agent("example-org-implementer").is_none());
    runtime.start(&context, worker_slot()).expect("starts");
    let error = runtime
        .start(&context, worker_slot())
        .map(|_| ())
        .expect_err("refused")
        .to_string();
    assert!(error.starts_with("refused: AgentExists: "), "{error}");
    poll_until_stopped(&mut runtime, &context, "example-org-implementer");
}

/// `claude-code` is built in: interactive, on `claude`, every route offered; a role slot reads
/// its kind from the effective settings, `claude-code` by default.
#[test]
fn claude_code_is_the_built_in_interactive_kind_and_the_default_of_a_slot() {
    active::isolate();
    let runtime = Runtime::new(Box::new(|_: &AgentName, _: &str| Ok(())));
    let kind = runtime
        .kind("claude-code")
        .expect("claude-code is built in");
    assert_eq!(kind.capabilities(), vec![Capability::Interactive]);
    assert_eq!(kind.binary(), "claude");
    assert_eq!(
        agent::routes(&kind.capabilities()),
        vec![Route::Attach, Route::Deliver, Route::Restart]
    );
    let instance = &config::active().instance;
    let slot = agent::slot(instance, "director").expect("the director's slot");
    assert_eq!(slot.kind.0, "claude-code");
    assert!(
        config::is_role(&slot.role, config::BuiltInRole::Director),
        "{slot:?}"
    );
    assert_eq!(slot.instance, instance.name);
    assert_eq!(slot.placement, instance.placement);
    assert_eq!(
        slot.name,
        AgentName(config::session_name(instance, &slot.role).0)
    );
}

// -------------------------------------------------------------------------------------------
// The CLI.
// -------------------------------------------------------------------------------------------

/// A `conductor.config/2` file of one instance `alpha` whose controller runs on a kind this
/// binary does not implement, and a `claude` on the case's `PATH` that lists no live session.
fn cli_case(name: &str) -> (Case, PathBuf) {
    let case = Case::new(name);
    let file = case.write(
        "conductor.yaml",
        &format!(
            "version: conductor.config/2\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: systemd\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
             \x20   records: {records}\n\
             \x20   state: {state}\n\
             \x20   roles:\n\
             \x20     - {{role: director, harness: claude, model: opus}}\n\
             \x20     - {{role: controller, kind: example-run-to-exit, harness: claude, model: opus}}\n",
            root = case.dir.join("home/example-org").display(),
            trees = case.dir.join("trees").display(),
            records = case.dir.join("records").display(),
            state = case.dir.join("state").display(),
        ),
    );
    case.script(
        "bin/claude",
        "#!/bin/sh\n\
         if [ \"$1\" = agents ]; then echo '[]'; exit 0; fi\n\
         if [ \"$1\" = --version ]; then echo '9.9.9 (Claude Code)'; exit 0; fi\n\
         exit 0\n",
    );
    (case, file)
}

fn run(case: &Case, file: &Path, args: &[&str]) -> Output {
    let path = format!("{}:/usr/bin:/bin", case.dir.join("bin").display());
    common::conductor(case.dir.join("home"))
        .arg("--config")
        .arg(file)
        .args(args)
        .current_dir(&case.dir)
        .env("PATH", path)
        .output()
        .expect("conductor runs")
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `roles[].kind` takes any kind word: the file validates.
#[test]
fn a_role_may_name_any_kind() {
    let (case, file) = cli_case("kind-word");
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
}

/// `agent start` on a role whose kind the binary does not implement is refused by name, one line,
/// exit 1, and nothing is started.
#[test]
fn agent_start_refuses_a_kind_the_binary_does_not_implement() {
    let (case, file) = cli_case("start-unknown");
    let output = run(
        &case,
        &file,
        &[
            "agent",
            "start",
            "--instance",
            "alpha",
            "--role",
            "controller",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with(
            "conductor: agent start: refused: UnknownAgentKind: kind \"example-run-to-exit\""
        ),
        "{}",
        describe(&output)
    );
    assert_eq!(stderr.lines().count(), 1, "{}", describe(&output));
}

/// `agent stop` goes through the `claude-code` kind exactly as `session stop` does: with no live
/// session both refuse `NotRunning` with the same reason.
#[test]
fn agent_stop_is_session_stop_through_the_kind() {
    let (case, file) = cli_case("stop-not-running");
    let agent = run(
        &case,
        &file,
        &["agent", "stop", "--instance", "alpha", "--role", "director"],
    );
    let session = run(&case, &file, &["session", "stop", "--role", "director"]);
    assert_eq!(agent.status.code(), Some(1), "{}", describe(&agent));
    assert_eq!(session.status.code(), Some(1), "{}", describe(&session));
    let said = String::from_utf8_lossy(&session.stderr);
    assert!(
        said.contains("session stop: refused: NotRunning: "),
        "{}",
        describe(&session)
    );
    assert_eq!(agent.stderr, session.stderr, "{}", describe(&agent));
}

/// The `agent` group's verbs are written, and `agent attach` on a role whose kind this binary
/// does not implement answers `UnknownAgentKind`.
#[test]
fn agent_verbs_are_written() {
    let (case, file) = cli_case("verbs");
    let output = run(&case, &file, &["agent", "--help"]);
    let help = String::from_utf8_lossy(&output.stdout);
    for verb in ["start", "deliver", "attach", "stop"] {
        assert!(help.contains(verb), "{help}");
    }
    assert!(!help.contains("not written yet"), "{help}");
    let output = run(
        &case,
        &file,
        &[
            "agent",
            "attach",
            "--instance",
            "alpha",
            "--role",
            "controller",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .starts_with("conductor: agent attach: refused: UnknownAgentKind: "),
        "{}",
        describe(&output)
    );
}

/// `conductor doctor` names each configured kind, its binary and the version it prints, and a
/// kind the binary does not implement.
#[test]
fn doctor_names_each_configured_kind() {
    let (case, file) = cli_case("doctor");
    let output = run(&case, &file, &["doctor"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .lines()
            .any(|line| line == "agent kind claude-code: claude 9.9.9 (Claude Code)"),
        "{}",
        describe(&output)
    );
    assert!(
        stdout
            .lines()
            .any(|line| line == "agent kind example-run-to-exit: not implemented by this binary"),
        "{}",
        describe(&output)
    );
}
