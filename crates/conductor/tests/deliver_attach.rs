//! `story:deliver-reaches-bg-session`: `conductor session deliver` to a background session,
//! through the host's real `tmux` and a stand-in `claude`: what a delivery's pane leaves in
//! `<state>/sessions`, and what it answers when the attach client does not read.
//!
//! `tests/deliver.rs` holds the route's calls against a fake `tmux`, and
//! `tests/adv_deliver_attach.rs` one delivery alone, two at once and a client that reads late.
//! Here each case is a [`Scratch`] directory with a `claude` stand-in first on `PATH`: `agents
//! --json` lists one live background session; `attach <id>` appends its process id to
//! `attach.pids`, prints one line and then, as `attach.mode` says, appends what it reads from
//! its terminal to `received` at once (no mode), after three seconds (`slow`), or never
//! (`never`), or ends, without reading, as soon as its terminal holds a line it could read
//! (`dies`); `silent` prints nothing and never reads; `lingers` reads until its terminal
//! closes and ends once the case wrote the file `go`. No real session, harness or default tmux server is touched.

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
        for pid in self.attach_pids().into_iter().chain(self.shell_pids()) {
            let _ = Command::new("kill").args(["-9", &pid]).output();
        }
    }
}

impl Case {
    fn new(name: &str, mode: &str) -> Self {
        let scratch = Scratch::new(Path::new("deliver-attach").join(name));
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
                 \x20 M=\"$(cat \"$D/attach.mode\")\"\n\
                 \x20 if [ \"$M\" = silent ]; then exec sleep 600; fi\n\
                 \x20 echo ready\n\
                 \x20 case \"$M\" in\n\
                 \x20 never) exec sleep 600;;\n\
                 \x20 slow) sleep 3;;\n\
                 \x20 dies) until bash -c 'read -t 0'; do sleep 0.02; done; exit 0;;\n\
                 \x20 lingers) cat >> \"$D/received\"; until [ -e \"$D/go\" ]; do sleep 0.05; done; exit 0;;\n\
                 \x20 esac\n\
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

    /// The stand-in `claude` first on `PATH` and the host's `tmux` after it.
    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.dir.join("home/bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    /// `conductor session deliver --name <SESSION> --text -` with `text` on standard input.
    fn deliver(&self, text: &str) -> Output {
        let mut child = common::conductor(self.dir.join("home"))
            .args(["session", "deliver", "--name", SESSION, "--text", "-"])
            .current_dir(self.dir.join("records"))
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .env("PATH", self.path())
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

    fn sessions(&self) -> PathBuf {
        self.dir.join("state/sessions")
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

    /// Puts an `sh` first on `PATH` that ignores a hangup, and so does what it runs: the
    /// shell of a delivery's pane, and its client, then outlive the pane's server, as they do
    /// when the signal reaches them late. Each such shell's process id is in `shell.pids`.
    fn shells_ignore_a_hangup(&self) {
        let fake = self.dir.join("home/bin/sh");
        fs::write(
            &fake,
            format!(
                "#!/bin/bash\necho $$ >> '{}'\ntrap '' HUP\nexec /bin/sh \"$@\"\n",
                self.dir.join("shell.pids").display()
            ),
        )
        .expect("write the stand-in sh");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// The process id of each stand-in `sh` a case started.
    fn shell_pids(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("shell.pids"))
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

    /// The names in `<state>/sessions` a reader of launcher sockets takes for one: those the
    /// watch (`src/watch.rs`) and `session attach` (`src/attach.rs`) list, which end in `.tmux`.
    fn launchers(&self) -> Vec<String> {
        names(&self.sessions())
            .into_iter()
            .filter(|name| name.ends_with(".tmux"))
            .collect()
    }

    /// What the deliveries' own directory holds.
    fn panes(&self) -> Vec<String> {
        names(&self.sessions().join("deliveries"))
    }
}

/// The names `dir` holds, in order; none when it is not there.
fn names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| {
            entry
                .expect("read a directory")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// An attach client that draws and never reads its terminal: the delivery waits for the text to
/// be taken, within its bound, and is then refused on one line that says so; nothing is
/// recorded, and neither the client nor its socket is left. The text was typed, so the refusal
/// is `DeliveryNotSeenRead` and not `SessionNotReachable`, which says that nothing was typed.
#[test]
fn a_delivery_the_attach_client_never_reads_is_refused_and_leaves_nothing() {
    let case = Case::new("never", "never");
    let output = case.deliver("[DECISION R-7] A merge now (DEC-20991231-01)");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("DeliveryNotSeenRead")
            && !stderr.contains("SessionNotReachable")
            && stderr.contains(SESSION)
            && stderr.contains("did not read the text within"),
        "the refusal names the error, the session and what was waited for: {stderr}"
    );
    assert!(output.stdout.is_empty(), "no delivery id: {stderr}");
    assert!(
        !case.dir.join("state/mailbox").exists(),
        "no body is kept for a delivery nobody read"
    );
    thread::sleep(Duration::from_millis(500));
    assert_eq!(case.attached(), Vec::<String>::new(), "no client is left");
    assert_eq!(case.panes(), Vec::<String>::new(), "no socket is left");
    assert_eq!(case.launchers(), Vec::<String>::new());
}

/// An attach client that draws nothing: nothing is typed, and after the bound the delivery is
/// refused `SessionNotReachable` on one line that says so; neither the client nor its socket is
/// left.
#[test]
fn a_delivery_to_an_attach_client_that_draws_nothing_is_refused_at_the_bound() {
    let case = Case::new("silent", "silent");
    let output = case.deliver("[DECISION R-7] A merge now (DEC-20991231-01)");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("SessionNotReachable")
            && stderr.contains(SESSION)
            && stderr.contains("it drew nothing within 20 s"),
        "the refusal names the error, the session and the bound: {stderr}"
    );
    assert!(!case.dir.join("state/mailbox").exists(), "no body is kept");
    thread::sleep(Duration::from_millis(500));
    assert_eq!(case.attached(), Vec::<String>::new(), "no client is left");
    assert_eq!(case.panes(), Vec::<String>::new(), "no socket is left");
}

/// An attach client that ends without reading, the moment the typed line is on its terminal
/// (it asks whether a line is there, and reads none): the text was typed and was still unread
/// when the client ended, however soon after Enter that was and whether or not the delivery
/// looked at the terminal in between. The delivery is refused `DeliveryNotSeenRead` on one
/// line, not `SessionNotReachable`, and nothing is recorded.
#[test]
fn a_delivery_whose_pane_ended_with_the_text_unread_is_refused_as_not_seen_read() {
    let case = Case::new("dies", "dies");
    let output = case.deliver("[DECISION R-7] A merge now (DEC-20991231-01)");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("DeliveryNotSeenRead")
            && !stderr.contains("SessionNotReachable")
            && stderr.contains(SESSION)
            && stderr.contains("ended with the text unread"),
        "the refusal names the error, the session and what was seen: {stderr}"
    );
    assert!(output.stdout.is_empty(), "no delivery id: {stderr}");
    assert!(!case.dir.join("state/mailbox").exists(), "no body is kept");
    assert_eq!(case.received(), "", "nobody read it");
    assert_eq!(case.panes(), Vec::<String>::new(), "no socket is left");
}

/// While a delivery's pane is up (the client reads three seconds after it drew), and after it,
/// `<state>/sessions` holds no name a reader of launcher sockets takes for one: the pane's
/// socket is in the deliveries' own directory, so the watch and `session attach` never list a
/// delivery as a session.
#[test]
fn a_delivery_s_pane_is_never_listed_as_a_launcher() {
    let case = Case::new("unlisted", "slow");
    let text = "[REPORT cortex -] watch: CI red on main";
    let (output, seen, panes) = thread::scope(|scope| {
        let running = scope.spawn(|| case.deliver(text));
        let mut seen = Vec::new();
        let mut panes = Vec::new();
        while !running.is_finished() {
            seen.extend(case.launchers());
            panes.extend(case.panes());
            thread::sleep(Duration::from_millis(20));
        }
        (running.join().expect("the delivery ends"), seen, panes)
    });
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(case.received(), format!("{text}\n"), "the text, whole");
    assert_eq!(seen, Vec::<String>::new(), "listed as a launcher meanwhile");
    assert!(
        panes.iter().any(|name| name.ends_with(".tmux")),
        "the pane's socket was in the deliveries' directory meanwhile: {panes:?}"
    );
    assert_eq!(case.launchers(), Vec::<String>::new());
    assert_eq!(case.panes(), Vec::<String>::new(), "no socket is left");
}

/// A pane whose shell and client outlive its server: the client ends only after the delivery
/// ended the pane and returned (the case lets it), and the pane's shell goes on to its next
/// steps only then, up to the `sleep` it ends in. Whatever the pane does after its delivery, the deliveries' directory holds nothing of
/// that delivery.
#[test]
fn a_client_that_ends_after_its_delivery_returned_leaves_nothing_in_the_deliveries_directory() {
    let case = Case::new("lingers", "lingers");
    case.shells_ignore_a_hangup();
    let text = "[REPORT cortex -] watch: CI red on main";
    let output = case.deliver(text);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(case.received(), format!("{text}\n"), "the text, whole");
    assert_eq!(case.shell_pids().len(), 1, "the pane ran the stand-in sh");
    assert_eq!(case.attached().len(), 1, "the client outlives the delivery");
    assert_eq!(case.panes(), Vec::<String>::new(), "nothing is left");
    fs::write(case.dir.join("go"), "").expect("let the client end");
    // The pane's shell has taken its steps after the client once it is the `sleep` it ends in.
    let shell = Path::new("/proc").join(&case.shell_pids()[0]).join("comm");
    let mut slept = false;
    for _ in 0..300 {
        slept = fs::read_to_string(&shell).is_ok_and(|name| name.trim() == "sleep");
        if slept {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(slept, "the pane's shell went on after its client");
    assert_eq!(case.attached(), Vec::<String>::new(), "the client ended");
    assert_eq!(
        case.panes(),
        Vec::<String>::new(),
        "nothing of the delivery is there after its client ended"
    );
}

/// A delivery that was killed before it ended its pane leaves a tmux server with a `claude
/// attach` client on a socket named after its process. The next delivery, to any session of the
/// state directory, ends each pane whose process is gone and removes its socket; a pane whose
/// process still runs is another delivery's, and stays.
#[test]
fn a_pane_left_by_a_delivery_that_was_killed_is_ended_by_the_next_delivery() {
    let case = Case::new("left", "");
    let panes = case.sessions().join("deliveries");
    fs::create_dir_all(&panes).expect("create the deliveries' directory");
    // A process id nothing runs under: one of a child that ended and was waited for.
    let gone = {
        let mut child = Command::new("true").spawn().expect("run true");
        let pid = child.id();
        child.wait().expect("it ends");
        pid
    };
    let pane = |owner: u32| format!("{owner}-0");
    for owner in [gone, std::process::id()] {
        let started = Command::new("tmux")
            .current_dir(&panes)
            .args(["-S", &format!("{}.tmux", pane(owner)), "-f", "/dev/null"])
            .args(["new-session", "-d", "-s", "ex-other", "-x", "160", "-y"])
            .args(["50", "--", "claude", "attach", "cc062faf"])
            .env("PATH", case.path())
            .env("SHELL", "/bin/sh")
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .output()
            .expect("tmux runs");
        assert!(started.status.success(), "{}", describe(&started));
    }
    for _ in 0..50 {
        if case.attach_pids().len() == 2 {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let before = case.attach_pids();
    assert_eq!(before.len(), 2, "both panes run a client");

    let text = "[REPORT cortex -] watch: CI red on main";
    let output = case.deliver(text);
    assert!(output.status.success(), "{}", describe(&output));
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        case.panes(),
        [format!("{}.tmux", pane(std::process::id()))],
        "the pane of the process that is gone is removed, the other kept"
    );
    assert_eq!(case.attached().len(), 1, "one client left: the kept pane's");

    let ended = Command::new("tmux")
        .current_dir(&panes)
        .args(["-S", &format!("{}.tmux", pane(std::process::id()))])
        .arg("kill-server")
        .env_remove("TMUX")
        .output()
        .expect("tmux runs");
    assert!(ended.status.success(), "{}", describe(&ended));
}
