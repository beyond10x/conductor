//! `story:collect-sessions`: the `sessions` collector over recorded fixtures
//! (`tests/fixtures/sessions/`), never over a real session list.
//!
//! Each case copies the Codex fixture tree into its own directory in this test target's temporary
//! directory and sets each file's modification time against one fixed clock. It points
//! [`Sources`] at that copy, at a Claude session-list fixture printed by `cat`, and at the home
//! directory `/fixture-home`, under which every fixture's working directory lies. It takes a
//! snapshot through the library and reads it back through the built binary, in a later process,
//! with `--state-dir` naming the case's `state/`.

use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use conductor_cli::collect::sessions::{self, Sources};
use conductor_cli::collect::{BOUND, Collector};
use conductor_cli::snapshot::{self, Taken};
use conductor_cli::store;
use conductor_model::observation::SnapshotState;
use serde_json::{Value, json};

/// The home directory the fixtures' working directories lie under.
const HOME: &str = "/fixture-home";

/// What only the sessions outside `~/example-org` carried: their working directories, names and
/// references. None of it may reach the store, a view or a failure reason.
const PRIVATE: [&str; 8] = [
    "elsewhere",
    "private-project",
    "fixture-private-name",
    "example-org-old",
    "near-miss",
    "00000000-0000-4000-8000-0000000000a4",
    "00000000-0000-4000-8000-0000000000a5",
    "00000000-0000-4000-8000-0000000000c4",
];

/// The clock every case reads: 2026-10-07T12:00:00Z.
fn now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_791_374_400)
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sessions")
}

/// A fresh directory for one case, with an empty `work/` to run the binary from.
fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("collect_sessions")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("work")).expect("create the case's directories");
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create the copy's directory");
    for entry in fs::read_dir(from).expect("read the fixture directory") {
        let entry = entry.expect("read a fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a fixture's type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy a fixture");
        }
    }
}

/// Sets every file under `dir` as modified at `at`.
fn touch_all(dir: &Path, at: SystemTime) {
    for entry in fs::read_dir(dir).expect("read the copy") {
        let entry = entry.expect("read an entry of the copy");
        if entry.file_type().expect("an entry's type").is_dir() {
            touch_all(&entry.path(), at);
        } else {
            touch(&entry.path(), at);
        }
    }
}

fn touch(file: &Path, at: SystemTime) {
    File::options()
        .write(true)
        .open(file)
        .and_then(|file| file.set_modified(at))
        .expect("set a file's modification time");
}

/// The Codex fixture tree `name` copied into `dir/codex`: every file written 10 minutes before
/// [`now`], except the thread of 2026-10-06, last written 61 minutes before it.
fn codex(dir: &Path, name: &str) -> PathBuf {
    let codex = dir.join("codex");
    copy_tree(&fixtures().join(name), &codex);
    touch_all(&codex, now() - Duration::from_secs(600));
    let stale = codex
        .join("2026/10/06/rollout-2026-10-06T09-00-00-00000000-0000-4000-8000-0000000000c5.jsonl");
    if stale.exists() {
        touch(&stale, now() - Duration::from_secs(61 * 60));
    }
    codex
}

/// The sources of one case: `agents` as the Claude session-list command, the Codex tree `codex`.
fn sources(agents: Vec<OsString>, codex: PathBuf) -> Sources {
    Sources {
        agents,
        codex,
        home: PathBuf::from(HOME),
        now: now(),
        bound: BOUND,
    }
}

/// `cat` printing the Claude session-list fixture `name`.
fn cat(name: &str) -> Vec<OsString> {
    vec![
        OsString::from("cat"),
        fixtures().join(name).into_os_string(),
    ]
}

/// Takes one snapshot into `dir/state` through the sessions collector over `sources`.
fn take(dir: &Path, sources: Sources) -> Taken {
    let store = store::open(&dir.join("state")).expect("open the store");
    let collector = Collector::new("sessions", move |record| {
        sessions::collect_from(&sources, record)
    });
    snapshot::take(store, &[collector]).expect("the driver ends the snapshot")
}

/// Runs the binary from `dir/work` with `--state-dir` naming `dir/state`.
fn conductor(dir: &Path, args: &[&str]) -> Output {
    let state = dir.join("state");
    Command::new(env!("CARGO_BIN_EXE_conductor"))
        .current_dir(dir.join("work"))
        .arg("--state-dir")
        .arg(&state)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The rows of the view `view`, as `--format jsonl` prints them.
fn rows(dir: &Path, view: &str) -> Vec<Value> {
    let output = conductor(dir, &["snapshot", view, "--format", "jsonl"]);
    assert!(output.status.success(), "{view}: {}", describe(&output));
    String::from_utf8(output.stdout)
        .expect("UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("one JSON object per line"))
        .collect()
}

/// The session rows of the snapshot `taken`, without their observation ids.
fn session_rows(dir: &Path, taken: &Taken) -> Vec<Value> {
    rows(dir, "sessions")
        .into_iter()
        .map(|mut row| {
            assert_eq!(row["snapshot_id"], taken.snapshot_id.0.0.as_str(), "{row}");
            let fields = row.as_object_mut().expect("a row is an object");
            fields.remove("observation_id");
            fields.remove("snapshot_id");
            row
        })
        .collect()
}

fn row(
    harness: &str,
    session_ref: &str,
    name: Option<&str>,
    cwd: &str,
    repository: Option<&str>,
    activity: &str,
) -> Value {
    json!({
        "harness": harness,
        "session_ref": session_ref,
        "name": name,
        "cwd": cwd,
        "repository": repository,
        "role": null,
        "activity": activity,
    })
}

/// What the fixtures read as: the live Claude sessions in list order, then the Codex threads
/// written in the last hour in path order.
fn expected() -> Vec<Value> {
    vec![
        row(
            "Claude",
            "f1x70001",
            Some("fixture-root"),
            "~/example-org",
            None,
            "Busy",
        ),
        row(
            "Claude",
            "f1x70002",
            Some("fixture-checkout"),
            "~/example-org/conductor",
            Some("conductor"),
            "Idle",
        ),
        row(
            "Claude",
            "f1x70003",
            Some("fixture-worktree"),
            "~/.local/state/worktree/trees/example-org/ess/batch-0-52",
            Some("ess"),
            "Busy",
        ),
        row("Claude", "", None, "", None, "Idle"),
        row("Claude", "", None, "", None, "Busy"),
        row(
            "Codex",
            "00000000-0000-4000-8000-0000000000c1",
            None,
            "~/example-org",
            None,
            "Unknown",
        ),
        row(
            "Codex",
            "00000000-0000-4000-8000-0000000000c2",
            None,
            "~/example-org/conductor",
            Some("conductor"),
            "Background",
        ),
        row(
            "Codex",
            "00000000-0000-4000-8000-0000000000c3",
            None,
            "~/.local/state/worktree/trees/example-org/ess/batch-0-52",
            Some("ess"),
            "Unknown",
        ),
        row("Codex", "", None, "", None, "Unknown"),
    ]
}

/// Every file under `dir`, recursively.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).expect("read a store directory") {
        let path = entry.expect("read a store entry").path();
        if path.is_dir() {
            found.extend(files(&path));
        } else {
            found.push(path);
        }
    }
    found
}

/// Asserts that no byte sequence of [`PRIVATE`] is in `text`, which `what` names.
fn assert_private(what: &str, text: &[u8]) {
    for private in PRIVATE {
        assert!(
            !text
                .windows(private.len())
                .any(|window| window == private.as_bytes()),
            "{what} carries {private:?}"
        );
    }
}

/// Asserts that no file of the store under `dir/state` carries anything of [`PRIVATE`].
fn assert_store_private(dir: &Path) {
    let stored = files(&dir.join("state"));
    assert!(!stored.is_empty(), "the case kept no store");
    for file in stored {
        let bytes = fs::read(&file).expect("read a store file");
        assert_private(&file.display().to_string(), &bytes);
    }
}

/// Acceptance: over a `claude agents --json` sample and Codex `session_meta` lines, the workspace
/// root has no repository, a checkout and a managed worktree name theirs, and each source gives
/// one row per live session; a listed session without a `pid` has exited, and a Codex thread not
/// written in the last hour is not live.
#[test]
fn each_live_session_is_recorded_with_the_repository_its_cwd_lies_in() {
    let dir = case_dir("both-sources");
    let codex = codex(&dir, "codex");
    let taken = take(&dir, sources(cat("claude-agents.json"), codex));
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    assert_eq!(taken.reason, None);

    let recorded = session_rows(&dir, &taken);
    assert_eq!(recorded, expected());
    assert!(
        !recorded.iter().any(|row| row["session_ref"] == "f1x70006"
            || row["session_ref"] == "00000000-0000-4000-8000-0000000000c5"),
        "an exited Claude session or a Codex thread quiet for an hour was recorded: {recorded:?}"
    );
}

/// Acceptance: a session outside `~/example-org` keeps its harness and state only. Its working
/// directory, name and reference are in no stored record and no view, in any format.
#[test]
fn a_session_outside_the_workspace_keeps_its_harness_and_state_only() {
    let dir = case_dir("outside");
    let codex = codex(&dir, "codex");
    let taken = take(&dir, sources(cat("claude-agents.json"), codex));
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");

    let outside: Vec<Value> = session_rows(&dir, &taken)
        .into_iter()
        .filter(|row| row["cwd"] == "")
        .collect();
    assert_eq!(
        outside,
        vec![
            row("Claude", "", None, "", None, "Idle"),
            row("Claude", "", None, "", None, "Busy"),
            row("Codex", "", None, "", None, "Unknown"),
        ]
    );
    assert_store_private(&dir);
    for format in ["text", "json", "jsonl", "markdown"] {
        let output = conductor(&dir, &["snapshot", "sessions", "--format", format]);
        assert!(output.status.success(), "{format}: {}", describe(&output));
        assert_private(
            &format!("`snapshot sessions --format {format}`"),
            &output.stdout,
        );
        assert_private(
            &format!("`snapshot sessions --format {format}`"),
            &output.stderr,
        );
    }
}

/// A Codex file whose first line is not written to its end yet is left for the next snapshot; it
/// fails nothing.
#[test]
fn a_codex_file_still_being_written_is_left_for_the_next_snapshot() {
    let dir = case_dir("being-written");
    let codex = codex(&dir, "codex");
    let partial = codex
        .join("2026/10/07/rollout-2026-10-07T11-50-00-00000000-0000-4000-8000-0000000000c6.jsonl");
    fs::write(
        &partial,
        r#"{"timestamp":"2026-10-07T11:50:00.000Z","ordinal":0,"type":"session_me"#,
    )
    .expect("write a partial first line");
    touch(&partial, now() - Duration::from_secs(5));
    fs::write(codex.join("2026/10/07/empty.jsonl"), "").expect("write an empty file");

    let taken = take(&dir, sources(cat("claude-agents.json"), codex));
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    assert_eq!(session_rows(&dir, &taken), expected());
}

/// Without a Codex session directory there are no Codex sessions; that is no failure.
#[test]
fn no_codex_directory_is_no_codex_session() {
    let dir = case_dir("no-codex");
    let taken = take(
        &dir,
        sources(cat("claude-agents.json"), dir.join("no-such-dir")),
    );
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let claude: Vec<Value> = expected()
        .into_iter()
        .filter(|row| row["harness"] == "Claude")
        .collect();
    assert_eq!(session_rows(&dir, &taken), claude);
}

/// A live session the list gives no reference for fails the collector, and the failure names
/// neither its working directory nor its name; nothing of that run is recorded.
#[test]
fn a_live_session_without_a_reference_fails_without_naming_where_it_runs() {
    let dir = case_dir("no-reference");
    let codex = codex(&dir, "codex");
    let taken = take(&dir, sources(cat("claude-agents-no-reference.json"), codex));
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken
        .reason
        .clone()
        .expect("a failed snapshot has a reason");
    assert!(reason.starts_with("sessions: "), "{reason}");
    assert!(reason.contains("neither `id` nor `sessionId`"), "{reason}");
    assert_private("the failure reason", reason.as_bytes());
    assert_store_private(&dir);
    assert_eq!(session_rows(&dir, &taken), Vec::<Value>::new());

    let snapshots = rows(&dir, "snapshots");
    assert_eq!(snapshots.len(), 1, "{snapshots:?}");
    assert_eq!(snapshots[0]["state"], "Failed");
    assert_eq!(snapshots[0]["failure_reason"], reason.as_str());
}

/// A recent Codex file whose first record is not `session_meta` fails the collector naming the
/// file, and nothing the record holds.
#[test]
fn a_codex_file_without_session_meta_fails_naming_the_file_only() {
    let dir = case_dir("malformed");
    let codex = codex(&dir, "codex-malformed");
    let taken = take(&dir, sources(cat("claude-agents.json"), codex));
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(reason.starts_with("sessions: "), "{reason}");
    assert!(
        reason.contains("rollout-2026-10-07T11-40-00-00000000-0000-4000-8000-0000000000c9.jsonl"),
        "{reason}"
    );
    assert!(reason.contains("session_meta"), "{reason}");
    assert_private("the failure reason", reason.as_bytes());
    assert_store_private(&dir);
}

/// The session list is read through `collect::run`: a command still running after the bound is
/// stopped, and the snapshot fails naming the collector, the program and the bound.
#[test]
fn the_session_list_command_is_stopped_at_its_bound() {
    let dir = case_dir("bound");
    let mut sources = sources(
        vec![OsString::from("sleep"), OsString::from("30")],
        dir.join("no-such-dir"),
    );
    sources.bound = Duration::from_secs(2);
    let started = Instant::now();
    let taken = take(&dir, sources);
    assert!(
        started.elapsed() < Duration::from_secs(7),
        "the snapshot took {:?}",
        started.elapsed()
    );
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(reason.starts_with("sessions: "), "{reason}");
    assert!(
        reason.contains("`sleep` gave no answer within 2 s"),
        "{reason}"
    );
}

/// A session-list command that exits unsuccessfully fails the collector, naming the command.
#[test]
fn a_session_list_command_that_fails_fails_the_collector() {
    let dir = case_dir("command-fails");
    let taken = take(
        &dir,
        sources(vec![OsString::from("false")], dir.join("no-such-dir")),
    );
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(reason.starts_with("sessions: "), "{reason}");
    assert!(reason.contains("`false` exited"), "{reason}");
}
