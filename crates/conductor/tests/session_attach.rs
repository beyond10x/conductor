//! `story:session-attach-command`, `story:session-attach-watch` and
//! `story:session-attach-refuses-sessions`, the tmux form: a session the systemd provider starts
//! runs its harness in a tmux server of its own on `<state>/sessions/<name>.tmux`, and `conductor
//! session attach` attaches the operator's terminal to it.
//!
//! Each case runs a real `tmux` in a [`Scratch`] directory: the session's server, started as the
//! provider's unit starts it (`tmux -D -S <name>.tmux -f <name>.tmux.conf`, the configuration
//! [`systemd::TMUX_CONF`]), with a stand-in harness that prints `ready`, reads a line and prints
//! `got <line>`; and an outer server whose one pane runs `conductor session attach`, standing in
//! for the operator's terminal, typed into with `send-keys`. A session started before the tmux
//! launcher is a typescript and a FIFO with no socket, its unit answered by a fake `systemctl`
//! first on `PATH`. No real session, unit or harness is touched.

mod common;
mod scratch;

use std::fs::{self, File};
use std::io::{ErrorKind, Read as _, Write as _};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use conductor_cli::confine::systemd;
use conductor_model::config::SessionName;
use scratch::Scratch;

/// How long a case waits for a screen or a file to show what it expects.
const WAIT: Duration = Duration::from_secs(15);

/// The stand-in harness: `ready`, then one line read and echoed back.
const HARNESS: &str = r#"echo ready; read x; echo "got $x"; sleep 60"#;

/// The variables a session's processes carry, removed from every process a case starts.
const SESSION_VARIABLES: [&str; 3] = ["CONDUCTOR_SESSION", "TMUX", "TMUX_PANE"];

struct Case {
    scratch: Scratch,
    servers: Vec<Child>,
}

impl Drop for Case {
    fn drop(&mut self) {
        for child in &mut self.servers {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let scratch = Scratch::new(Path::new("session-attach").join(name));
        for sub in ["home", "state/sessions", "bin", "outer"] {
            fs::create_dir_all(scratch.path().join(sub)).expect("create a case directory");
        }
        Self {
            scratch,
            servers: Vec::new(),
        }
    }

    fn dir(&self) -> &Path {
        self.scratch.path()
    }

    fn home(&self) -> PathBuf {
        self.dir().join("home")
    }

    fn state(&self) -> PathBuf {
        self.dir().join("state")
    }

    fn sessions(&self) -> PathBuf {
        self.state().join("sessions")
    }

    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.dir().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    /// `tmux` on the socket `socket`, relative to `dir`, as a session's own server is reached.
    fn tmux(dir: &Path, socket: &str, args: &[&str]) -> Output {
        let mut command = Command::new("tmux");
        command.current_dir(dir).args(["-S", socket]).args(args);
        for variable in SESSION_VARIABLES {
            command.env_remove(variable);
        }
        command.output().expect("tmux runs")
    }

    /// Starts the session `name`'s server as the provider's unit does, running `harness`, and
    /// waits until it answers.
    fn start_session(&mut self, name: &str, harness: &str) {
        let sessions = self.sessions();
        fs::write(
            sessions.join(format!("{name}.tmux.conf")),
            systemd::TMUX_CONF,
        )
        .expect("write the configuration");
        let mut command = Command::new("tmux");
        command
            .current_dir(&sessions)
            .args(["-D", "-S", &format!("{name}.tmux")])
            .args(["-f", &format!("{name}.tmux.conf")])
            .env("CONDUCTOR_SESSION", name)
            .env("CONDUCTOR_HARNESS", harness)
            .env(
                "CONDUCTOR_TYPESCRIPT",
                sessions.join(format!("{name}.typescript")),
            )
            .env("SHELL", "/bin/sh")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for variable in SESSION_VARIABLES {
            if variable != "CONDUCTOR_SESSION" {
                command.env_remove(variable);
            }
        }
        self.servers
            .push(command.spawn().expect("the session's tmux server starts"));
        let socket = format!("{name}.tmux");
        wait_for("the session's server", || {
            Self::tmux(
                &sessions,
                &socket,
                &["has-session", "-t", &format!("={name}")],
            )
            .status
            .success()
        });
        wait_for("the harness's first line", || {
            self.session_screen(name).contains("ready")
        });
    }

    /// The session `name`'s screen, as its own server holds it.
    fn session_screen(&self, name: &str) -> String {
        let output = Self::tmux(
            &self.sessions(),
            &format!("{name}.tmux"),
            &["capture-pane", "-p", "-t", &format!("={name}:")],
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Runs `conductor --state-dir <state> session attach <args>` in the outer server's one pane,
    /// the operator's stand-in terminal, which prints `exit=<code>` when it ends.
    fn attach_in_terminal(&mut self, args: &[&str]) {
        let conductor = common::conductor(self.home());
        let mut line = format!(
            "'{}' --state-dir '{}' session attach",
            conductor.get_program().to_string_lossy(),
            self.state().display()
        );
        for arg in args {
            line.push_str(&format!(" '{arg}'"));
        }
        line.push_str("; echo \"exit=$?\"; sleep 60");
        let mut command = Command::new("tmux");
        command
            .current_dir(self.dir().join("outer"))
            .args(["-S", "o.tmux", "-f", "/dev/null", "new-session", "-d"])
            .args(["-s", "outer", "-x", "200", "-y", "60", &line])
            .env("HOME", self.home())
            .env("SHELL", "/bin/sh")
            .env("PATH", self.path())
            .env_remove("CONDUCTOR_CONFIG")
            .env_remove("CONDUCTOR_INSTANCE");
        for variable in SESSION_VARIABLES {
            command.env_remove(variable);
        }
        let output = command.output().expect("the outer tmux runs");
        assert!(output.status.success(), "{}", describe(&output));
    }

    /// The operator's stand-in terminal's screen.
    fn terminal_screen(&self) -> String {
        let output = Self::tmux(
            &self.dir().join("outer"),
            "o.tmux",
            &["capture-pane", "-p", "-t", "=outer:"],
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Keys typed in the operator's stand-in terminal, as `tmux send-keys` names them.
    fn type_keys(&self, keys: &[&str]) {
        let mut args = vec!["send-keys", "-t", "=outer:"];
        args.extend_from_slice(keys);
        let output = Self::tmux(&self.dir().join("outer"), "o.tmux", &args);
        assert!(output.status.success(), "{}", describe(&output));
    }

    fn kill_outer(&self) {
        let _ = Self::tmux(&self.dir().join("outer"), "o.tmux", &["kill-server"]);
    }

    /// `conductor --state-dir <state> <args>`, its standard input `stdin`, with `vars` set.
    fn run(&self, args: &[&str], vars: &[(&str, &str)]) -> Output {
        let mut command = common::conductor(self.home());
        command
            .args(["--state-dir", &self.state().display().to_string()])
            .args(args)
            .env("PATH", self.path());
        for variable in SESSION_VARIABLES {
            command.env_remove(variable);
        }
        for (name, value) in vars {
            command.env(name, value);
        }
        command.output().expect("conductor runs")
    }

    /// A fake `systemctl` first on `PATH` whose `show -p ActiveState --value` prints the file
    /// `active` of the case, and logs its arguments to `systemctl.log`.
    fn fake_systemctl(&self, state: &str) {
        let d = self.dir().display();
        let path = self.dir().join("bin/systemctl");
        fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{d}/systemctl.log'\ncat '{d}/active'\n"),
        )
        .expect("write the fake");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        self.unit_state(state);
    }

    fn unit_state(&self, state: &str) {
        fs::write(self.dir().join("active"), format!("{state}\n")).expect("write the state");
    }
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(started.elapsed() < WAIT, "{what}: not within {WAIT:?}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The first `count` words of each line `session list` printed.
fn first_words(output: &Output, count: usize) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            line.split_whitespace()
                .take(count)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Attach shows the session's screen, then its live output; what the operator types reaches the
/// harness; `ctrl-]` then `d` ends the client with exit 0 and the session runs on; the
/// typescript still records the harness's output; `session list` marks the session attachable.
#[test]
fn attach_types_into_the_session_and_detaches_with_ctrl_bracket_d() {
    let mut case = Case::new("read-write");
    case.start_session("ex-alpha", HARNESS);

    let list = case.run(&["session", "list"], &[]);
    assert_eq!(list.status.code(), Some(0), "{}", describe(&list));
    assert_eq!(
        first_words(&list, 3),
        ["ex-alpha attach sessions"],
        "{}",
        describe(&list)
    );

    case.attach_in_terminal(&["--name", "ex-alpha"]);
    wait_for("the session's screen in the terminal", || {
        case.terminal_screen().contains("ready")
    });
    case.type_keys(&["-l", "hello"]);
    case.type_keys(&["Enter"]);
    wait_for("the typed line in the harness", || {
        case.session_screen("ex-alpha").contains("got hello")
    });
    wait_for("the harness's answer in the terminal", || {
        case.terminal_screen().contains("got hello")
    });

    case.type_keys(&["C-]"]);
    case.type_keys(&["d"]);
    wait_for("the client's exit", || {
        case.terminal_screen().contains("exit=0")
    });
    let alive = Case::tmux(
        &case.sessions(),
        "ex-alpha.tmux",
        &["has-session", "-t", "=ex-alpha"],
    );
    assert!(alive.status.success(), "the session runs on");
    wait_for("the typescript", || {
        fs::read_to_string(case.sessions().join("ex-alpha.typescript"))
            .unwrap_or_default()
            .contains("got hello")
    });
    case.kill_outer();
}

/// `--watch` shows the screen and the live output, and nothing typed reaches the harness;
/// `ctrl-]` then `d` ends it with exit 0.
#[test]
fn watch_shows_the_session_and_types_nothing() {
    let mut case = Case::new("watch");
    case.start_session("ex-beta", HARNESS);
    case.attach_in_terminal(&["--name", "ex-beta", "--watch"]);
    wait_for("the session's screen in the terminal", || {
        case.terminal_screen().contains("ready")
    });
    case.type_keys(&["-l", "intruder"]);
    case.type_keys(&["Enter"]);
    std::thread::sleep(Duration::from_millis(800));
    assert!(
        !case.session_screen("ex-beta").contains("intruder"),
        "a watcher typed into the session: {}",
        case.session_screen("ex-beta")
    );
    // The provider still types into the session while a watcher is attached (tmux refuses
    // `send-keys` then), and the harness's answer reaches the watcher live.
    systemd::type_text(
        &case.sessions(),
        &SessionName("ex-beta".to_owned()),
        "operator",
    )
    .expect("the provider types while a watcher is attached");
    wait_for("live output in the watcher", || {
        case.terminal_screen().contains("got operator")
    });
    case.type_keys(&["C-]"]);
    case.type_keys(&["d"]);
    wait_for("the watcher's exit", || {
        case.terminal_screen().contains("exit=0")
    });
    case.kill_outer();
}

/// Inside a session (`CONDUCTOR_SESSION` set, whatever the harness) attach exits 1 naming
/// `inside-session`, before it looks at any session.
#[test]
fn attach_refuses_inside_a_session() {
    let case = Case::new("inside");
    for (variable, value) in [("CONDUCTOR_SESSION", "ex-alpha")] {
        for watch in [false, true] {
            let mut args = vec!["session", "attach", "--name", "ex-alpha"];
            if watch {
                args.push("--watch");
            }
            let output = case.run(&args, &[(variable, value)]);
            assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("inside-session"),
                "{variable}: {}",
                describe(&output)
            );
        }
    }
}

/// A name no session holds exits 1 naming `unknown-session`.
#[test]
fn attach_refuses_an_unknown_session() {
    let case = Case::new("unknown");
    case.fake_systemctl("inactive");
    let output = case.run(&["session", "attach", "--name", "ex-nobody"], &[]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown-session"),
        "{}",
        describe(&output)
    );
}

/// A session started before the tmux launcher (a typescript and a FIFO, no socket): a read-write
/// attach exits 1 naming `not-attachable` and the restart; `--watch` replays the last 64 KiB of
/// its typescript, follows it, never writes to its FIFO, and exits 0 once its unit is gone;
/// `session list` marks it `watch`.
#[test]
fn watch_follows_the_typescript_of_a_session_before_tmux() {
    let case = Case::new("before-tmux");
    case.fake_systemctl("active");
    let sessions = case.sessions();
    let typescript = sessions.join("ex-old.typescript");
    let mut screen = String::from("HEAD-BEYOND-THE-TAIL");
    screen.push_str(&"x".repeat(70 * 1024));
    screen.push_str("the old screen\n");
    fs::write(&typescript, &screen).expect("write the typescript");
    let fifo = sessions.join("ex-old.fifo");
    nix::unistd::mkfifo(
        &fifo,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )
    .expect("make the FIFO");
    let mut keyboard = File::options()
        .read(true)
        .custom_flags(nix::fcntl::OFlag::O_NONBLOCK.bits())
        .open(&fifo)
        .expect("hold the FIFO's read end, as the launcher does");

    let list = case.run(&["session", "list"], &[]);
    assert_eq!(
        first_words(&list, 3),
        ["ex-old watch -"],
        "{}",
        describe(&list)
    );

    let refused = case.run(&["session", "attach", "--name", "ex-old"], &[]);
    assert_eq!(refused.status.code(), Some(1), "{}", describe(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("not-attachable") && stderr.contains("restart"),
        "{}",
        describe(&refused)
    );

    let mut command = common::conductor(case.home());
    command
        .args(["--state-dir", &case.state().display().to_string()])
        .args(["session", "attach", "--name", "ex-old", "--watch"])
        .env("PATH", case.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for variable in SESSION_VARIABLES {
        command.env_remove(variable);
    }
    let mut child = command.spawn().expect("the watcher starts");
    // Read its output as it comes: the 64 KiB replay alone fills a pipe.
    let mut stdout = child.stdout.take().expect("its stdout");
    let reader = std::thread::spawn(move || {
        let mut shown = Vec::new();
        stdout
            .read_to_end(&mut shown)
            .expect("read the watcher's output");
        shown
    });
    child
        .stdin
        .take()
        .expect("its stdin")
        .write_all(b"typed by the watcher\r")
        .expect("type at the watcher");
    std::thread::sleep(Duration::from_millis(500));
    fs::OpenOptions::new()
        .append(true)
        .open(&typescript)
        .expect("open the typescript")
        .write_all(b"live output\n")
        .expect("the harness writes");
    std::thread::sleep(Duration::from_millis(1500));
    case.unit_state("inactive");
    let started = Instant::now();
    while child.try_wait().expect("poll the watcher").is_none() {
        if started.elapsed() > WAIT {
            let _ = child.kill();
            panic!("the watcher did not end with its unit");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let output = child.wait_with_output().expect("the watcher's output");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let shown = reader.join().expect("the reader");
    let shown = String::from_utf8_lossy(&shown);
    assert!(shown.contains("the old screen"), "the replay: {shown:.200}");
    assert!(shown.contains("live output"), "the follow: {shown:.200}");
    assert!(
        !shown.contains("HEAD-BEYOND-THE-TAIL"),
        "the replay is the typescript's last 64 KiB only"
    );
    let mut buffer = [0_u8; 64];
    match keyboard.read(&mut buffer) {
        Ok(0) => {}
        Err(error) if error.kind() == ErrorKind::WouldBlock => {}
        Ok(read) => panic!(
            "the watcher wrote {:?} into the session's FIFO",
            String::from_utf8_lossy(&buffer[..read])
        ),
        Err(error) => panic!("read the FIFO: {error}"),
    }
}
