//! `story:dashboard-plan-board`: the dashboard's Plan page. `GET /v1/instances/example-org/plan` answers the board
//! of a checkout's planning store, `GET /v1/instances/example-org/plan/story` one story as the drawer shows it, and
//! `POST /v1/instances/example-org/plan` does what the operator chose, through the dashboard's one write path (the
//! Configuration page's): only from the page's own origin with its token. A move the store's
//! lifecycle refuses is answered with the store's reason and changes nothing; a legal one is
//! made by `aep plan artifact move` and recorded as one class-C decision by the operator.
//!
//! The cases serve [`served::serve`] in this process on a free port of `127.0.0.1`, over
//! a checkouts root under this test target's temporary directory holding one checkout, `alpha`,
//! whose planning store declares the story lifecycle; the store is made and read back with the
//! `aep` CLI, and the decisions with the built binary. Nothing is read from the environment.

mod active;
mod common;
mod served;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use conductor_cli::dashboard::Sources;
use serde_json::{Value, json};

/// The story lifecycle the fixture's project declares, as the protocol's.
const LIFECYCLE: &str = "kind: story\n\
initial: draft\n\
transitions:\n\
\x20 draft: [proposed, archived]\n\
\x20 proposed: [draft, active, rejected]\n\
\x20 active: [implemented, archived]\n\
\x20 implemented: [archived]\n\
\x20 rejected: [archived]\n\
\x20 archived: []\n";

/// The fixture story's body.
const BODY: &str = "## Why\n\nThe operator asked for it.\n\n## Acceptance\n\n\
- the `board_case` test reads the board\n\
- a second line\n  continued\n\n## Scope\n\n- crates/conductor/src/dashboard/plan.rs\n";

/// One case's directory: `home/` with the records and the checkouts, `state/` with the store.
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
    /// A case whose checkout `alpha` holds `story:a-b`, proposed.
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard-plan")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        let engineering = case.checkout().join(".engineering");
        let lifecycles = engineering.join("proto/artifacts/lifecycles");
        fs::create_dir_all(&lifecycles).expect("create the lifecycles");
        fs::create_dir_all(engineering.join("planning")).expect("create the planning store");
        fs::write(lifecycles.join("story.yaml"), LIFECYCLE).expect("write the lifecycle");
        fs::write(
            engineering.join("project.yaml"),
            "version: aep.project/5\nprotocol: adp/1\nprofile: development.standard\n\
             protocols: proto\nplanning_scope: \"alpha\"\n",
        )
        .expect("write the project");
        fs::write(case.dir.join("body.md"), BODY).expect("write the body");
        let body = case.dir.join("body.md");
        case.aep(&[
            "new",
            "story",
            "a-b",
            "--title",
            "A board",
            "--from",
            body.to_str().expect("a UTF-8 path"),
        ]);
        case.aep(&["move", "story:a-b", "--to", "proposed"]);
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.home().join("example-org/records")
    }

    fn checkouts(&self) -> PathBuf {
        self.home().join("checkouts")
    }

    fn checkout(&self) -> PathBuf {
        self.checkouts().join("alpha")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    /// Runs `aep plan artifact <args>` on the fixture's store; its JSON answer.
    fn aep(&self, args: &[&str]) -> Value {
        let output = Command::new("aep")
            .args(["plan", "artifact"])
            .args(args)
            .arg("--store")
            .arg(self.checkout().join(".engineering/planning"))
            .args(["--format", "json"])
            .current_dir(self.checkout())
            .env("HOME", self.home())
            .stdin(Stdio::null())
            .output()
            .expect("aep runs");
        assert!(
            output.status.success(),
            "aep plan artifact {args:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap_or(Value::Null)
    }

    /// The fixture story as `aep plan artifact show` reads it.
    fn story(&self) -> Value {
        self.aep(&["show", "story:a-b"])
    }

    /// Every decision of the case's store; none when there is no store.
    fn decisions(&self) -> Vec<Value> {
        if !self.state().join("store.sqlite").exists() {
            return Vec::new();
        }
        let output = common::conductor(self.home())
            .current_dir(&self.dir)
            .arg("--state-dir")
            .arg(self.state())
            .args(["decision", "decisions", "--format", "json"])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout)
            .expect("decisions as JSON")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// Asserts the story is proposed with no tag, and no decision was recorded.
    #[track_caller]
    fn unchanged(&self) {
        let story = self.story();
        assert_eq!(story["status"], "proposed", "{story}");
        assert_eq!(story["tags"], json!([]), "{story}");
        assert!(self.decisions().is_empty(), "{:?}", self.decisions());
    }

    fn sources(&self) -> Sources {
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = self.checkouts();
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = self.state();
        sources
    }

    fn start(&self) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        let (sources, state) = (self.sources(), self.state());
        served::serve(listener, sources, Some(state));
        port
    }
}

struct Answer {
    status: u16,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("no JSON ({error}): {}", self.body))
    }
}

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
        body: body.to_owned(),
    }
}

fn get(port: u16, path: &str) -> Answer {
    exchange(
        port,
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

fn token(port: u16) -> String {
    let page = get(port, "/");
    assert_eq!(page.status, 200, "GET /: {}", page.body);
    let marker = r#"<meta name="conductor-token" content=""#;
    let at = page.body.find(marker).expect("the page names its token");
    let rest = &page.body[at + marker.len()..];
    rest[..rest.find('"').expect("the token's closing quote")].to_owned()
}

/// A POST of `body` to `/v1/instances/example-org/plan` with the server's own host and origin, as JSON, each of
/// `headers` replacing the one of its name; `None` leaves it out.
fn post_with(port: u16, body: &Value, headers: &[(&str, Option<String>)]) -> Answer {
    let body = serde_json::to_vec(body).expect("a JSON body");
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
    let mut raw = b"POST /v1/instances/example-org/plan HTTP/1.1\r\n".to_vec();
    for (name, value) in named {
        if let Some(value) = value {
            raw.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
    }
    raw.extend_from_slice(b"\r\n");
    raw.extend_from_slice(&body);
    exchange(port, &raw)
}

fn post(port: u16, body: &Value) -> Answer {
    let token = token(port);
    post_with(port, body, &[("X-Conductor-Token", Some(token))])
}

fn approve() -> Value {
    json!({"repository": "alpha", "id": "story:a-b", "action": "Move", "to": "active"})
}

/// The card of `story:a-b` on the board.
fn card(port: u16) -> Value {
    let board = get(port, "/v1/instances/example-org/plan");
    assert_eq!(board.status, 200, "{}", board.body);
    let board = board.json();
    board["cards"]
        .as_array()
        .and_then(|cards| cards.iter().find(|card| card["id"] == "story:a-b"))
        .cloned()
        .unwrap_or_else(|| panic!("no card story:a-b: {board}"))
}

#[test]
fn the_board_reads_the_store_in_columns_and_creates_nothing() {
    let case = Case::new("board");
    let port = case.start();
    let answer = get(port, "/v1/instances/example-org/plan");
    assert_eq!(answer.status, 200, "{}", answer.body);
    let board = answer.json();
    assert_eq!(board["repositories"], json!(["alpha"]), "{board}");
    assert_eq!(board["repository"], "alpha", "{board}");
    assert_eq!(board["errors"], json!([]), "{board}");
    let card = card(port);
    assert_eq!(card["column"], "Proposed", "{card}");
    assert_eq!(card["title"], "A board", "{card}");
    assert_eq!(card["lane"], "unplanned", "no scope recorded: {card}");
    assert!(
        !case.state().join("store.sqlite").exists(),
        "a view creates no store"
    );
    let unknown = get(
        port,
        "/v1/instances/example-org/plan?repository=..%2Fescape",
    )
    .json();
    assert_eq!(unknown["cards"], json!([]), "{unknown}");
}

#[test]
fn the_drawer_reads_why_acceptance_scope_and_history() {
    let case = Case::new("drawer");
    case.aep(&[
        "evidence",
        "story:a-b",
        "--kind",
        "test_result",
        "--source",
        "cargo test --test board_case: 1 passed",
    ]);
    let port = case.start();
    let answer = get(
        port,
        "/v1/instances/example-org/plan/story?repository=alpha&id=story%3Aa-b",
    );
    assert_eq!(answer.status, 200, "{}", answer.body);
    let story = answer.json();
    assert_eq!(story["why"], "The operator asked for it.", "{story}");
    assert_eq!(
        story["acceptance"],
        json!([
            {"text": "the `board_case` test reads the board", "evidence": true},
            {"text": "a second line continued", "evidence": false},
        ]),
        "{story}"
    );
    assert_eq!(
        story["scope"],
        json!(["crates/conductor/src/dashboard/plan.rs"]),
        "{story}"
    );
    assert!(
        story["history"].as_array().is_some_and(|history| history
            .iter()
            .any(|entry| entry["what"] == "draft → proposed")),
        "{story}"
    );
    let mut moves: Vec<&str> = story["moves"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    moves.sort_unstable();
    assert_eq!(moves, ["active", "draft", "rejected"], "{story}");
    let missing = get(
        port,
        "/v1/instances/example-org/plan/story?repository=alpha&id=story%3Anone",
    );
    assert_eq!(missing.status, 404, "{}", missing.body);
}

#[test]
fn a_move_the_lifecycle_refuses_shows_the_stores_reason_and_changes_nothing() {
    let case = Case::new("refused");
    let port = case.start();
    let answer = post(
        port,
        &json!({"repository": "alpha", "id": "story:a-b", "action": "Move", "to": "implemented"}),
    );
    assert_eq!(answer.status, 409, "{}", answer.body);
    let refused = answer.json();
    assert_eq!(refused["refusal"], "MoveRefused", "{refused}");
    assert!(
        refused["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("may move to")),
        "the store's own reason: {refused}"
    );
    case.unchanged();
    assert_eq!(card(port)["column"], "Proposed");
}

#[test]
fn a_legal_move_is_made_by_the_store_and_recorded_as_the_operators_decision() {
    let case = Case::new("approve");
    let port = case.start();
    let answer = post(port, &approve());
    assert_eq!(answer.status, 200, "{}", answer.body);
    let written = answer.json();
    assert_eq!(written["status"], "active", "{written}");
    assert_eq!(case.story()["status"], "active");
    let decisions = case.decisions();
    assert_eq!(decisions.len(), 1, "{decisions:?}");
    let decision = &decisions[0];
    assert_eq!(
        decision["decision_id"], written["decision_id"],
        "{decision}"
    );
    assert_eq!(decision["decision_class"], "C", "{decision}");
    assert_eq!(decision["decided_by"], "Operator", "{decision}");
    assert_eq!(decision["recorded_by"], "dashboard", "{decision}");
    assert!(
        decision["evidence"]
            .as_str()
            .is_some_and(|evidence| evidence.contains("move story:a-b --to active")),
        "{decision}"
    );
    assert_eq!(card(port)["column"], "Accepted");
}

#[test]
fn a_write_without_the_token_or_from_a_foreign_origin_is_refused() {
    let case = Case::new("no-token");
    let port = case.start();
    let missing = post_with(port, &approve(), &[]);
    assert_eq!(missing.status, 403, "{}", missing.body);
    let wrong = post_with(
        port,
        &approve(),
        &[("X-Conductor-Token", Some("0".repeat(64)))],
    );
    assert_eq!(wrong.status, 403, "{}", wrong.body);
    let foreign = post_with(
        port,
        &approve(),
        &[
            ("X-Conductor-Token", Some(token(port))),
            ("Origin", Some("http://example.org".to_owned())),
        ],
    );
    assert_eq!(foreign.status, 403, "{}", foreign.body);
    let form = post_with(
        port,
        &approve(),
        &[
            ("X-Conductor-Token", Some(token(port))),
            ("Content-Type", Some("text/plain".to_owned())),
        ],
    );
    assert_eq!(form.status, 415, "{}", form.body);
    case.unchanged();
}

#[test]
fn important_puts_the_story_first_in_the_next_wave() {
    let case = Case::new("important");
    let port = case.start();
    let answer = post(
        port,
        &json!({"repository": "alpha", "id": "story:a-b", "action": "Important"}),
    );
    assert_eq!(answer.status, 200, "{}", answer.body);
    assert_eq!(case.story()["tags"], json!(["p1"]));
    let card = card(port);
    assert_eq!(card["important"], true, "{card}");
    assert_eq!(card["lane"], "next", "{card}");
    assert_eq!(case.decisions().len(), 1);
}

#[test]
fn start_and_comment_are_each_one_recorded_write() {
    let case = Case::new("start");
    let port = case.start();
    let early = post(
        port,
        &json!({"repository": "alpha", "id": "story:a-b", "action": "Start"}),
    );
    assert_eq!(
        early.status, 422,
        "only an accepted story starts: {}",
        early.body
    );
    assert_eq!(post(port, &approve()).status, 200);
    let started = post(
        port,
        &json!({"repository": "alpha", "id": "story:a-b", "action": "Start"}),
    );
    assert_eq!(started.status, 200, "{}", started.body);
    assert_eq!(card(port)["column"], "InProgress");
    let comment = post(
        port,
        &json!({"repository": "alpha", "id": "story:a-b", "action": "Comment",
                "text": "narrow the scope"}),
    );
    assert_eq!(comment.status, 200, "{}", comment.body);
    let story = get(
        port,
        "/v1/instances/example-org/plan/story?repository=alpha&id=story%3Aa-b",
    )
    .json();
    assert!(
        story["comments"]
            .as_str()
            .is_some_and(|comments| comments.contains("the operator: narrow the scope")),
        "{story}"
    );
    assert_eq!(case.decisions().len(), 3);
}
