//! Conformance checks of the work call's row against its own contract
//! (`story:control-plane-work-handler`): `last_report` of `GET /v1/work/{reference}` is "the
//! first line of the latest `[REPORT]` message handled for the dispatch". A report the session of
//! one repository writes, or a message it handles itself, must not become the last report of a
//! unit of work another repository's dispatch holds.
//!
//! Each case runs the built binary's `instance serve` with its loop off over a config file,
//! records and a state directory under this test target's temporary directory, as
//! `tests/work_route.rs` does, with helpers copied from it. The records' workspace file names the
//! repositories `alpha` and `beta`. A report reaches the dispatch as a controller sends one:
//! `conductor message send`, then one watch tick of the loop, run by a child of this test binary
//! ([`child`]) with the case's config file as its instance.

mod common;

use std::fs;
use std::io::{Read as _, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::bail;
use conductor_cli::config;
use conductor_cli::run::{self, Headroom, Seams};
use conductor_cli::session::{Deliverer, Delivery};
use serde_json::{Value, json};

/// The variable that makes a run of this test binary the child tick ([`child`]).
const CHILD: &str = "CONDUCTOR_SEC_WORK_ROUTE_CHILD";

/// The instance's token, built from pieces: a fixture, not a secret.
fn token() -> String {
    ["sec", "work", "fixture"].join("-") + &"7".repeat(40)
}

/// One case's directory, short so `<state>/api.sock` fits a socket address.
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
            .join("swr")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        let case = Self { dir };
        for sub in ["dispatches", "decisions", ".engineering"] {
            fs::create_dir_all(case.records().join(sub)).expect("create a records directory");
        }
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::create_dir_all(case.state()).expect("create the state directory");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        fs::write(
            case.records().join(".engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers:\n  - name: alpha\n    source: ../alpha\n  \
             - name: beta\n    source: ../beta\n",
        )
        .expect("write the workspace file");
        fs::write(case.file(), case.text()).expect("write the config file");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("h")
    }

    fn records(&self) -> PathBuf {
        self.home().join("r")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("s")
    }

    fn socket(&self) -> PathBuf {
        self.state().join("api.sock")
    }

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    fn text(&self) -> String {
        format!(
            "version: conductor.config/2\n\
             daemon: {{store: {store:?}}}\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   placement: local\n\
             \x20   session_prefix: eo\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            root = self.home().join("example-org"),
            trees = self.home().join("trees"),
            records = self.records(),
            state = self.state(),
            cache = self.dir.join("c"),
        )
    }

    /// The runtime of the case, its loop off, serving on its socket once this answers.
    fn serve(&self) -> Running {
        let child = common::conductor(self.home())
            .current_dir(self.state())
            .arg("--config")
            .arg(self.file())
            .args(["instance", "serve"])
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .env(config::INSTANCE_VARIABLE, "example-org")
            .env(conductor_cli::instance::TOKEN_VARIABLE, token())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("instance serve starts");
        let mut running = Running(child);
        let until = Instant::now() + Duration::from_secs(30);
        while UnixStream::connect(self.socket()).is_err() {
            if let Ok(Some(status)) = running.0.try_wait() {
                let mut stderr = String::new();
                if let Some(mut pipe) = running.0.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                panic!("instance serve exited {status} before it listened: {stderr}");
            }
            assert!(
                Instant::now() < until,
                "no socket at {}",
                self.socket().display()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        running
    }

    /// `conductor <args>` with the case's home and config file, `stdin` on standard input, and
    /// `session` as `CONDUCTOR_SESSION` when given (a command a session runs).
    fn conductor_as(&self, session: Option<&str>, args: &[&str], stdin: &str) -> Output {
        let mut command = common::conductor(self.home());
        command
            .args(args)
            .current_dir(self.records())
            .env(config::CONFIG_VARIABLE, self.file())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(session) = session {
            command.env(conductor_cli::actor::SESSION_VARIABLE, session);
        }
        let mut child = command.spawn().expect("conductor runs");
        child
            .stdin
            .take()
            .expect("its standard input")
            .write_all(stdin.as_bytes())
            .expect("write its standard input");
        child.wait_with_output().expect("conductor ends")
    }

    /// [`Case::conductor_as`] with no session, which must succeed; its standard output.
    fn ok(&self, args: &[&str], stdin: &str) -> String {
        let output = self.conductor_as(None, args, stdin);
        assert!(
            output.status.success(),
            "{args:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// A message `first_line` from `alpha`'s controller into conductor's mailbox; its id.
    fn send(&self, first_line: &str) -> String {
        self.ok(
            &[
                "message",
                "send",
                "--to",
                "conductor",
                "--from",
                "eo-alpha",
                "--text",
                "-",
            ],
            &format!("{first_line}\n"),
        )
        .trim()
        .to_owned()
    }

    /// One watch tick of the loop over the case's instance, in a child of this test binary.
    fn tick(&self) {
        let output = Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "--exact",
                "child",
                "--nocapture",
                "--test-threads",
                "1",
                "-q",
            ])
            .env("HOME", self.home())
            .env("USER", "operator")
            .env_remove(config::INSTANCE_VARIABLE)
            .env_remove(conductor_cli::actor::SESSION_VARIABLE)
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env(config::CONFIG_VARIABLE, self.file())
            .env(CHILD, "tick")
            .current_dir(self.records())
            .stdin(Stdio::null())
            .output()
            .expect("the child runs");
        assert!(
            output.status.success(),
            "the tick failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// A child process killed when the case ends.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The deliverer of the child tick: no session runs, so nothing is delivered.
struct Nobody;

impl Deliverer for Nobody {
    fn deliver(&self, name: &str, _text: &str) -> anyhow::Result<Delivery> {
        bail!("no session {name} runs in this case")
    }
}

/// The headroom of the child tick: always.
struct Plenty;

impl Headroom for Plenty {
    fn headroom(&self) -> anyhow::Result<bool> {
        Ok(true)
    }
}

/// The child tick ([`Case::tick`]); a plain run of the suite passes at once.
#[test]
fn child() {
    if std::env::var(CHILD).is_err() {
        return;
    }
    let file = PathBuf::from(std::env::var_os(config::CONFIG_VARIABLE).expect("a config file"));
    config::set_active(Some(&file)).expect("the case's config loads");
    let host = conductor_cli::confine::Host;
    let program = PathBuf::from(common::conductor(Path::new("/nonexistent")).get_program());
    let seams = Seams {
        providers: &host,
        deliverer: &Nobody,
        headroom: &Plenty,
        handle_request: None,
        program,
        gc: conductor_cli::gc::gc,
    };
    match run::handle_lines(None, None, &[], &seams) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("conductor: {error:#}");
            std::process::exit(1);
        }
    }
}

/// An HTTP answer: its status and its body.
#[derive(Debug)]
struct Answer {
    status: u16,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("no JSON ({error}): {self:?}"))
    }
}

/// Sends `head` (the request line and headers, no blank line) and `body` to the case's socket.
fn exchange(case: &Case, head: &str, body: &[u8]) -> Answer {
    let mut stream = UnixStream::connect(case.socket()).expect("connect to the runtime's socket");
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("a read timeout");
    let mut request = format!("{head}Connection: close\r\n\r\n").into_bytes();
    request.extend_from_slice(body);
    let _ = stream.write_all(&request);
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {text:?}"));
    Answer {
        status,
        body: body.to_owned(),
    }
}

/// `POST /v1/work` with `body` as JSON and the instance's token.
fn post(case: &Case, body: &Value) -> Answer {
    let bytes = body.to_string().into_bytes();
    let head = format!(
        "POST /v1/work HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nX-Conductor-Token: {}\r\n",
        bytes.len(),
        token()
    );
    exchange(case, &head, &bytes)
}

/// `GET /v1/work/<reference>`, as the daemon forwards a read: no token.
fn get(case: &Case, reference: &str) -> Answer {
    exchange(
        case,
        &format!("GET /v1/work/{reference} HTTP/1.1\r\nHost: localhost\r\n"),
        b"",
    )
}

/// A unit of work for `repository` under `reference`; its dispatch id once sent.
fn sent(case: &Case, repository: &str, reference: &str) -> String {
    let answer = post(
        case,
        &json!({
            "repository": repository,
            "brief": "Add the export button to the report page",
            "reference": reference,
        }),
    );
    assert_eq!(answer.status, 201, "{answer:?}");
    answer.json()["dispatch_id"]
        .as_str()
        .expect("a dispatch id")
        .to_owned()
}

/// A `[REPORT alpha <id>]` that `alpha`'s controller writes naming `beta`'s dispatch is handled
/// for `beta`'s dispatch, and becomes the last report of `beta`'s unit of work, text no session
/// serving `beta` produced. The contract: the last `[REPORT]` handled for the dispatch.
#[test]
fn a_report_of_one_repository_is_not_the_last_report_of_another_repositorys_unit() {
    let case = Case::new("xrepo");
    let _runtime = case.serve();
    let _alpha = sent(&case, "alpha", "WI-A");
    let beta = sent(&case, "beta", "WI-B");

    case.send(&format!(
        "[REPORT alpha {beta}] progress text from alpha's session"
    ));
    case.tick();

    let row = get(&case, "WI-B");
    assert_eq!(row.status, 200, "{row:?}");
    let row = row.json();
    assert_eq!(row["repository"], "beta", "{row}");
    assert!(
        row.get("last_report").is_none(),
        "the last report of beta's unit is a report alpha's session wrote: {row}"
    );
}

/// `message handle-message --dispatch-id`, run by a session, attaches the session's own report
/// message to a dispatch of another repository, and that dispatch's unit then shows it as its
/// last report. A session refused here leaves nothing to check.
#[test]
fn a_session_does_not_attach_its_message_to_another_repositorys_dispatch() {
    let case = Case::new("attach");
    let _runtime = case.serve();
    let alpha = sent(&case, "alpha", "WI-A");
    let beta = sent(&case, "beta", "WI-B");

    let message = case.send(&format!(
        "[REPORT alpha {alpha}] progress on alpha's export"
    ));
    let handled = case.conductor_as(
        Some("eo-alpha"),
        &[
            "message",
            "handle-message",
            "--message-id",
            &message,
            "--dispatch-id",
            &beta,
        ],
        "",
    );
    if !handled.status.success() {
        return;
    }

    let row = get(&case, "WI-B");
    assert_eq!(row.status, 200, "{row:?}");
    let row = row.json();
    assert!(
        row.get("last_report").is_none(),
        "a session attached its own report on alpha to beta's unit: {row}"
    );
}

// Round 2: the fix of the two cases above.

/// The case's workspace file with `beta-2` beside `alpha` and `beta`: a name `beta` is a prefix of.
fn with_beta_2(case: &Case) {
    fs::write(
        case.records().join(".engineering/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: alpha\n    source: ../alpha\n  \
         - name: beta\n    source: ../beta\n  - name: beta-2\n    source: ../beta-2\n",
    )
    .expect("write the workspace file");
}

/// `message send` of `first_line` as `alpha`'s session (`CONDUCTOR_SESSION` `eo-alpha`); its id,
/// or `None` when the command refuses the line.
fn send_as_alpha(case: &Case, first_line: &str) -> Option<String> {
    let output = case.conductor_as(
        Some("eo-alpha"),
        &[
            "message",
            "send",
            "--to",
            "conductor",
            "--from",
            "eo-alpha",
            "--text",
            "-",
        ],
        &format!("{first_line}\n"),
    );
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// A `[REPORT]` line whose `<repo>` is not the dispatch's repository by exact name moves no
/// dispatch and becomes no unit's last report: another case, a prefix and a suffix of another
/// repository's name, path forms, and two ids on one line.
#[test]
fn a_report_naming_a_lookalike_repository_moves_no_dispatch_of_another() {
    let case = Case::new("lookalike");
    with_beta_2(&case);
    let _runtime = case.serve();
    let alpha = sent(&case, "alpha", "WI-A");
    let beta = sent(&case, "beta", "WI-B");
    let beta_2 = sent(&case, "beta-2", "WI-B2");

    let lines = [
        format!("[REPORT Beta {beta}] done evidence"),
        "[REPORT BETA -] done evidence".to_owned(),
        format!("[REPORT beta {beta_2}] done evidence"),
        format!("[REPORT beta-2 {beta}] done evidence"),
        format!("[REPORT bet {beta}] done evidence"),
        format!("[REPORT ./beta {beta}] done evidence"),
        format!("[REPORT beta/ {beta}] done evidence"),
        format!("[REPORT example-org/beta {beta}] done evidence"),
        format!("[REPORT alpha {alpha} {beta}] done evidence"),
        format!("[REPORT alpha {beta} {beta}] done evidence"),
    ];
    for line in &lines {
        let _ = send_as_alpha(&case, line);
    }
    case.tick();

    for reference in ["WI-B", "WI-B2"] {
        let row = get(&case, reference);
        assert_eq!(row.status, 200, "{row:?}");
        let row = row.json();
        assert_eq!(row["state"], "Sent", "a lookalike report moved it: {row}");
        assert!(
            row.get("last_report").is_none(),
            "a lookalike report became its last report: {row}"
        );
    }
}

/// `message handle-message --dispatch-id` with a dispatch of another repository than the first
/// line names, one that does not exist, or a first line of another kind or with extra words is
/// refused with exit 1, and the message stays `Received` with no dispatch.
#[test]
fn a_refused_handle_message_leaves_the_message_as_it_was() {
    let case = Case::new("refused");
    let _runtime = case.serve();
    let alpha = sent(&case, "alpha", "WI-A");
    let beta = sent(&case, "beta", "WI-B");

    let attempts = [
        (format!("[REPORT alpha {alpha}] progress"), beta.clone()),
        (
            format!("[REPORT alpha {alpha}] progress"),
            "DSP-29991231-99".to_owned(),
        ),
        ("[RESOURCE alpha] build-slot 1".to_owned(), beta.clone()),
        (
            format!("[REPORT alpha {alpha} extra] progress"),
            beta.clone(),
        ),
        (format!("[REPORT alpha -] progress {beta}"), beta.clone()),
    ];
    for (line, dispatch) in &attempts {
        let Some(message) = send_as_alpha(&case, line) else {
            continue;
        };
        let handled = case.conductor_as(
            None,
            &[
                "message",
                "handle-message",
                "--message-id",
                &message,
                "--dispatch-id",
                dispatch,
            ],
            "",
        );
        let said = String::from_utf8_lossy(&handled.stderr).into_owned();
        assert_eq!(
            handled.status.code(),
            Some(1),
            "{line:?} with {dispatch}: {said}"
        );
        assert!(said.contains("MessageDispatchMismatch"), "{line:?}: {said}");
        let rows: Value =
            serde_json::from_str(&case.ok(&["message", "messages", "--format", "json"], ""))
                .expect("the view is JSON");
        let row = rows
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row["message_id"] == message.as_str())
            })
            .unwrap_or_else(|| panic!("no row of {message}: {rows}"));
        assert_eq!(row["state"], "Received", "{line:?}: {row}");
        assert!(row["dispatch_id"].is_null(), "{line:?}: {row}");
    }
    let row = get(&case, "WI-B").json();
    assert!(row.get("last_report").is_none(), "{row}");
}

/// The session of `alpha` writes a `[REPORT]` line that names `beta` and `beta`'s dispatch
/// consistently. No check binds a line's `<repo>` to the session that sent it (`message send`
/// takes any `--from`, the guard lets every session run it), so `alpha`'s session ends `beta`'s
/// unit of work and its line becomes that unit's last report.
#[test]
#[ignore = "pre-existing, story:report-is-tied-to-its-session"]
fn a_session_of_one_repository_does_not_report_for_another_repositorys_unit() {
    let case = Case::new("as-beta");
    let _runtime = case.serve();
    let _alpha = sent(&case, "alpha", "WI-A");
    let beta = sent(&case, "beta", "WI-B");

    let sent_line = send_as_alpha(
        &case,
        &format!("[REPORT beta {beta}] done evidence from alpha's session"),
    );
    assert!(sent_line.is_some(), "message send refused the line");
    case.tick();

    let row = get(&case, "WI-B").json();
    assert_eq!(
        row["state"], "Sent",
        "alpha's session moved beta's unit of work: {row}"
    );
    assert!(
        row.get("last_report").is_none(),
        "alpha's session wrote beta's last report: {row}"
    );
}

/// The `start_ref` rule claims what `git check-ref-format --allow-onelevel` admits: every value
/// it takes, git admits; and it takes the ordinary branch, tag and commit names.
#[test]
fn a_start_ref_is_taken_only_where_git_takes_it_as_a_ref_name() {
    use conductor_cli::dispatch::valid_start_ref;
    let ordinary = [
        "main",
        "release/1.5.0",
        "v1.5.0",
        "feature/x.y",
        "0123456789abcdef0123456789abcdef01234567",
    ];
    for value in ordinary {
        assert!(valid_start_ref(value), "{value:?} is refused");
    }
    let candidates = [
        "-x",
        "--upload-pack=x",
        "a..b",
        "a...b",
        "HEAD@{1}",
        "@{-1}",
        "x^",
        "x^{tree}",
        "x~1",
        "a:b",
        ":/msg",
        "../x",
        "/x",
        "x/",
        "a//b",
        "x.lock",
        "x.",
        "a b",
        "a\nb",
        "a*",
        "a?",
        "a[",
        "a\\b",
        "a\u{7f}",
        "@",
        ".hidden",
        "foo/.bar",
        "foo.lock/bar",
        "x/.",
        "x/..y",
    ];
    let home = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("swr-ref");
    fs::create_dir_all(&home).expect("create the git home");
    let mut taken_not_by_git = Vec::new();
    for value in candidates {
        if !valid_start_ref(value) {
            continue;
        }
        let git = Command::new("git")
            .args(["check-ref-format", "--allow-onelevel", value])
            .env("HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git runs");
        if !git.status.success() {
            taken_not_by_git.push(value);
        }
    }
    assert!(
        taken_not_by_git.is_empty(),
        "start_ref takes what git check-ref-format refuses: {taken_not_by_git:?}"
    );
}

/// A brief's bidirectional embedding, override and isolate characters (U+202A to U+202E, U+2066
/// to U+2069) reach neither `conductor status --full` nor the text and Markdown of `dispatch
/// dispatches`, where they reorder what the terminal shows of the row.
#[test]
fn no_bidirectional_control_of_a_brief_reaches_status_or_a_view() {
    let bidi =
        |character: char| matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
    let case = Case::new("bidi");
    let _runtime = case.serve();
    let answer = post(
        &case,
        &json!({
            "repository": "alpha",
            "brief": "ship \u{202e}gnihton\u{202c} and \u{2066}isolated\u{2069} text",
            "reference": "WI-BIDI",
        }),
    );
    assert_eq!(answer.status, 201, "{answer:?}");

    let mut found = Vec::new();
    for args in [
        &["dispatch", "dispatches", "--format", "text"][..],
        &["dispatch", "dispatches", "--format", "markdown"][..],
        &["status", "--full"][..],
    ] {
        let printed = case.ok(args, "");
        assert!(
            printed.contains("gnihton"),
            "{args:?} shows no brief: {printed}"
        );
        let held: Vec<char> = printed.chars().filter(|c| bidi(*c)).collect();
        if !held.is_empty() {
            found.push((args.join(" "), held));
        }
    }
    assert!(
        found.is_empty(),
        "bidirectional controls printed: {found:?}"
    );
}
