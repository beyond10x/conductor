//! `story:records-committed-by-daemon`: the daily tick, and `conductor run --once
//! --commit-records`, commit the records directory through the bot route (`b10x-gates bot --
//! commit -F <file> -- <paths>`) when it has changes, with the message `Conductor: records of
//! <UTC day>: <n> dispatch lines, <m> decision lines`, and push it (`b10x-gates bot -- push origin
//! main`) when it has an `origin`. `records check` runs first; a failing check, a refused commit or
//! a refused push is one `records-commit-refused` line, never a retry; nothing to commit writes no
//! line.
//!
//! Each case is a directory of its own under this test target's temporary directory, removed when
//! the case passes: `home/`, the `HOME` the program runs with; `records/`, a git repository on
//! `main` with one commit; `state/`; `bin/` with the stand-in `b10x-gates`, first on `PATH`;
//! `fake/`, what it reads and writes; and the config file `conductor.yaml` naming them.
//!
//! The stand-in, written here and never committed, logs each call's arguments one line to
//! `fake/gates.calls`, keeps the message file of a commit as `fake/message`, and answers a commit
//! with git's first line, `[main 0123abc] <title>`, and a push with `0`. With `fake/mode` holding
//! `refuse` it refuses a commit, and with `refuse-push` a push, each with two lines on standard
//! error and exit 1. It commits nothing: the records stay as they were.
//!
//! `run --once --commit-records` is run through the built binary; the daily tick through the
//! library in a child of this test binary ([`child`]).

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{Result, bail};
use conductor_cli::config;
use conductor_cli::run::{self, Headroom, Seams};
use conductor_cli::session::{Deliverer, Delivery};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The variable that holds a child run's instructions, as one JSON object.
const CHILD: &str = "CONDUCTOR_TEST_RUN_RECORDS_CHILD";

/// The commit id the stand-in prints.
const COMMIT: &str = "0123abc";

/// The last line the stand-in prints when it refuses.
const REFUSED: &str = "Error: bot Git operation refused";

// ---------------------------------------------------------------------------------------------
// The child run
// ---------------------------------------------------------------------------------------------

/// The deliverer of a child run: the daily tick delivers nothing.
struct Nobody;

impl Deliverer for Nobody {
    fn deliver(&self, name: &str, _text: &str) -> Result<Delivery> {
        bail!("the daily tick delivers nothing, and was asked to deliver to {name}")
    }
}

/// The headroom of a child run: none is asked.
struct NoHeadroom;

impl Headroom for NoHeadroom {
    fn headroom(&self) -> Result<bool> {
        Ok(false)
    }
}

/// The child run of this test binary ([`Case::child`]); a plain run of the suite passes at once.
/// It fixes the case's config file as the process's instance and runs the daily tick for `day`,
/// printing the tick's summary as JSON.
#[test]
fn child() {
    let Ok(spec) = std::env::var(CHILD) else {
        return;
    };
    let spec: Value = serde_json::from_str(&spec).expect("the child's instructions are JSON");
    let file = PathBuf::from(std::env::var_os(config::CONFIG_VARIABLE).expect("a config file"));
    config::set_active(Some(&file)).expect("the case's config loads");
    let host = conductor_cli::confine::Host::default();
    let seams = Seams {
        providers: &host,
        deliverer: &Nobody,
        headroom: &NoHeadroom,
        handle_request: None,
        program: PathBuf::from(spec["program"].as_str().expect("the conductor binary")),
        gc: conductor_cli::gc::gc,
    };
    let day = spec["day"].as_str().expect("the day");
    match run::daily_tick(None, None, day, &seams) {
        Ok(summary) => {
            let actions: Vec<&str> = summary
                .actions
                .iter()
                .map(|event| conductor_cli::records::dispatch_event(*event))
                .collect();
            println!(
                "{}",
                json!({"tick_id": summary.tick_id, "actions": actions})
            );
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("conductor: {error:#}");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// A case directory of its own; see the module documentation.
struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind, its records repository with it; a failed one keeps
/// its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

fn executable(path: &Path, text: &str) {
    write(path, text);
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Today's UTC day, `YYYY-MM-DD`.
fn today() -> String {
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("the clock");
    now[..10].to_owned()
}

impl Case {
    /// A case named `name` whose records directory is a git repository of its own when
    /// `repository`.
    fn new(name: &str, repository: bool) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("run_records")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        case.fixture(repository);
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn fake(&self) -> PathBuf {
        self.dir.join("fake")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn path(&self) -> String {
        format!("{}:/usr/bin:/bin", self.bin().display())
    }

    fn fixture(&self, repository: bool) {
        write(
            &self.config(),
            &format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n\
                 \x20   cache: {cache}\n\
                 \x20   roles:\n\
                 \x20     - {{role: controller, harness: claude, model: sonnet, profile: {profile}, \
                 settings: {settings}}}\n\
                 \x20   confinement: {{provider: none}}\n",
                root = self.home().join("example-org").display(),
                trees = self.dir.join("trees").display(),
                records = self.records().display(),
                state = self.dir.join("state").display(),
                cache = self.dir.join("cache").display(),
                profile = self.dir.join("roles/repo-controller.md").display(),
                settings = self.dir.join("roles/controller-settings.json").display(),
            ),
        );
        write(
            &self.dir.join("roles/repo-controller.md"),
            "# Repository controller\n",
        );
        write(&self.dir.join("roles/controller-settings.json"), "{}\n");
        fs::create_dir_all(self.dir.join("state")).expect("create the state directory");
        fs::create_dir_all(self.fake()).expect("create the fake directory");
        write(
            &self.records().join("charters/alpha.md"),
            "# Charter: alpha\n\nKeep main green.\n",
        );
        if repository {
            self.git(&["init", "--quiet", "--initial-branch", "main"]);
            self.git(&["add", "--all"]);
            self.git(&["commit", "--quiet", "--message", "records"]);
        }
        let fake = self.fake();
        let fake = fake.display();
        executable(
            &self.bin().join("b10x-gates"),
            &format!(
                "#!/bin/sh\n\
                 D='{fake}'\n\
                 printf '%s\\n' \"$*\" >> \"$D/gates.calls\"\n\
                 next=''\n\
                 for arg; do\n\
                 \x20 if [ \"$next\" = file ]; then cp \"$arg\" \"$D/message\"; fi\n\
                 \x20 next=''\n\
                 \x20 if [ \"$arg\" = -F ]; then next=file; fi\n\
                 done\n\
                 mode=$(cat \"$D/mode\" 2>/dev/null)\n\
                 case \"$3\" in\n\
                 commit)\n\
                 \x20 if [ \"$mode\" = refuse ]; then\n\
                 \x20   echo 'the scan found nothing' >&2; echo '{REFUSED}' >&2; exit 1\n\
                 \x20 fi\n\
                 \x20 printf '[main {COMMIT}] %s\\n' \"$(head -n 1 \"$D/message\")\"\n\
                 \x20 echo ' 2 files changed, 2 insertions(+)'; exit 0;;\n\
                 push)\n\
                 \x20 if [ \"$mode\" = refuse-push ]; then\n\
                 \x20   echo 'remote: no' >&2; echo '{REFUSED}' >&2; exit 1\n\
                 \x20 fi\n\
                 \x20 exit 0;;\n\
                 esac\n\
                 exit 0\n"
            ),
        );
    }

    /// `git <args>` in the records directory, which must succeed.
    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=example",
                "-c",
                "user.email=example@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(self.records())
            .env("HOME", self.home())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .stdin(Stdio::null())
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            describe(&output)
        );
    }

    /// Appends `line` to the records' log `log` (`dispatches` or `decisions`), in the month file
    /// of its `at`.
    fn append(&self, log: &str, line: &Value) {
        let at = line["at"].as_str().expect("an at");
        let file = self.records().join(log).join(format!("{}.jsonl", &at[..7]));
        let mut text = fs::read_to_string(&file).unwrap_or_default();
        text.push_str(&format!("{line}\n"));
        write(&file, &text);
    }

    /// Two dispatch lines and a decision line of `day`, and one dispatch line of another day,
    /// which the counts leave out.
    fn lines_of(&self, day: &str) {
        let at = format!("{day}T09:00:00Z");
        self.append(
            "dispatches",
            &json!({"at": at, "event": "cycle", "repo": "-", "line": "spawned none"}),
        );
        self.append(
            "dispatches",
            &json!({"at": format!("{day}T10:00:00Z"), "event": "started", "repo": "alpha",
                    "id": "dispatch-1"}),
        );
        self.append(
            "dispatches",
            &json!({"at": "2026-01-01T00:00:00Z", "event": "cycle", "repo": "-",
                    "line": "spawned none"}),
        );
        self.append(
            "decisions",
            &json!({"at": at, "event": "requested", "repo": "alpha", "id": "req-1",
                    "question": "which?"}),
        );
    }

    /// `conductor <args>` from the records directory, with the case's home, config file and
    /// stand-in.
    fn conductor(&self, args: &[&str]) -> Output {
        common::conductor(self.home())
            .args(args)
            .current_dir(self.records())
            .env(config::CONFIG_VARIABLE, self.config())
            .env("PATH", self.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// A child run of the daily tick for `day`; its summary.
    fn child(&self, day: &str) -> Value {
        let program = PathBuf::from(common::conductor(self.home()).get_program());
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
            .env_remove(config::INSTANCE_VARIABLE)
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env(config::CONFIG_VARIABLE, self.config())
            .env("PATH", self.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(CHILD, json!({"day": day, "program": program}).to_string())
            .current_dir(self.records())
            .stdin(Stdio::null())
            .output()
            .expect("the child runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let summary = stdout
            .lines()
            .rev()
            .find_map(|line| serde_json::from_str::<Value>(line).ok());
        match (output.status.success(), summary) {
            (true, Some(summary)) => summary,
            _ => panic!("the child run failed: {}", describe(&output)),
        }
    }

    /// Every call of the stand-in, its arguments, in order.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.fake().join("gates.calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The message of the commit the stand-in was asked for.
    fn message(&self) -> String {
        fs::read_to_string(self.fake().join("message")).expect("the stand-in kept a message")
    }

    /// Every line of the records' dispatch log whose event is one of the two this story adds.
    fn commit_lines(&self) -> Vec<Value> {
        let dir = self.records().join("dispatches");
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        files
            .iter()
            .flat_map(|file| {
                fs::read_to_string(file)
                    .expect("read the dispatch log")
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).expect("a dispatch line"))
                    .collect::<Vec<_>>()
            })
            .filter(|line| {
                line["event"] == "records-committed" || line["event"] == "records-commit-refused"
            })
            .collect()
    }
}

/// The commit's arguments to the stand-in, with the message file's path replaced by `<file>`.
fn shown(call: &str) -> String {
    let words: Vec<&str> = call.split(' ').collect();
    words
        .iter()
        .enumerate()
        .map(|(at, word)| {
            if at > 0 && words[at - 1] == "-F" {
                "<file>"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

/// Changes in the records are committed through the bot route with the day's counts; the line
/// carries the id the route printed. Without an `origin` nothing is pushed.
#[test]
fn changed_records_are_committed_through_the_bot_route_with_the_days_counts() {
    let case = Case::new("committed", true);
    let day = today();
    case.lines_of(&day);
    let output = case.conductor(&["run", "--once", "--commit-records"]);
    assert!(output.status.success(), "{}", describe(&output));

    let calls: Vec<String> = case.calls().iter().map(|call| shown(call)).collect();
    assert_eq!(
        calls,
        ["bot -- commit -F <file> -- decisions dispatches"],
        "one commit of the changed paths, in the route's order, and no push without an origin"
    );
    let message = format!("Conductor: records of {day}: 2 dispatch lines, 1 decision lines");
    assert_eq!(case.message().trim_end(), message);

    let lines = case.commit_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["event"], "records-committed");
    assert_eq!(lines[0]["repo"], "-");
    assert_eq!(lines[0]["commit"], COMMIT);
    assert_eq!(lines[0]["line"], message.as_str());
    assert_eq!(lines[0]["evidence"], "no origin");
    assert!(
        lines[0].get("tick_id").is_none(),
        "the commit alone runs no tick: {lines:?}"
    );
}

/// A records repository with an `origin` is pushed after the commit.
#[test]
fn with_an_origin_the_commit_is_pushed_to_main() {
    let case = Case::new("pushed", true);
    case.git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/records.git",
    ]);
    case.lines_of(&today());
    let output = case.conductor(&["run", "--once", "--commit-records"]);
    assert!(output.status.success(), "{}", describe(&output));
    let calls: Vec<String> = case.calls().iter().map(|call| shown(call)).collect();
    assert_eq!(
        calls,
        [
            "bot -- commit -F <file> -- decisions dispatches",
            "bot -- push origin main"
        ]
    );
    let lines = case.commit_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["event"], "records-committed");
    assert_eq!(lines[0]["evidence"], "pushed origin main");
}

/// A refused commit is one `records-commit-refused` line with the route's last line: no retry,
/// no push, and the command exits 1.
#[test]
fn a_refused_commit_is_one_line_with_the_routes_last_line() {
    let case = Case::new("refused", true);
    case.git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/records.git",
    ]);
    write(&case.fake().join("mode"), "refuse\n");
    case.lines_of(&today());
    let output = case.conductor(&["run", "--once", "--commit-records"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let calls: Vec<String> = case.calls().iter().map(|call| shown(call)).collect();
    assert_eq!(
        calls,
        ["bot -- commit -F <file> -- decisions dispatches"],
        "one attempt, no push"
    );
    let lines = case.commit_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["event"], "records-commit-refused");
    assert_eq!(lines[0]["reason"], "commit");
    assert_eq!(lines[0]["line"], REFUSED);
    assert!(lines[0].get("commit").is_none(), "{lines:?}");
}

/// A refused push after a commit is one `records-commit-refused` line carrying the commit.
#[test]
fn a_refused_push_is_one_line_with_the_commit_it_did_not_push() {
    let case = Case::new("push-refused", true);
    case.git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/records.git",
    ]);
    write(&case.fake().join("mode"), "refuse-push\n");
    case.lines_of(&today());
    let output = case.conductor(&["run", "--once", "--commit-records"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert_eq!(case.calls().len(), 2, "{:?}", case.calls());
    let lines = case.commit_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["event"], "records-commit-refused");
    assert_eq!(lines[0]["reason"], "push");
    assert_eq!(lines[0]["commit"], COMMIT);
    assert_eq!(lines[0]["line"], REFUSED);
}

/// Records with nothing to commit call no route and write no line.
#[test]
fn nothing_to_commit_writes_no_line() {
    let case = Case::new("nothing", true);
    case.lines_of(&today());
    case.git(&["add", "--all"]);
    case.git(&["commit", "--quiet", "--message", "the day's lines"]);
    let output = case.conductor(&["run", "--once", "--commit-records"]);
    assert!(output.status.success(), "{}", describe(&output));
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    assert!(case.commit_lines().is_empty(), "{:?}", case.commit_lines());
}

/// A failing `records check` refuses the commit with its output in the line, before any route
/// is called.
#[test]
fn a_failing_records_check_refuses_the_commit_with_its_output() {
    let case = Case::new("check-fails", true);
    case.lines_of(&today());
    case.append(
        "dispatches",
        &json!({"at": format!("{}T11:00:00Z", today()), "event": "done", "repo": "alpha",
                "id": "dispatch-1", "evidence": "see DEC-20260101-99"}),
    );
    let output = case.conductor(&["run", "--once", "--commit-records"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    let lines = case.commit_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["event"], "records-commit-refused");
    assert_eq!(lines[0]["reason"], "records check");
    let said = lines[0]["line"].as_str().unwrap_or_default();
    assert!(
        said.contains("UnknownDecision") && said.contains("DEC-20260101-99"),
        "the check's output is in the line: {said}"
    );
}

/// A records directory that is not the top of a git repository of its own is refused, and
/// nothing of the repository around it is staged or committed.
#[test]
fn records_that_are_no_repository_of_their_own_are_refused() {
    let case = Case::new("no-repository", false);
    case.lines_of(&today());
    let output = case.conductor(&["run", "--once", "--commit-records"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    let lines = case.commit_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["event"], "records-commit-refused");
    assert_eq!(lines[0]["reason"], "repository");
}

/// `--commit-records` runs with `--once` only.
#[test]
fn commit_records_without_once_is_refused() {
    let case = Case::new("without-once", true);
    case.lines_of(&today());
    let output = case.conductor(&["run", "--commit-records"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--once"),
        "{}",
        describe(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// The daily tick commits the records after `metric compute` and the brief: `REPORT.md` is in
/// the commit, the message counts the tick's day, and the line carries the tick's id.
#[test]
fn the_daily_tick_commits_the_records_after_metric_compute_and_the_brief() {
    let case = Case::new("daily", true);
    let day = "2026-10-16";
    case.lines_of(day);
    let tick = case.child(day);
    assert_eq!(
        tick["actions"],
        json!(["records-committed"]),
        "the commit is the daily tick's one line: {tick}"
    );
    let calls: Vec<String> = case.calls().iter().map(|call| shown(call)).collect();
    assert_eq!(
        calls,
        ["bot -- commit -F <file> -- decisions dispatches REPORT.md"]
    );
    assert_eq!(
        case.message().trim_end(),
        format!("Conductor: records of {day}: 2 dispatch lines, 1 decision lines")
    );
    let lines = case.commit_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["tick_id"], tick["tick_id"]);
    assert_eq!(lines[0]["commit"], COMMIT);
}
