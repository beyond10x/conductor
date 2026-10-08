//! `story:token-spend`: `conductor resource usage`, the token spend per repository and per
//! session read from the session transcripts, and the same per-repository table on the dashboard.
//!
//! Each case lays the fixtures of `tests/fixtures/usage/` out as a home directory of its own in
//! this test target's temporary directory: each transcript under `.claude/projects/<its working
//! directory, encoded>/`, where the encoding writes every character but an ASCII letter or digit
//! as `-`, and the repository directories `example-org/alpha`, `example-org/beta` and
//! `example-org/beta-gamma`. It runs the built binary with `HOME` naming that directory and
//! `--state-dir` naming the case's `state/`, and nothing else from the environment is read.
//!
//! The fixtures hold invented sessions only. What the sessions outside `~/example-org` carry (their
//! directory names, titles, branches and text) is listed in [`PRIVATE`], and none of it may reach
//! any output format, standard error or the dashboard.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use conductor_cli::dashboard::{self, Sources};
use serde_json::{Value, json};
use time::OffsetDateTime;

/// What only the transcripts carried, or only the directories outside `~/example-org` named.
const PRIVATE: [&str; 11] = [
    "elsewhere",
    "fixture-private",
    "fixture-home",
    "fixture alpha title",
    "Fixture",
    "msg_fixture",
    "claude-fixture",
    "5555-4555",
    "6666-4666",
    "8888-4888",
    "wf_fixture",
];

/// One transcript of a case: the working directory of its session, relative to the home; its
/// path under that directory's project directory; and its fixture.
type Placed = (&'static str, &'static str, &'static str);

const ALPHA: &str = "example-org/alpha";
const TREE: &str = ".local/state/worktree/trees/example-org/beta/beta-w01-u1";

/// Every clean fixture: two sessions of `alpha` (one in a subdirectory of its checkout) with two
/// sub-agents of the first, one nested under `workflows/`; one of `beta-gamma`, which must not
/// be read as `beta`; two of a `beta` tree, the second a fork that repeats the first one's two
/// calls; one outside `~/example-org`; and one under it in a directory that is no repository.
const CLEAN: [Placed; 11] = [
    (
        ALPHA,
        "11111111-1111-4111-8111-111111111111.jsonl",
        "alpha-main.jsonl",
    ),
    (
        ALPHA,
        "11111111-1111-4111-8111-111111111111/subagents/agent-fixture01.jsonl",
        "alpha-agent.jsonl",
    ),
    (
        ALPHA,
        "11111111-1111-4111-8111-111111111111/subagents/agent-fixture01.meta.json",
        "alpha-agent.meta.json",
    ),
    (
        ALPHA,
        "11111111-1111-4111-8111-111111111111/subagents/workflows/wf_fixture/agent-fixture02.jsonl",
        "alpha-workflow-agent.jsonl",
    ),
    (
        ALPHA,
        "11111111-1111-4111-8111-111111111111/subagents/workflows/wf_fixture/journal.jsonl",
        "alpha-workflow-journal.jsonl",
    ),
    (
        "example-org/alpha/crates/fixture",
        "77777777-7777-4777-8777-777777777777.jsonl",
        "alpha-subdirectory.jsonl",
    ),
    (
        "example-org/beta-gamma",
        "22222222-2222-4222-8222-222222222222.jsonl",
        "beta-gamma.jsonl",
    ),
    (
        TREE,
        "33333333-3333-4333-8333-333333333333.jsonl",
        "beta-tree.jsonl",
    ),
    (
        TREE,
        "44444444-4444-4444-8444-444444444444.jsonl",
        "beta-fork.jsonl",
    ),
    (
        "elsewhere/fixture-private-name",
        "55555555-5555-4555-8555-555555555555.jsonl",
        "private.jsonl",
    ),
    (
        "example-org/fixture-private-old",
        "66666666-6666-4666-8666-666666666666.jsonl",
        "private-old.jsonl",
    ),
];

/// The day the fixtures' calls are on; the day before it holds one call of `alpha`.
const DAY: &str = "2026-10-07";
const DAY_BEFORE: &str = "2026-10-06";
const SINCE: &str = "2026-10-07T00:00:00Z";

/// The formats `--format` takes.
const FORMATS: [&str; 4] = ["text", "json", "jsonl", "markdown"];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/usage")
}

/// A path as the session tool names its project directory: every character but an ASCII letter
/// or digit written `-`.
fn encode(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// One case: its directory, holding `home/`, `work/` (where the binary runs) and, never created
/// by these commands, `state/`.
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
    /// Lays out `placed`, each fixture's days moved to `day` and the day before it.
    fn new(name: &str, placed: &[Placed], day: &str, day_before: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("usage")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        let home = case.home();
        for repository in ["alpha", "beta", "beta-gamma"] {
            fs::create_dir_all(home.join("example-org").join(repository))
                .expect("create a repository directory");
        }
        fs::write(home.join("example-org/NOTES"), "not a repository\n").expect("write a file");
        fs::create_dir_all(case.dir.join("work")).expect("create the working directory");
        for (cwd, file, fixture) in placed {
            let text = fs::read_to_string(fixtures().join(fixture)).expect("read a fixture");
            // One pass: replacing DAY_BEFORE first would let the DAY replacement turn the new
            // day before into the day itself whenever day_before equals DAY.
            let text = text
                .replace(DAY, "\u{0}DAY\u{0}")
                .replace(DAY_BEFORE, day_before)
                .replace("\u{0}DAY\u{0}", day);
            let path = home
                .join(".claude/projects")
                .join(encode(&home.join(cwd)))
                .join(file);
            fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
            fs::write(&path, text).expect("lay out a fixture");
        }
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    /// Runs `conductor --state-dir <case>/state <args>` from `work/`, with `HOME` naming the
    /// case's home.
    fn conductor(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conductor"))
            .current_dir(self.dir.join("work"))
            .env("HOME", self.home())
            .arg("--state-dir")
            .arg(self.dir.join("state"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// Every file under the case's home, with its bytes.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("list a case directory") {
                let path = entry.expect("a directory entry").path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.insert(path.clone(), fs::read(&path).expect("read a case file"));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.home(), &mut out);
        out
    }

    /// The command wrote nothing: no store under `--state-dir`, no file in the working
    /// directory, and the home exactly as it was.
    fn assert_untouched(&self, before: &BTreeMap<PathBuf, Vec<u8>>, what: &str) {
        assert!(
            !self.dir.join("state").exists(),
            "{what} created the state directory"
        );
        assert_eq!(
            fs::read_dir(self.dir.join("work"))
                .expect("list the working directory")
                .count(),
            0,
            "{what} wrote into its working directory"
        );
        assert!(*before == self.files(), "{what} changed a file under HOME");
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

/// `output` exited 0 with nothing on standard error; its standard output.
fn ok(output: &Output, what: &str) -> String {
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{what}: {}",
        describe(output)
    );
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

/// No [`PRIVATE`] string in `text`.
fn assert_private_kept(text: &str, what: &str) {
    for private in PRIVATE {
        assert!(
            !text.contains(private),
            "{what} carries {private:?}:\n{text}"
        );
    }
}

/// What `--format json` prints for the clean fixtures since [`SINCE`]. Each message counts once
/// however many lines carry it, a synthetic message not at all, a call before `--since` not at
/// all, and a call the fork repeats once, in the session that recorded it first.
fn expected_since() -> Value {
    json!({
        "since": SINCE,
        "repositories": [
            {"repository": "other", "sessions": 2, "calls": 2, "input": 101, "output": 201,
             "cache_read": 5010, "cache_write": 601, "cache_read_percent": 77.7},
            {"repository": "alpha", "sessions": 2, "calls": 3, "input": 19, "output": 31,
             "cache_read": 700, "cache_write": 94, "cache_read_percent": 10.9},
            {"repository": "beta", "sessions": 2, "calls": 3, "input": 3, "output": 4,
             "cache_read": 370, "cache_write": 25, "cache_read_percent": 5.7},
            {"repository": "beta-gamma", "sessions": 1, "calls": 1, "input": 2, "output": 3,
             "cache_read": 370, "cache_write": 0, "cache_read_percent": 5.7},
        ],
        "sessions": [
            {"repository": "other", "session": "55555555", "calls": 1, "input": 100,
             "output": 200, "cache_read": 5000, "cache_write": 600, "latest_context": 5700},
            {"repository": "alpha", "session": "11111111", "calls": 2, "input": 15,
             "output": 27, "cache_read": 700, "cache_write": 90, "latest_context": 455},
            {"repository": "beta-gamma", "session": "22222222", "calls": 1, "input": 2,
             "output": 3, "cache_read": 370, "cache_write": 0, "latest_context": 372},
            {"repository": "beta", "session": "33333333", "calls": 2, "input": 2,
             "output": 2, "cache_read": 250, "cache_write": 20, "latest_context": 161},
            {"repository": "beta", "session": "44444444", "calls": 1, "input": 1,
             "output": 2, "cache_read": 120, "cache_write": 5, "latest_context": 126},
            {"repository": "other", "session": "66666666", "calls": 1, "input": 1,
             "output": 1, "cache_read": 10, "cache_write": 1, "latest_context": 12},
            {"repository": "alpha", "session": "77777777", "calls": 1, "input": 4,
             "output": 4, "cache_read": 0, "cache_write": 4, "latest_context": 8},
        ],
        "subagents": [
            {"repository": "alpha", "session": "11111111", "agents": 2, "calls": 2,
             "input": 4, "output": 6, "cache_read": 150, "cache_write": 30},
        ],
    })
}

fn json_of(text: &str, what: &str) -> Value {
    serde_json::from_str(text)
        .unwrap_or_else(|error| panic!("{what} is no JSON ({error}):\n{text}"))
}

/// The three tables, in the order every format prints them, each with its columns in order.
const TABLES: [(&str, &[&str]); 3] = [
    (
        "repositories",
        &[
            "repository",
            "sessions",
            "calls",
            "input",
            "output",
            "cache_read",
            "cache_write",
            "cache_read_percent",
        ],
    ),
    (
        "sessions",
        &[
            "repository",
            "session",
            "calls",
            "input",
            "output",
            "cache_read",
            "cache_write",
            "latest_context",
        ],
    ),
    (
        "subagents",
        &[
            "repository",
            "session",
            "agents",
            "calls",
            "input",
            "output",
            "cache_read",
            "cache_write",
        ],
    ),
];

/// A row of a JSON table as the words of a text or Markdown row, in `columns`' order: a string
/// as it is, anything else as JSON.
fn words(row: &Value, columns: &[&str]) -> Vec<String> {
    columns
        .iter()
        .map(|column| match &row[*column] {
            Value::String(text) => text.clone(),
            Value::Null => panic!("row {row} has no column {column}"),
            other => other.to_string(),
        })
        .collect()
}

#[test]
fn usage_counts_each_message_once_per_repository_and_session() {
    let case = Case::new("since", &CLEAN, DAY, DAY_BEFORE);
    let before = case.files();
    let output = case.conductor(&["resource", "usage", "--since", SINCE, "--format", "json"]);
    let printed = json_of(
        &ok(&output, "resource usage --format json"),
        "--format json",
    );
    assert_eq!(printed, expected_since(), "--format json since {SINCE}");
    case.assert_untouched(&before, "resource usage");
}

#[test]
fn without_since_every_call_counts() {
    let case = Case::new("all", &CLEAN, DAY, DAY_BEFORE);
    let output = case.conductor(&["resource", "usage", "--format", "json"]);
    let printed = json_of(&ok(&output, "resource usage"), "--format json");
    assert_eq!(printed["since"], Value::Null);
    assert_eq!(
        printed["repositories"][1],
        json!({"repository": "alpha", "sessions": 2, "calls": 4, "input": 1019,
               "output": 1031, "cache_read": 1700, "cache_write": 1094,
               "cache_read_percent": 22.8}),
        "the call of the day before counts without --since: {printed:#}"
    );
    assert_eq!(
        printed["sessions"][1],
        json!({"repository": "alpha", "session": "11111111", "calls": 3, "input": 1015,
               "output": 1027, "cache_read": 1700, "cache_write": 1090,
               "latest_context": 455}),
        "{printed:#}"
    );
}

/// Every format prints the same rows, and none of them, nor standard error, carries a name or a
/// line of a transcript outside `~/example-org`.
#[test]
fn every_format_prints_the_same_rows_and_nothing_private() {
    let case = Case::new("formats", &CLEAN, DAY, DAY_BEFORE);
    let expected = expected_since();
    let mut printed = BTreeMap::new();
    for format in FORMATS {
        let output = case.conductor(&["resource", "usage", "--since", SINCE, "--format", format]);
        let text = ok(&output, &format!("--format {format}"));
        assert_private_kept(&text, &format!("--format {format}"));
        printed.insert(format, text);
    }
    let default = case.conductor(&["resource", "usage", "--since", SINCE]);
    assert_eq!(
        ok(&default, "no --format"),
        printed["text"],
        "text is the default format"
    );

    let mut jsonl: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for line in printed["jsonl"].lines() {
        assert!(
            line.starts_with(r#"{"table":"#),
            "a jsonl line does not open with its table: {line}"
        );
        let mut row = json_of(line, "a --format jsonl line");
        let table = row
            .as_object_mut()
            .and_then(|row| row.remove("table"))
            .unwrap_or_else(|| panic!("a jsonl line names no table: {line}"));
        jsonl
            .entry(table.as_str().expect("a table name").to_owned())
            .or_default()
            .push(row);
    }
    for (table, _) in TABLES {
        assert_eq!(
            Value::Array(jsonl.remove(table).unwrap_or_default()),
            expected[table],
            "--format jsonl, table {table}"
        );
    }
    assert!(
        jsonl.is_empty(),
        "--format jsonl names other tables: {jsonl:?}"
    );

    let text_lines: Vec<Vec<&str>> = printed["text"]
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    for (table, columns) in TABLES {
        let header: Vec<&str> = columns.to_vec();
        assert!(
            text_lines.contains(&header),
            "--format text has no header {columns:?} for {table}:\n{}",
            printed["text"]
        );
        let markdown_header = format!("| {} |", columns.join(" | "));
        assert!(
            printed["markdown"]
                .lines()
                .any(|line| line == markdown_header),
            "--format markdown has no header {markdown_header:?}:\n{}",
            printed["markdown"]
        );
        for row in expected[table].as_array().expect("rows") {
            let words = words(row, columns);
            assert!(
                text_lines
                    .iter()
                    .any(|line| line.iter().copied().eq(words.iter().map(String::as_str))),
                "--format text has no row {words:?}:\n{}",
                printed["text"]
            );
            let markdown_row = format!("| {} |", words.join(" | "));
            assert!(
                printed["markdown"].lines().any(|line| line == markdown_row),
                "--format markdown has no row {markdown_row:?}:\n{}",
                printed["markdown"]
            );
        }
    }
}

/// A complete line that is not JSON is not counted, and is named on standard error by its
/// repository only, after every table; the command exits 1. A last line without its line break
/// is a write in progress and is not named.
#[test]
fn a_line_that_cannot_be_read_is_named_by_its_repository_only() {
    let mut placed = vec![CLEAN[0]];
    placed.push((
        "elsewhere/fixture-private-broken",
        "88888888-8888-4888-8888-888888888888.jsonl",
        "malformed.jsonl",
    ));
    let case = Case::new("malformed", &placed, DAY, DAY_BEFORE);
    for format in FORMATS {
        let output = case.conductor(&["resource", "usage", "--since", SINCE, "--format", format]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
        assert_eq!(
            stderr, "conductor: resource usage: other: 1 transcript line(s) could not be read\n",
            "--format {format}"
        );
        assert_private_kept(&stdout, &format!("--format {format}"));
        assert_private_kept(&stderr, &format!("--format {format}, standard error"));
        if format == "json" {
            assert_eq!(
                json_of(&stdout, "--format json")["repositories"],
                json!([
                    {"repository": "alpha", "sessions": 1, "calls": 2, "input": 15,
                     "output": 27, "cache_read": 700, "cache_write": 90,
                     "cache_read_percent": 99.9},
                    {"repository": "other", "sessions": 1, "calls": 1, "input": 1,
                     "output": 1, "cache_read": 1, "cache_write": 1,
                     "cache_read_percent": 0.1},
                ]),
                "the readable lines still count"
            );
        }
    }
}

#[test]
fn a_flag_that_holds_no_such_value_is_refused() {
    let case = Case::new("refused", &CLEAN, DAY, DAY_BEFORE);
    for (args, names) in [
        (&["--since", "2026-10-07"][..], "--since"),
        (&["--since", "yesterday"][..], "--since"),
        (&["--format", "yaml"][..], "--format"),
    ] {
        let output = case.conductor(&[&["resource", "usage"][..], args].concat());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{args:?}: {}",
            describe(&output)
        );
        assert!(output.stdout.is_empty(), "{args:?}: {}", describe(&output));
        assert!(
            stderr.starts_with(&format!("conductor: resource usage: {names} ")),
            "{args:?}: {stderr:?}"
        );
    }
}

/// `HOME` that is not an absolute path is refused: the transcripts and the repositories are
/// found under it.
#[test]
fn a_home_that_is_not_absolute_is_refused() {
    let case = Case::new("home", &CLEAN, DAY, DAY_BEFORE);
    let output = Command::new(env!("CARGO_BIN_EXE_conductor"))
        .current_dir(case.dir.join("work"))
        .env("HOME", "relative-home")
        .arg("--state-dir")
        .arg(case.dir.join("state"))
        .args(["resource", "usage"])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(output.stdout.is_empty(), "{}", describe(&output));
}

/// One `GET` of `path` on the dashboard at `port`; the body of its 200 answer.
fn get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    let (head, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
    assert!(head.starts_with("HTTP/1.1 200 "), "GET {path}: {head}");
    body.to_owned()
}

/// `/data.json` carries the per-repository table under `usage`, the rows `conductor resource
/// usage --since <today, 00:00Z>` prints, measured on the disk measurement's cadence; the page
/// shows them, and neither carries anything private.
#[test]
fn the_dashboard_carries_the_repository_table_under_usage() {
    let now = OffsetDateTime::now_utc();
    let today = now.date().to_string();
    let yesterday = now
        .date()
        .previous_day()
        .expect("a day before today")
        .to_string();
    let case = Case::new("dashboard", &CLEAN, &today, &yesterday);
    let home = case.home();
    let since = format!("{today}T00:00:00Z");

    let printed = case.conductor(&["resource", "usage", "--since", &since, "--format", "json"]);
    let printed = json_of(&ok(&printed, "resource usage"), "--format json");
    assert_eq!(
        printed["repositories"],
        expected_since()["repositories"],
        "the fixtures moved to today count as on {DAY}"
    );

    let root = home.join("example-org/conductor");
    fs::create_dir_all(root.join("dispatches")).expect("create dispatches/");
    fs::create_dir_all(root.join("decisions")).expect("create decisions/");
    fs::create_dir_all(home.join("var-tmp")).expect("create the fixture /var/tmp");
    fs::write(home.join("agents.json"), "[]").expect("write the session list");
    fs::write(home.join("sweep.json"), r#"{"items": []}"#).expect("write the sweep");
    fs::write(home.join("gc.json"), r#"{"assessments": []}"#).expect("write the gc");
    let cat = |file: &str| vec![OsString::from("cat"), home.join(file).into_os_string()];
    let mut sources = Sources::new(home.clone(), root);
    sources.agents = cat("agents.json");
    sources.sweep = cat("sweep.json");
    sources.gc = cat("gc.json");
    sources.var_tmp = home.join("var-tmp");
    sources.measure_timeout = Duration::from_secs(30);
    assert_eq!(sources.measure_every, Duration::from_secs(600));

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
    let port = listener.local_addr().expect("the bound address").port();
    std::thread::spawn(move || dashboard::serve(listener, sources));

    let deadline = Instant::now() + Duration::from_secs(60);
    let data = loop {
        let body = get(port, "/data.json");
        let data = json_of(&body, "/data.json");
        if data["usage"]["measured_at"].is_string() {
            assert_private_kept(&body, "/data.json");
            break data;
        }
        assert!(
            Instant::now() < deadline,
            "no usage measurement in 60 s: {data:#}"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let usage = &data["usage"];
    assert_eq!(usage["since"], json!(since), "{usage:#}");
    assert_eq!(usage["rows"], printed["repositories"], "{usage:#}");
    assert_eq!(usage["errors"], json!([]), "{usage:#}");

    let page = get(port, "/");
    assert_private_kept(&page, "the page");
    assert!(
        page.contains(&format!("<h2>Token spend since {since}</h2>")),
        "the page has no token spend heading:\n{page}"
    );
    for row in printed["repositories"].as_array().expect("rows") {
        let cells: String = words(row, TABLES[0].1)
            .iter()
            .map(|word| format!("<td>{word}</td>"))
            .collect();
        assert!(
            page.contains(&format!("<tr>{cells}</tr>")),
            "the page has no row {cells}:\n{page}"
        );
    }
}
