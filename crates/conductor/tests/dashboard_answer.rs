//! `story:decisions-batch-answer`: the Decisions page answers open escalated requests in a batch.
//! `POST /api/answers` records each answer as the operator's decision (`recorded_by`
//! `dashboard`, his words or "chosen on the Decisions page") and moves its request to Answered,
//! answering the batch row by row; it takes only answers to open escalated requests, and only with
//! the token the page was served with, from the page's own origin, as JSON of a bounded length.
//!
//! The cases serve [`dashboard::serve_over`] in this process on a free port of `127.0.0.1` over a
//! store under this test target's temporary directory, as `dashboard_fast.rs` does; the requests
//! are raised and escalated, and the store read back, with the built binary over that store.
//! Nothing is read from the environment: the home, the records and the store are the case's.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use conductor_cli::dashboard::{self, Sources};
use serde_json::{Value, json};

/// The yes-no request of every case.
const YES_NO: &str = "REQ-release";
/// The one-of request.
const ONE_OF: &str = "REQ-store";
/// The several-of request.
const SEVERAL_OF: &str = "REQ-checks";
/// The class-H item.
const HANDS: &str = "REQ-token";
/// A request raised and never escalated.
const OPEN: &str = "REQ-open";

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// One case's directory: `home/` with the session list and the records, `state/` with the store.
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
    /// A case named `name` whose store holds the five requests: four escalated, one open.
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard-answer")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        for id in [YES_NO, ONE_OF, SEVERAL_OF, HANDS, OPEN] {
            case.raise(id);
        }
        case.escalate_o(
            YES_NO,
            "yes-no",
            r#"[{"letter":"Y","label":"release today","cost":"an hour"},{"letter":"N","label":"wait","cost":"a day"}]"#,
            "Y",
            "N",
        );
        case.escalate_o(
            ONE_OF,
            "one-of",
            r#"[{"letter":"A","label":"file store","cost":"nothing"},{"letter":"B","label":"tree store","cost":"half a day"},{"letter":"C","label":"no cache","cost":"slower"}]"#,
            "B",
            "A",
        );
        case.escalate_o(
            SEVERAL_OF,
            "several-of",
            r#"[{"letter":"A","label":"unit","cost":"4 min"},{"letter":"B","label":"integration","cost":"12 min"},{"letter":"C","label":"docs","cost":"2 min"}]"#,
            "A,B",
            "A",
        );
        case.decide(&[
            "decision",
            "escalate-request",
            "--request-id",
            HANDS,
            "--escalation-class",
            "H",
            "--reason",
            "only the operator holds the login",
            "--no-dry-run",
            "the step needs the operator's login",
        ]);
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.home().join("example-org/conductor")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    /// Runs the built binary with `args` over the case's store; its standard output, or its
    /// standard error when it failed, with whether it succeeded.
    fn run(&self, args: &[&str]) -> (bool, String) {
        fs::create_dir_all(self.home()).expect("create home/");
        let output = common::conductor(self.home())
            .current_dir(&self.dir)
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        let success = output.status.success();
        let text = if success {
            output.stdout
        } else {
            output.stderr
        };
        (success, String::from_utf8_lossy(&text).into_owned())
    }

    /// Runs `args`, asserting it succeeded.
    fn decide(&self, args: &[&str]) -> String {
        let (success, text) = self.run(args);
        assert!(success, "`{}`: {text}", args.join(" "));
        text
    }

    fn raise(&self, id: &str) {
        self.decide(&[
            "decision",
            "raise-decision-request",
            "--request-id",
            id,
            "--repository",
            "alpha",
            "--question",
            &format!("Question of {id}?"),
            "--options",
            "A: now; B: later",
            "--recommendation",
            "A",
            "--raised-at",
            "2026-10-13T09:00:00Z",
        ]);
    }

    fn escalate_o(&self, id: &str, kind: &str, options: &str, recommended: &str, default: &str) {
        self.decide(&[
            "decision",
            "escalate-request",
            "--request-id",
            id,
            "--escalation-class",
            "O",
            "--reason",
            "the operator's call",
            "--kind",
            kind,
            "--lettered-options",
            options,
            "--recommended-letters",
            recommended,
            "--default-letters",
            default,
            "--deadline",
            "2026-10-14T09:00:00Z",
        ]);
    }

    /// The rows of the view `view` of the case's store, as JSON.
    fn view(&self, view: &str) -> Vec<Value> {
        let text = self.decide(&["decision", view, "--format", "json"]);
        serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|error| panic!("`decision {view}` printed no JSON ({error}): {text}"))
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// The request `id` as the `requests` view shows it.
    fn request(&self, id: &str) -> Value {
        self.view("requests")
            .into_iter()
            .find(|row| row["request_id"] == id)
            .unwrap_or_else(|| panic!("no request {id}"))
    }

    /// Every decision of the store.
    fn decisions(&self) -> Vec<Value> {
        self.view("decisions")
    }

    /// The sources over the case's home; `cat` stands in for every command.
    fn sources(&self) -> Sources {
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = self.state();
        sources
    }

    /// Serves the case's sources over its store in this process; the port.
    fn start(&self) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        let (sources, state) = (self.sources(), self.state());
        std::thread::spawn(move || dashboard::serve_over(listener, sources, Some(state)));
        port
    }
}

// ---------------------------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------------------------

/// One answer: its status, its head and its body.
struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("no JSON ({error}): {}", self.body))
    }
}

/// Sends `raw` to the server on `port` and reads the whole answer.
fn exchange(port: u16, raw: &[u8]) -> Answer {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the server");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    stream.write_all(raw).expect("send the request");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read the answer");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {head}"));
    Answer {
        status,
        head: head.to_owned(),
        body: body.to_owned(),
    }
}

/// `GET path` with the server's own host.
fn get(port: u16, path: &str) -> Answer {
    exchange(
        port,
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

/// The token the page was served with: the `content` of `<meta name="conductor-token">` in the
/// `index.html` `/` answers.
fn token(port: u16) -> String {
    let page = get(port, "/");
    assert_eq!(page.status, 200, "GET /: {}", page.body);
    let at = page
        .body
        .find(r#"<meta name="conductor-token" content=""#)
        .unwrap_or_else(|| panic!("the page names no token: {}", page.body));
    let rest = &page.body[at + r#"<meta name="conductor-token" content=""#.len()..];
    rest[..rest.find('"').expect("the token's closing quote")].to_owned()
}

/// A POST of `body` to `/api/answers`: the server's own host and origin, JSON, the token given,
/// and every `(name, value)` of `headers`, each replacing the one of its name; a value `None`
/// leaves the header out.
fn post_with(port: u16, body: &[u8], headers: &[(&str, Option<String>)]) -> Answer {
    let mut named: Vec<(String, Option<String>)> = vec![
        ("Host".to_owned(), Some(format!("127.0.0.1:{port}"))),
        (
            "Origin".to_owned(),
            Some(format!("http://127.0.0.1:{port}")),
        ),
        (
            "Content-Type".to_owned(),
            Some("application/json".to_owned()),
        ),
        ("Content-Length".to_owned(), Some(body.len().to_string())),
        ("Connection".to_owned(), Some("close".to_owned())),
    ];
    for (name, value) in headers {
        match named
            .iter_mut()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
        {
            Some(entry) => entry.1.clone_from(value),
            None => named.push(((*name).to_owned(), value.clone())),
        }
    }
    let mut raw = b"POST /api/answers HTTP/1.1\r\n".to_vec();
    for (name, value) in named {
        if let Some(value) = value {
            raw.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
    }
    raw.extend_from_slice(b"\r\n");
    raw.extend_from_slice(body);
    exchange(port, &raw)
}

/// A POST of the batch `answers` with the page's token.
fn post(port: u16, answers: &Value) -> Answer {
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": answers})).expect("a JSON body");
    post_with(port, &body, &[("X-Conductor-Token", Some(token))])
}

/// The result row of the request `id` in a batch's answer.
fn result(answer: &Answer, id: &str) -> Value {
    assert_eq!(answer.status, 200, "{}\n{}", answer.head, answer.body);
    answer.json()["results"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["request_id"] == id).cloned())
        .unwrap_or_else(|| panic!("no result for {id}: {}", answer.body))
}

/// Asserts the row of `id` was refused with `error`.
#[track_caller]
fn refused(answer: &Answer, id: &str, error: &str) {
    let row = result(answer, id);
    assert_eq!(row["outcome"], "refused", "{row}");
    assert_eq!(row["error"], error, "{row}");
    assert!(
        row["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "a refusal says why: {row}"
    );
}

/// Asserts the batch was refused whole with `status`, and nothing was recorded.
#[track_caller]
fn refused_whole(case: &Case, answer: &Answer, status: u16) {
    assert_eq!(answer.status, status, "{}\n{}", answer.head, answer.body);
    assert!(
        case.decisions().is_empty(),
        "nothing is recorded: {:?}",
        case.decisions()
    );
    assert_eq!(case.request(YES_NO)["state"], "Escalated");
}

/// One answer to the yes-no request, which every refusal case sends.
fn one_answer() -> Value {
    json!([{"request_id": YES_NO, "choice": ["Y"]}])
}

// ---------------------------------------------------------------------------------------------
// Recorded
// ---------------------------------------------------------------------------------------------

/// Acceptance 1: one POST answers each request by its kind; each answer is recorded as the
/// operator's decision with the chosen letters, his words or "chosen on the Decisions page", and
/// `recorded_by` `dashboard`; its request is Answered with that decision; the answer names each
/// row recorded with its decision's id; `/api/decisions` lists none of them open right after.
#[test]
fn a_batch_records_each_answer_as_the_operators_decision() {
    let case = Case::new("recorded");
    let port = case.start();
    let before = get(port, "/api/decisions");
    assert_eq!(before.json()["open"].as_array().map(Vec::len), Some(4));

    let answer = post(
        port,
        &json!([
            {"request_id": YES_NO, "choice": ["Y"], "words": "ship it today"},
            {"request_id": ONE_OF, "choice": ["B"]},
            {"request_id": SEVERAL_OF, "choice": ["C", "A"], "words": "  "},
            {"request_id": HANDS, "choice": ["done"]},
        ]),
    );
    assert!(
        answer.head.contains("Cache-Control: no-store"),
        "{}",
        answer.head
    );
    assert!(
        answer.head.contains("X-Content-Type-Options: nosniff"),
        "{}",
        answer.head
    );
    assert!(
        answer.head.contains("Content-Security-Policy:"),
        "{}",
        answer.head
    );

    let decisions = case.decisions();
    assert_eq!(decisions.len(), 4, "{decisions:?}");
    for (id, choice, words, class) in [
        (YES_NO, "Y", "ship it today", "O"),
        (ONE_OF, "B", "chosen on the Decisions page", "O"),
        (SEVERAL_OF, "A,C", "chosen on the Decisions page", "O"),
        (HANDS, "done", "chosen on the Decisions page", "H"),
    ] {
        let row = result(&answer, id);
        assert_eq!(row["outcome"], "recorded", "{row}");
        let decision_id = row["decision_id"].as_str().expect("the decision's id");
        let request = case.request(id);
        assert_eq!(request["state"], "Answered", "{request}");
        assert_eq!(request["decision_id"], decision_id, "{request}");
        let decision = decisions
            .iter()
            .find(|decision| decision["decision_id"] == decision_id)
            .unwrap_or_else(|| panic!("no decision {decision_id}"));
        assert_eq!(decision["decided_by"], "Operator", "{decision}");
        assert_eq!(decision["decision_class"], class, "{decision}");
        assert_eq!(decision["choice"], choice, "{decision}");
        assert_eq!(decision["recorded_by"], "dashboard", "{decision}");
        assert_eq!(decision["operator_words"], words, "{decision}");
        assert_eq!(
            decision["question"],
            format!("Question of {id}?"),
            "{decision}"
        );
        assert_eq!(decision["repository"], "alpha", "{decision}");
    }

    let after = get(port, "/api/decisions");
    assert_eq!(after.status, 200);
    assert_eq!(
        after.json()["open"].as_array().map(Vec::len),
        Some(0),
        "the page's next read finds nothing open: {}",
        after.body
    );
}

/// The page sends `Sec-Fetch-Site: same-origin` and, on a same-origin POST, a browser may send
/// no `Origin`: that is taken; neither is refused.
#[test]
fn a_same_origin_post_without_an_origin_is_taken() {
    let case = Case::new("same-origin");
    let port = case.start();
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let answer = post_with(
        port,
        &body,
        &[
            ("X-Conductor-Token", Some(token)),
            ("Origin", None),
            ("Sec-Fetch-Site", Some("same-origin".to_owned())),
        ],
    );
    assert_eq!(result(&answer, YES_NO)["outcome"], "recorded");
}

// ---------------------------------------------------------------------------------------------
// Refused per row
// ---------------------------------------------------------------------------------------------

/// Acceptance 1: a batch is answered row by row: a row the server refuses names its error and
/// why, and records nothing, while the others are recorded.
#[test]
fn a_batch_is_answered_row_by_row() {
    let case = Case::new("row-by-row");
    let port = case.start();
    let answer = post(
        port,
        &json!([
            {"request_id": YES_NO, "choice": ["Y"]},
            {"request_id": ONE_OF, "choice": ["Z"]},
            {"request_id": "REQ-nowhere", "choice": ["A"]},
        ]),
    );
    assert_eq!(result(&answer, YES_NO)["outcome"], "recorded");
    refused(&answer, ONE_OF, "ChoiceInvalid");
    refused(&answer, "REQ-nowhere", "RequestNotFound");
    assert_eq!(case.decisions().len(), 1, "{:?}", case.decisions());
    assert_eq!(case.request(ONE_OF)["state"], "Escalated");
}

/// Acceptance 2: only an escalated request is answered here: an open one is refused
/// (`RequestStateConflict`) and no decision is recorded.
#[test]
fn a_request_that_is_not_escalated_is_refused() {
    let case = Case::new("not-escalated");
    let port = case.start();
    let answer = post(port, &json!([{"request_id": OPEN, "choice": ["A"]}]));
    refused(&answer, OPEN, "RequestStateConflict");
    assert!(case.decisions().is_empty(), "{:?}", case.decisions());
    assert_eq!(case.request(OPEN)["state"], "Open");
}

/// Acceptance 1: a request answered meanwhile (here, by conductor relaying the operator's answer
/// with the same command) is refused, not recorded twice, and leaves no decision behind.
#[test]
fn a_request_answered_meanwhile_is_refused_and_leaves_no_orphan_decision() {
    let case = Case::new("answered-meanwhile");
    let port = case.start();
    let token = token(port);
    let relayed = case.decide(&[
        "decision",
        "answer-escalated-as-operator",
        "--request-id",
        YES_NO,
        "--choice",
        "N",
        "--recorded-by",
        "conductor",
        "--operator-words",
        "no, wait",
    ]);
    let relayed = relayed.trim().to_owned();
    assert_eq!(case.request(YES_NO)["decision_id"], relayed.as_str());

    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let answer = post_with(port, &body, &[("X-Conductor-Token", Some(token))]);
    refused(&answer, YES_NO, "RequestStateConflict");
    let decisions = case.decisions();
    assert_eq!(decisions.len(), 1, "no second decision: {decisions:?}");
    assert_eq!(decisions[0]["decision_id"], relayed.as_str());
    assert_eq!(case.request(YES_NO)["decision_id"], relayed.as_str());
}

/// Acceptance 2: a letter that is none of the request's options is refused (`ChoiceInvalid`).
#[test]
fn a_letter_outside_the_options_is_refused() {
    let case = Case::new("outside-options");
    let port = case.start();
    let answer = post(
        port,
        &json!([
            {"request_id": YES_NO, "choice": ["A"]},
            {"request_id": SEVERAL_OF, "choice": ["A", "D"]},
            {"request_id": HANDS, "choice": ["A"]},
            {"request_id": ONE_OF, "choice": []},
        ]),
    );
    for id in [YES_NO, SEVERAL_OF, HANDS, ONE_OF] {
        refused(&answer, id, "ChoiceInvalid");
    }
    assert!(case.decisions().is_empty(), "{:?}", case.decisions());
}

/// Acceptance 2: a one-of or yes-no request takes one letter; two are refused (`ChoiceInvalid`),
/// as is the same letter twice on a several-of request.
#[test]
fn two_letters_on_a_one_letter_request_are_refused() {
    let case = Case::new("two-letters");
    let port = case.start();
    let answer = post(
        port,
        &json!([
            {"request_id": ONE_OF, "choice": ["A", "B"]},
            {"request_id": YES_NO, "choice": ["Y", "N"]},
            {"request_id": SEVERAL_OF, "choice": ["A", "A"]},
        ]),
    );
    for id in [ONE_OF, YES_NO, SEVERAL_OF] {
        refused(&answer, id, "ChoiceInvalid");
    }
    assert!(case.decisions().is_empty(), "{:?}", case.decisions());
}

// ---------------------------------------------------------------------------------------------
// Refused whole
// ---------------------------------------------------------------------------------------------

/// Acceptance 2: a POST without the token is refused, and nothing is recorded.
#[test]
fn a_post_without_the_token_is_refused() {
    let case = Case::new("no-token");
    let port = case.start();
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let answer = post_with(port, &body, &[]);
    refused_whole(&case, &answer, 403);
}

/// Acceptance 2: a POST with another token is refused, the answer does not echo the token, and
/// nothing is recorded.
#[test]
fn a_post_with_a_wrong_token_is_refused() {
    let case = Case::new("wrong-token");
    let port = case.start();
    let right = token(port);
    assert!(right.len() >= 32, "a token of 32 bytes or more: {right:?}");
    let mut wrong = right.clone();
    let last = if wrong.ends_with('0') { "1" } else { "0" };
    wrong.replace_range(wrong.len() - 1.., last);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let answer = post_with(port, &body, &[("X-Conductor-Token", Some(wrong.clone()))]);
    assert!(!answer.body.contains(&right) && !answer.head.contains(&right));
    assert!(!answer.body.contains(&wrong) && !answer.head.contains(&wrong));
    refused_whole(&case, &answer, 403);
}

/// Acceptance 2: a POST naming another `Host` is refused, as every request is.
#[test]
fn a_post_naming_a_foreign_host_is_refused() {
    let case = Case::new("foreign-host");
    let port = case.start();
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let answer = post_with(
        port,
        &body,
        &[
            ("X-Conductor-Token", Some(token)),
            ("Host", Some(format!("evil.example:{port}"))),
        ],
    );
    refused_whole(&case, &answer, 403);
}

/// Acceptance 2: a POST from another origin is refused, and so is one with neither an `Origin`
/// nor `Sec-Fetch-Site: same-origin`.
#[test]
fn a_post_from_a_foreign_origin_is_refused() {
    let case = Case::new("foreign-origin");
    let port = case.start();
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    for headers in [
        vec![("Origin", Some("http://evil.example".to_owned()))],
        vec![(
            "Origin",
            Some(format!("http://127.0.0.1:{}", port.wrapping_add(1))),
        )],
        vec![("Origin", Some("null".to_owned()))],
        vec![("Origin", None)],
        vec![
            ("Origin", None),
            ("Sec-Fetch-Site", Some("cross-site".to_owned())),
        ],
    ] {
        let mut headers = headers;
        headers.push(("X-Conductor-Token", Some(token.clone())));
        let answer = post_with(port, &body, &headers);
        refused_whole(&case, &answer, 403);
    }
}

/// Acceptance 2: only JSON is taken: `text/plain`, which a cross-site form may send without a
/// preflight, is refused.
#[test]
fn a_post_of_plain_text_is_refused() {
    let case = Case::new("text-plain");
    let port = case.start();
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let answer = post_with(
        port,
        &body,
        &[
            ("X-Conductor-Token", Some(token)),
            ("Content-Type", Some("text/plain".to_owned())),
        ],
    );
    refused_whole(&case, &answer, 415);
}

/// A body longer than 64 KiB is refused before it is read, and a batch of more than 50 rows is
/// refused whole.
#[test]
fn an_oversize_body_or_batch_is_refused() {
    let case = Case::new("oversize");
    let port = case.start();
    let token = token(port);
    let words = "x".repeat(70 * 1024);
    let body = serde_json::to_vec(
        &json!({"answers": [{"request_id": YES_NO, "choice": ["Y"], "words": words}]}),
    )
    .expect("a JSON body");
    let answer = post_with(port, &body, &[("X-Conductor-Token", Some(token.clone()))]);
    refused_whole(&case, &answer, 413);

    let rows: Vec<Value> = (0..51)
        .map(|_| json!({"request_id": YES_NO, "choice": ["Y"]}))
        .collect();
    let body = serde_json::to_vec(&json!({"answers": rows})).expect("a JSON body");
    let answer = post_with(port, &body, &[("X-Conductor-Token", Some(token))]);
    refused_whole(&case, &answer, 413);
}

/// A body without a `Content-Length`, or sent chunked, is refused.
#[test]
fn a_chunked_body_or_one_without_a_length_is_refused() {
    let case = Case::new("chunked");
    let port = case.start();
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let mut chunked = format!("{:x}\r\n", body.len()).into_bytes();
    chunked.extend_from_slice(&body);
    chunked.extend_from_slice(b"\r\n0\r\n\r\n");
    let answer = post_with(
        port,
        &chunked,
        &[
            ("X-Conductor-Token", Some(token.clone())),
            ("Content-Length", None),
            ("Transfer-Encoding", Some("chunked".to_owned())),
        ],
    );
    refused_whole(&case, &answer, 411);

    let answer = post_with(
        port,
        &body,
        &[
            ("X-Conductor-Token", Some(token.clone())),
            ("Transfer-Encoding", Some("chunked".to_owned())),
        ],
    );
    refused_whole(&case, &answer, 411);

    let answer = post_with(
        port,
        &body,
        &[("X-Conductor-Token", Some(token)), ("Content-Length", None)],
    );
    refused_whole(&case, &answer, 411);
}

/// A body that is no batch is refused whole.
#[test]
fn a_body_that_is_no_batch_is_refused() {
    let case = Case::new("no-batch");
    let port = case.start();
    let token = token(port);
    for body in [
        b"not json".to_vec(),
        b"[]".to_vec(),
        br#"{"answers": [{"request_id": 7, "choice": ["Y"]}]}"#.to_vec(),
    ] {
        let answer = post_with(port, &body, &[("X-Conductor-Token", Some(token.clone()))]);
        refused_whole(&case, &answer, 400);
    }
}

/// `/api/answers` takes POST alone; every other path still takes GET alone.
#[test]
fn the_answers_path_takes_post_alone() {
    let case = Case::new("methods");
    let port = case.start();
    let answer = get(port, "/api/answers");
    assert_eq!(answer.status, 405, "{}", answer.body);
    assert!(answer.head.contains("Allow: POST"), "{}", answer.head);
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let mut raw = format!(
        "POST /api/decisions HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nContent-Type: application/json\r\nX-Conductor-Token: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    raw.extend_from_slice(&body);
    assert_eq!(exchange(port, &raw).status, 405);
    assert!(case.decisions().is_empty());
}

// ---------------------------------------------------------------------------------------------
// Adversary (story:decisions-batch-answer): the two writes of one answer
// ---------------------------------------------------------------------------------------------

/// The decisions of the case's store in force, each with the request it answers, when any.
fn in_force_unanswering(case: &Case) -> Vec<Value> {
    let answering: Vec<Value> = case
        .view("requests")
        .into_iter()
        .filter(|request| request["state"] == "Answered")
        .map(|request| request["decision_id"].clone())
        .collect();
    case.decisions()
        .into_iter()
        .filter(|decision| decision["state"] == "InForce")
        .filter(|decision| !answering.contains(&decision["decision_id"]))
        .collect()
}

/// Risk 1: the page and conductor's relay answer the same request at once, several times over.
/// One answer wins: exactly one decision is in force, the request is Answered with it, and every
/// other attempt is refused or reversed, so no decision in force answers no request.
#[test]
fn adv_concurrent_page_and_cli_answers_leave_one_decision_in_force() {
    let case = Case::new("adv-concurrent");
    let port = case.start();
    let token = token(port);
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    std::thread::scope(|scope| {
        for round in 0..4 {
            let (case, body, token) = (&case, &body, token.clone());
            scope.spawn(move || {
                post_with(port, body, &[("X-Conductor-Token", Some(token))]);
            });
            scope.spawn(move || {
                case.run(&[
                    "decision",
                    "answer-escalated-as-operator",
                    "--request-id",
                    YES_NO,
                    "--choice",
                    if round % 2 == 0 { "N" } else { "Y" },
                    "--recorded-by",
                    "conductor",
                    "--operator-words",
                    "relayed",
                ]);
            });
        }
    });
    let request = case.request(YES_NO);
    assert_eq!(request["state"], "Answered", "{request}");
    let in_force: Vec<Value> = case
        .decisions()
        .into_iter()
        .filter(|decision| decision["state"] == "InForce")
        .collect();
    assert_eq!(in_force.len(), 1, "one decision in force: {in_force:#?}");
    assert_eq!(
        request["decision_id"], in_force[0]["decision_id"],
        "{request}"
    );
    let orphans = in_force_unanswering(&case);
    assert!(orphans.is_empty(), "orphan decisions: {orphans:#?}");
}

/// Risk 1: the second of the two writes (the request's move to Answered) fails for a reason other
/// than another writer (here its stream cannot be written, as on a full disk). The doc of
/// `answer_as_operator` promises "no decision answers no request": the decision just recorded
/// is not to stay in force while its request is still Escalated.
#[cfg(unix)]
#[test]
fn adv_a_failed_move_of_the_request_leaves_no_decision_in_force() {
    use std::os::unix::fs::PermissionsExt as _;
    let case = Case::new("adv-failed-move");
    let port = case.start();
    let token = token(port);
    let stream = fs::read_dir(case.state().join("tree/tenants"))
        .expect("the store's tenants")
        .filter_map(Result::ok)
        .map(|tenant| {
            tenant
                .path()
                .join("streams/conductor.decision.DecisionRequest")
                .join(YES_NO)
        })
        .find(|stream| stream.is_dir())
        .expect("the yes-no request's stream directory");
    fs::set_permissions(&stream, fs::Permissions::from_mode(0o555)).expect("make it read-only");
    let body = serde_json::to_vec(&json!({"answers": one_answer()})).expect("a JSON body");
    let answer = post_with(port, &body, &[("X-Conductor-Token", Some(token))]);
    fs::set_permissions(&stream, fs::Permissions::from_mode(0o755)).expect("make it writable");
    let row = result(&answer, YES_NO);
    assert_ne!(row["outcome"], "recorded", "the move failed: {row}");
    assert_eq!(case.request(YES_NO)["state"], "Escalated");
    let orphans = in_force_unanswering(&case);
    assert!(
        orphans.is_empty(),
        "the answer was {row}, and these decisions are in force answering no request: {orphans:#?}"
    );
}
