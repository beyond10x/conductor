//! `story:install-prerequisites`, its command part: `conductor doctor` reports each program
//! conductor starts as present, with the first line its `--version` prints, or missing; reads the
//! `b10x.toml` pins of the current directory or the nearest one above it (below `HOME`) and
//! reports an ecosystem CLI older than its pin; and exits 1 when anything is missing or older.
//! `conductor snapshot start-snapshot` refuses at start, naming the missing program, when one its
//! collectors start is not there, instead of failing inside a collector.
//!
//! Each case runs the built binary with the environment cleared: `HOME` names the case's
//! `home/`, the working directory is under it (so no `b10x.toml` of this checkout is found), and
//! `PATH` holds only the fake programs the case writes, each printing a fixed version line.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Each program conductor starts, with the version line its fake prints.
const PROGRAMS: [(&str, &str); 8] = [
    ("git", "git version 2.51.0"),
    ("gh", "gh version 2.100.0 (2026-09-04)"),
    ("claude", "2.1.295 (Claude Code)"),
    ("jq", "jq-1.8.2"),
    ("task", "3.53.1+ff3372fc"),
    ("aep", "aep 0.69.1"),
    ("worktree", "worktree-cli 0.13.0"),
    ("ess", "ess 0.56.0"),
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
            .join("doctor")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home/work", "bin"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn work(&self) -> PathBuf {
        self.dir.join("home/work")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    /// A program `name` on the case's `PATH` that prints `line` and exits `code`.
    fn program(&self, name: &str, line: &str, code: i32) {
        let file = self.bin().join(name);
        fs::write(
            &file,
            format!("#!/bin/sh\nprintf '%s\\n' '{line}'\nexit {code}\n"),
        )
        .expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// Every program of [`PROGRAMS`] but `except`.
    fn programs_but(&self, except: &[&str]) {
        for (name, line) in PROGRAMS {
            if !except.contains(&name) {
                self.program(name, line, 0);
            }
        }
    }

    fn conductor(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conductor"))
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    fn doctor(&self, cwd: &Path) -> Output {
        self.conductor(cwd, &["doctor"])
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

fn stdout(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn doctor_reports_each_program_present_with_its_version() {
    let case = Case::new("present");
    case.programs_but(&[]);
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    for (name, version) in PROGRAMS {
        let expected = format!("{name}: present, {version}");
        assert!(
            lines.contains(&expected),
            "doctor prints {expected:?}: {}",
            shown(&output)
        );
    }
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("pins: no pins") && line.contains("b10x.toml")),
        "without a b10x.toml doctor says there are no pins: {}",
        shown(&output)
    );
}

#[test]
fn doctor_exits_1_naming_a_missing_program() {
    let case = Case::new("missing");
    case.programs_but(&["aep", "jq"]);
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    for name in ["aep", "jq"] {
        assert!(
            lines.contains(&format!("{name}: missing")),
            "doctor names {name} missing: {}",
            shown(&output)
        );
    }
    assert!(
        lines.contains(&"git: present, git version 2.51.0".to_owned()),
        "{}",
        shown(&output)
    );
}

/// A file on `PATH` that cannot be run is no program.
#[test]
fn doctor_counts_a_file_that_cannot_run_as_missing() {
    let case = Case::new("not-executable");
    case.programs_but(&[]);
    fs::set_permissions(case.bin().join("ess"), fs::Permissions::from_mode(0o644))
        .expect("chmod ess");
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(
        stdout(&output).contains(&"ess: missing".to_owned()),
        "{}",
        shown(&output)
    );
}

#[test]
fn doctor_reports_a_cli_older_than_its_pin() {
    let case = Case::new("pins");
    case.programs_but(&[]);
    let repository = case.home().join("repository");
    let below = repository.join("crates/x");
    fs::create_dir_all(&below).expect("create the repository");
    let pins = repository.join("b10x.toml");
    fs::write(
        &pins,
        "# pins\n[pins]\naep = \"0.70\"   # the newest 0.70.x\ness = \"0.55\"\nworktree = \"0.13.0\"\n",
    )
    .expect("write b10x.toml");

    let output = case.doctor(&below);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&format!("pins: {}", pins.display())),
        "doctor names the pin file it read: {}",
        shown(&output)
    );
    assert!(
        lines.contains(&"aep: 0.69.1 is older than its pin 0.70".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        lines.contains(&"ess: 0.56.0 meets its pin 0.55".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        lines.contains(&"worktree: 0.13.0 meets its pin 0.13.0".to_owned()),
        "{}",
        shown(&output)
    );

    // At or above the pin, doctor exits 0.
    case.program("aep", "aep 0.70.2", 0);
    let output = case.doctor(&below);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        stdout(&output).contains(&"aep: 0.70.2 meets its pin 0.70".to_owned()),
        "{}",
        shown(&output)
    );
}

/// The installer reads no `b10x.toml` in `HOME` or above it, and neither does doctor.
#[test]
fn doctor_reads_no_pins_from_home_or_above() {
    let case = Case::new("pins-home");
    case.programs_but(&[]);
    fs::write(case.home().join("b10x.toml"), "[pins]\naep = \"9.9\"\n").expect("write b10x.toml");
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        stdout(&output)
            .iter()
            .any(|line| line.starts_with("pins: no pins")),
        "{}",
        shown(&output)
    );
}

/// The collectors start `aep` on every run: without it `start-snapshot` refuses at start, with
/// one line naming it, and takes no snapshot.
#[test]
fn start_snapshot_refuses_at_start_naming_a_missing_program() {
    let case = Case::new("snapshot");
    case.programs_but(&["aep"]);
    let config = case.dir.join("conductor.yaml");
    fs::write(
        &config,
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   sources:\n\
             \x20     - github: example-org\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n",
            root = case.dir.join("checkouts").display(),
            trees = case.dir.join("trees").display(),
            records = case.dir.join("records").display(),
        ),
    )
    .expect("write the config file");
    let state = case.dir.join("state");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_conductor"))
            .arg("--state-dir")
            .arg(&state)
            .args(args)
            .current_dir(case.work())
            .env_clear()
            .env("HOME", case.home())
            .env("PATH", case.bin())
            .env("CONDUCTOR_CONFIG", &config)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    };
    let output = run(&[
        "snapshot",
        "start-snapshot",
        "--disk-free-bytes",
        "100000000000",
    ]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{}", shown(&output));
    assert!(
        stderr.contains("`aep`") && stderr.contains("conductor doctor"),
        "the refusal names the program and the command that lists them: {}",
        shown(&output)
    );
    assert!(output.stdout.is_empty(), "{}", shown(&output));
    let listed = run(&["snapshot", "snapshots", "--format", "json"]);
    let rows: serde_json::Value = if listed.status.success() {
        serde_json::from_slice(&listed.stdout).expect("the view prints JSON")
    } else {
        serde_json::json!([])
    };
    assert_eq!(rows, serde_json::json!([]), "no snapshot is started");
}
