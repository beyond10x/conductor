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
mod fake_provider;

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use conductor_cli::config;
use conductor_model::config::{ForgeRoute, Source};
use fake_provider::Child;

/// The child run of this test binary ([`fake_provider::command`]); a plain run of the suite
/// passes it at once.
#[test]
fn child() {
    if let Some(run) = Child::from_env() {
        if let Some(file) = run.config_file() {
            config::set_active(Some(&file)).expect("the case's config loads");
        }
        run.exit();
    }
}

/// Each program conductor starts, with the version line its fake prints. `aep`, `worktree` and
/// `ess` print a version newer than any pin a build carries (`story:doctor-minimum-versions`), so
/// a case without a pin file passes whatever the repository's `b10x.toml` pins; so does
/// `substrate-daemon`, which every placement of a config file now needs.
const PROGRAMS: [(&str, &str); 9] = [
    ("git", "git version 2.51.0"),
    ("gh", "gh version 2.100.0 (2026-09-04)"),
    ("claude", "2.1.295 (Claude Code)"),
    ("jq", "jq-1.8.2"),
    ("aep", "aep 99.1.0"),
    ("worktree", "worktree-cli 99.1.0"),
    ("ess", "ess 99.1.0"),
    ("substrate-daemon", "substrate-daemon 99.1.0"),
    ("b10x-gates", "b10x-gates 0.1.27"),
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
    /// sequence), with its checkouts, trees and records in the case's directory, on the default
    /// route, `gates`, with a `gates_policy` (`story:doctor-checks-guard-forge-writes`), a file
    /// that is there ([`Case::policy`]).
    fn config(&self, sources: &str) -> PathBuf {
        let policy = format!("\x20   gates_policy: {}\n", self.policy().display());
        self.config_of(sources, &policy)
    }

    /// The Gates policy file of [`Case::config`], written: a `gates_policy` that names no readable
    /// file is not ready.
    fn policy(&self) -> PathBuf {
        let policy = self.dir.join("gates-policy.json");
        fs::write(&policy, "{}\n").expect("write the policy");
        policy
    }

    /// [`Case::config`] without a `gates_policy`, the instance ending with `rest`: whole lines
    /// of the instance, each indented four spaces.
    fn config_of(&self, sources: &str, rest: &str) -> PathBuf {
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
                 \x20   records: {records}\n\
                 {rest}",
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

    /// A fake `uname` on the case's `PATH`, found before the system's: `-s` prints `system`,
    /// every other flag a fixed word (`story:install-hints`).
    fn uname(&self, system: &str) {
        let file = self.bin().join("uname");
        fs::write(
            &file,
            format!(
                "#!/bin/sh\n\
                 case \"$1\" in\n\
                 \x20 -s) printf '%s\\n' '{system}' ;;\n\
                 \x20 *) printf '%s\\n' 'example' ;;\n\
                 esac\n"
            ),
        )
        .expect("write uname");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
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

/// With a config file whose source is on GitHub, so that `gh` is needed and read.
#[test]
fn doctor_reports_each_program_present_with_its_version() {
    let case = Case::new("present");
    case.programs_but(&[]);
    let config = case.config("[{github: example-org}]");
    let output = case.doctor_for(&config);
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

/// The line of an instance that puts it on the direct route ([`Case::config_of`]).
const DIRECT: &str = "\x20   forge: {route: direct}\n";

/// An instance whose only source is on GitLab needs no `gh`: without it on `PATH` doctor prints
/// `gh: not needed` and exits 0 when every other program is there.
#[test]
fn doctor_needs_no_gh_for_a_gitlab_only_instance() {
    let case = Case::new("gitlab-only");
    case.programs_but(&["gh"]);
    case.program(GLAB.0, GLAB.1, 0);
    // The bot route carries no write to a GitLab host, so a GitLab instance that is ready takes
    // the direct route (`story:doctor-checks-guard-forge-writes`).
    let config = case.config_of(
        "[{gitlab: example-group, host: gitlab.example.com}]",
        DIRECT,
    );
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
/// not yet written its instance is not told `gh: missing` (a user's report on v0.7.0).
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
    // On the direct route, so the login is the one thing not ready: the bot route carries no
    // write to the GitLab host (`story:doctor-checks-guard-forge-writes`).
    let config = case.config_of(
        "[{github: example-org}, {gitlab: example-group, host: gitlab.example.com}]",
        DIRECT,
    );
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
             \x20   records: {records}\n\
             \x20   state: {state}\n",
            root = case.dir.join("checkouts").display(),
            trees = case.dir.join("trees").display(),
            records = case.dir.join("records").display(),
            // The `--state-dir` below: a snapshot takes no other (`story:instance-isolation`).
            state = case.dir.join("state").display(),
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

// ---------------------------------------------------------------------------------------------
// story:install-hints: under each missing or too-old program, the command that installs it here.
// ---------------------------------------------------------------------------------------------

/// The line right after `row` in `lines`, if `row` is there.
fn under<'l>(lines: &'l [String], row: &str) -> Option<&'l str> {
    let at = lines.iter().position(|line| line == row)?;
    lines.get(at + 1).map(String::as_str)
}

/// On a fake macOS with `brew` on `PATH`, a missing `jq` is installed with `brew install jq` and
/// a missing `claude` from its install page; a present program gets no install line.
#[test]
fn doctor_prints_brew_install_jq_under_a_missing_jq_on_macos() {
    let case = Case::new("install-macos");
    case.programs_but(&["jq", "claude"]);
    case.uname("Darwin");
    case.program("brew", "Homebrew 4.6.0", 0);
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    assert_eq!(
        under(&lines, "jq: missing"),
        Some("  install: brew install jq"),
        "{}",
        shown(&output)
    );
    assert_eq!(
        under(&lines, "claude: missing"),
        Some("  install: see https://docs.anthropic.com/en/docs/claude-code/setup"),
        "{}",
        shown(&output)
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("  install: "))
            .count(),
        2,
        "only the missing programs get an install line: {}",
        shown(&output)
    );
}

/// `task` is no program conductor starts: with it off `PATH` and every other program there,
/// doctor prints no `task` row and is ready.
#[test]
fn doctor_is_ready_without_task() {
    let case = Case::new("no-task");
    case.programs_but(&[]);
    assert!(
        !case.bin().join("task").exists(),
        "the case puts no task on PATH"
    );
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        !lines.iter().any(|line| line.starts_with("task")),
        "doctor prints no task row: {}",
        shown(&output)
    );
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

/// On a fake Linux, a missing `jq` is the distribution's package for the package manager on
/// `PATH`, and the upstream release URL when none is known.
#[test]
fn doctor_prints_the_distribution_package_or_the_upstream_url_for_jq_on_linux() {
    let case = Case::new("install-linux");
    case.programs_but(&["jq"]);
    case.uname("Linux");
    for (manager, expected) in [
        ("apt-get", "  install: sudo apt-get install jq"),
        ("dnf", "  install: sudo dnf install jq"),
        ("pacman", "  install: sudo pacman -S jq"),
    ] {
        case.program(manager, "a package manager", 0);
        let output = case.doctor(&case.work());
        assert_eq!(
            under(&stdout(&output), "jq: missing"),
            Some(expected),
            "{manager}: {}",
            shown(&output)
        );
        fs::remove_file(case.bin().join(manager)).expect("take the package manager off PATH");
    }
    let output = case.doctor(&case.work());
    assert_eq!(
        under(&stdout(&output), "jq: missing"),
        Some("  install: see https://github.com/jqlang/jq/releases/latest"),
        "{}",
        shown(&output)
    );
}

/// The repository each pinned program is released from.
fn repository_of(program: &str) -> &'static str {
    match program {
        "aep" => "https://github.com/beyond10x/aep",
        "ess" => "https://github.com/beyond10x/ess",
        "worktree" => "https://github.com/beyond10x/worktree",
        _ => "https://github.com/beyond10x/substrate",
    }
}

/// The download of `name`'s release archive `<v>`, for this build's target, checked against the
/// release's `SHA256SUMS` and unpacked into `~/.local/bin`, as `SETUP.md` step 4 writes it
/// (`story:install-hints`).
fn archive_install(name: &str, version: &str) -> String {
    let repo = repository_of(name);
    let target = conductor_cli::upgrade::TARGET;
    let sum = if target.ends_with("apple-darwin") {
        "shasum -a 256"
    } else {
        "sha256sum"
    };
    let base = format!("{repo}/releases/download/{version}");
    let stem = format!("{name}-{version}-{target}");
    format!(
        "curl -fsSLO {base}/{stem}.tar.gz && curl -fsSLO {base}/SHA256SUMS && {sum} --check \
         --ignore-missing SHA256SUMS && tar -xzf {stem}.tar.gz -C ~/.local/bin \
         --strip-components=1 {stem}/{name}"
    )
}

/// `aep`, `ess` and `worktree`, missing or older than their pin, name the direct install from
/// their release archive (the v1.0.0 onboarding runs followed the plugin marketplace page doctor
/// named before, which asks the person first): an exact pin is the version downloaded, a pin
/// `x.y` the newest `x.y.z`, `<v>`, named after the command.
#[test]
fn doctor_prints_the_release_archive_install_for_aep_ess_and_worktree() {
    let case = Case::new("install-setup");
    case.programs_but(&["ess", "worktree"]);
    case.uname("Linux");
    case.program("apt-get", "a package manager", 0);
    case.program("aep", "aep 0.69.1", 0);
    let repository = case.home().join("repository");
    fs::create_dir_all(&repository).expect("create the repository");
    fs::write(
        repository.join("b10x.toml"),
        "[pins]\naep = \"0.70\"\ness = \"0.55\"\nworktree = \"0.13.0\"\n",
    )
    .expect("write b10x.toml");
    let output = case.doctor(&repository);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    let newest = |program: &str, line: &str| {
        format!(
            "  install: {} (<v>: the newest {line}.z release at {}/releases)",
            archive_install(program, "<v>"),
            repository_of(program)
        )
    };
    assert_eq!(
        under(&lines, "aep: present, aep 0.69.1"),
        Some(newest("aep", "0.70").as_str()),
        "too old: {}",
        shown(&output)
    );
    assert_eq!(
        under(&lines, "ess: missing"),
        Some(newest("ess", "0.55").as_str()),
        "{}",
        shown(&output)
    );
    assert_eq!(
        under(&lines, "worktree: missing"),
        Some(format!("  install: {}", archive_install("worktree", "0.13.0")).as_str()),
        "{}",
        shown(&output)
    );
    assert!(
        !lines.iter().any(|line| line.contains("agentplugins")),
        "no install line names the plugin marketplace: {}",
        shown(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// The Gates CLI: the bot route `forge.route: gates`, the default, commits and pushes through it.
// ---------------------------------------------------------------------------------------------

/// The direct install of the newest `b10x-gates` release, as `SETUP.md` step 4 writes it.
const GATES_INSTALL: &str = "  install: curl -fsSLO \
    https://github.com/beyond10x/gates/releases/latest/download/b10x-gates-linux-x86_64 && \
    curl -fsSLO https://github.com/beyond10x/gates/releases/latest/download/SHA256SUMS && \
    sha256sum --check --ignore-missing SHA256SUMS && install -m 755 b10x-gates-linux-x86_64 \
    ~/.local/bin/b10x-gates";

/// An instance on the default route, `gates`, needs `b10x-gates`: without it doctor names it
/// missing, under it the direct install of its release, and is not ready.
#[test]
fn doctor_needs_the_gates_cli_under_the_gates_route() {
    let case = Case::new("gates-route");
    case.programs_but(&["b10x-gates"]);
    let config = case.config("[{github: example-org}]");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    if conductor_cli::upgrade::TARGET == "x86_64-unknown-linux-gnu" {
        assert_eq!(
            under(&lines, "b10x-gates: missing"),
            Some(GATES_INSTALL),
            "{}",
            shown(&output)
        );
    } else {
        assert!(
            lines.contains(&"b10x-gates: missing".to_owned()),
            "{}",
            shown(&output)
        );
    }
    assert_eq!(
        last(&output),
        "not ready: b10x-gates (missing)",
        "{}",
        shown(&output)
    );
}

/// Without a config file the instance `SETUP.md` writes takes the default route, `gates`, so
/// `b10x-gates` is needed already.
#[test]
fn doctor_needs_the_gates_cli_without_a_config_file() {
    let case = Case::new("gates-no-config");
    case.programs_but(&["b10x-gates"]);
    let output = case.doctor(&case.work());
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(
        stdout(&output).contains(&"b10x-gates: missing".to_owned()),
        "{}",
        shown(&output)
    );
}

/// When every instance takes `forge.route: direct`, nothing commits through the bot route:
/// `b10x-gates: not needed`, and doctor is ready without it.
#[test]
fn doctor_needs_no_gates_cli_when_every_instance_is_direct() {
    let case = Case::new("gates-direct");
    case.programs_but(&["b10x-gates"]);
    let config = case.config("[{github: example-org}]");
    let text = fs::read_to_string(&config).expect("the config file");
    fs::write(&config, format!("{text}\x20   forge: {{route: direct}}\n"))
        .expect("write the config file");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        stdout(&output).contains(&"b10x-gates: not needed".to_owned()),
        "{}",
        shown(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:worktree-activation: whether each instance's checkouts root is in an activated workspace.
// ---------------------------------------------------------------------------------------------

impl Case {
    /// The file the fake `worktree` appends each call's arguments to, one call a line.
    fn worktree_calls(&self) -> PathBuf {
        self.dir.join("worktree-calls.log")
    }

    /// A fake `worktree` 0.18.1 on the case's `PATH`: `--version` prints the version line of
    /// [`PROGRAMS`]; `inspect` refuses as the real one does, on standard error, with
    /// `workspace-not-activated` when the workspace is not `activated` and otherwise with
    /// `invalid-worktree-id`, the refusal of an id no tree can have, which comes after the
    /// activation check. Every call but `--version` is recorded; any but `inspect` exits 1.
    fn worktree(&self, activated: bool) {
        let code = if activated {
            "invalid-worktree-id"
        } else {
            "workspace-not-activated"
        };
        let file = self.bin().join("worktree");
        fs::write(
            &file,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then printf '%s\\n' 'worktree-cli 99.1.0'; exit 0; fi\n\
                 printf '%s\\n' \"$*\" >> '{calls}'\n\
                 if [ \"$1\" = inspect ]; then\n\
                 printf '%s\\n' '{{\"code\":\"{code}\",\"message\":\"{code}: refused\",\"ok\":false,\"version\":4}}' >&2\n\
                 exit 1\n\
                 fi\n\
                 exit 1\n",
                calls = self.worktree_calls().display(),
            ),
        )
        .expect("write worktree");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// The checkouts root of [`Case::config`]'s instance, holding one git checkout `alpha`.
    fn checkouts(&self) -> PathBuf {
        let root = self.dir.join("checkouts");
        fs::create_dir_all(root.join("alpha/.git")).expect("create a checkout");
        root
    }
}

/// A checkouts root `worktree` finds in no activated workspace: doctor prints it `not activated`,
/// under it the one command that activates it, `conductor setup --apply` for the checkout it
/// asked about, which needs no profile the person has (every v1.0.0 onboarding run asked for
/// one), and is `not ready`; it never activates anything itself.
#[test]
fn doctor_names_a_checkouts_root_not_in_an_activated_worktree_workspace() {
    let case = Case::new("worktree-not-activated");
    case.programs_but(&[]);
    case.worktree(false);
    let root = case.checkouts();
    let config = case.config("[{github: example-org}]");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    let row = format!("worktree workspace: {} not activated", root.display());
    let activate = format!(
        "  activate: conductor setup --apply --repository {}",
        root.join("alpha").display()
    );
    assert!(
        !lines.iter().any(|line| line.contains("<profile>")),
        "no line asks for a profile the person does not have: {}",
        shown(&output)
    );
    assert_eq!(
        under(&lines, &row),
        Some(activate.as_str()),
        "{}",
        shown(&output)
    );
    assert_eq!(
        last(&output),
        format!(
            "not ready: worktree workspace (not activated: {})",
            root.display()
        ),
        "{}",
        shown(&output)
    );
    let calls = fs::read_to_string(case.worktree_calls()).unwrap_or_default();
    let asked = format!("--repo {}", root.join("alpha").display());
    assert!(
        calls
            .lines()
            .any(|call| call.starts_with("inspect ") && call.contains(&asked)),
        "doctor asks worktree about a checkout under the root:\n{calls}"
    );
    assert!(
        !calls.lines().any(|call| call.starts_with("activate")),
        "doctor never activates:\n{calls}"
    );
}

/// A checkouts root in an activated workspace: doctor prints it `activated`, no activation
/// command, and `ready`.
#[test]
fn doctor_names_a_checkouts_root_in_an_activated_worktree_workspace_ready() {
    let case = Case::new("worktree-activated");
    case.programs_but(&[]);
    case.worktree(true);
    let root = case.checkouts();
    let config = case.config("[{github: example-org}]");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&format!("worktree workspace: {} activated", root.display())),
        "{}",
        shown(&output)
    );
    assert!(
        !lines.iter().any(|line| line.starts_with("  activate: ")),
        "{}",
        shown(&output)
    );
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

/// `story:spec-v1`: the confinement providers' rows went with `confinement.*` (design
/// architecture-next § 11, `conductor.config.DoctorReport`): doctor prints no provider row, even
/// with a provider that is not available, and its verdict stays last.
#[test]
fn doctor_prints_no_provider_row() {
    let case = Case::new("providers");
    case.programs_but(&[]);
    let fake = case.dir.join("fake");
    fs::create_dir_all(&fake).expect("create the fake's directory");
    fs::write(fake.join("systemd.unavailable"), "no cgroup delegation\n").expect("systemd is not");
    let output = fake_provider::without_harness(
        fake_provider::command_in_clear_env(&case.home(), &fake, &["doctor"])
            .current_dir(case.work())
            .env("PATH", case.bin())
            .output()
            .expect("the child runs"),
    );
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        !lines.iter().any(|line| line.starts_with("provider ")),
        "{}",
        shown(&output)
    );
    for (program, _) in PROGRAMS {
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with(&format!("{program}: "))),
            "no row for {program}: {}",
            shown(&output)
        );
    }
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

// ---------------------------------------------------------------------------------------------
// story:doctor-checks-guard-forge-writes: the route each instance's forge writes take, and the
// guard's answer to one forge write of each role it decides.
// ---------------------------------------------------------------------------------------------

/// The sources of the cases below, and the `origin` of the checkout `alpha` on each.
const GITHUB_SOURCE: &str = "[{github: example-org}]";
const GITHUB_ORIGIN: &str = "https://github.com/example-org/alpha.git";
const GITLAB_SOURCE: &str = "[{gitlab: example-group, host: gitlab.example.com}]";
const GITLAB_ORIGIN: &str = "https://gitlab.example.com/example-group/alpha.git";

impl Case {
    /// A git checkout at `dir` whose `origin` is `url`: the `.git/config` the guard reads it
    /// from. No git runs.
    fn origin(&self, dir: &Path, url: &str) {
        fs::create_dir_all(dir.join(".git")).expect("create the checkout");
        fs::write(
            dir.join(".git/config"),
            format!("[core]\n\tbare = false\n[remote \"origin\"]\n\turl = {url}\n"),
        )
        .expect("write the checkout's git config");
    }

    /// The checkout `alpha` under the checkouts root, its `origin` `url`.
    fn checkout(&self, url: &str) -> PathBuf {
        let dir = self.checkouts().join("alpha");
        self.origin(&dir, url);
        dir
    }

    /// The guard's PreToolUse hook under `config`, asked about the Bash `command` of a session
    /// whose working directory is `cwd`.
    fn hook(&self, config: &Path, cwd: &Path, command: &str) -> Output {
        let payload = serde_json::json!({
            "session_id": "example-session",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "cwd": cwd,
            "tool_input": {"command": command},
        });
        let mut child = common::conductor(self.home())
            .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
            .current_dir(self.work())
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the hook starts");
        child
            .stdin
            .take()
            .expect("the hook's standard input")
            .write_all(payload.to_string().as_bytes())
            .expect("write the payload");
        child.wait_with_output().expect("the hook ends")
    }
}

/// Every file and directory under `dir`, sorted: a run that writes nothing leaves it as it was.
fn tree(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut left = vec![dir.to_path_buf()];
    while let Some(dir) = left.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() && !path.is_symlink() {
                left.push(path.clone());
            }
            found.push(path);
        }
    }
    found.sort();
    found
}

/// The one line of `lines` that starts with `start`.
fn the_line<'l>(lines: &'l [String], start: &str, output: &Output) -> &'l str {
    let found: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with(start))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "one line starts with {start:?}: {}",
        shown(output)
    );
    found[0]
}

/// A config whose guard denies the controller's forge writes: a GitLab instance on the default
/// route, `gates`, where the guard lets `glab` only read and the bot route carries nothing to
/// the host (the report of 2026-11-07). Doctor asks the guard about a merge request create in
/// the controller's own checkout, prints one line for the role with the guard's reason, ends
/// `not ready` naming the role, and records nothing.
#[test]
fn doctor_is_not_ready_when_the_guard_denies_the_controller_s_forge_write() {
    let case = Case::new("guard-denied");
    case.programs_but(&["gh"]);
    case.program(GLAB.0, GLAB.1, 0);
    case.checkout(GITLAB_ORIGIN);
    let config = case.config(GITLAB_SOURCE);
    let before = tree(&case.dir);
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    let line = the_line(&lines, "guard controller: ", &output);
    assert!(
        line.starts_with("guard controller: denied by `glab mr create` is a GitLab write")
            && line.ends_with("writes go through the bot route; checkouts denied: alpha"),
        "the line carries the guard's reason: {}",
        shown(&output)
    );
    assert_eq!(
        last(&output),
        "not ready: forge route alpha (no bot route for gitlab.example.com), guard controller \
         (denied)",
        "{}",
        shown(&output)
    );
    assert_eq!(
        tree(&case.dir),
        before,
        "doctor records no denial and writes no file: {}",
        shown(&output)
    );
}

/// Each role the guard decides gets one line: `allowed` for a pull request create in its own
/// repository, the controller's in its checkout and conductor's in its records, on the direct
/// route (`gh pr create -R <origin>`) and on the bot route alike. Doctor is ready and writes
/// nothing.
#[test]
fn doctor_prints_allowed_for_each_role_whose_forge_write_the_guard_allows() {
    let case = Case::new("guard-allowed");
    case.programs_but(&[]);
    case.checkout(GITHUB_ORIGIN);
    case.origin(
        &case.dir.join("records"),
        "https://github.com/example-org/records.git",
    );
    let policy = case.policy();
    for direct in [true, false] {
        // Both configs are one file: each is written when its turn comes.
        let (config, route) = if direct {
            (
                case.config_of(GITHUB_SOURCE, DIRECT),
                "forge route alpha: direct".to_owned(),
            )
        } else {
            (
                case.config(GITHUB_SOURCE),
                format!("forge route alpha: gates, policy {}", policy.display()),
            )
        };
        let before = tree(&case.dir);
        let output = case.doctor_for(&config);
        assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
        let lines = stdout(&output);
        for line in [
            route.as_str(),
            "guard conductor: allowed",
            "guard controller: allowed",
        ] {
            assert!(
                lines.contains(&line.to_owned()),
                "doctor prints {line:?}: {}",
                shown(&output)
            );
        }
        assert_eq!(last(&output), "ready", "{}", shown(&output));
        assert_eq!(
            tree(&case.dir),
            before,
            "doctor writes no file: {}",
            shown(&output)
        );
    }
}

/// A role with no repository on a forge has no forge write to ask the guard about: no checkout
/// under the checkouts root, a checkout whose git config names no origin, records that are no
/// checkout. Doctor says so on the role's line and stays ready.
#[test]
fn doctor_asks_the_guard_nothing_for_a_role_without_a_repository_on_a_forge() {
    let case = Case::new("guard-not-checked");
    case.programs_but(&[]);
    let root = case.dir.join("checkouts");
    let records = case.dir.join("records");
    let config = case.config_of(GITHUB_SOURCE, DIRECT);

    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let lines = stdout(&output);
    for line in [
        format!(
            "guard conductor: not checked (the origin of {} names no repository)",
            records.display()
        ),
        format!(
            "guard controller: not checked (no git checkout under {})",
            root.display()
        ),
    ] {
        assert!(
            lines.contains(&line),
            "doctor prints {line:?}: {}",
            shown(&output)
        );
    }
    assert_eq!(last(&output), "ready", "{}", shown(&output));

    case.checkouts();
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let line = format!(
        "guard controller: not checked (no checkout under {} has an origin that names a \
         repository)",
        root.display()
    );
    assert!(
        stdout(&output).contains(&line),
        "doctor prints {line:?}: {}",
        shown(&output)
    );
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

/// Doctor decides with the function the hook decides with: to the sample call doctor builds for
/// the controller, the PreToolUse hook answers the same verdict, and a denial with the reason
/// doctor printed.
#[test]
fn doctor_and_the_hook_answer_the_sample_forge_write_alike() {
    let case = Case::new("guard-as-the-hook");
    case.programs_but(&["gh"]);
    case.program(GLAB.0, GLAB.1, 0);
    let checkout = case.checkout(GITLAB_ORIGIN);
    let sample = "glab mr create -R example-group/alpha --fill";

    let config = case.config(GITLAB_SOURCE);
    let output = case.doctor_for(&config);
    let lines = stdout(&output);
    let line = the_line(&lines, "guard controller: ", &output);
    let (reason, denied) = line
        .strip_prefix("guard controller: denied by ")
        .and_then(|rest| rest.rsplit_once("; checkouts denied: "))
        .unwrap_or_else(|| panic!("the guard denies the sample: {}", shown(&output)));
    assert_eq!(denied, "alpha", "{}", shown(&output));
    let hook = case.hook(&config, &checkout, sample);
    assert_eq!(hook.status.code(), Some(2), "{}", shown(&hook));
    assert!(
        String::from_utf8_lossy(&hook.stderr)
            .lines()
            .any(|answered| answered == format!("conductor guard: denied: {reason}")),
        "the hook denies it with the reason doctor printed, {reason:?}: {}",
        shown(&hook)
    );

    let config = case.config_of(GITLAB_SOURCE, DIRECT);
    let output = case.doctor_for(&config);
    assert!(
        stdout(&output).contains(&"guard controller: allowed".to_owned()),
        "{}",
        shown(&output)
    );
    let hook = case.hook(&config, &checkout, sample);
    assert_eq!(hook.status.code(), Some(0), "{}", shown(&hook));
}

/// With several instances in the file, each role's line names its instance.
#[test]
fn doctor_names_the_instance_of_each_role_when_the_file_has_several() {
    let case = Case::new("guard-instances");
    case.programs_but(&[]);
    case.program(GLAB.0, GLAB.1, 0);
    case.checkout(GITHUB_ORIGIN);
    let other = case.dir.join("beta");
    case.origin(&other.join("checkouts/widget"), GITLAB_ORIGIN);
    let config = case.config_of(GITHUB_SOURCE, DIRECT);
    let text = fs::read_to_string(&config).expect("the config file");
    fs::write(
        &config,
        format!(
            "{text}\
             \x20 - name: beta\n\
             \x20   session_prefix: beta\n\
             \x20   sources: {GITLAB_SOURCE}\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n\
             \x20   gates_policy: {policy}\n",
            root = other.join("checkouts").display(),
            trees = other.join("trees").display(),
            records = other.join("records").display(),
            policy = case.policy().display(),
        ),
    )
    .expect("write the config file");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    assert!(
        lines.contains(&"guard controller of alpha: allowed".to_owned()),
        "{}",
        shown(&output)
    );
    assert!(
        the_line(&lines, "guard controller of beta: ", &output)
            .starts_with("guard controller of beta: denied by "),
        "{}",
        shown(&output)
    );
    assert_eq!(
        last(&output),
        "not ready: forge route beta (no bot route for gitlab.example.com), guard controller of \
         beta (denied)",
        "{}",
        shown(&output)
    );
}

/// An instance on the default route, `gates`, that names no `gates_policy`: the bot route has no
/// policy to write under. Doctor names the two fixes on its line, a `gates_policy` or
/// `forge.route: direct`, by their YAML path, and is not ready.
#[test]
fn doctor_is_not_ready_on_the_gates_route_without_a_gates_policy() {
    let case = Case::new("route-no-policy");
    case.programs_but(&[]);
    let config = case.config_of(GITHUB_SOURCE, "");
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    let line = the_line(&lines, "forge route alpha: ", &output);
    for fix in ["instances[0].gates_policy", "forge.route: direct"] {
        assert!(
            line.contains(fix),
            "the line names the fix {fix:?}: {}",
            shown(&output)
        );
    }
    assert_eq!(
        last(&output),
        "not ready: forge route alpha (no gates_policy)",
        "{}",
        shown(&output)
    );
}

/// An instance on the route `gates` with a source on a host the bot route has no route for, a
/// GitLab host: doctor names the host and the two fixes on its line, `forge.route: direct` or
/// the bot's route for that host, and is not ready. On the direct route the same instance is
/// ready.
#[test]
fn doctor_is_not_ready_when_the_bot_route_has_no_route_for_a_source_s_host() {
    let case = Case::new("route-no-host");
    case.programs_but(&["gh"]);
    case.program(GLAB.0, GLAB.1, 0);
    let config = case.config(GITLAB_SOURCE);
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    let line = the_line(&lines, "forge route alpha: ", &output);
    for fix in [
        "instances[0].forge.route: direct",
        "a route for gitlab.example.com",
    ] {
        assert!(
            line.contains(fix),
            "the line names the fix {fix:?}: {}",
            shown(&output)
        );
    }
    assert_eq!(
        last(&output),
        "not ready: forge route alpha (no bot route for gitlab.example.com)",
        "{}",
        shown(&output)
    );

    let config = case.config_of(GITLAB_SOURCE, DIRECT);
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        stdout(&output).contains(&"forge route alpha: direct".to_owned()),
        "{}",
        shown(&output)
    );
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

/// The variable that names the Gates policy to `conductor broker serve`.
const POLICY_VARIABLE: &str = "B10X_GATES_POLICY";

/// A policy only in the environment stays `not ready`: doctor cannot see the broker's
/// environment, and the instance names no policy. With `B10X_GATES_POLICY` naming a readable file
/// in doctor's own environment, the fix on the route line names that file as the `gates_policy`
/// to write, as an absolute path `config validate` accepts: a relative value is joined with
/// doctor's working directory, as `conductor setup` joins it. A value that names no readable
/// file is no path to write, and the fix names none.
#[test]
fn doctor_names_the_policy_of_its_environment_as_the_gates_policy_to_write() {
    let case = Case::new("route-policy-in-environment");
    case.programs_but(&[]);
    let policy = case.policy();
    fs::write(case.work().join("gates-policy.json"), "{}\n").expect("write the policy");
    let config = case.config_of(GITHUB_SOURCE, "");
    let doctor = |value: &std::ffi::OsStr| {
        common::conductor(case.home())
            .arg("doctor")
            .current_dir(case.work())
            .env_clear()
            .env("HOME", case.home())
            .env("PATH", case.bin())
            .env("CONDUCTOR_CONFIG", &config)
            .env(POLICY_VARIABLE, value)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    };
    let missing = case.dir.join("no-such-policy.json");
    for (value, named) in [
        (policy.clone().into_os_string(), Some(policy.clone())),
        (
            "gates-policy.json".into(),
            Some(case.work().join("gates-policy.json")),
        ),
        (missing.into_os_string(), None),
        (case.dir.clone().into_os_string(), None),
    ] {
        let output = doctor(value.as_os_str());
        assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
        let lines = stdout(&output);
        let fix = named.map_or_else(String::new, |named| {
            format!(": {}, the path {POLICY_VARIABLE} names", named.display())
        });
        assert_eq!(
            the_line(&lines, "forge route alpha: ", &output),
            format!(
                "forge route alpha: gates without a gates_policy: set \
                 instances[0].gates_policy{fix}, or instances[0].forge.route: direct"
            ),
            "{value:?}: {}",
            shown(&output)
        );
        assert_eq!(
            last(&output),
            "not ready: forge route alpha (no gates_policy)",
            "{}",
            shown(&output)
        );
    }
}

/// A `gates_policy` that names no readable file (none there, a directory, a file this process
/// cannot read) is no policy the bot route writes under: doctor names the path and the fixes on
/// the route line and is not ready. A readable one is ready, and doctor prints none of its
/// content.
#[test]
fn doctor_is_not_ready_on_a_gates_policy_that_is_no_readable_file() {
    let case = Case::new("route-policy-unreadable");
    case.programs_but(&[]);
    let missing = case.dir.join("no-such-policy.json");
    let directory = case.dir.join("policies");
    fs::create_dir_all(&directory).expect("create the directory");
    let unreadable = case.dir.join("unreadable-policy.json");
    fs::write(&unreadable, "{}\n").expect("write the policy");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).expect("take its mode");
    // A process that reads every file anyway (root) cannot see the third case.
    let reads_anyway = fs::File::open(&unreadable).is_ok();
    for policy in [&missing, &directory, &unreadable] {
        if *policy == unreadable && reads_anyway {
            continue;
        }
        let config = case.config_of(
            GITHUB_SOURCE,
            &format!("\x20   gates_policy: {}\n", policy.display()),
        );
        let output = case.doctor_for(&config);
        assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
        let lines = stdout(&output);
        assert_eq!(
            the_line(&lines, "forge route alpha: ", &output),
            format!(
                "forge route alpha: gates, and gates_policy {} is not a readable file: write the \
                 policy there, set instances[0].gates_policy to a readable policy file, or \
                 instances[0].forge.route: direct",
                policy.display()
            ),
            "{}",
            shown(&output)
        );
        assert_eq!(
            last(&output),
            format!(
                "not ready: forge route alpha (gates_policy {} is not a readable file)",
                policy.display()
            ),
            "{}",
            shown(&output)
        );
    }

    let policy = case.dir.join("gates-policy.json");
    fs::write(&policy, "{\"marker\": \"example-policy-content\"}\n").expect("write the policy");
    let config = case.config_of(
        GITHUB_SOURCE,
        &format!("\x20   gates_policy: {}\n", policy.display()),
    );
    let output = case.doctor_for(&config);
    assert_eq!(last(&output), "ready", "{}", shown(&output));
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("example-policy-content"),
        "doctor prints no content of the policy: {}",
        shown(&output)
    );
}

/// The checkouts of the lead's cases below: `alpha` on GitHub, the rest on a GitLab host.
const LEAD_CHECKOUTS: [&str; 6] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];

/// Doctor asks the guard about every checkout of the lead, not the first: a `local` source over
/// a directory of checkouts on the route `gates`, one on GitHub and five on a GitLab host. The
/// hook denies the controller of each GitLab checkout its merge request, so the role's line
/// reads `denied by` the guard's reason and names the checkouts denied, three and how many more,
/// and the verdict names the role once. The route check reads the checkouts' origins too: a
/// GitLab origin under a `local` source is the same reason, with the same fix, as a `gitlab`
/// source on `gates`. With the GitLab checkouts gone the line reads `allowed`.
#[test]
fn doctor_asks_the_guard_about_every_checkout_of_the_lead() {
    let case = Case::new("guard-every-checkout");
    case.programs_but(&[]);
    case.program(GLAB.0, GLAB.1, 0);
    let root = case.checkouts();
    for name in LEAD_CHECKOUTS {
        let url = if name == "alpha" {
            GITHUB_ORIGIN.to_owned()
        } else {
            format!("https://gitlab.example.com/example-group/{name}.git")
        };
        case.origin(&root.join(name), &url);
    }
    let config = case.config(&format!("[{{local: {}}}]", root.display()));
    let output = case.doctor_for(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let lines = stdout(&output);
    let line = the_line(&lines, "guard controller: ", &output);
    let (reason, denied) = line
        .strip_prefix("guard controller: denied by ")
        .and_then(|rest| rest.rsplit_once("; checkouts denied: "))
        .unwrap_or_else(|| panic!("the guard denies the lead: {}", shown(&output)));
    assert_eq!(
        denied,
        "beta, delta, epsilon and 2 more",
        "{}",
        shown(&output)
    );
    let hook = case.hook(
        &config,
        &root.join("zeta"),
        "glab mr create -R example-group/zeta --fill",
    );
    assert_eq!(hook.status.code(), Some(2), "{}", shown(&hook));
    assert!(
        String::from_utf8_lossy(&hook.stderr)
            .lines()
            .any(|answered| answered == format!("conductor guard: denied: {reason}")),
        "the hook denies the fifth GitLab checkout with the reason doctor printed: {}",
        shown(&hook)
    );
    let route = the_line(&lines, "forge route alpha: ", &output).to_owned();
    assert_eq!(
        last(&output),
        "not ready: forge route alpha (no bot route for gitlab.example.com), guard controller \
         (denied)",
        "{}",
        shown(&output)
    );
    // The same reason and fix as for a `gitlab` source on the same host. Both files are one.
    let gitlab = case.config(GITLAB_SOURCE);
    let output = case.doctor_for(&gitlab);
    assert_eq!(
        the_line(&stdout(&output), "forge route alpha: ", &output),
        route,
        "{}",
        shown(&output)
    );

    let config = case.config(&format!("[{{local: {}}}]", root.display()));
    for name in &LEAD_CHECKOUTS[3..] {
        fs::remove_dir_all(root.join(name)).expect("remove a checkout");
    }
    let output = case.doctor_for(&config);
    let lines = stdout(&output);
    assert!(
        the_line(&lines, "guard controller: ", &output)
            .ends_with("; checkouts denied: beta, gamma"),
        "{}",
        shown(&output)
    );
    for name in &LEAD_CHECKOUTS[1..3] {
        fs::remove_dir_all(root.join(name)).expect("remove a checkout");
    }
    let output = case.doctor_for(&config);
    assert_eq!(
        the_line(&stdout(&output), "guard controller: ", &output),
        "guard controller: allowed",
        "{}",
        shown(&output)
    );
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

/// `not checked` is only for a role with no repository on a forge. A role whose repository has
/// an `origin` that names one is always asked: on either route, with or without a
/// `gates_policy`, for an origin on GitHub, on a GitLab host or on any other host, written as a
/// URL or as `user@host:path`, its line reads `allowed` or `denied by`, never `not checked`.
#[test]
fn doctor_asks_the_guard_for_every_role_whose_repository_has_an_origin() {
    let case = Case::new("guard-always-asked");
    case.programs_but(&[]);
    case.program(GLAB.0, GLAB.1, 0);
    for (origin, records) in [
        (GITHUB_ORIGIN, "https://github.com/example-org/records.git"),
        (
            "git@github.com:example-org/alpha.git",
            "git@github.com:example-org/records.git",
        ),
        (
            GITLAB_ORIGIN,
            "https://gitlab.example.com/example-group/records.git",
        ),
        (
            "https://git.example.net/example-org/alpha.git",
            "ssh://git@git.example.net/example-org/records.git",
        ),
    ] {
        case.checkout(origin);
        case.origin(&case.dir.join("records"), records);
        for route in ["direct", "gates with a policy", "gates without one"] {
            let config = match route {
                "direct" => case.config_of(GITHUB_SOURCE, DIRECT),
                "gates with a policy" => case.config(GITHUB_SOURCE),
                _ => case.config_of(GITHUB_SOURCE, ""),
            };
            let output = case.doctor_for(&config);
            let lines = stdout(&output);
            for role in ["conductor", "controller"] {
                let start = format!("guard {role}: ");
                let line = the_line(&lines, &start, &output);
                assert!(
                    line == format!("{start}allowed")
                        || line.starts_with(&format!("{start}denied by ")),
                    "{origin} on {route}: the guard is asked for the {role}: {}",
                    shown(&output)
                );
            }
        }
    }
}

/// Each fenced block of `text` that is a whole `conductor.config/2` file, its indentation taken
/// off: not a console block, whose first line is a `$ ` prompt (`conductor config migrate`'s
/// output names the version and a placement too).
fn config_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, Vec<String>)> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        if line.trim_start().starts_with("```") {
            match open.take() {
                Some((_, lines)) => blocks.push(lines.join("\n") + "\n"),
                None => open = Some((indent, Vec::new())),
            }
        } else if let Some((indent, lines)) = &mut open {
            lines.push(line.get(*indent..).unwrap_or("").to_owned());
        }
    }
    // A block that starts with a `$ ` prompt is a command and what it printed, no file.
    blocks
        .into_iter()
        .filter(|block| {
            block.contains("version: conductor.config/2")
                && block.contains("placement:")
                && !block.trim_start().starts_with("$ ")
        })
        .collect()
}

/// Every whole config file a reader is meant to run (the README's, the example file and those of
/// the hand-written pages) names a forge route doctor is ready on, for each of its instances:
/// `forge.route: direct`, or the route `gates` with a `gates_policy` and no `gitlab` source, so
/// its verdict names no forge route. It is read here by the rule, not by running doctor: the
/// pages' files name a store, a secrets service and boxes a test must not reach. `SETUP.md` is
/// not among them: the file of its step 6 is the one `conductor setup --apply` writes, whose
/// route setup derives from the repository's origin (`setup.rs`), and its values are words in
/// angle brackets. A block that does not parse fails the case, unless [`UNPARSED`] names its page
/// and why it does not parse.
#[test]
fn every_config_file_a_page_shows_names_a_forge_route_doctor_is_ready_on() {
    let home = Path::new("/srv/home");
    let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut pages = vec![top.join("README.md")];
    let mut walk = vec![top.join("website/docs")];
    while let Some(folder) = walk.pop() {
        for entry in fs::read_dir(&folder).expect("read the pages") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "reference") {
                    walk.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "md") {
                pages.push(path);
            }
        }
    }
    pages.sort();
    let mut texts: Vec<(String, String)> = vec![(
        "conductor.example.yaml".to_owned(),
        fs::read_to_string(top.join("conductor.example.yaml")).expect("the example file"),
    )];
    for page in pages {
        let text = fs::read_to_string(&page).expect("the page");
        let name = page
            .strip_prefix(&top)
            .unwrap_or(&page)
            .display()
            .to_string();
        texts.extend(
            config_blocks(&text)
                .into_iter()
                .map(|block| (name.clone(), block)),
        );
    }
    let mut read = 0;
    let mut problems: Vec<String> = Vec::new();
    let mut unparsed: Vec<(&str, &str)> = Vec::new();
    for (page, text) in &texts {
        let parsed = match config::parse(text, home) {
            Ok(parsed) => parsed,
            Err(refused) => {
                let said = format!("{refused:?}");
                match UNPARSED
                    .iter()
                    .find(|(listed, why)| listed == page && said.contains(why))
                {
                    Some(listed) => unparsed.push(*listed),
                    None => problems.push(format!(
                        "{page}: a whole config file that does not parse, and UNPARSED does not \
                         name it: {said}\n{text}"
                    )),
                }
                continue;
            }
        };
        read += 1;
        for instance in &parsed.instances {
            let name = &instance.name.0;
            if instance.forge.route == ForgeRoute::Direct {
                continue;
            }
            if instance.gates_policy.is_none() {
                problems.push(format!(
                    "{page}: instance {name} is on the route gates without a gates_policy: \
                     doctor ends not ready: forge route {name} (no gates_policy)"
                ));
            }
            for source in &instance.sources {
                if let Source::GitLab(gitlab) = source {
                    problems.push(format!(
                        "{page}: instance {name} is on the route gates with a gitlab source: \
                         doctor ends not ready: forge route {name} (no bot route for {})",
                        gitlab.host
                    ));
                }
            }
        }
    }
    for listed in UNPARSED {
        let found = unparsed.iter().filter(|one| **one == listed).count();
        if found != 1 {
            problems.push(format!(
                "UNPARSED names {listed:?}, and {found} blocks of the page match it, not one"
            ));
        }
    }
    assert!(
        read >= 6,
        "only {read} whole config files of the pages read"
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The whole config files of the pages that do not parse on purpose, each by its page and the
/// problem it shows: the one block of `configure-an-instance.md` written to show
/// `config validate`'s answer to a broken file.
const UNPARSED: [(&str, &str); 1] = [(
    "website/docs/guides/configure-an-instance.md",
    "instances[0].cadence.cycle",
)];
