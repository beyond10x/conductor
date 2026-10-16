//! Adversary cases for wave 17 unit A1 (`story:gitlab-source-host`, `story:gh-optional`,
//! `story:doctor-verdict`, `story:doctor-minimum-versions`).
//!
//! Each case runs the built binary with the environment cleared, as `tests/doctor.rs` does:
//! `HOME` names the case's `home/`, the working directory is under it, and `PATH` holds only the
//! fake programs the case writes.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

/// Each program conductor starts but `glab`, with the version line its fake prints (as
/// `tests/doctor.rs`).
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

const GLAB: (&str, &str) = ("glab", "glab 1.46.0 (2024-09-01)");

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
            .join("adv_w17_doctor")
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

    fn program(&self, name: &str, line: &str, code: i32) {
        let file = self.bin().join(name);
        fs::write(
            &file,
            format!("#!/bin/sh\nprintf '%s\\n' '{line}'\nexit {code}\n"),
        )
        .expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    fn programs_but(&self, except: &[&str]) {
        for (name, line) in PROGRAMS {
            if !except.contains(&name) {
                self.program(name, line, 0);
            }
        }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let file = self.dir.join(name);
        fs::write(&file, text).expect("write a case file");
        file
    }

    /// One instance's lines of a config file: `name`, `sources` (a YAML flow sequence) and
    /// `extra` lines, its directories under the case's own.
    fn instance(&self, name: &str, sources: &str, extra: &str) -> String {
        format!(
            "\x20 - name: {name}\n\
             \x20   sources: {sources}\n\
             {extra}\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n",
            root = self.dir.join(format!("checkouts-{name}")).display(),
            trees = self.dir.join(format!("trees-{name}")).display(),
            records = self.dir.join(format!("records-{name}")).display(),
        )
    }

    /// `conductor <args>` in `cwd`, with `env` set.
    fn run(&self, cwd: &Path, args: &[&str], env: &[(&str, &Path)]) -> Output {
        let mut command = common::conductor(self.home());
        command
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .stdin(Stdio::null());
        for (name, value) in env {
            command.env(name, value);
        }
        command.output().expect("conductor runs")
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

fn last(output: &Output) -> String {
    stdout(output).pop().unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// story:gitlab-source-host: a host is a host name.
// ---------------------------------------------------------------------------------------------

/// `conductor --config <file> config validate` for a one-instance file whose only source is
/// `{source}`.
fn validate_source(case: &Case, name: &str, source: &str) -> Output {
    let file = case.write(
        &format!("{name}.yaml"),
        &format!(
            "version: conductor.config/1\ninstances:\n{}",
            case.instance("one", &format!("[{{{source}}}]"), "")
        ),
    );
    let file = file.to_string_lossy().into_owned();
    case.run(
        &case.work(),
        &["--config", &file, "config", "validate"],
        &[],
    )
}

/// A host with an empty label (`gitlab..example.com`) is no host name: every label of a DNS name
/// holds at least one character. `Reader::host` checks only the first and last character of the
/// whole name and the character set, so it accepts it.
#[test]
fn adv_w17_a_host_with_an_empty_label_is_refused() {
    let case = Case::new("host-empty-label");
    let output = validate_source(
        &case,
        "empty-label",
        "gitlab: example-group, host: gitlab..example.com",
    );
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("instances[0].sources[0].host: "),
        "{}",
        shown(&output)
    );
}

/// A port is digits: `u16::from_str` takes a leading `+`, so `Reader::host` accepts
/// `gitlab.example.com:+443` and `config show` echoes the `+`.
#[test]
fn adv_w17_a_host_whose_port_carries_a_sign_is_refused() {
    let case = Case::new("host-port-sign");
    let output = validate_source(
        &case,
        "port-sign",
        "gitlab: example-group, host: \"gitlab.example.com:+443\"",
    );
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("instances[0].sources[0].host: "),
        "{}",
        shown(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:gh-optional: which instance doctor reads, and `--config`.
// ---------------------------------------------------------------------------------------------

/// The global `--config` names the instance doctor reads, before `CONDUCTOR_CONFIG`, on either
/// side of the subcommand. Green on the commit; it catches `Group::Doctor => doctor_with(None)`,
/// which no case of `tests/doctor.rs` does (each names the file by the variable only).
#[test]
fn adv_w17_the_config_flag_names_doctor_s_instance_before_the_variable() {
    let case = Case::new("config-flag");
    case.programs_but(&["gh"]);
    case.program(GLAB.0, GLAB.1, 0);
    let gitlab = case.write(
        "gitlab.yaml",
        &format!(
            "version: conductor.config/1\ninstances:\n{}",
            case.instance("alpha", "[{gitlab: example-group}]", "")
        ),
    );
    let github = case.write(
        "github.yaml",
        &format!(
            "version: conductor.config/1\ninstances:\n{}",
            case.instance("alpha", "[{github: example-org}]", "")
        ),
    );
    let flag = gitlab.to_string_lossy().into_owned();
    for args in [
        ["--config", flag.as_str(), "doctor"],
        ["doctor", "--config", flag.as_str()],
    ] {
        let output = case.run(
            &case.work(),
            &args,
            &[("CONDUCTOR_CONFIG", github.as_path())],
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            shown(&output)
        );
        assert!(
            stdout(&output).contains(&"gh: not needed".to_owned()),
            "{args:?}: {}",
            shown(&output)
        );
        assert_eq!(last(&output), "ready", "{args:?}: {}", shown(&output));
    }
}

/// A valid file of two gitlab-only instances and no `default` (`config validate` passes it,
/// `tests/config.rs` `two_instances_and_no_instance_is_an_error_naming_both`) starts no `gh`
/// whichever instance runs. `needed_forges` goes through `config::resolve`, which refuses to
/// choose an instance, and so counts `gh` as needed: doctor reports `gh: missing` and exits 1 for
/// a team with no GitHub repository, the case `story:gh-optional` exists for.
#[test]
fn adv_w17_doctor_needs_no_gh_for_two_gitlab_instances_without_a_default() {
    let case = Case::new("two-gitlab-no-default");
    case.programs_but(&["gh"]);
    case.program(GLAB.0, GLAB.1, 0);
    let config = case.write(
        "conductor.yaml",
        &format!(
            "version: conductor.config/1\ninstances:\n{}{}",
            case.instance(
                "alpha",
                "[{gitlab: group-a}]",
                "\x20   session_prefix: alpha\n"
            ),
            case.instance(
                "beta",
                "[{gitlab: group-b, host: gitlab.example.com}]",
                "\x20   session_prefix: beta\n"
            ),
        ),
    );
    let flag = config.to_string_lossy().into_owned();
    let validated = case.run(
        &case.work(),
        &["--config", &flag, "config", "validate"],
        &[],
    );
    assert_eq!(
        validated.status.code(),
        Some(0),
        "the file is valid: {}",
        shown(&validated)
    );

    let output = case.run(
        &case.work(),
        &["doctor"],
        &[("CONDUCTOR_CONFIG", config.as_path())],
    );
    let lines = stdout(&output);
    assert!(
        lines.contains(&"gh: not needed".to_owned()),
        "{}",
        shown(&output)
    );
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

/// The default instance's conductor serves every instance of the file, and a snapshot of the
/// gitlab instance `beta` starts `glab` (`collect::programs`). Doctor reads only the selected
/// instance, the github-only default, so with no `glab` on `PATH` it prints `glab: not needed`
/// and `ready`, and the first `CONDUCTOR_INSTANCE=beta conductor snapshot start-snapshot` is
/// refused for the program doctor said was not needed.
#[test]
fn adv_w17_doctor_needs_glab_when_any_instance_of_the_file_has_a_gitlab_source() {
    let case = Case::new("default-github-other-gitlab");
    case.programs_but(&[]);
    let config = case.write(
        "conductor.yaml",
        &format!(
            "version: conductor.config/1\ndefault: alpha\ninstances:\n{}{}",
            case.instance("alpha", "[{github: example-org}]", ""),
            case.instance(
                "beta",
                "[{gitlab: example-group}]",
                "\x20   session_prefix: beta\n"
            ),
        ),
    );
    let output = case.run(
        &case.work(),
        &["doctor"],
        &[("CONDUCTOR_CONFIG", config.as_path())],
    );
    assert!(
        stdout(&output).contains(&"glab: missing".to_owned()),
        "{}",
        shown(&output)
    );
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert_eq!(
        last(&output),
        "not ready: glab (missing)",
        "{}",
        shown(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:doctor-verdict: a config that does not load.
// ---------------------------------------------------------------------------------------------

/// Every command but `config`, `trust`, `init` and `doctor` stops on a config file that does not
/// load (`Cli::run`, `set_active(..)?`). Doctor prints one `config: …` line, counts both forge
/// CLIs as needed, and with every program there ends `ready` and exits 0: the verdict says ready
/// for a conductor that runs no command.
#[test]
fn adv_w17_doctor_is_not_ready_when_the_config_does_not_load() {
    let case = Case::new("config-invalid");
    case.programs_but(&[]);
    case.program(GLAB.0, GLAB.1, 0);
    let config = case.write(
        "conductor.yaml",
        &format!(
            "version: conductor.config/1\ninstances:\n{}",
            case.instance(
                "alpha",
                "[{gitlab: example-group, host: \"https://x\"}]",
                ""
            )
        ),
    );
    let output = case.run(
        &case.work(),
        &["doctor"],
        &[("CONDUCTOR_CONFIG", config.as_path())],
    );
    assert!(
        stdout(&output)
            .iter()
            .any(|line| line.starts_with("config: ")),
        "doctor names the config problem: {}",
        shown(&output)
    );
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert_ne!(last(&output), "ready", "{}", shown(&output));
}

// ---------------------------------------------------------------------------------------------
// story:doctor-minimum-versions: a b10x.toml that does not pin aep.
// ---------------------------------------------------------------------------------------------

/// The build's pins are conductor's minimum versions ("refuses an aep, ess or worktree older than
/// its build's pins"). A product repository's `b10x.toml` that pins only `worktree` replaces them
/// all (`doctor_with`: `Some(file) => pins(..)`, `None => build_pins()`), so `aep 0.63.1`, the
/// story's own example of an aep that cannot open a current planning store, passes as `ready`
/// whenever doctor runs inside such a repository.
#[test]
fn adv_w17_a_pin_file_without_an_aep_pin_keeps_the_build_s_aep_minimum() {
    let pin = repository_pins()
        .into_iter()
        .find(|(program, _)| program == "aep")
        .map(|(_, pin)| pin)
        .expect("the repository's b10x.toml pins aep");
    let case = Case::new("pin-file-without-aep");
    case.programs_but(&[]);
    case.program("aep", "aep 0.63.1", 0);
    let repository = case.home().join("repository");
    fs::create_dir_all(&repository).expect("create the repository");
    fs::write(repository.join("b10x.toml"), "[pins]\nworktree = \"0.1\"\n")
        .expect("write b10x.toml");
    let output = case.run(&repository, &["doctor"], &[]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert_eq!(
        last(&output),
        "not ready: aep (too old)",
        "{}",
        shown(&output)
    );
    assert!(
        stdout(&output)
            .iter()
            .any(|line| line.starts_with("aep: 0.63.1 is older than") && line.ends_with(&pin)),
        "{}",
        shown(&output)
    );
}
