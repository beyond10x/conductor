//! The read verbs and their bounded answers (`story:tool-broker-read`, acceptance 4; design
//! § 6.3): `show` prints a range of one file bounded by `--max` and the grant's output limit, and
//! `unchanged since <time>` for a re-read of an unchanged range in the same session; `search`
//! prints the lines that hold a text under the grant's paths; `diff` prints the checkout's change
//! as a stat by default. A success ends in one summary line; a failure is at most 40 lines, the
//! last naming the log that holds the whole output.

mod support;

use std::fs;
use std::path::Path;
use std::process::Command;

use conductor_broker::{FAILURE_LINES, SessionGrant, failure};
use conductor_model::tool::Verb;
use serde_json::json;
use support::{Case, INSTANCE, ask, grant, lines, reader};

const SOURCE: &str = "\
use std::fmt;

/// Adds one.
pub fn add_one(value: i64) -> i64 {
    let next = value + 1;
    next
}

pub fn other() {}
";

fn last(text: &str) -> &str {
    text.lines().last().unwrap_or_default()
}

#[test]
fn show_prints_the_file_and_one_summary_line() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("src/lib.rs", SOURCE)]);
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    case.serve();

    let answer = ask(&socket, "show", json!({"path": "src/lib.rs"}));

    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert!(answer.text.starts_with(SOURCE), "{}", answer.text);
    assert_eq!(
        last(&answer.text),
        format!("show src/lib.rs: lines 1-9 of 9, {} bytes", SOURCE.len())
    );
}

#[test]
fn show_prints_a_range_of_lines() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("src/lib.rs", SOURCE)]);
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    case.serve();

    let answer = ask(
        &socket,
        "show",
        json!({"path": "src/lib.rs", "lines": "4:5"}),
    );

    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert_eq!(
        lines(&answer),
        vec![
            "pub fn add_one(value: i64) -> i64 {",
            "    let next = value + 1;",
            "show src/lib.rs: lines 4-5 of 9, 62 bytes",
        ]
    );
}

#[test]
fn show_prints_the_block_of_a_symbol() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("src/lib.rs", SOURCE)]);
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    case.serve();

    let answer = ask(
        &socket,
        "show",
        json!({"path": "src/lib.rs", "symbol": "add_one"}),
    );

    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert_eq!(
        lines(&answer)[..4],
        [
            "pub fn add_one(value: i64) -> i64 {",
            "    let next = value + 1;",
            "    next",
            "}",
        ]
    );
    assert!(
        last(&answer.text).starts_with("show src/lib.rs: lines 4-7 of 9"),
        "{}",
        answer.text
    );
}

#[test]
fn show_of_a_symbol_the_file_does_not_hold_fails_bounded() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("src/lib.rs", SOURCE)]);
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    case.serve();

    let answer = ask(
        &socket,
        "show",
        json!({"path": "src/lib.rs", "symbol": "missing"}),
    );

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert!(answer.text.contains("missing"), "{}", answer.text);
    assert!(last(&answer.text).starts_with("log: "), "{}", answer.text);
}

#[test]
fn show_is_bounded_by_max_and_by_the_grants_limit() {
    let mut case = Case::new();
    let long = "x".repeat(50) + "\n" + &"y".repeat(50) + "\n";
    let dir = case.workdir("a", &[("long.txt", long.as_str())]);
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    let narrow = case.register(
        "example-org-b",
        INSTANCE,
        &dir,
        SessionGrant {
            max_output_bytes: 20,
            ..reader()
        },
    );
    case.serve();

    let by_max = ask(&socket, "show", json!({"path": "long.txt", "max": 10}));
    let by_grant = ask(&narrow, "show", json!({"path": "long.txt", "max": 1000}));

    assert_eq!(by_max.exit, 0, "{}", by_max.text);
    let body: Vec<&str> = by_max.text.lines().collect();
    assert_eq!(body[0], "x".repeat(10));
    assert_eq!(
        last(&by_max.text),
        "show long.txt: lines 1-2 of 2, 102 bytes, cut at 10 bytes"
    );
    assert_eq!(by_grant.text.lines().next(), Some("x".repeat(20).as_str()));
    assert!(
        last(&by_grant.text).ends_with("cut at 20 bytes"),
        "{}",
        by_grant.text
    );
}

#[test]
fn a_reread_of_an_unchanged_range_prints_unchanged_since() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("a.txt", "one\ntwo\n")]);
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    let other = case.register("example-org-b", INSTANCE, &dir, reader());
    case.serve();

    let first = ask(&socket, "show", json!({"path": "a.txt"}));
    let again = ask(&socket, "show", json!({"path": "a.txt"}));
    let other_session = ask(&other, "show", json!({"path": "a.txt"}));
    let other_range = ask(&socket, "show", json!({"path": "a.txt", "lines": "1:1"}));
    fs::write(dir.join("a.txt"), "one\nthree\n").expect("change the file");
    let changed = ask(&socket, "show", json!({"path": "a.txt"}));

    assert!(first.text.starts_with("one\ntwo\n"), "{}", first.text);
    assert_eq!(again.exit, 0);
    assert_eq!(lines(&again).len(), 1, "{}", again.text);
    assert!(
        again.text.starts_with("show a.txt: unchanged since "),
        "{}",
        again.text
    );
    assert!(
        other_session.text.starts_with("one\ntwo\n"),
        "{}",
        other_session.text
    );
    assert!(
        other_range.text.starts_with("one\n"),
        "{}",
        other_range.text
    );
    assert!(changed.text.starts_with("one\nthree\n"), "{}", changed.text);
}

#[test]
fn search_prints_each_matching_line_and_a_summary() {
    let mut case = Case::new();
    let dir = case.workdir(
        "a",
        &[
            ("src/lib.rs", SOURCE),
            ("src/main.rs", "fn main() { add_one(1); }\n"),
            ("docs/notes.md", "nothing here\n"),
        ],
    );
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    case.serve();

    let everywhere = ask(&socket, "search", json!({"pattern": "add_one"}));
    let in_docs = ask(
        &socket,
        "search",
        json!({"pattern": "add_one", "path": "docs"}),
    );

    assert_eq!(everywhere.exit, 0, "{}", everywhere.text);
    assert_eq!(
        lines(&everywhere),
        vec![
            "src/lib.rs:4: pub fn add_one(value: i64) -> i64 {",
            "src/main.rs:1: fn main() { add_one(1); }",
            "search add_one: 2 matches in 2 files",
        ]
    );
    assert_eq!(
        lines(&in_docs),
        vec!["search add_one: 0 matches in 0 files"]
    );
}

#[test]
fn search_reads_only_the_grants_paths() {
    let mut case = Case::new();
    let dir = case.workdir(
        "a",
        &[("src/lib.rs", "needle\n"), ("secret/key.txt", "needle\n")],
    );
    let socket = case.register(
        "example-org-a",
        INSTANCE,
        &dir,
        SessionGrant {
            paths: vec!["src".to_owned()],
            ..reader()
        },
    );
    case.serve();

    let answer = ask(&socket, "search", json!({"pattern": "needle"}));
    let refused = ask(
        &socket,
        "search",
        json!({"pattern": "needle", "path": "secret"}),
    );

    assert_eq!(
        lines(&answer),
        vec![
            "src/lib.rs:1: needle",
            "search needle: 1 matches in 1 files"
        ]
    );
    assert_eq!(lines(&refused), vec!["NotGranted: paths: src"]);
}

/// Runs git with `args` in `dir`, isolated from the user's git config.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "example")
        .env("GIT_AUTHOR_EMAIL", "example@example.org")
        .env("GIT_COMMITTER_NAME", "example")
        .env("GIT_COMMITTER_EMAIL", "example@example.org")
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?}");
}

#[test]
fn diff_prints_a_stat_by_default_and_the_change_with_patch() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("a.txt", "one\n"), ("b.txt", "two\n")]);
    git(&dir, &["init", "--quiet"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "--quiet", "-m", "start"]);
    fs::write(dir.join("a.txt"), "one\nmore\n").expect("change a file");
    let socket = case.register("example-org-a", INSTANCE, &dir, reader());
    case.serve();

    let stat = ask(&socket, "diff", json!({}));
    let patch = ask(&socket, "diff", json!({"patch": true}));

    assert_eq!(stat.exit, 0, "{}", stat.text);
    assert!(stat.text.contains("a.txt | 1 +"), "{}", stat.text);
    assert!(!stat.text.contains("b.txt"), "{}", stat.text);
    assert_eq!(last(&stat.text), "diff: 1 file changed, 1 insertion(+)");
    assert_eq!(patch.exit, 0, "{}", patch.text);
    assert!(patch.text.contains("+more"), "{}", patch.text);
    assert_eq!(last(&patch.text), "diff: 1 file changed, 1 insertion(+)");
}

#[test]
fn diff_outside_a_repository_fails_bounded_with_a_log() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("a.txt", "one\n")]);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Diff]));
    case.serve();

    let answer = ask(&socket, "diff", json!({}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert!(lines(&answer).len() <= FAILURE_LINES, "{}", answer.text);
    let log = last(&answer.text)
        .strip_prefix("log: ")
        .unwrap_or_else(|| panic!("no log line: {}", answer.text));
    assert!(Path::new(log).is_file(), "{log}");
}

#[test]
fn a_failure_is_at_most_forty_lines_and_its_log_holds_every_line() {
    let case = Case::new();
    let text: String = (1..=100).map(|n| format!("line {n}\n")).collect();

    let answer = failure(&case.dir(), "show a.txt: failed", &text);

    assert_eq!(FAILURE_LINES, 40);
    assert_eq!(answer.exit, 1);
    let shown = lines(&answer);
    assert_eq!(shown.len(), FAILURE_LINES, "{}", answer.text);
    assert_eq!(shown[0], "show a.txt: failed");
    let log = shown[FAILURE_LINES - 1]
        .strip_prefix("log: ")
        .expect("the last line names the log");
    let logged = fs::read_to_string(log).expect("read the log");
    assert!(logged.contains("line 1\n") && logged.contains("line 100\n"));
}
