//! Adversary cases of wave 50, unit U1 (`story:deliver-reaches-bg-session`), pass 2: the pane a
//! delivery to a background session makes for itself, through the host's real `tmux` and a
//! stand-in `claude`.
//!
//! Each case is a [`Scratch`] directory with a `claude` stand-in first on `PATH`: `agents
//! --json` lists one live background session; `attach <id>` appends its process id to
//! `attach.pids`, prints one line and then, as `attach.mode` says, appends what it reads from
//! its terminal to `received` until its pane ends (no mode), reads one line and ends (`once`),
//! never reads (`never`), or never reads from a terminal whose carriage return ends no line
//! (`cooked`). No real session, harness or default tmux server is touched; every tmux server is
//! on a socket under the case's own directory and is killed with the case.

mod common;
mod scratch;

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use scratch::Scratch;

struct Case {
    _scratch: Scratch,
    dir: PathBuf,
    session: String,
}

impl Drop for Case {
    fn drop(&mut self) {
        // Whatever a case left running ends with it: each pane's server, then each client.
        let panes = self.dir.join("state/sessions/deliveries");
        for entry in fs::read_dir(&panes).into_iter().flatten().flatten() {
            let _ = Command::new("tmux")
                .current_dir(&panes)
                .arg("-S")
                .arg(entry.file_name())
                .arg("kill-server")
                .env_remove("TMUX")
                .output();
        }
        for pid in self.attach_pids() {
            let _ = Command::new("kill").args(["-9", &pid]).output();
        }
    }
}

impl Case {
    fn new(name: &str, session: &str, mode: &str) -> Self {
        let scratch = Scratch::new(Path::new("adv2-deliver-attach").join(name));
        for sub in ["home/bin", "records", "state/sessions"] {
            fs::create_dir_all(scratch.path().join(sub)).expect("create a case directory");
        }
        let dir = scratch.path().canonicalize().expect("the case's directory");
        fs::write(
            dir.join("conductor.yaml"),
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n",
                root = dir.join("home/example-org").display(),
                trees = dir.join("trees").display(),
                records = dir.join("records").display(),
                state = dir.join("state").display(),
            ),
        )
        .expect("write the config file");
        let fake = dir.join("home/bin/claude");
        fs::write(
            &fake,
            format!(
                "#!/bin/sh\n\
                 D='{d}'\n\
                 if [ \"$1\" = agents ]; then cat \"$D/agents.json\"; exit 0; fi\n\
                 if [ \"$1\" = attach ]; then\n\
                 \x20 echo $$ >> \"$D/attach.pids\"\n\
                 \x20 case \"$(cat \"$D/attach.mode\")\" in\n\
                 \x20 never) echo ready; exec sleep 600;;\n\
                 \x20 cooked) stty -icrnl; echo ready; exec sleep 600;;\n\
                 \x20 once) echo ready; IFS= read -r line; printf '%s\\n' \"$line\" >> \"$D/received\"; exit 0;;\n\
                 \x20 esac\n\
                 \x20 echo ready\n\
                 \x20 exec cat >> \"$D/received\"\n\
                 fi\n\
                 exit 2\n",
                d = dir.display(),
            ),
        )
        .expect("write the stand-in claude");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        fs::write(dir.join("attach.mode"), mode).expect("write the mode");
        let entry = serde_json::json!([{
            "id": "cc062faf", "sessionId": "cc062faf-0000", "name": session,
            "kind": "background", "cwd": dir.join("home/example-org/cortex"),
            "status": "idle", "state": "done", "pid": 4242,
        }]);
        fs::write(dir.join("agents.json"), entry.to_string()).expect("write the session list");
        Self {
            _scratch: scratch,
            dir,
            session: session.to_owned(),
        }
    }

    /// `conductor session deliver --name <session> --text -`, started, with `text` written to
    /// its standard input; the stand-in `claude` first on `PATH` and the host's `tmux` after it.
    fn start(&self, text: &str) -> Child {
        let path = format!(
            "{}:{}",
            self.dir.join("home/bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut child = common::conductor(self.dir.join("home"))
            .args(["session", "deliver", "--name", &self.session, "--text", "-"])
            .current_dir(self.dir.join("records"))
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .env("PATH", path)
            .env("SHELL", "/bin/sh")
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("conductor runs");
        child
            .stdin
            .take()
            .expect("its standard input")
            .write_all(text.as_bytes())
            .expect("write its standard input");
        child
    }

    fn deliver(&self, text: &str) -> Output {
        self.start(text).wait_with_output().expect("conductor ends")
    }

    /// What the stand-in `claude attach` read from its terminal.
    fn received(&self) -> String {
        fs::read_to_string(self.dir.join("received")).unwrap_or_default()
    }

    /// The process id of each `claude attach` a case started.
    fn attach_pids(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("attach.pids"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The `claude attach` processes still running.
    fn attached(&self) -> Vec<String> {
        self.attach_pids()
            .into_iter()
            .filter(|pid| Path::new("/proc").join(pid).exists())
            .collect()
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A live background session whose name is 95 characters, which a launcher socket
/// `<name>.tmux` holds (a socket path is 107 bytes at most, and tmux is run from the socket's
/// directory): the delivery's own socket `<name>-<process id>-<count>.tmux` is longer, and the
/// session, live and listed, must still be reached.
#[test]
fn a_live_background_session_with_a_name_a_launcher_socket_holds_is_reached() {
    let session = format!("ex-{}", "r".repeat(92));
    assert!(
        format!("{session}.tmux").len() <= 107,
        "a launcher holds it"
    );
    let case = Case::new("long-name", &session, "");
    let text = "[REPORT cortex -] watch: CI red on main";
    let output = case.deliver(text);
    assert!(
        output.status.success(),
        "refused although the session is live: {}",
        describe(&output)
    );
    assert_eq!(case.received(), format!("{text}\n"), "the text, whole");
}

/// The attach client reads the text and ends at once, its pane with it. The session read the
/// text, so the delivery is not answered `SessionNotReachable`: `conductor run` reads that
/// refusal for a live session as "nothing was typed" and offers the same text again on every
/// tick.
#[test]
fn a_delivery_the_attach_client_read_before_it_ended_is_not_answered_refused() {
    let case = Case::new("read-then-ended", "ex-cortex", "once");
    let text = "[DECISION R-7] A merge now (DEC-20991231-01)";
    let output = case.deliver(text);
    let received = case.received();
    assert!(
        output.status.success() || !received.contains(text),
        "refused, to be offered again, yet the session read {received:?}\n{}",
        describe(&output)
    );
}

/// The attach client never reads, and its terminal is one on which the carriage return that
/// ends the delivery ends no line: nothing the client could read ever becomes readable, the
/// terminal reports no input, and the client still has not read a byte. A delivery nobody read
/// is not answered `delivered`.
#[test]
#[ignore = "stand-in only: a client in raw mode makes every byte readable"]
fn a_delivery_nobody_read_on_a_terminal_that_reports_no_input_is_not_answered_delivered() {
    let case = Case::new("cooked", "ex-cortex", "cooked");
    let output = case.deliver("[DECISION R-7] A merge now (DEC-20991231-01)");
    assert!(
        !output.status.success(),
        "answered delivered, yet the client never read: {}",
        describe(&output)
    );
}

/// A delivery that is asked to end (SIGTERM, as a stopping loop or an operator sends) while it
/// waits for the client to read ends its pane first: no `claude attach` client stays attached
/// to the session until some later delivery happens to clean up.
#[test]
fn a_delivery_ended_by_a_signal_while_it_waits_leaves_no_client() {
    let case = Case::new("signal", "ex-cortex", "never");
    let child = case.start("[REPORT cortex -] watch: CI red on main");
    for _ in 0..100 {
        if !case.attach_pids().is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(case.attach_pids().len(), 1, "the pane runs a client");
    // Past the settle before the paste, the paste and Enter: inside the wait for the reader.
    thread::sleep(Duration::from_secs(3));
    let ended = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .output()
        .expect("kill runs");
    assert!(ended.status.success(), "the delivery still ran");
    let output = child.wait_with_output().expect("conductor ends");
    thread::sleep(Duration::from_secs(1));
    assert_eq!(
        case.attached(),
        Vec::<String>::new(),
        "a client is left attached after the delivery ended: {}",
        describe(&output)
    );
}
