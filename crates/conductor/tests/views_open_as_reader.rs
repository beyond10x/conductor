//! Every read-only command opens the store as a reader (`story:views-open-as-reader`): while
//! another process holds the database's write lock, each view still answers within five seconds,
//! since a reader never takes that lock.
//!
//! The commands are those [`conductor_cli::cli::LEAVES`] declares as reads, less the few that
//! open no store of the state directory (the config, the doctor, the rules, the dashboard server,
//! and the host's ledger, which `host` opens as a reader of its own).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::cli::{Access, LEAVES};

/// How long a view may take while the writer lock is held.
const ANSWER_WITHIN: Duration = Duration::from_secs(5);

/// The reads that open no store under the state directory.
const NO_STATE_STORE: &[&[&str]] = &[
    &["config", "show"],
    &["config", "validate"],
    &["doctor"],
    &["rules", "show"],
    &["host", "grants"],
    &["host", "reclaim-runs"],
];

/// The arguments a read needs beyond its words to get as far as opening the store.
fn extra(words: &[&str]) -> &'static [&'static str] {
    match words {
        ["decision", "show"] => &["--decision-id", "d-1"],
        ["repository", "shipped"] => &["--since", "2026-01-01T00:00:00Z"],
        _ => &[],
    }
}

/// Holds the write lock of the database under `state` (`state/store.sqlite`) in an open write
/// transaction, as a writer in the middle of its commit does.
fn hold_writer_lock(state: &Path) -> rusqlite::Connection {
    let lock = rusqlite::Connection::open(state.join("store.sqlite")).expect("open the database");
    lock.execute_batch("BEGIN IMMEDIATE")
        .expect("hold the database's write lock");
    lock
}

#[test]
fn every_view_answers_while_the_writer_lock_is_held() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("views_open_as_reader");
    let _ = fs::remove_dir_all(&scratch);
    let home = scratch.join("home");
    let state = scratch.join("state");
    let path = scratch.join("path");
    fs::create_dir_all(&home).expect("a home");
    fs::create_dir_all(&path).expect("an empty PATH");
    conductor_cli::store::open(&state).expect("create the store");
    let lock = hold_writer_lock(&state);

    let reads: Vec<&[&str]> = LEAVES
        .iter()
        .filter(|(_, access)| matches!(access, Access::Reads | Access::ConductorReads))
        .map(|(words, _)| *words)
        .filter(|words| !NO_STATE_STORE.contains(words))
        .collect();
    assert!(reads.len() > 30, "the walk found only {reads:?}");

    let mut running: Vec<(&[&str], Child)> = reads
        .iter()
        .map(|words| {
            let child = common::conductor(&home)
                .args(*words)
                .args(extra(words))
                .env("PATH", &path)
                .arg("--state-dir")
                .arg(&state)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap_or_else(|error| panic!("start `conductor {}`: {error}", words.join(" ")));
            (*words, child)
        })
        .collect();

    let deadline = Instant::now() + ANSWER_WITHIN;
    let mut waiting: Vec<String> = Vec::new();
    while !running.is_empty() && Instant::now() < deadline {
        running.retain_mut(|(_, child)| child.try_wait().expect("ask after a view").is_none());
        thread::sleep(Duration::from_millis(20));
    }
    for (words, child) in &mut running {
        waiting.push(words.join(" "));
        let _ = child.kill();
        let _ = child.wait();
    }
    drop(lock);

    assert!(
        waiting.is_empty(),
        "these views did not answer within {ANSWER_WITHIN:?} while the tree's writer lock was \
         held, so they open the store as a writer: {waiting:?}"
    );
}
