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
use std::net::{SocketAddr, TcpListener};
use std::os::unix::fs::{FileTypeExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
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
        for entry in child::entries(&self.registry())
            .into_iter()
            .chain(child::entries(&self.box_registry()))
        {
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

    /// The registry of the case's box: the harness's state directory a box names in the
    /// instance's state directory (`CLAUDE_CONFIG_DIR`, `story:box-forge-work`).
    fn box_registry(&self) -> PathBuf {
        self.state().join("claude").join(child::REGISTRY)
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
        self.run_at(Side::Plain, args, input)
    }

    /// `conductor <args>` as [`Self::run_with_input`] runs it, on `side`: in the case's box
    /// (the filter, and the box's `CLAUDE_CONFIG_DIR`), or on its host (neither).
    fn run_at(&self, side: Side, args: &[&str], input: &str) -> Output {
        let mut command = common::conductor(self.home());
        if side == Side::Box {
            command.env("CLAUDE_CONFIG_DIR", self.state().join("claude"));
        }
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
        if side != Side::Host {
            refuse_unix_sockets(&mut command);
        }
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
        self.controller()["controller_id"]
            .as_str()
            .expect("a controller id")
            .to_owned()
    }

    /// The case's one controller, as `controller controllers` lists it.
    fn controller(&self) -> Value {
        let output = self.run(&["controller", "controllers", "--format", "json"]);
        let rows: Value = serde_json::from_slice(&succeeded(&output, "controller controllers"))
            .expect("the controllers as JSON");
        rows[0].clone()
    }

    /// Makes the case's instance one in a box (`placement: systemd`) whose conductord listens on
    /// `port`, and keeps the daemon's token where a local client reads it.
    fn make_boxed(&self, port: u16) {
        let text = fs::read_to_string(self.dir.join("conductor.yaml")).expect("the config file");
        let text = text
            .replace(
                "version: conductor.config/1\n",
                &format!("version: conductor.config/2\ndaemon: {{port: {port}}}\n"),
            )
            .replace(
                "  - name: example-org\n",
                "  - name: example-org\n    placement: systemd\n",
            )
            .replace("    confinement: {provider: none}\n", "");
        write(&self.dir.join("conductor.yaml"), &text);
        // The box's harness reads its trust from its own state directory.
        let trust = fs::read_to_string(self.home().join(".claude.json")).expect("the trust file");
        write(&self.state().join("claude/.claude.json"), &trust);
        write(
            &self
                .home()
                .join(conductor_cli::daemon::DIR)
                .join(conductor_cli::daemon::api::TOKEN_FILE),
            DAEMON_TOKEN,
        );
    }

    /// The case's runtime, `conductor instance serve` in its box without the loop, dialing its
    /// API out to `dial`.
    fn serve_in_box(&self, dial: SocketAddr) -> Running {
        let mut command = common::conductor(self.home());
        command
            .args(["instance", "serve"])
            .current_dir(self.state())
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.dir.join("bin").display()),
            )
            .env("CLAUDE_CONFIG_DIR", self.state().join("claude"))
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .env(conductor_cli::config::INSTANCE_VARIABLE, "example-org")
            .env(conductor_cli::instance::TOKEN_VARIABLE, INSTANCE_TOKEN)
            .env(conductor_cli::instance::DIAL_VARIABLE, dial.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        refuse_unix_sockets(&mut command);
        let running = Running(command.spawn().expect("instance serve starts"));
        let until = Instant::now() + Duration::from_secs(30);
        while conductor_cli::daemon::dial::POOL.idle("example-org") == 0 {
            assert!(Instant::now() < until, "the runtime dialed nothing in");
            std::thread::sleep(Duration::from_millis(50));
        }
        running
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

/// Where a case's command runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// Under the filter, as every command of a case without a box runs.
    Plain,
    /// In the case's box: under the filter, with the box's `CLAUDE_CONFIG_DIR`.
    Box,
    /// On the box's host: neither.
    Host,
}

/// The token the daemon minted for the instance's runtime.
const INSTANCE_TOKEN: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// conductord's own token, which a local client reads from its directory.
const DAEMON_TOKEN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// A runtime started for a case, killed when dropped.
struct Running(std::process::Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Held by each case that serves conductord: the dial-in pool is one per process
/// ([`conductor_cli::daemon::dial::POOL`]) and keyed by the instance's name, which every case
/// shares, so two such cases at once would take each other's connections.
static CONDUCTORD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// conductord's API in this process, its one instance `example-org` routed over the connections
/// its runtime dials in, with the dial-in listener: the API's port and the listener's address.
fn conductord() -> (u16, SocketAddr) {
    use conductor_cli::daemon::api::{self, Api, Route};
    let api = Arc::new(Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None));
    api.publish(
        vec![json!({"name": "example-org", "state": "Running"})],
        [(
            "example-org".to_owned(),
            Route::dialed("example-org", INSTANCE_TOKEN.to_owned()),
        )]
        .into_iter()
        .collect(),
        Vec::new(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the API's listener");
    let port = listener.local_addr().expect("its address").port();
    let served = Arc::clone(&api);
    std::thread::spawn(move || api::serve(&listener, &served));
    let dialed = TcpListener::bind("127.0.0.1:0").expect("bind the dial-in listener");
    let dial = dialed.local_addr().expect("its address");
    std::thread::spawn(move || conductor_cli::daemon::dial::serve(&dialed, &api));
    (port, dial)
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
    // The resume tells the session first that alpha has no open dispatch (#202).
    let inbox = case.wait_for_inbox(4);
    assert_eq!(inbox.len(), 4, "{inbox:?}");
    assert!(
        inbox[2]["message"]["content"].as_str().is_some_and(
            |text| text.starts_with("[REPORT alpha -] resumed: alpha has no open dispatch")
        ),
        "{inbox:?}"
    );
    assert_eq!(inbox[3]["message"]["content"], "conductor: alpha: go on");

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

/// Issue #226 (`story:resume-starts-gone-session`): a controller whose session died unstopped (its
/// box restarted) and was then paused keeps the session's output, with the `init` line its first
/// run wrote. A resume whose harness ends at once, as one that cannot resume the transcript does,
/// is no resume: `resume-controller` exits 1 naming how the session ended, prints no `resumed`
/// line, and the controller stays `Paused`; no live session holds the name.
#[test]
fn a_resume_that_ends_at_once_leaves_the_controller_paused() {
    active::isolate();
    let case = Case::new("resume-ends-at-once");
    let output = case.run(&["spawn", "--repository", "alpha"]);
    let id = String::from_utf8(succeeded(&output, "spawn")).expect("UTF-8");
    let id = id.trim().to_owned();
    let pid = case.live().and_then(|entry| entry.pid).expect("a live pid");
    assert_eq!(case.wait_for_inbox(1).len(), 1);
    // The box restarts: the session's process ends with no stop.
    // SAFETY: a signal to the process group the case's session leads.
    unsafe {
        libc::kill(-i32::try_from(pid).expect("a pid"), libc::SIGKILL);
    }
    assert!(gone(pid), "the session's process {pid} ended");
    let controller = case.controller_id();
    let output = case.run(&[
        "controller",
        "pause-controller",
        "--controller-id",
        &controller,
    ]);
    succeeded(&output, "pause-controller of a gone session");
    assert_eq!(case.controller()["state"], "Paused");
    assert!(
        fs::read_to_string(case.registry().join(format!("{id}.out")))
            .is_ok_and(|text| text.contains("\"init\"")),
        "the gone session's output holds its first run's init line"
    );

    // The harness cannot resume: it ends at once, after it took its first input.
    let bin = case.dir.join("bin");
    fs::rename(bin.join("claude"), bin.join("claude-stand-in")).expect("keep the stand-in");
    let claude = bin.join("claude");
    write(
        &claude,
        &format!(
            "#!/bin/sh\n\
             for arg; do\n\
             \x20 if [ \"$arg\" = --resume ]; then\n\
             \x20   printf '%s\\n' \"$*\" >> '{log}'\n\
             \x20   sleep 0.5\n\
             \x20   echo 'No conversation found to resume' >&2\n\
             \x20   exit 1\n\
             \x20 fi\n\
             done\n\
             exec '{stand_in}' \"$@\"\n",
            log = case.dir.join("claude.log").display(),
            stand_in = bin.join("claude-stand-in").display(),
        ),
    );
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("make it runnable");

    let output = case.run(&[
        "controller",
        "resume-controller",
        "--controller-id",
        &controller,
    ]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("the controller stays paused")
            && said.contains("ended as it started")
            && said.contains("No conversation found to resume"),
        "{}",
        describe(&output)
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("resumed"),
        "{}",
        describe(&output)
    );
    assert!(
        case.calls()
            .iter()
            .any(|call| call.contains(&format!("--resume {id}"))),
        "the resume was tried: {:?}",
        case.calls()
    );
    assert_eq!(case.controller()["state"], "Paused");
    assert!(case.live().is_none(), "no live session holds the name");
}

/// Issue #226: on the host, a boxed instance's paused controller is not resumed: the session would
/// start outside the box, where its transcript is not. `resume-controller` refuses `OutsideBox`
/// before the harness is called, and the controller stays `Paused`.
#[test]
fn on_the_host_a_boxed_instance_resumes_no_session_outside_its_box() {
    active::isolate();
    let case = Case::new("host-resumes-nothing");
    case.make_boxed(7399);
    let output = case.run_at(Side::Box, &["spawn", "--repository", "alpha"], "");
    succeeded(&output, "spawn in the box");
    let pid = child::live_named(&case.box_registry(), "ex-alpha")
        .and_then(|entry| entry.pid)
        .expect("a live child session in the box");
    assert_eq!(case.wait_for_inbox(1).len(), 1);
    let controller = case.controller_id();
    let output = case.run_at(
        Side::Box,
        &[
            "controller",
            "pause-controller",
            "--controller-id",
            &controller,
        ],
        "",
    );
    succeeded(&output, "pause-controller in the box");
    assert!(gone(pid), "the paused session's process {pid} ended");
    assert_eq!(case.controller()["state"], "Paused");

    let before = case.calls().len();
    let output = case.run_at(
        Side::Host,
        &[
            "controller",
            "resume-controller",
            "--controller-id",
            &controller,
        ],
        "",
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("the controller stays paused")
            && said.contains("OutsideBox")
            && said.contains("conductor controller stop-controller"),
        "{}",
        describe(&output)
    );
    assert!(
        !case.calls()[before..]
            .iter()
            .any(|call| call.contains("--resume")),
        "the harness was not asked to resume: {:?}",
        case.calls()
    );
    assert_eq!(case.controller()["state"], "Paused");
    assert!(child::entries(&case.registry()).is_empty());
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

/// Issues #216 and #220: on the host, a boxed instance's controller (`spawn`) and director
/// (`session start --role director`) start nowhere: each refuses `OutsideBox`, naming `conductor
/// instance cycle`, before the harness is called or anything is recorded.
#[test]
fn on_the_host_a_boxed_instance_starts_no_session_outside_its_box() {
    let case = Case::new("host-starts-nothing");
    case.make_boxed(7399);

    let output = case.run_at(Side::Host, &["spawn", "--repository", "alpha"], "");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("spawn: refused: OutsideBox") && said.contains("conductor instance cycle"),
        "{}",
        describe(&output)
    );

    let output = case.run_at(Side::Host, &["session", "start", "--role", "director"], "");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("session start: refused: OutsideBox")
            && said.contains("conductor instance cycle"),
        "{}",
        describe(&output)
    );
    assert_eq!(
        case.calls(),
        Vec::<String>::new(),
        "the harness was not called"
    );
    assert!(
        !case.records().join("dispatches").exists(),
        "nothing was recorded"
    );
    assert!(child::entries(&case.box_registry()).is_empty());
    assert!(child::entries(&case.registry()).is_empty());
}

/// Issue #215: a delivery to a child session whose process is gone names the session gone and
/// does not ask the session list; from the host, a boxed session whose every entry ended is named
/// gone too, without the runtime.
#[test]
fn a_delivery_to_a_gone_child_session_names_it_gone() {
    let case = Case::new("gone-session");
    let output = case.run(&["spawn", "--repository", "alpha"]);
    succeeded(&output, "spawn");
    let entry = case.live().expect("a live child session");
    let pid = entry.pid.expect("a pid");
    assert_eq!(case.wait_for_inbox(1).len(), 1);
    // SAFETY: a signal to the process group the case's session leads.
    unsafe {
        libc::kill(-i32::try_from(pid).expect("a pid"), libc::SIGKILL);
    }
    let until = Instant::now() + Duration::from_secs(10);
    while !gone(pid) {
        assert!(Instant::now() < until, "the session's process {pid} ended");
        std::thread::sleep(Duration::from_millis(50));
    }
    let before = case.calls().len();
    let output = case.run_with_input(
        &["session", "deliver", "--name", "ex-alpha", "--text", "-"],
        "conductor: alpha: hello",
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("SessionNotReachable")
            && said.contains("the session is gone")
            && said.contains(&entry.session_id),
        "{}",
        describe(&output)
    );
    assert_eq!(
        case.calls()[before..].to_vec(),
        Vec::<String>::new(),
        "the session list was not asked"
    );

    // From the host, an entry of the box that ended (an exit code recorded) is named gone.
    case.make_boxed(7399);
    let ended = case.box_registry().join("e0000001.json");
    write(
        &ended,
        &json!({
            "name": "ex-beta", "session_id": "e0000001", "cwd": case.checkout(),
            "pid": 4_194_305, "started_at": 1, "exit_code": 137,
        })
        .to_string(),
    );
    let output = case.run_at(
        Side::Host,
        &["session", "deliver", "--name", "ex-beta", "--text", "-"],
        "conductor: beta: hello",
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("SessionNotReachable")
            && said.contains("the session is gone")
            && said.contains("exit code 137")
            && !said.contains("runtime was not reached"),
        "{}",
        describe(&output)
    );
}

/// `story:host-commands-reach-boxed-session`: a controller runs in the box of a boxed instance,
/// started there as a child session, where the host can neither signal its process nor see its
/// registry as its own. From the host, `session deliver` puts the text on the session's input and
/// `stop-controller` ends the session's process before the record is stopped, both through
/// conductord and the runtime the box dialed in.
#[test]
fn host_commands_reach_a_child_session_in_the_box_through_the_runtime() {
    let _one = CONDUCTORD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    active::isolate();
    let case = Case::new("host-reaches-box");
    let (port, dial) = conductord();
    case.make_boxed(port);
    let _runtime = case.serve_in_box(dial);

    let output = case.run_at(Side::Box, &["spawn", "--repository", "alpha"], "");
    let id = String::from_utf8(succeeded(&output, "spawn in the box")).expect("UTF-8");
    let id = id.trim();
    let entry = child::live_named(&case.box_registry(), "ex-alpha")
        .expect("a live child session in the box's registry");
    assert_eq!(entry.session_id, id);
    let pid = entry.pid.expect("a pid");
    assert!(case.live().is_none(), "the host's own registry holds none");
    assert_eq!(case.wait_for_inbox(1).len(), 1);

    // Deliver from the host: the runtime writes it to the session's input.
    let text = "NEED answered: ship it\nwith a second line";
    let output = case.run_at(
        Side::Host,
        &["session", "deliver", "--name", "ex-alpha", "--text", "-"],
        text,
    );
    let printed =
        String::from_utf8(succeeded(&output, "session deliver from the host")).expect("UTF-8");
    assert!(!printed.trim().is_empty(), "the delivery's id: {printed:?}");
    let inbox = case.wait_for_inbox(2);
    assert_eq!(inbox.len(), 2, "{inbox:?}");
    assert_eq!(inbox[1]["message"]["content"], text);

    // Stop from the host: the runtime ends the process, then the record is stopped.
    let controller = case.controller_id();
    let output = case.run_at(
        Side::Host,
        &[
            "controller",
            "stop-controller",
            "--controller-id",
            &controller,
            "--reason",
            "the operator stops it",
        ],
        "",
    );
    let printed =
        String::from_utf8(succeeded(&output, "stop-controller from the host")).expect("UTF-8");
    assert_eq!(printed.trim(), format!("stopped ex-alpha ({id})"));
    assert!(gone(pid), "the stopped session's process {pid} ended");
    assert!(child::live_named(&case.box_registry(), "ex-alpha").is_none());
    assert_eq!(case.controller()["state"], "Stopped");
}

/// With the box's runtime not reached, the host's `session deliver` and `stop-controller` for a
/// session in the box refuse: nothing is delivered, the session runs on, and the controller stays
/// `Running`.
#[test]
fn host_commands_refuse_and_change_nothing_without_the_runtime() {
    let _one = CONDUCTORD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    active::isolate();
    let case = Case::new("host-without-runtime");
    let (port, _dial) = conductord();
    case.make_boxed(port);

    let output = case.run_at(Side::Box, &["spawn", "--repository", "alpha"], "");
    succeeded(&output, "spawn in the box");
    let pid = child::live_named(&case.box_registry(), "ex-alpha")
        .and_then(|entry| entry.pid)
        .expect("a live child session in the box");
    assert_eq!(case.wait_for_inbox(1).len(), 1);

    let output = case.run_at(
        Side::Host,
        &["session", "deliver", "--name", "ex-alpha", "--text", "-"],
        "NEED answered: ship it",
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("SessionNotReachable") && said.contains("runtime was not reached"),
        "{}",
        describe(&output)
    );

    let controller = case.controller_id();
    let output = case.run_at(
        Side::Host,
        &[
            "controller",
            "stop-controller",
            "--controller-id",
            &controller,
            "--reason",
            "the operator stops it",
        ],
        "",
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("not recorded stopped"),
        "{}",
        describe(&output)
    );
    assert_eq!(case.controller()["state"], "Running");
    assert!(child::alive(pid, ""), "the session's process {pid} runs on");
    assert_eq!(case.inbox().len(), 1, "nothing was delivered");
}

/// `story:pause-reaches-boxed-session`: from the host, `pause-controller` for a controller whose
/// session runs in the box goes to the box's runtime as `stop-controller` does: the runtime ends
/// the session's process, then the record is `Paused`.
#[test]
fn host_pause_reaches_a_child_session_in_the_box_through_the_runtime() {
    let _one = CONDUCTORD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    active::isolate();
    let case = Case::new("host-pauses-box");
    let (port, dial) = conductord();
    case.make_boxed(port);
    let _runtime = case.serve_in_box(dial);

    let output = case.run_at(Side::Box, &["spawn", "--repository", "alpha"], "");
    let id = String::from_utf8(succeeded(&output, "spawn in the box")).expect("UTF-8");
    let id = id.trim();
    let pid = child::live_named(&case.box_registry(), "ex-alpha")
        .and_then(|entry| entry.pid)
        .expect("a live child session in the box");
    assert!(case.live().is_none(), "the host's own registry holds none");
    assert_eq!(case.wait_for_inbox(1).len(), 1);

    let controller = case.controller_id();
    let output = case.run_at(
        Side::Host,
        &[
            "controller",
            "pause-controller",
            "--controller-id",
            &controller,
        ],
        "",
    );
    let printed =
        String::from_utf8(succeeded(&output, "pause-controller from the host")).expect("UTF-8");
    assert_eq!(printed.trim(), format!("stopped ex-alpha ({id})"));
    assert!(gone(pid), "the paused session's process {pid} ended");
    assert!(child::live_named(&case.box_registry(), "ex-alpha").is_none());
    assert_eq!(case.controller()["state"], "Paused");
}

/// With the box's runtime not reached, the host's `pause-controller` for a session in the box
/// refuses: the session runs on and the controller stays `Running`.
#[test]
fn host_pause_refuses_and_changes_nothing_without_the_runtime() {
    let _one = CONDUCTORD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    active::isolate();
    let case = Case::new("host-pause-without-runtime");
    let (port, _dial) = conductord();
    case.make_boxed(port);

    let output = case.run_at(Side::Box, &["spawn", "--repository", "alpha"], "");
    succeeded(&output, "spawn in the box");
    let pid = child::live_named(&case.box_registry(), "ex-alpha")
        .and_then(|entry| entry.pid)
        .expect("a live child session in the box");
    assert_eq!(case.wait_for_inbox(1).len(), 1);

    let controller = case.controller_id();
    let output = case.run_at(
        Side::Host,
        &[
            "controller",
            "pause-controller",
            "--controller-id",
            &controller,
        ],
        "",
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("runtime was not reached") && said.contains("not recorded paused"),
        "{}",
        describe(&output)
    );
    assert_eq!(case.controller()["state"], "Running");
    assert!(child::alive(pid, ""), "the session's process {pid} runs on");
}

/// Issue #215: `session deliver-message`, a placed leaf that is not written, is not in the help
/// of `conductor session`; `session deliver` is.
#[test]
fn the_unwritten_deliver_message_leaf_is_not_in_the_help() {
    let case = Case::new("deliver-message-help");
    let output = case.run_at(Side::Host, &["session", "--help"], "");
    let help = String::from_utf8(succeeded(&output, "session --help")).expect("UTF-8");
    assert!(help.contains("deliver "), "{help}");
    assert!(!help.contains("deliver-message"), "{help}");
}

/// Issue #220 (`story:controller-name-not-conductor`): under a session prefix the director is
/// `<prefix>-director`, never `<prefix>-conductor`, which is the controller of a repository named
/// `conductor`. With that repository's controller live, `session stop --role director` finds no
/// director and leaves the controller running.
#[test]
fn the_director_s_name_is_never_a_repository_controller_s() {
    let case = Case::new("director-name");
    let checkout = case.root().join("conductor");
    write(&checkout.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&case.records().join("charters/conductor.md"), CHARTER);
    write(
        &case.home().join(".claude.json"),
        &json!({"projects": {
            case.checkout().display().to_string(): {"hasTrustDialogAccepted": true},
            checkout.display().to_string(): {"hasTrustDialogAccepted": true},
        }})
        .to_string(),
    );
    let shown: Value = serde_json::from_slice(&succeeded(
        &case.run(&["config", "show", "--format", "json"]),
        "config show",
    ))
    .expect("the config as JSON");
    assert_eq!(
        shown["instances"][0]["conductor"]["session_name"], "ex-director",
        "{shown:#}"
    );

    succeeded(
        &case.run(&["spawn", "--repository", "conductor"]),
        "spawn the conductor repository's controller",
    );
    let controller = child::live_named(&case.registry(), "ex-conductor")
        .expect("the controller ex-conductor runs");

    let output = case.run(&["session", "stop", "--role", "director"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("NotRunning"),
        "{}",
        describe(&output)
    );
    assert!(
        child::live_named(&case.registry(), "ex-conductor").is_some(),
        "the controller {} runs on",
        controller.session_id
    );
}

impl Case {
    /// Makes the case's instance a boxed one whose runtime's loop runs `run --once` in its box
    /// with no unit of work to start (`story:box-runtime-starts-director`): no charter, and forge
    /// and planning stand-ins that list nothing.
    fn make_boxed_for_the_keeper(&self) {
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("bind a free port")
            .local_addr()
            .expect("its address")
            .port();
        self.make_boxed(port);
        fs::remove_file(self.records().join("charters/alpha.md")).expect("remove the charter");
        for (name, body) in [
            ("gh", "echo '[]'"),
            ("glab", "echo '[]'"),
            ("aep", "echo '[]'"),
        ] {
            let file = self.dir.join("bin").join(name);
            write(&file, &format!("#!/bin/sh\n{body}\n"));
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
                .expect("make it runnable");
        }
    }

    /// One watch tick and one cycle tick of the instance's loop, in its box.
    fn tick_in_box(&self) -> Output {
        self.run_at(Side::Box, &["run", "--once"], "")
    }

    /// The box's live entry of the director `ex-director`.
    fn director(&self) -> Option<child::Entry> {
        child::live_named(&self.box_registry(), "ex-director")
    }

    /// The dispatch log's `director-started` lines.
    fn director_started(&self) -> Vec<Value> {
        let Ok(read) = fs::read_dir(self.records().join("dispatches")) else {
            return Vec::new();
        };
        read.filter_map(Result::ok)
            .flat_map(|entry| {
                fs::read_to_string(entry.path())
                    .unwrap_or_default()
                    .lines()
                    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                    .collect::<Vec<_>>()
            })
            .filter(|line| line["event"] == "director-started")
            .collect()
    }

    /// The keeper's record, `<state>/run/director.json`.
    fn keep(&self) -> Value {
        serde_json::from_str(
            &fs::read_to_string(self.state().join("run/director.json")).expect("director.json"),
        )
        .expect("director.json is JSON")
    }

    /// Ends the director's session as a crash would: `SIGKILL` to its process group, then waits
    /// until its process is gone.
    fn kill_director(&self) {
        let pid = self
            .director()
            .and_then(|entry| entry.pid)
            .expect("a live director");
        // SAFETY: a signal to the process group the case's director session leads.
        unsafe {
            libc::kill(-i32::try_from(pid).expect("a pid"), libc::SIGKILL);
        }
        assert!(gone(pid), "the director's process {pid} ended");
    }
}

/// `story:box-runtime-starts-director`, acceptance 1: a runtime in its instance's box (sockets
/// refused, the box's `CLAUDE_CONFIG_DIR`) whose instance has a director role and no director
/// session starts one within one watch tick: the box's registry holds a live `ex-director` whose
/// working directory is the records directory, started from the profile the binary carries
/// (`--append-system-prompt-file`, no `--agent`), and the dispatch log says `director-started`,
/// `none ran`.
#[test]
fn a_boxed_runtime_starts_its_director_in_the_box() {
    let case = Case::new("keeper-starts");
    case.make_boxed_for_the_keeper();
    let output = case.tick_in_box();
    let entry = case
        .director()
        .unwrap_or_else(|| panic!("a live director in the box: {}", describe(&output)));
    assert_eq!(
        Path::new(&entry.cwd),
        case.records(),
        "it works in the records"
    );
    let start = case
        .calls()
        .into_iter()
        .find(|call| call.starts_with("-p ") && call.contains("-n ex-director "))
        .unwrap_or_else(|| panic!("the director's start: {:?}", case.calls()));
    assert!(!start.contains("--agent"), "{start}");
    let words: Vec<&str> = start.split(' ').collect();
    let file = words
        .iter()
        .position(|word| *word == "--append-system-prompt-file")
        .map(|at| words[at + 1])
        .unwrap_or_else(|| panic!("--append-system-prompt-file: {start}"));
    let shown = succeeded(
        &case.run_at(Side::Host, &["role", "show", "--role", "director"], ""),
        "role show",
    );
    assert_eq!(
        fs::read(file).expect("the profile file"),
        shown,
        "the profile is the binary's"
    );
    let lines = case.director_started();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["session"], "ex-director");
    assert_eq!(lines[0]["reason"], "none ran");
    assert_eq!(case.keep()["state"], "Running");
}

/// `story:box-runtime-starts-director`, acceptance 2: a director whose session ends is started
/// again by the next tick, `gone`; after a second early end the keeper waits the loop's back-off,
/// five minutes after the last start, and that tick starts none.
#[test]
fn a_boxed_runtime_starts_its_director_again_once_it_ends() {
    let case = Case::new("keeper-restarts");
    case.make_boxed_for_the_keeper();
    let output = case.tick_in_box();
    assert!(case.director().is_some(), "{}", describe(&output));
    case.kill_director();
    let output = case.tick_in_box();
    assert!(
        case.director().is_some(),
        "started again: {}",
        describe(&output)
    );
    let lines = case.director_started();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[1]["reason"], "gone");
    case.kill_director();
    let output = case.tick_in_box();
    assert!(
        case.director().is_none(),
        "no third start in that tick: {}",
        describe(&output)
    );
    assert_eq!(case.director_started().len(), 2);
    let keep = case.keep();
    assert_eq!(keep["state"], "Waiting", "{keep}");
    assert_eq!(keep["attempts"], 2, "{keep}");
    let at = |key: &str| {
        time::OffsetDateTime::parse(
            keep[key]
                .as_str()
                .unwrap_or_else(|| panic!("{key}: {keep}")),
            &time::format_description::well_known::Rfc3339,
        )
        .expect("an RFC 3339 instant")
    };
    assert_eq!(
        at("next_start_at") - at("started_at"),
        time::Duration::minutes(5),
        "{keep}"
    );
}

/// `story:box-runtime-starts-director`: while the keeper's director runs, a start by hand in the
/// box is refused `SessionRunning`, and the registry holds one director.
#[test]
fn a_hand_start_refuses_while_the_keepers_director_runs() {
    let case = Case::new("keeper-hand-start");
    case.make_boxed_for_the_keeper();
    let output = case.tick_in_box();
    assert!(case.director().is_some(), "{}", describe(&output));
    let output = case.run_at(Side::Box, &["session", "start", "--role", "director"], "");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SessionRunning"),
        "{}",
        describe(&output)
    );
    let live = child::entries(&case.box_registry())
        .into_iter()
        .filter(|entry| entry.name == "ex-director" && entry.live_pid().is_some())
        .count();
    assert_eq!(live, 1, "one director");
}
