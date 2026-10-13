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

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

/// Each program conductor starts, with the version line its fake prints. `aep`, `worktree` and
/// `ess` print a version newer than any pin a build carries (`story:doctor-minimum-versions`), so
/// a case without a pin file passes whatever the repository's `b10x.toml` pins.
const PROGRAMS: [(&str, &str); 8] = [
    ("git", "git version 2.51.0"),
    ("gh", "gh version 2.100.0 (2026-09-04)"),
    ("claude", "2.1.295 (Claude Code)"),
    ("jq", "jq-1.8.2"),
    ("task", "3.53.1+ff3372fc"),
    ("aep", "aep 99.1.0"),
    ("worktree", "worktree-cli 99.1.0"),
    ("ess", "ess 99.1.0"),
];

/// The pins of the repository's own `b10x.toml`, which the build carries as its minimum versions.
fn repository_pins() -> Vec<(String, String)> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../b10x.toml");
    let text = fs::read_to_string(&file).expect("the repository has a b10x.toml");
    conductor_cli::doctor::pins(&text, &file).expect("the repository's pins read")
}

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
        common::conductor(self.home())
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

    /// A config file of one instance `alpha` whose `sources` are `sources` (a YAML flow
    /// sequence), with its checkouts, trees and records in the case's directory.
    fn config(&self, sources: &str) -> PathBuf {
        let file = self.dir.join("conductor.yaml");
        fs::write(
            &file,
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: alpha\n\
                 \x20   sources: {sources}\n\
                 \x20   checkouts:\n\
                 \x20     root: {root}\n\
                 \x20     trees: {trees}\n\
                 \x20   records: {records}\n",
                root = self.dir.join("checkouts").display(),
                trees = self.dir.join("trees").display(),
                records = self.dir.join("records").display(),
            ),
        )
        .expect("write the config file");
        file
    }

    /// `conductor doctor` in `work/` for the instance `config` names (`CONDUCTOR_CONFIG`).
    fn doctor_for(&self, config: &Path) -> Output {
        common::conductor(self.home())
            .arg("doctor")
            .current_dir(self.work())
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
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
        lines.contains(&"pins: none".to_owned()),
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
    case.program("aep", "aep 0.69.1", 0);
    case.program("worktree", "worktree-cli 0.13.0", 0);
    case.program("ess", "ess 0.56.0", 0);
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
    fs::write(case.home().join("b10x.toml"), "[pins]\naep = \"999.9\"\n").expect("write b10x.toml");
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        stdout(&output).contains(&"pins: none".to_owned()),
        "{}",
        shown(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:gh-optional: a forge's CLI is needed only for a source on that forge.
// ---------------------------------------------------------------------------------------------

/// The GitLab CLI, with the version line its fake prints.
const GLAB: (&str, &str) = ("glab", "glab 1.46.0 (2024-09-01)");

/// An instance whose only source is on GitLab needs no `gh`: without it on `PATH` doctor prints
/// `gh: not needed` and exits 0 when every other program is there.
#[test]
fn doctor_needs_no_gh_for_a_gitlab_only_instance() {
    let case = Case::new("gitlab-only");
    case.programs_but(&["gh"]);
    case.program(GLAB.0, GLAB.1, 0);
    let config = case.config("[{gitlab: example-group, host: gitlab.example.com}]");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&"gh: not needed".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        lines.contains(&format!("glab: present, {}", GLAB.1)),
        "{}",
        shown(&output)
    );
}

/// Without a config file no source is named, so neither forge CLI is needed: a GitLab team that has
/// not yet written its instance is not told `gh: missing` (the team's report on v0.7.0).
#[test]
fn doctor_needs_no_forge_cli_without_a_config_file() {
    let case = Case::new("no-config-no-gh");
    case.programs_but(&["gh", "glab"]);
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    for line in ["gh: not needed", "glab: not needed"] {
        assert!(lines.contains(&line.to_owned()), "{}", shown(&output));
    }
}

/// An instance whose only source is on GitHub needs no `glab`.
#[test]
fn doctor_needs_no_glab_for_a_github_only_instance() {
    let case = Case::new("github-only");
    case.programs_but(&[]);
    let config = case.config("[{github: example-org}]");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&"glab: not needed".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        lines.contains(&"gh: present, gh version 2.100.0 (2026-09-04)".to_owned()),
        "{}",
        shown(&output)
    );
}

/// An instance with a `gitlab` source needs `glab`: without it doctor exits 1 naming it.
#[test]
fn doctor_exits_1_naming_glab_for_a_gitlab_source_without_it() {
    let case = Case::new("gitlab-no-glab");
    case.programs_but(&["gh"]);
    let config = case.config("[{gitlab: example-group}]");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&"glab: missing".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        lines.contains(&"gh: not needed".to_owned()),
        "{}",
        shown(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:doctor-verdict: doctor ends with one ready or not-ready line.
// ---------------------------------------------------------------------------------------------

/// The last line `output` printed.
fn last(output: &Output) -> String {
    stdout(output).pop().unwrap_or_default()
}

/// Doctor's last line is `ready` when it exits 0, and otherwise `not ready: ` with each program
/// that failed and why, in doctor's order; without a pin file its pins line is `pins: none`,
/// naming no path.
#[test]
fn doctor_ends_with_one_ready_or_not_ready_line() {
    let case = Case::new("verdict");

    // A passing run, without a pin file.
    case.programs_but(&[]);
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(last(&output), "ready", "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&"pins: none".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        lines
            .iter()
            .filter(|line| line.starts_with("pins: "))
            .all(|line| line.as_str() == "pins: none"),
        "the pins line names no path: {}",
        shown(&output)
    );

    // One program missing and one pinned whose version is not read.
    let repository = case.home().join("repository");
    fs::create_dir_all(&repository).expect("create the repository");
    fs::write(repository.join("b10x.toml"), "[pins]\naep = \"0.70\"\n").expect("write b10x.toml");
    fs::remove_file(case.bin().join("jq")).expect("take jq off PATH");
    case.program("aep", "aep (development build)", 0);
    let output = case.doctor(&repository);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert_eq!(
        last(&output),
        "not ready: jq (missing), aep (version unread)",
        "{}",
        shown(&output)
    );

    // Every program there, one older than its pin.
    case.programs_but(&[]);
    case.program("aep", "aep 0.69.1", 0);
    let output = case.doctor(&repository);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert_eq!(
        last(&output),
        "not ready: aep (too old)",
        "{}",
        shown(&output)
    );
}

/// A pin that is no version, of a program whose version is read, is `(pin unread)`: the pin is
/// what to fix, not the program.
#[test]
fn doctor_names_a_pin_that_is_no_version_pin_unread() {
    let case = Case::new("pin-unread");
    case.programs_but(&[]);
    let repository = case.home().join("repository");
    fs::create_dir_all(&repository).expect("create the repository");
    fs::write(repository.join("b10x.toml"), "[pins]\naep = \"latest\"\n").expect("write b10x.toml");
    let output = case.doctor(&repository);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(
        stdout(&output)
            .contains(&"aep: its pin \"latest\" is not a version: write x.y.z or x.y".to_owned()),
        "{}",
        shown(&output)
    );
    assert_eq!(
        last(&output),
        "not ready: aep (pin unread)",
        "{}",
        shown(&output)
    );
}

/// A config file that does not load makes the run not ready, every program there or not: every
/// command but `config`, `trust`, `init` and `doctor` stops on it. No config file at all is the
/// built-in instance, and ready.
#[test]
fn doctor_is_not_ready_on_a_config_file_that_does_not_load() {
    let case = Case::new("config-unreadable");
    case.programs_but(&[]);
    case.program(GLAB.0, GLAB.1, 0);
    let config = case.dir.join("conductor.yaml");
    fs::write(&config, "version: conductor.config/0\n").expect("write the config file");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    let named: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with("config: "))
        .collect();
    assert!(!named.is_empty(), "{}", shown(&output));
    assert!(
        named.iter().all(|line| !line.contains("--instance")),
        "doctor takes no --instance: {}",
        shown(&output)
    );
    assert_eq!(
        last(&output),
        "not ready: config (unreadable)",
        "{}",
        shown(&output)
    );

    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        !stdout(&output)
            .iter()
            .any(|line| line.starts_with("config: ")),
        "{}",
        shown(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:doctor-minimum-versions: without a pin file, the build's own pins.
// ---------------------------------------------------------------------------------------------

/// Without a `b10x.toml`, doctor compares `aep`, `ess` and `worktree` with the pins the binary
/// was built with, the repository's own `b10x.toml`: each older one is `too old`, and doctor
/// exits 1.
#[test]
fn without_a_pin_file_doctor_refuses_a_cli_older_than_the_build_s_pins() {
    let pins = repository_pins();
    for (name, line, version) in [
        ("aep", "aep 0.63.1", "0.63.1"),
        ("ess", "ess 0.58.0", "0.58.0"),
        ("worktree", "worktree 0.17.0", "0.17.0"),
    ] {
        let pin = pins
            .iter()
            .find(|(program, _)| program == name)
            .map(|(_, pin)| pin.clone())
            .unwrap_or_else(|| panic!("the repository's b10x.toml pins {name}"));
        let case = Case::new(&format!("build-pins-{name}"));
        case.programs_but(&[]);
        case.program(name, line, 0);
        let output = case.doctor(&case.work());
        assert_eq!(output.status.code(), Some(1), "{name}: {}", shown(&output));
        let lines = stdout(&output);
        assert!(
            lines.contains(&"pins: none".to_owned()),
            "{name}: {}",
            shown(&output)
        );
        let older = format!("{name}: {version} is older than the build's pin {pin}");
        assert!(lines.contains(&older), "{}", shown(&output));
        assert_eq!(
            last(&output),
            format!("not ready: {name} (too old)"),
            "{name}: {}",
            shown(&output)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// story:doctor-identity: the machine, and the account each forge CLI is logged in as.
// ---------------------------------------------------------------------------------------------

/// What `uname <flag>` prints on the machine the tests run on.
fn uname(flag: &str) -> String {
    let output = std::process::Command::new("uname")
        .arg(flag)
        .output()
        .expect("uname runs");
    assert!(output.status.success(), "uname {flag} exits 0");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Doctor prints the machine before the program rows: its host name, its operating system and
/// release, its architecture, as `uname` gives them, and the conductor version that ran. `PATH`
/// holds only the fakes, so doctor finds `uname` where the system keeps it.
#[test]
fn doctor_prints_the_machine_before_the_program_rows() {
    let case = Case::new("machine");
    case.programs_but(&[]);
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    let expected = [
        format!("host: {}", uname("-n")),
        format!("system: {} {}", uname("-s"), uname("-r")),
        format!("architecture: {}", uname("-m")),
        format!("conductor {}", env!("CARGO_PKG_VERSION")),
    ];
    assert_eq!(
        lines.get(..4),
        Some(&expected[..]),
        "the first four lines are the machine: {}",
        shown(&output)
    );
    let first_program = lines
        .iter()
        .position(|line| line.starts_with("git: "))
        .expect("a git row");
    assert!(first_program >= 4, "{}", shown(&output));
}

/// The token and the e-mail address the fake `gh` prints beside the login; doctor prints neither.
const FAKE_TOKEN: &str = "example-token-not-a-secret";
const FAKE_EMAIL: &str = "example-user@example.com";

impl Case {
    /// A forge CLI `name` on the case's `PATH`: `--version` prints `version`, and
    /// `auth status --hostname <host>` prints `status` (each line on standard output) and exits
    /// `code` for `host`, and exits 1 naming no account for any other host.
    fn forge(&self, name: &str, version: &str, host: &str, status: &[&str], code: i32) {
        let quoted: Vec<String> = status.iter().map(|line| format!("'{line}'")).collect();
        let file = self.bin().join(name);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then printf '%s\\n' '{version}'; exit 0; fi\n\
                 if [ \"$1\" = auth ] && [ \"$2\" = status ] && [ \"$3\" = --hostname ] \
                 && [ \"$4\" = '{host}' ]; then\n\
                 printf '%s\\n' {lines}\n\
                 exit {code}\n\
                 fi\n\
                 printf '%s\\n' 'not logged in to that host' >&2\n\
                 exit 1\n",
                lines = quoted.join(" "),
            ),
        )
        .expect("write the forge CLI");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }
}

/// For each host of the config file's sources whose CLI is present, doctor names the account the
/// CLI is logged in as there, or `not logged in`, and nothing else the CLI printed; a CLI not
/// logged in to such a host makes the run not ready.
#[test]
fn doctor_names_each_forge_login_and_never_a_token_or_an_e_mail() {
    let case = Case::new("logins");
    case.programs_but(&[]);
    case.forge(
        "gh",
        "gh version 2.100.0 (2026-09-04)",
        "github.com",
        &[
            "github.com",
            "  ✓ Logged in to github.com account example-user (keyring)",
            "  - Active account: true",
            "  - Git operations protocol: https",
            &format!("  - Token: {FAKE_TOKEN}"),
            &format!("  - Account e-mail: {FAKE_EMAIL}"),
        ],
        0,
    );
    case.forge(
        GLAB.0,
        GLAB.1,
        "gitlab.example.com",
        &[
            "gitlab.example.com",
            "  x gitlab.example.com has not been authenticated with glab.",
        ],
        1,
    );
    let config =
        case.config("[{github: example-org}, {gitlab: example-group, host: gitlab.example.com}]");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&"gh: logged in to github.com as example-user".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        lines.contains(&"glab: not logged in to gitlab.example.com".to_owned()),
        "{}",
        shown(&output)
    );
    let everything = shown(&output);
    for secret in [FAKE_TOKEN, FAKE_EMAIL] {
        assert!(
            !everything.contains(secret),
            "doctor printed {secret:?}: {everything}"
        );
    }
    assert_eq!(
        last(&output),
        "not ready: glab (not logged in to gitlab.example.com)",
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
        common::conductor(case.home())
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
    assert_eq!(
        stderr.trim_end(),
        "conductor: snapshot start-snapshot: `aep` is not on PATH, and a snapshot's collectors \
         start it; `conductor doctor` lists every program conductor starts",
        "the refusal names the program, the collectors that start it and the command that lists \
         them: {}",
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
