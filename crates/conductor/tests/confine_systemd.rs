//! `story:session-limits-systemd`: the `systemd` provider starts each session in a
//! `systemd-run --user` unit inside its instance's slice, with the role's limits, and reads the
//! box back from the unit's cgroup.
//!
//! - `conductor spawn` (a controller, under the session prefix `ex`) and `conductor session start
//!   --role conductor` (no prefix) set the slice's limits with `systemctl --user set-property`,
//!   start the unit with `systemd-run --user`, type the first prompt into the session's FIFO and
//!   measure the unit's cgroup, and `spawn` prints the session's id; the instance's slice is
//!   `conductor_<prefix>.slice` or `conductor.slice`, siblings of each other; `conductor session
//!   stop` stops the unit; `resume` starts a new unit running `claude --resume <id>` once the
//!   old one is gone; `controller pause-controller`, `resume-controller` and `stop-controller`
//!   stop and resume a controller's unit and record its state.
//! - `available` reads the user manager's delegated controllers from a fixture
//!   `cgroup.controllers`.
//!
//! The commands run over [`Host`] with its cgroup root a fixture directory: this test binary runs
//! itself again as a child ([`fake_provider::command`]) whose `child` case runs the command line
//! through `Cli::run_with`. Each case is a directory of its own: `home/` (the `HOME`), `records/`,
//! `state/`, `work/`, `cgroup/` (the fixture cgroup root) and `bin/`, first on `PATH`, with
//! - `claude`: `agents --json` prints the case's `agents.json`; any other call is logged;
//! - `systemd-run`: logs its arguments to `systemd-run.log`, one per line and `----` after each
//!   call, makes the session live (`agents-live.json` becomes `agents.json`), and runs the
//!   launcher it was given (the arguments from `/bin/sh` on) in the background;
//! - `systemctl`: logs its arguments to `systemctl.log` the same way, answers `show -p
//!   ControlGroup` with the case's `control-group`, and `show -p LoadState -p ActiveState` with
//!   the first line of the case's `unit-states` (`LoadState=… ActiveState=…`, one line per call,
//!   the last one kept), or a unit that is not there when there is none;
//! - `script`: the launcher's PTY program; logs its arguments to `script.args` and copies what is
//!   typed into the FIFO to `typed`, as many bytes as `typed.size` says.
//!
//! No real `systemd-run`, `systemctl`, `script` or `claude` runs.

mod fake_provider;

use std::env;
use std::fs;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Output};
use std::time::{Duration, Instant};

use clap::Parser as _;
use conductor_cli::cli::Cli;
use conductor_cli::config;
use conductor_cli::confine::{Host, Launch, Providers as _, systemd};
use conductor_cli::session::CONDUCTOR_PROMPT;
use conductor_model::config::{ConfinementProvider, Gibibytes, Limits, Network, SessionName};
use conductor_model::session::{SessionRequest, StartedSession};
use serde_json::{Value, json};

/// The child's first argument that asks it to resume a conductor session through the provider
/// itself, as no command resumes conductor's: `resume <session id>`.
const RESUME: &str = "resume";

/// The child run of this test binary; a plain run of the suite passes it at once.
#[test]
fn child() {
    let (Ok(args), Some(dir)) = (
        env::var(fake_provider::ARGS),
        env::var_os(fake_provider::FAKE),
    ) else {
        return;
    };
    let args: Vec<String> = serde_json::from_str(&args).expect("the child's arguments");
    let dir = PathBuf::from(dir);
    let config_file = dir.join("conductor.yaml");
    config::set_active(Some(&config_file)).expect("the case's config loads");
    let host = Host {
        cgroup_root: dir.join("cgroup"),
    };
    // The case passes `--config <file>` first; the resume form follows it.
    let rest = if args.first().map(String::as_str) == Some("--config") {
        &args[2..]
    } else {
        &args[..]
    };
    let code = if rest.first().map(String::as_str) == Some(RESUME) {
        resume_in_child(&host, &dir, &rest[1])
    } else {
        let mut argv = vec!["conductor".to_owned()];
        argv.extend(args);
        match Cli::try_parse_from(argv) {
            Err(error) => {
                let _ = error.print();
                2
            }
            Ok(cli) => match cli.run_with(&host) {
                Ok(code) if code == ExitCode::SUCCESS => 0,
                Ok(_) => 1,
                Err(error) => {
                    eprintln!("conductor: {error:#}");
                    1
                }
            },
        }
    };
    std::process::exit(code)
}

/// Resumes the conductor session `id` of the case `dir` through the systemd provider and prints
/// the session it answers as JSON.
fn resume_in_child(host: &Host, dir: &Path, id: &str) -> i32 {
    let records = dir.join("records");
    let launch = Launch {
        request: SessionRequest {
            role: "conductor".to_owned(),
            session_name: SessionName("conductor".to_owned()),
            cwd: records.display().to_string(),
            limits: Limits {
                memory_max: Some(Gibibytes(4)),
                cpu_weight: None,
                tasks_max: None,
            },
            writable: Vec::new(),
            network: Network::Open,
            forge_token_env: Vec::new(),
        },
        program: "claude".to_owned(),
        args: ["-n", "conductor", "--model", "opus"]
            .map(str::to_owned)
            .to_vec(),
        env: Vec::new(),
        session_prefix: None,
        instance_limits: Limits {
            memory_max: None,
            cpu_weight: None,
            tasks_max: None,
        },
        state: dir.join("state"),
    };
    let session = StartedSession {
        role: "conductor".to_owned(),
        session_name: SessionName("conductor".to_owned()),
        provider: "systemd".to_owned(),
        unit: Some("conductor_conductor.service".to_owned()),
        session_id: Some(id.to_owned()),
    };
    let mut provider = host.provider(ConfinementProvider::Systemd);
    match provider.resume(&session, &launch) {
        Ok(resumed) => {
            println!(
                "{}",
                json!({
                    "provider": resumed.provider,
                    "unit": resumed.unit,
                    "session_id": resumed.session_id,
                })
            );
            0
        }
        Err(error) => {
            eprintln!("conductor: {error:#}");
            1
        }
    }
}

/// A case directory of its own; see the module documentation.
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

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

fn runnable(path: &Path, text: &str) {
    write(path, text);
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make it runnable");
}

/// This process's user id, as the provider reads it.
fn uid() -> u32 {
    fs::metadata("/proc/self").expect("/proc/self").uid()
}

/// The user manager's cgroup under a cgroup root.
fn manager(uid: u32) -> String {
    format!("user.slice/user-{uid}.slice/user@{uid}.service")
}

impl Case {
    /// The case `name`: its directory, fakes and the delegated controllers `cpu memory pids`.
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("confine-systemd")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "records", "state", "work", "bin", "cgroup"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        case.controllers("cpuset cpu io memory pids\n");
        write(&case.dir.join("agents.json"), "[]\n");
        let d = case.dir.display();
        runnable(
            &case.bin("claude"),
            &format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = agents ]; then cat '{d}/agents.json'; exit 0; fi\n\
                 printf '%s\\n' \"$*\" >> '{d}/claude.log'\n"
            ),
        );
        runnable(
            &case.bin("systemd-run"),
            &format!(
                "#!/bin/sh\n\
                 printf '%s\\n' \"$@\" '----' >> '{d}/systemd-run.log'\n\
                 [ -e '{d}/agents-live.json' ] && cp '{d}/agents-live.json' '{d}/agents.json'\n\
                 while [ $# -gt 0 ] && [ \"$1\" != /bin/sh ]; do shift; done\n\
                 if [ $# -gt 0 ]; then setsid \"$@\" </dev/null >/dev/null 2>&1 & fi\n\
                 exit 0\n"
            ),
        );
        runnable(
            &case.bin("systemctl"),
            &format!(
                "#!/bin/sh\n\
                 printf '%s\\n' \"$@\" '----' >> '{d}/systemctl.log'\n\
                 case \" $* \" in\n\
                 *' ControlGroup '*) cat '{d}/control-group' ;;\n\
                 *' LoadState '*)\n\
                 \x20 if [ -s '{d}/unit-states' ]; then\n\
                 \x20   head -n 1 '{d}/unit-states' | tr ' ' '\\n'\n\
                 \x20   if [ \"$(wc -l < '{d}/unit-states')\" -gt 1 ]; then\n\
                 \x20     tail -n +2 '{d}/unit-states' > '{d}/unit-states.next'\n\
                 \x20     mv '{d}/unit-states.next' '{d}/unit-states'\n\
                 \x20   fi\n\
                 \x20 else printf 'LoadState=not-found\\nActiveState=inactive\\n'; fi ;;\n\
                 esac\n\
                 exit 0\n"
            ),
        );
        runnable(
            &case.bin("script"),
            &format!(
                "#!/bin/sh\n\
                 printf '%s\\n' \"$@\" > '{d}/script.args'\n\
                 exec timeout 20 head -c \"$(cat '{d}/typed.size')\" > '{d}/typed'\n"
            ),
        );
        case
    }

    fn bin(&self, name: &str) -> PathBuf {
        self.dir.join("bin").join(name)
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn path(&self) -> String {
        format!("{}:/usr/bin:/bin", self.dir.join("bin").display())
    }

    /// The user manager's delegated controllers, as its `cgroup.controllers` lists them.
    fn controllers(&self, text: &str) {
        write(
            &self
                .dir
                .join("cgroup")
                .join(manager(uid()))
                .join("cgroup.controllers"),
            text,
        );
    }

    /// The unit's cgroup `path`, as `systemctl --user show -p ControlGroup` prints it, with its
    /// limit files.
    fn unit_cgroup(&self, path: &str, memory: &str, cpu: &str, pids: &str) {
        write(&self.dir.join("control-group"), &format!("{path}\n"));
        let at = self.dir.join("cgroup").join(path.trim_start_matches('/'));
        write(&at.join("memory.max"), &format!("{memory}\n"));
        write(&at.join("cpu.weight"), &format!("{cpu}\n"));
        write(&at.join("pids.max"), &format!("{pids}\n"));
    }

    /// The session `name` appears live once the unit starts; the first prompt is `prompt`.
    fn session(&self, name: &str, prompt: &str) {
        write(
            &self.dir.join("agents-live.json"),
            &json!([{"name": name, "pid": 4242, "sessionId": "listed-id", "kind": "interactive"}])
                .to_string(),
        );
        write(
            &self.dir.join("typed.size"),
            &typed(prompt).len().to_string(),
        );
    }

    /// `conductor --config <case> <args>` over the host with the fixture cgroup root, from
    /// `cwd`.
    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        let config = self.config();
        let mut all = vec!["--config", config.to_str().expect("a UTF-8 path")];
        all.extend_from_slice(args);
        let output = fake_provider::command(&self.home(), &self.dir, &all)
            .current_dir(cwd)
            .env("PATH", self.path())
            .env("TMPDIR", self.dir.join("tmp"))
            .env("GH_TOKEN", "not-a-token")
            .output()
            .expect("the child runs");
        fake_provider::without_harness(output)
    }

    /// The calls a fake logged to `<tool>.log`, each its arguments.
    fn calls(&self, tool: &str) -> Vec<Vec<String>> {
        let text = fs::read_to_string(self.dir.join(format!("{tool}.log"))).unwrap_or_default();
        let mut calls = Vec::new();
        let mut call = Vec::new();
        for line in text.lines() {
            if line == "----" {
                calls.push(std::mem::take(&mut call));
            } else {
                call.push(line.to_owned());
            }
        }
        calls
    }

    /// What was typed into the session's FIFO, once the fake `script` has read it all.
    fn typed(&self, prompt: &str) -> String {
        let want = typed(prompt);
        let file = self.dir.join("typed");
        let started = Instant::now();
        loop {
            let text = fs::read_to_string(&file).unwrap_or_default();
            if text.len() >= want.len() || started.elapsed() > Duration::from_secs(20) {
                return text;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// The arguments the fake `script` was started with by the launcher.
    fn script_args(&self) -> Vec<String> {
        let file = self.dir.join("script.args");
        let started = Instant::now();
        while !file.exists() && started.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(50));
        }
        fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// What the fake `systemctl show -p LoadState -p ActiveState` answers, one line per call, the
    /// last one kept.
    fn unit_states(&self, states: &[&str]) {
        let mut text = states.join("\n");
        text.push('\n');
        write(&self.dir.join("unit-states"), &text);
    }

    /// The rows of `controller controllers --format json`.
    fn controller_rows(&self) -> Vec<Value> {
        let output = self.run(
            &self.records(),
            &["controller", "controllers", "--format", "json"],
        );
        assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
        match serde_json::from_slice(&output.stdout).expect("the view prints JSON") {
            Value::Array(rows) => rows,
            other => panic!("the view printed {other}, not an array"),
        }
    }

    /// The measurement of the session `name`, if one was written.
    fn record(&self, name: &str) -> Option<Value> {
        fs::read_to_string(
            self.state()
                .join("sessions")
                .join(format!("{name}.envelope.json")),
        )
        .ok()
        .map(|text| serde_json::from_str(&text).expect("the record is JSON"))
    }
}

/// What the provider types for `prompt`: the prompt as one bracketed paste, then Enter.
fn typed(prompt: &str) -> String {
    format!("\x1b[200~{prompt}\x1b[201~\r")
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The arguments of one `systemd-run` call after the launcher's `/bin/sh -c <code>
/// conductor-launch`: the FIFO, the typescript and the command `script` runs.
fn launcher_tail(call: &[String]) -> (String, String, String) {
    let at = call
        .iter()
        .position(|arg| arg == "/bin/sh")
        .unwrap_or_else(|| panic!("no launcher in {call:?}"));
    assert_eq!(call[at + 1], "-c", "{call:?}");
    assert_eq!(call[at + 3], "conductor-launch", "{call:?}");
    assert_eq!(call.len(), at + 7, "{call:?}");
    (
        call[at + 4].clone(),
        call[at + 5].clone(),
        call[at + 6].clone(),
    )
}

/// The value of the `--session-id` the command `script` runs passes to `claude`.
fn session_id_of(command: &str) -> String {
    let rest = command
        .split("'--session-id' '")
        .nth(1)
        .unwrap_or_else(|| panic!("no --session-id in {command:?}"));
    rest.split('\'').next().unwrap_or_default().to_owned()
}

// -------------------------------------------------------------------------------------------
// A controller, under the session prefix `ex`.
// -------------------------------------------------------------------------------------------

const CHARTER: &str = "Charter for alpha: $(touch expanded) 'single'\nforge: github\n";

/// The controller case: the checkout `alpha` trusted, its charter, the controller role's profile
/// and settings, and an instance `ex` with its own limits.
fn controller_case(name: &str) -> Case {
    let case = Case::new(name);
    let root = case.home().join("example-org");
    let checkout = root.join("alpha");
    write(&checkout.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&case.records().join("charters/alpha.md"), CHARTER);
    let profile = case.dir.join("roles/repo-controller.md");
    let settings = case.dir.join("roles/controller-settings.json");
    write(&profile, "# Repository controller\n");
    // The settings pin the instance (`story:instance-isolation`).
    write(
        &settings,
        &json!({
            "env": {"CONDUCTOR_INSTANCE": "example-org"},
            "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
                "type": "command",
                "command": "CONDUCTOR_INSTANCE=example-org conductor guard \
                            record-guard-decision --from-pre-tool-use || exit 2",
            }]}]},
        })
        .to_string(),
    );
    write(
        &case.home().join(".claude.json"),
        &json!({"projects": {checkout.display().to_string(): {"hasTrustDialogAccepted": true}}})
            .to_string(),
    );
    write(
        &case.config(),
        &format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   session_prefix: ex\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
             \x20   records: {records}\n\
             \x20   state: {state}\n\
             \x20   roles:\n\
             \x20     - {{role: conductor, harness: claude, model: opus}}\n\
             \x20     - {{role: controller, harness: claude, model: sonnet, profile: {profile}, \
             settings: {settings}, limits: {{memory_max: 4G, cpu_weight: 50, tasks_max: 256}}}}\n\
             \x20   confinement: {{mode: report, provider: systemd, limits: {{memory_max: 8G, \
             cpu_weight: 200, tasks_max: 1024}}}}\n",
            root = root.display(),
            trees = case.dir.join("trees").display(),
            records = case.records().display(),
            state = case.state().display(),
            profile = profile.display(),
            settings = settings.display(),
        ),
    );
    case
}

/// The instance's slice sits directly in the user manager's root slice, a sibling of every other
/// instance's: no `conductor.slice` above it.
const CONTROLLER_CGROUP: &str = "/user.slice/user-1000.slice/user@1000.service/\
                                 conductor_ex.slice/conductor_ex-alpha.service";

/// `conductor spawn` through the systemd provider: the instance's slice carries the instance's
/// limits, the unit `conductor_ex-alpha.service` runs in it with the controller role's limits
/// and the spike's launcher, the charter is typed into the FIFO, and the measurement reads the
/// unit's cgroup files.
#[test]
fn a_controller_starts_in_its_unit_inside_the_prefixed_slice_with_its_role_limits() {
    let case = controller_case("controller");
    case.session("ex-alpha", CHARTER);
    case.unit_cgroup(CONTROLLER_CGROUP, "4294967296", "50", "256");
    let output = case.run(&case.records(), &["spawn", "--repository", "alpha"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));

    let checkout = case.home().join("example-org/alpha");
    let state = case.state().join("sessions");
    let systemctl = case.calls("systemctl");
    // First: no unit of the session's name is left from an earlier session.
    assert_eq!(
        systemctl.first().map(Vec::as_slice),
        Some(
            &[
                "--user",
                "show",
                "-p",
                "LoadState",
                "-p",
                "ActiveState",
                "conductor_ex-alpha.service"
            ]
            .map(str::to_owned)[..]
        ),
        "{systemctl:?}"
    );
    assert_eq!(
        systemctl.get(1).map(Vec::as_slice),
        Some(
            &[
                "--user",
                "set-property",
                "--runtime",
                "conductor_ex.slice",
                "MemoryMax=8G",
                "CPUWeight=200",
                "TasksMax=1024"
            ]
            .map(str::to_owned)[..]
        ),
        "{systemctl:?}"
    );
    let runs = case.calls("systemd-run");
    assert_eq!(runs.len(), 1, "{runs:?}");
    let run = &runs[0];
    let head: Vec<String> = [
        "--user",
        "--unit",
        "conductor_ex-alpha.service",
        "--slice",
        "conductor_ex.slice",
        "--collect",
        "-p",
        "MemoryMax=4G",
        "-p",
        "CPUWeight=50",
        "-p",
        "TasksMax=256",
        "-p",
        "StandardOutput=null",
        &format!("--working-directory={}", checkout.display()),
    ]
    .map(str::to_owned)
    .to_vec();
    assert_eq!(&run[..head.len()], &head[..], "{run:?}");
    for set in [
        "TERM=xterm-256color".to_owned(),
        "SHELL=/bin/bash".to_owned(),
        format!("PATH={}", case.path()),
        "CONDUCTOR_INSTANCE=example-org".to_owned(),
        format!("CONDUCTOR_CONFIG={}", case.config().display()),
    ] {
        assert!(
            run.windows(2).any(|pair| pair[0] == "-E" && pair[1] == set),
            "-E {set}: {run:?}"
        );
    }
    let (fifo, typescript, command) = launcher_tail(run);
    assert_eq!(fifo, state.join("ex-alpha.fifo").display().to_string());
    assert_eq!(
        typescript,
        state.join("ex-alpha.typescript").display().to_string()
    );
    let id = session_id_of(&command);
    assert_eq!(id.len(), 36, "a UUID: {command}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{id}\n"),
        "spawn prints the session's id, as it does under the none provider: {}",
        describe(&output)
    );
    assert!(
        command.starts_with(&format!(
            "stty cols 160 rows 48; exec 'claude' '--session-id' '{id}' '-n' 'ex-alpha' \
             '--append-system-prompt-file' '{}' '--model' 'sonnet'",
            case.dir.join("roles/repo-controller.md").display()
        )),
        "{command}"
    );
    // The launcher ran: `script` got the command and the typescript, and the charter was typed.
    assert_eq!(
        case.script_args(),
        ["-qfec".to_owned(), command.clone(), typescript.clone()],
    );
    assert_eq!(case.typed(CHARTER), typed(CHARTER));

    assert_eq!(
        systemctl.get(2).map(Vec::as_slice),
        Some(
            &[
                "--user",
                "show",
                "-p",
                "ControlGroup",
                "--value",
                "conductor_ex-alpha.service"
            ]
            .map(str::to_owned)[..]
        ),
        "{systemctl:?}"
    );
    let record = case.record("ex-alpha").expect("the measurement is written");
    assert_eq!(record["provider"], "systemd", "{record:#}");
    assert_eq!(record["cgroup"], CONTROLLER_CGROUP, "{record:#}");
    assert_eq!(record["memory_max"], 4_294_967_296_i64, "{record:#}");
    assert_eq!(record["cpu_weight"], 50, "{record:#}");
    assert_eq!(record["tasks_max"], 256, "{record:#}");
    assert_eq!(
        record["session"]["unit"], "conductor_ex-alpha.service",
        "{record:#}"
    );
    assert_eq!(record["session"]["session_id"], id.as_str(), "{record:#}");
    assert!(record["unboxed"].is_null(), "{record:#}");
    assert!(
        record["writable"].is_null() && record["fence_missing"].is_string(),
        "no write fence yet, and the record says so: {record:#}"
    );
    assert!(
        fs::read_to_string(case.dir.join("claude.log"))
            .unwrap_or_default()
            .is_empty(),
        "claude was not launched outside the unit"
    );
}

// -------------------------------------------------------------------------------------------
// Conductor, without a prefix.
// -------------------------------------------------------------------------------------------

/// The conductor case: an instance without a prefix and without limits, its conductor role
/// limited in memory only.
fn conductor_case(name: &str) -> Case {
    let case = Case::new(name);
    write(
        &case.config(),
        &format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
             \x20   records: {records}\n\
             \x20   state: {state}\n\
             \x20   roles:\n\
             \x20     - {{role: conductor, harness: claude, model: opus, limits: \
             {{memory_max: 4G}}, forge_token_env: [GH_TOKEN]}}\n\
             \x20     - {{role: controller, harness: claude, model: opus}}\n\
             \x20   confinement: {{mode: report, provider: systemd}}\n",
            root = case.home().join("example-org").display(),
            trees = case.dir.join("trees").display(),
            records = case.records().display(),
            state = case.state().display(),
        ),
    );
    case
}

const CONDUCTOR_CGROUP: &str = "/user.slice/user-1000.slice/user@1000.service/conductor.slice/\
                                conductor_conductor.service";

/// `conductor session start --role conductor` through the systemd provider: the slice
/// `conductor.slice` is reset to no limit, the unit `conductor_conductor.service` sets only the
/// role's memory limit, the forge token variable is passed by name, the prompt is typed into the
/// FIFO, and a limit file reading `max` is recorded absent.
#[test]
fn conductor_starts_in_its_unit_inside_the_unprefixed_slice() {
    let case = conductor_case("conductor");
    case.session("conductor", CONDUCTOR_PROMPT);
    case.unit_cgroup(CONDUCTOR_CGROUP, "4294967296", "100", "max");
    let output = case.run(
        &case.dir.join("work"),
        &["session", "start", "--role", "conductor"],
    );
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));

    let systemctl = case.calls("systemctl");
    assert_eq!(
        systemctl.first().map(Vec::as_slice),
        Some(
            &[
                "--user",
                "show",
                "-p",
                "LoadState",
                "-p",
                "ActiveState",
                "conductor_conductor.service"
            ]
            .map(str::to_owned)[..]
        ),
        "{systemctl:?}"
    );
    assert_eq!(
        systemctl.get(1).map(Vec::as_slice),
        Some(
            &[
                "--user",
                "set-property",
                "--runtime",
                "conductor.slice",
                "MemoryMax=infinity",
                "CPUWeight=",
                "TasksMax=infinity"
            ]
            .map(str::to_owned)[..]
        ),
        "{systemctl:?}"
    );
    let runs = case.calls("systemd-run");
    assert_eq!(runs.len(), 1, "{runs:?}");
    let run = &runs[0];
    let head: Vec<String> = [
        "--user",
        "--unit",
        "conductor_conductor.service",
        "--slice",
        "conductor.slice",
        "--collect",
        "-p",
        "MemoryMax=4G",
        "-p",
        "StandardOutput=null",
        &format!("--working-directory={}", case.records().display()),
    ]
    .map(str::to_owned)
    .to_vec();
    assert_eq!(&run[..head.len()], &head[..], "{run:?}");
    assert!(
        run.windows(2)
            .any(|pair| pair[0] == "-E" && pair[1] == "GH_TOKEN"),
        "the forge token variable by name: {run:?}"
    );
    assert!(
        !run.iter().any(|arg| arg.contains("not-a-token")),
        "no token value on the command line: {run:?}"
    );
    let (fifo, _, command) = launcher_tail(run);
    assert_eq!(
        fifo,
        case.state()
            .join("sessions/conductor.fifo")
            .display()
            .to_string()
    );
    let id = session_id_of(&command);
    assert!(
        command.starts_with(&format!(
            "stty cols 160 rows 48; exec 'claude' '--session-id' '{id}' '-n' 'conductor' \
             '--agent' 'conductor' '--model' 'opus'"
        )),
        "{command}"
    );
    assert_eq!(case.typed(CONDUCTOR_PROMPT), typed(CONDUCTOR_PROMPT));

    let record = case
        .record("conductor")
        .expect("the measurement is written");
    assert_eq!(record["cgroup"], CONDUCTOR_CGROUP, "{record:#}");
    assert_eq!(record["memory_max"], 4_294_967_296_i64, "{record:#}");
    assert_eq!(record["cpu_weight"], 100, "{record:#}");
    assert!(
        record["tasks_max"].is_null(),
        "pids.max reads max: {record:#}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!(
            "started conductor: provider systemd, unit conductor_conductor.service, session {id}"
        )),
        "{}",
        describe(&output)
    );
}

/// `conductor session stop --role conductor` stops a session the systemd provider started with
/// `systemctl --user stop <unit>`.
#[test]
fn conductor_stops_with_systemctl_stop() {
    let case = conductor_case("stop");
    case.session("conductor", CONDUCTOR_PROMPT);
    case.unit_cgroup(CONDUCTOR_CGROUP, "4294967296", "100", "max");
    let work = case.dir.join("work");
    let started = case.run(&work, &["session", "start", "--role", "conductor"]);
    assert_eq!(started.status.code(), Some(0), "{}", describe(&started));
    case.typed(CONDUCTOR_PROMPT);
    case.unit_states(&["LoadState=loaded ActiveState=active"]);
    let stopped = case.run(&work, &["session", "stop", "--role", "conductor"]);
    assert_eq!(stopped.status.code(), Some(0), "{}", describe(&stopped));
    let systemctl = case.calls("systemctl");
    assert_eq!(
        systemctl.last().map(Vec::as_slice),
        Some(&["--user", "stop", "conductor_conductor.service"].map(str::to_owned)[..]),
        "{systemctl:?}"
    );
    assert!(case.record("conductor").is_none(), "the record is removed");
}

/// `resume` starts a new unit of the same name in the slice, running `claude --resume <id>`
/// with the launch's arguments; it answers the same session in that unit.
#[test]
fn resume_starts_a_new_unit_running_claude_resume() {
    let case = conductor_case("resume");
    write(&case.dir.join("typed.size"), "0");
    let id = "0a1b2c3d-0000-4000-8000-00000000abcd";
    let output = case.run(&case.dir.join("work"), &[RESUME, id]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let answered: Value =
        serde_json::from_slice(&output.stdout).expect("the resumed session as JSON");
    assert_eq!(
        answered,
        json!({"provider": "systemd", "unit": "conductor_conductor.service", "session_id": id})
    );
    let runs = case.calls("systemd-run");
    assert_eq!(runs.len(), 1, "{runs:?}");
    let run = &runs[0];
    assert_eq!(
        &run[..8],
        &[
            "--user",
            "--unit",
            "conductor_conductor.service",
            "--slice",
            "conductor.slice",
            "--collect",
            "-p",
            "MemoryMax=4G"
        ]
        .map(str::to_owned)[..],
        "{run:?}"
    );
    let (_, _, command) = launcher_tail(run);
    assert_eq!(
        command,
        format!(
            "stty cols 160 rows 48; exec 'claude' '--resume' '{id}' '-n' 'conductor' '--model' \
             'opus'"
        )
    );
}

// -------------------------------------------------------------------------------------------
// Availability and names.
// -------------------------------------------------------------------------------------------

/// `available` reads the user manager's `cgroup.controllers`: available when `cpu`, `memory`
/// and `pids` are delegated, not when one is missing or the file is not there, naming why.
#[test]
fn available_reads_the_delegated_controllers() {
    let case = Case::new("available");
    let host = Host {
        cgroup_root: case.dir.join("cgroup"),
    };
    let answer = host.provider(ConfinementProvider::Systemd).available();
    assert_eq!(answer.provider, "systemd");
    assert!(answer.available, "{answer:?}");
    assert_eq!(answer.reason, None);

    case.controllers("cpuset cpu io memory\n");
    let answer = host.provider(ConfinementProvider::Systemd).available();
    assert!(!answer.available, "{answer:?}");
    let reason = answer.reason.expect("a reason");
    assert!(
        reason.contains("pids") && reason.contains("cgroup.controllers"),
        "{reason}"
    );

    fs::remove_file(
        case.dir
            .join("cgroup")
            .join(manager(uid()))
            .join("cgroup.controllers"),
    )
    .expect("remove the controllers");
    let answer = host.provider(ConfinementProvider::Systemd).available();
    assert!(!answer.available, "{answer:?}");
    assert!(
        answer
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("cgroup.controllers")),
        "{answer:?}"
    );
}

/// Every `limits.*` row of `docs/config.md`, the role's and the instance's, names the systemd
/// provider as its reader.
#[test]
fn the_limits_rows_of_the_config_page_name_the_systemd_provider() {
    let page = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/config.md");
    let text = fs::read_to_string(&page).expect("read docs/config.md");
    let rows: Vec<&str> = text
        .lines()
        .filter(|line| {
            line.starts_with("| `roles[].limits.") || line.starts_with("| `confinement.limits.")
        })
        .collect();
    assert_eq!(rows.len(), 6, "{rows:#?}");
    for row in rows {
        let reader = row.split('|').nth(4).unwrap_or_default();
        assert!(
            reader.contains("systemd provider"),
            "the row does not name the systemd provider as its reader: {row}"
        );
    }
}

/// The unit and slice names: `conductor_<session name>.service`, a grouped repository's `/`
/// escaped as systemd escapes it; `conductor_<prefix>.slice`, or `conductor.slice`.
#[test]
fn unit_and_slice_names() {
    assert_eq!(
        systemd::unit_name(&SessionName("ex-alpha".to_owned())),
        "conductor_ex-alpha.service"
    );
    assert_eq!(
        systemd::unit_name(&SessionName("ex-group/alpha".to_owned())),
        "conductor_ex-group\\x2falpha.service"
    );
    assert_eq!(systemd::slice_name(Some("ex")), "conductor_ex.slice");
    assert_eq!(systemd::slice_name(None), "conductor.slice");
}

/// systemd reads each `-` of a slice's name as one step down: `conductor-ex.slice` is a child of
/// `conductor.slice`, the slice of the instance without a prefix, whose limits would then cap it.
/// No instance's slice has a `-` in its name, a prefix's own `-` escaped as `\x2d`, so every
/// instance's slice is a child of the root slice, and a sibling of every other.
#[test]
fn every_instance_slice_is_a_sibling_of_the_others() {
    let slices: Vec<String> = [None, Some("ex"), Some("ex-a"), Some("lrun"), Some("a-b-c")]
        .into_iter()
        .map(systemd::slice_name)
        .collect();
    for slice in &slices {
        let stem = slice.strip_suffix(".slice").expect("a slice");
        assert!(
            !stem.contains('-'),
            "{slice} would nest inside another slice"
        );
    }
    assert_eq!(slices[2], "conductor_ex\\x2da.slice");
    let mut distinct = slices.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), slices.len(), "{slices:?}");
}

// -------------------------------------------------------------------------------------------
// Stopping and resuming a controller through its instance's provider.
// -------------------------------------------------------------------------------------------

/// How many of `calls` start with `head`.
fn count_calls(calls: &[Vec<String>], head: &[&str]) -> usize {
    calls
        .iter()
        .filter(|call| call.len() >= head.len() && call[..head.len()] == *head)
        .count()
}

/// `controller pause-controller` stops a controller's unit with `systemctl --user stop` and
/// records it `Paused`; `controller resume-controller` starts a new unit of the same name in
/// the instance's slice running `claude --resume <id>`, records it `Running` and its new
/// measurement; `controller stop-controller` stops the unit again and records it `Stopped`.
#[test]
fn a_controller_pauses_resumes_and_stops_through_its_unit() {
    let case = controller_case("lifecycle");
    case.session("ex-alpha", CHARTER);
    case.unit_cgroup(CONTROLLER_CGROUP, "4294967296", "50", "256");
    let records = case.records();
    let spawned = case.run(&records, &["spawn", "--repository", "alpha"]);
    assert_eq!(spawned.status.code(), Some(0), "{}", describe(&spawned));
    case.typed(CHARTER);
    let id = String::from_utf8_lossy(&spawned.stdout).trim().to_owned();
    assert_eq!(id.len(), 36, "{}", describe(&spawned));
    let rows = case.controller_rows();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let controller = rows[0]["controller_id"]
        .as_str()
        .expect("a controller id")
        .to_owned();
    let state = |case: &Case| case.controller_rows()[0]["state"].clone();

    // Pause: the unit runs, and is stopped.
    case.unit_states(&["LoadState=loaded ActiveState=active"]);
    let paused = case.run(
        &records,
        &[
            "controller",
            "pause-controller",
            "--controller-id",
            &controller,
        ],
    );
    assert_eq!(paused.status.code(), Some(0), "{}", describe(&paused));
    assert_eq!(
        String::from_utf8_lossy(&paused.stdout),
        format!("stopped ex-alpha ({id})\n")
    );
    let systemctl = case.calls("systemctl");
    assert_eq!(
        count_calls(
            &systemctl,
            &["--user", "stop", "conductor_ex-alpha.service"]
        ),
        1,
        "{systemctl:?}"
    );
    assert_eq!(state(&case), "Paused");
    assert!(
        case.record("ex-alpha").is_some(),
        "the record keeps the session's id for the resume"
    );

    // Resume: the stopped unit is gone, and a new one resumes the session.
    write(&case.dir.join("agents.json"), "[]\n");
    write(&case.dir.join("typed.size"), "0");
    case.unit_states(&["LoadState=not-found ActiveState=inactive"]);
    let resumed = case.run(
        &records,
        &[
            "controller",
            "resume-controller",
            "--controller-id",
            &controller,
        ],
    );
    assert_eq!(resumed.status.code(), Some(0), "{}", describe(&resumed));
    assert_eq!(
        String::from_utf8_lossy(&resumed.stdout),
        format!("resumed ex-alpha ({id}) in conductor_ex-alpha.service\n")
    );
    let runs = case.calls("systemd-run");
    assert_eq!(runs.len(), 2, "{runs:?}");
    let run = &runs[1];
    assert_eq!(
        &run[..6],
        &[
            "--user",
            "--unit",
            "conductor_ex-alpha.service",
            "--slice",
            "conductor_ex.slice",
            "--collect"
        ]
        .map(str::to_owned)[..],
        "{run:?}"
    );
    let (_, _, command) = launcher_tail(run);
    assert!(
        command.starts_with(&format!(
            "stty cols 160 rows 48; exec 'claude' '--resume' '{id}' '-n' 'ex-alpha' \
             '--append-system-prompt-file' '{}' '--model' 'sonnet'",
            case.dir.join("roles/repo-controller.md").display()
        )),
        "{command}"
    );
    assert!(!command.contains("--session-id"), "{command}");
    assert_eq!(state(&case), "Running");
    let record = case
        .record("ex-alpha")
        .expect("the resumed box is measured");
    assert_eq!(record["session"]["session_id"], id.as_str(), "{record:#}");
    assert_eq!(
        record["session"]["unit"], "conductor_ex-alpha.service",
        "{record:#}"
    );
    assert_eq!(record["memory_max"], 4_294_967_296_i64, "{record:#}");

    // Stop: the unit runs again, and is stopped; the controller is recorded stopped.
    case.unit_states(&["LoadState=loaded ActiveState=active"]);
    let stopped = case.run(
        &records,
        &[
            "controller",
            "stop-controller",
            "--controller-id",
            &controller,
            "--reason",
            "the wave closed",
        ],
    );
    assert_eq!(stopped.status.code(), Some(0), "{}", describe(&stopped));
    let systemctl = case.calls("systemctl");
    assert_eq!(
        count_calls(
            &systemctl,
            &["--user", "stop", "conductor_ex-alpha.service"]
        ),
        2,
        "{systemctl:?}"
    );
    assert_eq!(state(&case), "Stopped");
}

/// A controller whose unit is gone already, as after a crash, is recorded stopped without a
/// `systemctl stop`, which would fail on a unit that is not loaded.
#[test]
fn a_controller_whose_unit_is_gone_is_recorded_stopped() {
    let case = controller_case("gone");
    case.session("ex-alpha", CHARTER);
    case.unit_cgroup(CONTROLLER_CGROUP, "4294967296", "50", "256");
    let records = case.records();
    let spawned = case.run(&records, &["spawn", "--repository", "alpha"]);
    assert_eq!(spawned.status.code(), Some(0), "{}", describe(&spawned));
    case.typed(CHARTER);
    let controller = case.controller_rows()[0]["controller_id"]
        .as_str()
        .expect("a controller id")
        .to_owned();
    case.unit_states(&["LoadState=not-found ActiveState=inactive"]);
    let stopped = case.run(
        &records,
        &[
            "controller",
            "stop-controller",
            "--controller-id",
            &controller,
            "--reason",
            "it crashed",
        ],
    );
    assert_eq!(stopped.status.code(), Some(0), "{}", describe(&stopped));
    let systemctl = case.calls("systemctl");
    assert_eq!(
        count_calls(&systemctl, &["--user", "stop"]),
        0,
        "{systemctl:?}"
    );
    assert_eq!(case.controller_rows()[0]["state"], "Stopped");
}

/// A resume issued while the old unit is still stopping waits until systemd has dropped it
/// (`reset-failed` once it is inactive), then starts the new unit under the same name.
#[test]
fn resume_waits_until_the_old_unit_is_gone() {
    let case = conductor_case("resume-wait");
    write(&case.dir.join("typed.size"), "0");
    case.unit_states(&[
        "LoadState=loaded ActiveState=deactivating",
        "LoadState=loaded ActiveState=inactive",
        "LoadState=not-found ActiveState=inactive",
    ]);
    let id = "0a1b2c3d-0000-4000-8000-00000000abcd";
    let output = case.run(&case.dir.join("work"), &[RESUME, id]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let systemctl = case.calls("systemctl");
    let shows = systemctl
        .iter()
        .filter(|call| call.iter().any(|arg| arg == "LoadState"))
        .count();
    assert_eq!(shows, 3, "{systemctl:?}");
    assert_eq!(
        count_calls(
            &systemctl,
            &["--user", "reset-failed", "conductor_conductor.service"]
        ),
        1,
        "{systemctl:?}"
    );
    assert_eq!(case.calls("systemd-run").len(), 1);
}

/// A resume while the old unit still runs starts nothing: two sessions would share one
/// transcript.
#[test]
fn resume_refuses_while_the_old_unit_still_runs() {
    let case = conductor_case("resume-running");
    case.unit_states(&["LoadState=loaded ActiveState=active"]);
    let output = case.run(
        &case.dir.join("work"),
        &[RESUME, "0a1b2c3d-0000-4000-8000-00000000abcd"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("conductor_conductor.service") && stderr.contains("active"),
        "{stderr}"
    );
    assert!(case.calls("systemd-run").is_empty());
}
