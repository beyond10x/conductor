//! Adversary cases of wave 49, unit U11 (`story:deliver-reaches-bg-session`), pass 1: `conductor
//! session deliver` to a background session, through a real `tmux` and a stand-in `claude`.
//!
//! `tests/deliver.rs` holds the route against a fake `tmux` that starts no server, so nothing
//! there shows what a pane, a socket and a second delivery do to each other. Here each case is a
//! [`Scratch`] directory with the real `tmux` of the host on a socket of its own under
//! `state/sessions/`, and a `claude` stand-in first on `PATH`: `agents --json` prints the case's
//! `agents.json`; `attach <id>` appends its process id to `attach.pids`, prints one line and
//! appends what it reads from its terminal to `received`, at once or, as `attach.mode` says,
//! after a wait. No real session, harness or default tmux server is touched.

mod common;
mod scratch;

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use scratch::Scratch;

/// The background session every case delivers to.
const SESSION: &str = "ex-cortex";

struct Case {
    _scratch: Scratch,
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        // Whatever a case left running ends with it.
        for pid in self.attach_pids() {
            let _ = Command::new("kill").args(["-9", &pid]).output();
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let scratch = Scratch::new(Path::new("adv-deliver-attach").join(name));
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
                 \x20 if [ \"$(cat \"$D/attach.mode\" 2>/dev/null)\" = slow ]; then\n\
                 \x20   echo connecting; sleep 3\n\
                 \x20 fi\n\
                 \x20 echo ready\n\
                 \x20 exec cat >> \"$D/received\"\n\
                 fi\n\
                 exit 2\n",
                d = dir.display(),
            ),
        )
        .expect("write the stand-in claude");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        let entry = serde_json::json!([{
            "id": "cc062faf", "sessionId": "cc062faf-0000", "name": SESSION,
            "kind": "background", "cwd": dir.join("home/example-org/cortex"),
            "status": "idle", "state": "done", "pid": 4242,
        }]);
        fs::write(dir.join("agents.json"), entry.to_string()).expect("write the session list");
        Self {
            _scratch: scratch,
            dir,
        }
    }

    /// `conductor session deliver --name <SESSION> --text -` with `text` on standard input, the
    /// stand-in `claude` first on `PATH` and the host's `tmux` after it.
    fn deliver(&self, text: &str) -> Output {
        let path = format!(
            "{}:{}",
            self.dir.join("home/bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut child = common::conductor(self.dir.join("home"))
            .args(["session", "deliver", "--name", SESSION, "--text", "-"])
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
        child.wait_with_output().expect("conductor ends")
    }

    /// What the stand-in `claude attach` read from its terminal, of every pane.
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

    fn socket(&self) -> PathBuf {
        self.dir.join(format!("state/sessions/{SESSION}.tmux"))
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

/// One delivery, alone: the attach client takes the text as it was written (quotes, a line that
/// starts with a dash, tmux key names, shell and tmux format characters), and the pane, its
/// client and its socket are gone afterwards.
#[test]
fn one_delivery_reaches_the_attach_client_whole_and_leaves_nothing() {
    let case = Case::new("alone");
    let text = "[REPORT cortex -] \"quoted\" 'single' $(id) `x` Enter C-c \\ ; #{pane_id} %1\n\
                -n --help a line that starts with a dash\n\
                last line; still one message";
    let output = case.deliver(text);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(case.received(), format!("{text}\n"), "the text, whole");
    thread::sleep(Duration::from_millis(500));
    assert_eq!(case.attached(), Vec::<String>::new(), "no client is left");
    assert!(!case.socket().exists(), "no socket is left");
}

/// Two deliveries to one background session at the same time (the loop's and an operator's, or
/// two operators'): both name one socket path and one tmux session name. Each that answers
/// `delivered` (exit 0) put its text in front of the session; neither leaves a `claude attach`
/// client running; and, the session being live throughout, neither is refused.
#[test]
fn two_deliveries_at_once_to_one_background_session_both_arrive_and_leave_nothing() {
    let case = Case::new("at-once");
    let mut outcomes: Vec<(String, Output)> = Vec::new();
    for round in 0..4 {
        let texts = [
            format!("[REPORT cortex -] round {round} from the loop"),
            format!("[REPORT cortex -] round {round} from the operator"),
        ];
        let outputs: Vec<Output> = thread::scope(|scope| {
            let running: Vec<_> = texts
                .iter()
                .map(|text| scope.spawn(|| case.deliver(text)))
                .collect();
            running
                .into_iter()
                .map(|handle| handle.join().expect("a delivery ends"))
                .collect()
        });
        outcomes.extend(texts.into_iter().zip(outputs));
    }
    thread::sleep(Duration::from_secs(1));
    let received = case.received();
    let left = case.attached();
    let lost: Vec<&String> = outcomes
        .iter()
        .filter(|(text, output)| output.status.success() && !received.contains(text.as_str()))
        .map(|(text, _)| text)
        .collect();
    let refused: Vec<String> = outcomes
        .iter()
        .filter(|(_, output)| !output.status.success())
        .map(|(text, output)| format!("{text}: {}", String::from_utf8_lossy(&output.stderr).trim()))
        .collect();
    assert!(
        lost.is_empty() && left.is_empty() && refused.is_empty(),
        "answered delivered but never read by the session: {lost:#?}\n\
         `claude attach` clients still running after every delivery ended: {left:?}\n\
         refused although the session was live: {refused:#?}\n\
         received:\n{received}"
    );
}

/// The attach client draws a first line and takes its input three seconds later: the pane is
/// ended half a second after Enter, so the session never reads the text. A delivery the session
/// never read is not answered `delivered`.
#[test]
fn a_delivery_the_attach_client_never_read_is_not_answered_delivered() {
    let case = Case::new("slow");
    fs::write(case.dir.join("attach.mode"), "slow").expect("write the mode");
    let text = "[DECISION R-7] A merge now (DEC-20991231-01)";
    let output = case.deliver(text);
    thread::sleep(Duration::from_secs(4));
    let received = case.received();
    assert!(
        !output.status.success() || received.contains(text),
        "answered delivered, yet the session read {received:?}\n{}",
        describe(&output)
    );
}
