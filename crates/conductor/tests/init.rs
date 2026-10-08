//! `story:first-run-seed`: `conductor init` writes the active instance's records skeleton — a
//! git repository holding `NORTHSTAR.md`, `docs/handoff/<today>.md` with a first task, `rules.md`
//! with one section per role, a `README.md` saying what the directory is, and `decisions/`,
//! `dispatches/` and `charters/` — commits it, so the hand-over is the one committed last, and
//! refuses to overwrite any file that is already there. It prints what it wrote.
//!
//! Each case runs the built binary with `HOME` naming the case's `home/`, `CONDUCTOR_CONFIG` a
//! config file the case writes, `CONDUCTOR_INSTANCE` removed, and git's identity from the
//! environment, with no system or global git config read.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use time::OffsetDateTime;

/// The files `conductor init` writes, under the records directory, but the hand-over.
const FILES: [&str; 3] = ["NORTHSTAR.md", "rules.md", "README.md"];

/// The directories it makes.
const DIRECTORIES: [&str; 3] = ["decisions", "dispatches", "charters"];

/// The sections of `rules.md`, one per role.
const SECTIONS: [&str; 3] = [
    "## Conductor",
    "## Repository controller",
    "## conductor-dev",
];

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
            .join("init")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create home");
        let dir = dir.canonicalize().expect("the case's directory");
        Self { dir }
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn config(&self) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   sources:\n\
             \x20     - github: example-org\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n",
            root = self.dir.join("checkouts").display(),
            trees = self.dir.join("trees").display(),
            records = self.records().display(),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    fn git(&self, command: &mut Command) -> Output {
        command
            .env("HOME", self.dir.join("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Example Operator")
            .env("GIT_AUTHOR_EMAIL", "operator@example.invalid")
            .env("GIT_COMMITTER_NAME", "Example Operator")
            .env("GIT_COMMITTER_EMAIL", "operator@example.invalid")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .output()
            .expect("the program runs")
    }

    fn init(&self, config: &Path) -> Output {
        self.git(
            Command::new(env!("CARGO_BIN_EXE_conductor"))
                .arg("init")
                .current_dir(&self.dir)
                .env("CONDUCTOR_CONFIG", config)
                .env_remove("CONDUCTOR_INSTANCE"),
        )
    }

    /// `git -C records <args>`, its standard output.
    fn records_git(&self, args: &[&str]) -> String {
        let output = self.git(Command::new("git").arg("-C").arg(self.records()).args(args));
        assert!(output.status.success(), "git {args:?}: {}", shown(&output));
        String::from_utf8(output.stdout).expect("UTF-8")
    }
}

fn shown(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Today's hand-over, `docs/handoff/<YYYY-MM-DD>.md`, by UTC.
fn handoff() -> String {
    let today = OffsetDateTime::now_utc().date();
    format!(
        "docs/handoff/{:04}-{:02}-{:02}.md",
        today.year(),
        u8::from(today.month()),
        today.day()
    )
}

#[test]
fn init_writes_and_commits_the_records_skeleton() {
    let case = Case::new("writes");
    let config = case.config();
    let output = case.init(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let records = case.records();
    assert!(records.join(".git").is_dir(), "a git repository");

    let handoff = handoff();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for file in FILES.iter().copied().chain([handoff.as_str()]) {
        let text = fs::read_to_string(records.join(file))
            .unwrap_or_else(|error| panic!("{file}: {error}; {}", shown(&output)));
        assert!(!text.trim().is_empty(), "{file} is empty");
        assert!(
            stdout.contains(&records.join(file).display().to_string()),
            "init prints {file}: {}",
            shown(&output)
        );
    }
    for directory in DIRECTORIES {
        assert!(records.join(directory).is_dir(), "{directory}/");
    }
    let rules = fs::read_to_string(records.join("rules.md")).expect("read rules.md");
    for section in SECTIONS {
        assert!(
            rules.lines().any(|line| line == section),
            "rules.md has {section}: {rules}"
        );
    }
    let first = fs::read_to_string(records.join(&handoff)).expect("read the hand-over");
    assert!(
        first.to_lowercase().contains("first task"),
        "the hand-over names a first task: {first}"
    );

    // The start prompt reads the hand-over committed last.
    let last = case.records_git(&[
        "log",
        "-1",
        "--name-only",
        "--format=",
        "--",
        "docs/handoff/",
    ]);
    assert_eq!(last.trim(), handoff, "the hand-over is committed");
    let status = case.records_git(&["status", "--porcelain"]);
    assert_eq!(status, "", "every file init wrote is committed");
}

#[test]
fn init_refuses_to_overwrite_a_file_and_writes_nothing() {
    let case = Case::new("refuses");
    let config = case.config();
    fs::create_dir_all(case.records()).expect("create records");
    fs::write(case.records().join("rules.md"), "# our rules\n").expect("write rules.md");

    let output = case.init(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("rules.md"),
        "the refusal names the file: {}",
        shown(&output)
    );
    assert_eq!(
        fs::read_to_string(case.records().join("rules.md")).expect("read rules.md"),
        "# our rules\n"
    );
    let left: Vec<String> = fs::read_dir(case.records())
        .expect("read records")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec!["rules.md".to_owned()], "nothing else is written");
}

#[test]
fn init_twice_refuses_the_second_time() {
    let case = Case::new("twice");
    let config = case.config();
    let first = case.init(&config);
    assert_eq!(first.status.code(), Some(0), "{}", shown(&first));
    let before = case.records_git(&["rev-parse", "HEAD"]);
    let second = case.init(&config);
    assert_eq!(second.status.code(), Some(1), "{}", shown(&second));
    assert_eq!(case.records_git(&["rev-parse", "HEAD"]), before);
}
