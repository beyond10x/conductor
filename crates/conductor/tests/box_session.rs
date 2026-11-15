//! `story:box-session-without-bg`: a controller starts, takes a message, stops and resumes in a
//! process that may not create a Unix socket, as a runtime in a substrate box may not.
//!
//! Every `conductor` command of a case runs under a seccomp filter that refuses
//! `socket(AF_UNIX)` with `EACCES`, as a box does ([`refuse_unix_sockets`]; Linux only), and its
//! programs inherit it. The config file asks for the `none` provider; conductor takes the `child`
//! provider in its place because the socket is refused ([`conductor_cli::confine::child`]).
//!
//! `claude` is a stand-in first on `PATH`: `claude agents --json` prints `[]`, as the harness does
//! when its background service cannot be reached; `claude -p` with stream-json input reads one
//! message per line until its input ends, keeps each in `inbox`, and answers each with the
//! harness's `init` line, the message replayed and a `result`; anything else (`--bg`, `attach`,
//! `stop`, `rm`) exits 1. Every call is logged to `claude.log`.
//!
//! Each case is a directory of its own under this test target's temporary directory: `home/`
//! (the `HOME` the binary runs with) with the checkouts root `example-org/` holding the checkout
//! `alpha` and `~/.claude.json` marking it trusted, `records/` with the charter, `state/`,
//! `roles/` and the config file.

#![cfg(target_os = "linux")]

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::{FileTypeExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime};

use conductor_cli::collect::Collector;
use conductor_cli::collect::sessions::{self, Sources};
use conductor_cli::confine::child;
use conductor_cli::snapshot;
use conductor_model::config::Checkouts;
use serde_json::{Value, json};

/// The charter the controller starts with.
const CHARTER: &str = "Charter for alpha: keep the build green.\n";

/// One case; see the module documentation. A case that passed leaves nothing behind, and no
/// stand-in session of it runs on.
struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        for entry in child::entries(&self.registry()) {
            if let Some(pid) = entry.pid.and_then(|pid| i32::try_from(pid).ok()) {
                // SAFETY: a signal to the process group the case's session leads.
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }
        }
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("box_session")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        case.fixture();
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn root(&self) -> PathBuf {
        self.home().join("example-org")
    }

    fn checkout(&self) -> PathBuf {
        self.root().join("alpha")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn registry(&self) -> PathBuf {
        self.home().join(".claude").join(child::REGISTRY)
    }

    fn inbox(&self) -> Vec<Value> {
        fs::read_to_string(self.dir.join("inbox"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).expect("a JSON line in the inbox"))
            .collect()
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("claude.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn fixture(&self) {
        write(&self.checkout().join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&self.records().join("charters/alpha.md"), CHARTER);
        fs::create_dir_all(self.state()).expect("create the state directory");
        let profile = self.dir.join("roles/repo-controller.md");
        let settings = self.dir.join("roles/controller-settings.json");
        write(&profile, "# Repository controller\n");
        write(
            &settings,
            &json!({"env": {"CONDUCTOR_INSTANCE": "example-org"}}).to_string(),
        );
        write(
            &self.dir.join("conductor.yaml"),
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
                 \x20     - {{role: controller, harness: claude, model: sonnet, profile: \
                 {profile}, settings: {settings}}}\n\
                 \x20   confinement: {{provider: none}}\n",
                root = self.root().display(),
                trees = self.dir.join("trees").display(),
                records = self.records().display(),
                state = self.state().display(),
                profile = profile.display(),
                settings = settings.display(),
            ),
        );
        write(
            &self.home().join(".claude.json"),
            &json!({"projects": {self.checkout().display().to_string():
                {"hasTrustDialogAccepted": true}}})
            .to_string(),
        );
        let claude = self.dir.join("bin/claude");
        write(
            &claude,
            &format!(
                "#!/bin/sh\n\
                 printf '%s\\n' \"$*\" >> '{log}'\n\
                 case \"$1\" in\n\
                 \x20 agents) echo '[]'; exit 0 ;;\n\
                 \x20 -p) ;;\n\
                 \x20 *) echo \"no background service: $1\" >&2; exit 1 ;;\n\
                 esac\n\
                 id=; prev=\n\
                 for arg; do\n\
                 \x20 if [ \"$prev\" = --session-id ] || [ \"$prev\" = --resume ]; then id=$arg; fi\n\
                 \x20 prev=$arg\n\
                 done\n\
                 while IFS= read -r line; do\n\
                 \x20 printf '%s\\n' \"$line\" >> '{inbox}'\n\
                 \x20 printf '{{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"%s\"}}\\n' \"$id\"\n\
                 \x20 printf '%s\\n' \"$line\"\n\
                 \x20 printf '{{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"%s\"}}\\n' \"$id\"\n\
                 done\n\
                 echo 'input ended' >> '{log}'\n",
                log = self.dir.join("claude.log").display(),
                inbox = self.dir.join("inbox").display(),
            ),
        );
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// `conductor <args>` from the records directory with the case's home, config file and
    /// stand-in, `input` on its standard input, under the filter that refuses Unix sockets.
    fn run_with_input(&self, args: &[&str], input: &str) -> Output {
        let mut command = common::conductor(self.home());
        command
            .args(args)
            .current_dir(self.records())
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.dir.join("bin").display()),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        refuse_unix_sockets(&mut command);
        let mut child = command.spawn().expect("conductor runs");
        child
            .stdin
            .take()
            .expect("its standard input")
            .write_all(input.as_bytes())
            .expect("write its standard input");
        child.wait_with_output().expect("conductor ends")
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_with_input(args, "")
    }

    /// The id of the case's one controller, from `controller controllers`.
    fn controller_id(&self) -> String {
        let output = self.run(&["controller", "controllers", "--format", "json"]);
        let rows: Value = serde_json::from_slice(&succeeded(&output, "controller controllers"))
            .expect("the controllers as JSON");
        rows[0]["controller_id"]
            .as_str()
            .expect("a controller id")
            .to_owned()
    }

    /// The registry's live entry of the controller `ex-alpha`.
    fn live(&self) -> Option<child::Entry> {
        child::live_named(&self.registry(), "ex-alpha")
    }

    /// Waits until the inbox holds `count` messages.
    fn wait_for_inbox(&self, count: usize) -> Vec<Value> {
        let started = Instant::now();
        loop {
            let inbox = self.inbox();
            if inbox.len() >= count || started.elapsed() > Duration::from_secs(20) {
                return inbox;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Every socket file under the case's directory.
    fn sockets(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut dirs = vec![self.dir.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir).expect("read a directory") {
                let entry = entry.expect("an entry");
                let kind = entry.file_type().expect("its type");
                if kind.is_dir() {
                    dirs.push(entry.path());
                } else if kind.is_socket() {
                    found.push(entry.path());
                }
            }
        }
        found
    }
}

/// Makes `command` run under a seccomp filter that refuses `socket(AF_UNIX, …)` with `EACCES` and
/// allows every other call, as a substrate box refuses it; the filter holds for every program the
/// command starts.
fn refuse_unix_sockets(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    const LOAD_WORD_ABS: u16 = 0x20;
    const JUMP_EQ: u16 = 0x05 | 0x10;
    const RETURN: u16 = 0x06;
    const ALLOW: u32 = 0x7fff_0000;
    const ERRNO: u32 = 0x0005_0000;
    let statement = |code: u16, k: u32| libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    };
    let jump = |k: u32, jt: u8, jf: u8| libc::sock_filter {
        code: JUMP_EQ,
        jt,
        jf,
        k,
    };
    let socket = u32::try_from(libc::SYS_socket).expect("a syscall number");
    let unix = u32::try_from(libc::AF_UNIX).expect("a family");
    let refused = ERRNO | u32::try_from(libc::EACCES).expect("an errno");
    // seccomp_data: the call's number at 0, its first argument's low word at 16.
    let mut program = vec![
        statement(LOAD_WORD_ABS, 0),
        jump(socket, 0, 3),
        statement(LOAD_WORD_ABS, 16),
        jump(unix, 0, 1),
        statement(RETURN, refused),
        statement(RETURN, ALLOW),
    ];
    // SAFETY: between fork and exec the closure makes two `prctl` calls, which allocate nothing;
    // the program it points at is the closure's own copy.
    unsafe {
        command.pre_exec(move || {
            let filter = libc::sock_fprog {
                len: u16::try_from(program.len()).unwrap_or(0),
                filter: program.as_mut_ptr(),
            };
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
                || libc::prctl(
                    libc::PR_SET_SECCOMP,
                    libc::SECCOMP_MODE_FILTER,
                    &raw const filter,
                ) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn succeeded(output: &Output, what: &str) -> Vec<u8> {
    assert!(output.status.success(), "{what}: {}", describe(output));
    output.stdout.clone()
}

fn gone(pid: u32) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if !child::alive(pid, "") {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// The sessions view after a snapshot of the sessions collector over the stand-in's list.
fn sessions_view(case: &Case) -> Vec<Value> {
    let sources = Sources {
        agents: vec![
            case.dir.join("bin/claude").into_os_string(),
            OsString::from("agents"),
            OsString::from("--json"),
        ],
        codex: case.dir.join("no-codex"),
        home: case.home(),
        now: SystemTime::now(),
        bound: Duration::from_secs(30),
    };
    let checkouts = Checkouts {
        root: case.root().display().to_string(),
        trees: case.dir.join("trees").display().to_string(),
    };
    let collector = Collector::new("sessions", move |record| {
        sessions::collect_in(&sources, &checkouts, record)
    });
    snapshot::take_in(&case.state(), &[collector]).expect("the snapshot is taken");
    let view = common::conductor(case.home())
        .current_dir(case.records())
        .arg("--state-dir")
        .arg(case.state())
        .args(["snapshot", "sessions", "--format", "jsonl"])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    String::from_utf8_lossy(&succeeded(&view, "snapshot sessions"))
        .lines()
        .map(|line| serde_json::from_str(line).expect("a JSON row"))
        .collect()
}

/// The variable that makes [`unix_socket_probe`] probe.
const PROBE: &str = "BOX_SESSION_PROBE";

/// Under the filter ([`the_filter_refuses_a_unix_socket_and_allows_a_socket_pair`]): a Unix socket
/// is refused as a box refuses it, and a socket pair, which a box allows, is not. A plain run of
/// the suite passes it at once.
#[test]
fn unix_socket_probe() {
    if std::env::var_os(PROBE).is_none() {
        return;
    }
    let refused = std::os::unix::net::UnixDatagram::unbound().expect_err("the socket is refused");
    assert_eq!(refused.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(child::unix_sockets_refused());
    std::os::unix::net::UnixStream::pair().expect("a socket pair is made");
}

/// The filter refuses a Unix socket and nothing else the case needs: this test binary, run under
/// it, finds the socket refused and a socket pair made ([`unix_socket_probe`]).
#[test]
fn the_filter_refuses_a_unix_socket_and_allows_a_socket_pair() {
    let mut command = Command::new(std::env::current_exe().expect("this test binary"));
    command
        .args(["--exact", "unix_socket_probe", "--test-threads=1"])
        .env(PROBE, "1");
    refuse_unix_sockets(&mut command);
    let output = command.output().expect("the probe runs");
    assert!(output.status.success(), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "{}",
        describe(&output)
    );
}

/// spawn → deliver → pause (stop) → resume → deliver → stop, all where no Unix socket may be
/// made: the controller runs as a child process with stream-json input, takes the charter and
/// each message on its input, is listed live with its id and the `child` provider's record, is
/// stopped by a signal and resumed from its id, and no command calls `--bg`, `attach`, `stop` or
/// `rm`, and no socket file is made.
#[test]
fn a_controller_runs_through_spawn_deliver_stop_and_resume_without_a_unix_socket() {
    active::isolate();
    let case = Case::new("lifecycle");

    // Spawn: the child provider, chosen for the refused socket, starts `claude -p`.
    let output = case.run(&["spawn", "--repository", "alpha"]);
    let printed = String::from_utf8(succeeded(&output, "spawn")).expect("UTF-8");
    let id = printed.trim().to_owned();
    assert_eq!(id.len(), 36, "spawn prints the session's UUID: {printed:?}");
    let entry = case.live().expect("a live child session named ex-alpha");
    assert_eq!(entry.session_id, id);
    assert_eq!(entry.cwd, case.checkout().display().to_string());
    let first_pid = entry.pid.expect("a pid");
    let calls = case.calls();
    let launch = calls
        .iter()
        .find(|call| call.starts_with("-p "))
        .expect("a `claude -p` launch");
    for word in [
        "--input-format stream-json",
        "--output-format stream-json",
        "--verbose",
        &format!("--session-id {id}"),
        "-n ex-alpha",
        "--model sonnet",
        "--permission-mode bypassPermissions",
        "--setting-sources user,project,local",
    ] {
        assert!(launch.contains(word), "{word:?} in {launch:?}");
    }
    let inbox = case.wait_for_inbox(1);
    assert_eq!(inbox.len(), 1, "{inbox:?}");
    assert_eq!(inbox[0]["type"], "user");
    assert_eq!(inbox[0]["message"]["content"], CHARTER);

    // The measurement names the child provider and the id.
    let record: Value = serde_json::from_slice(
        &fs::read(case.state().join("sessions/ex-alpha.envelope.json")).expect("a measurement"),
    )
    .expect("JSON");
    assert_eq!(record["session"]["provider"], child::NAME, "{record:#}");
    assert_eq!(record["session"]["session_id"], id, "{record:#}");
    assert_eq!(record["unboxed"], child::CHILD_UNBOXED, "{record:#}");

    // Deliver: one stream-json message on the session's input.
    let text = "conductor: alpha: build the release notes\nwith a second line";
    let output = case.run_with_input(
        &["session", "deliver", "--name", "ex-alpha", "--text", "-"],
        text,
    );
    succeeded(&output, "session deliver");
    let inbox = case.wait_for_inbox(2);
    assert_eq!(inbox.len(), 2, "{inbox:?}");
    assert_eq!(inbox[1]["message"]["content"], text);

    // The sessions collector lists it live, with its id and the record's provider.
    let rows = sessions_view(&case);
    let row = rows
        .iter()
        .find(|row| row["name"] == "ex-alpha")
        .unwrap_or_else(|| panic!("ex-alpha among {rows:#?}"));
    assert_eq!(row["session_ref"], id, "{row:#}");
    assert_eq!(row["repository"], "alpha", "{row:#}");
    assert_eq!(row["envelope_unboxed"], child::CHILD_UNBOXED, "{row:#}");

    // Pause: the session is stopped by a signal and its files removed.
    let controller = case.controller_id();
    let output = case.run(&[
        "controller",
        "pause-controller",
        "--controller-id",
        &controller,
    ]);
    succeeded(&output, "pause-controller");
    assert!(
        gone(first_pid),
        "the paused session's process {first_pid} ended"
    );
    assert!(case.live().is_none());
    assert!(!case.registry().join(format!("{id}.in")).exists());

    // Resume: `claude -p --resume <id>`, which takes the next message.
    let output = case.run(&[
        "controller",
        "resume-controller",
        "--controller-id",
        &controller,
    ]);
    succeeded(&output, "resume-controller");
    let entry = case.live().expect("the resumed session is live");
    assert_eq!(entry.session_id, id);
    let second_pid = entry.pid.expect("a pid");
    assert_ne!(second_pid, first_pid);
    assert!(
        case.calls()
            .iter()
            .any(|call| call.starts_with("-p ") && call.contains(&format!("--resume {id}"))),
        "{:?}",
        case.calls()
    );
    let output = case.run_with_input(
        &["session", "deliver", "--name", "ex-alpha", "--text", "-"],
        "conductor: alpha: go on",
    );
    succeeded(&output, "session deliver after the resume");
    let inbox = case.wait_for_inbox(3);
    assert_eq!(inbox.len(), 3, "{inbox:?}");
    assert_eq!(inbox[2]["message"]["content"], "conductor: alpha: go on");

    // Stop.
    let output = case.run(&[
        "controller",
        "stop-controller",
        "--controller-id",
        &controller,
        "--reason",
        "the case is over",
    ]);
    succeeded(&output, "stop-controller");
    assert!(
        gone(second_pid),
        "the stopped session's process {second_pid} ended"
    );
    assert!(case.live().is_none());

    // No background service was asked for, and no socket file was made.
    for call in case.calls() {
        assert!(
            call.starts_with("-p ") || call == "agents --json" || call == "input ended",
            "an unexpected harness call: {call:?}"
        );
    }
    assert_eq!(case.sockets(), Vec::<PathBuf>::new());
}

/// A delivery to a child session that is not running is refused, naming both routes, and records
/// nothing.
#[test]
fn a_delivery_to_no_running_session_is_refused() {
    let case = Case::new("no-session");
    let output = case.run_with_input(
        &["session", "deliver", "--name", "ex-alpha", "--text", "-"],
        "conductor: alpha: hello",
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SessionNotReachable"),
        "{}",
        describe(&output)
    );
    assert!(case.inbox().is_empty());
}
