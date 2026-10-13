//! `conductor doctor` (`story:install-prerequisites`): each program conductor starts, present
//! with the first line its `--version` prints, or missing; and, when the nearest `b10x.toml`
//! pins one of them, whether it is older than that pin; for `aep`, `ess` or `worktree` that no
//! `b10x.toml` pins, whether it is older than the pin the binary was built with ([`build_pins`],
//! `story:doctor-minimum-versions`). One `conductor.config.Prerequisite` row per program. A
//! forge's CLI is needed only when an instance of the config file has a source on that forge
//! (`story:gh-optional`): `gh` for a `github` source, `glab` for a `gitlab` source; the other is
//! `not needed`. A config file that does not load makes the run not ready. The last line is the
//! verdict, `ready` or `not ready: ` and why (`story:doctor-verdict`).
//!
//! [`find`] is the one check of whether a program is there: `conductor snapshot start-snapshot`
//! runs it on the programs its collectors start ([`crate::collect::programs`]) and refuses at
//! start, naming each one missing, rather than failing inside a collector.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use conductor_model::config::{Instance, Prerequisite, PrerequisiteState, Source};

use crate::collect;
use crate::config;

/// Every program conductor starts: its collectors, its gate and its tasks. A forge's CLI of
/// [`FORGES`] is among them only for an instance with a source on that forge ([`forges`]).
pub const PROGRAMS: [&str; 9] = [
    "git", "gh", "glab", "claude", "jq", "task", "aep", "worktree", "ess",
];

/// The forges' CLIs: `gh` for GitHub, `glab` for GitLab.
pub const FORGES: [&str; 2] = ["gh", "glab"];

/// The pin file the ecosystem installer's `pin` command writes, found from the working
/// directory upward.
pub const PIN_FILE: &str = "b10x.toml";

/// The pins this binary was built with (`story:doctor-minimum-versions`): `name=spec` pairs
/// separated by `,`, the `aep`, `ess` and `worktree` pins of the product repository's own
/// `b10x.toml` at build time (`build.rs`); empty when the build found no such file.
const BUILD_PINS: &str = env!("CONDUCTOR_BUILD_PINS");

/// [`BUILD_PINS`] as `(program, spec)` pairs, in their order: what `conductor doctor` compares
/// with for each of their programs that the `b10x.toml` it finds does not pin ([`merge_pins`]).
#[must_use]
pub fn build_pins() -> Vec<(String, String)> {
    BUILD_PINS
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, spec)| (name.to_owned(), spec.to_owned()))
        .collect()
}

/// The pins `conductor doctor` checks: each of `project` (a `b10x.toml`'s), in its order, then
/// each of `build` ([`build_pins`]) whose program `project` does not pin. The project's pin wins
/// for a program both pin; the build's stays a minimum for one the project leaves out.
#[must_use]
pub fn merge_pins(
    project: &[(String, String)],
    build: &[(String, String)],
) -> Vec<(String, String)> {
    project
        .iter()
        .cloned()
        .chain(
            build
                .iter()
                .filter(|(program, _)| !project.iter().any(|(name, _)| name == program))
                .cloned(),
        )
        .collect()
}

/// How long `<program> --version` may take.
const VERSION_BOUND: Duration = Duration::from_secs(10);

/// The forge CLIs of [`FORGES`] that `instance` starts (`story:gh-optional`): `gh` when it has a
/// `github` source, `glab` when it has a `gitlab` source.
#[must_use]
pub fn forges(instance: &Instance) -> Vec<&'static str> {
    FORGES
        .into_iter()
        .filter(|forge| {
            instance.sources.iter().any(|source| match source {
                Source::GitHub(_) => *forge == "gh",
                Source::GitLab(_) => *forge == "glab",
                Source::Local(_) => false,
            })
        })
        .collect()
}

/// Where `program` is on `PATH`: the first directory holding a regular file of that name that
/// may be run. `None` when there is none, or `PATH` is not set.
#[must_use]
pub fn find(program: &str) -> Option<PathBuf> {
    find_in(program, env::var_os("PATH").as_deref()?)
}

fn find_in(program: &str, path: &OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(program))
        .find(|candidate| {
            fs::metadata(candidate)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// The programs of `programs` that [`find`] does not find, in their order.
#[must_use]
pub fn missing<'p>(programs: &[&'p str]) -> Vec<&'p str> {
    programs
        .iter()
        .copied()
        .filter(|program| find(program).is_none())
        .collect()
}

/// Refuses, naming every program of `programs` that is not on `PATH`, for `what`: the subject
/// that starts them, read with the plural verb "start" (such as "a snapshot's collectors").
///
/// # Errors
///
/// A program is missing.
pub fn require(programs: &[&str], what: &str) -> Result<()> {
    let missing = missing(programs);
    if missing.is_empty() {
        return Ok(());
    }
    let named: Vec<String> = missing
        .iter()
        .map(|program| format!("`{program}`"))
        .collect();
    bail!(
        "{} not on PATH, and {what} start {}; `conductor doctor` lists every program conductor \
         starts",
        if named.len() == 1 {
            format!("{} is", named[0])
        } else {
            format!("{} are", named.join(", "))
        },
        if named.len() == 1 { "it" } else { "them" }
    )
}

/// The first line `program --version` prints, on standard output or else standard error, empty
/// lines aside; why there is none when the run fails or prints nothing.
fn version_line(program: &Path) -> Result<String, String> {
    let output = collect::run(Command::new(program).arg("--version"), VERSION_BOUND)
        .map_err(|error| format!("{error:#}"))?;
    let first = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(str::to_owned)
    };
    first(&output.stdout)
        .or_else(|| first(&output.stderr))
        .ok_or_else(|| match output.status.code() {
            Some(code) => format!("`--version` exited {code} and printed nothing"),
            None => "`--version` was stopped by a signal".to_owned(),
        })
}

/// The version a `--version` line names: its first word that, after a leading `v` or a
/// `<name>-` prefix, is `x.y` or `x.y.z` in digits, read up to the first character that is not
/// part of it (`3.53.1+ff3372fc` is `3.53.1`, `jq-1.8.2` is `1.8.2`).
#[must_use]
pub fn version_of(line: &str) -> Option<(u64, u64, u64)> {
    line.split_whitespace().find_map(|word| {
        let word = word
            .rsplit('-')
            .find(|part| starts_with_digit(part))
            .or(Some(word))?;
        let word = word.strip_prefix('v').unwrap_or(word);
        let numeric: String = word
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let parts: Vec<&str> = numeric.trim_end_matches('.').split('.').collect();
        let number = |index: usize| parts.get(index).and_then(|part| part.parse::<u64>().ok());
        match parts.len() {
            2 => Some((number(0)?, number(1)?, 0)),
            3 => Some((number(0)?, number(1)?, number(2)?)),
            _ => None,
        }
    })
}

fn starts_with_digit(text: &str) -> bool {
    text.trim_start_matches('v')
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
}

/// A pin of `b10x.toml`: `x.y.z`, exactly that release, or `x.y`, the newest `x.y.*` (a leading
/// `v` is allowed), as the installer's `pin` command writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pin {
    /// Exactly `x.y.z`.
    Exact(u64, u64, u64),
    /// The newest `x.y.*`.
    Line(u64, u64),
}

impl Pin {
    /// Reads `0.32.0` or `0.59`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bare = text.trim().trim_start_matches('v');
        let parts: Vec<&str> = bare.split('.').collect();
        let number = |index: usize| -> Option<u64> {
            let part = parts.get(index)?;
            (!part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
                .then(|| part.parse().ok())
                .flatten()
        };
        match parts.len() {
            2 => Some(Self::Line(number(0)?, number(1)?)),
            3 => Some(Self::Exact(number(0)?, number(1)?, number(2)?)),
            _ => None,
        }
    }

    /// Whether `version` is older than anything this pin allows.
    #[must_use]
    pub fn newer_than(self, version: (u64, u64, u64)) -> bool {
        match self {
            Self::Exact(major, minor, patch) => version < (major, minor, patch),
            Self::Line(major, minor) => (version.0, version.1) < (major, minor),
        }
    }
}

/// The nearest `b10x.toml` from `start` upward, as the installer finds it: the home directory itself
/// and everything above it are not searched, so a stray file in the home directory pins nothing.
#[must_use]
pub fn pin_file(start: &Path, home: Option<&Path>) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if Some(dir) == home {
            return None;
        }
        let candidate = dir.join(PIN_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// The `[pins]` table of a `b10x.toml`: each `name = "spec"` line, comments and blank lines
/// aside, in the file's order. Any other line in that table is refused, naming its number.
///
/// # Errors
///
/// A line of `[pins]` is not `name = "spec"`.
pub fn pins(text: &str, file: &Path) -> Result<Vec<(String, String)>> {
    let mut pins = Vec::new();
    let mut in_pins = false;
    for (index, raw) in text.lines().enumerate() {
        let line = uncommented(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_pins = line == "[pins]";
            continue;
        }
        if !in_pins {
            continue;
        }
        let parsed = line.split_once('=').and_then(|(name, value)| {
            let name = name.trim().trim_matches('"');
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))?;
            (!name.is_empty()).then(|| (name.to_owned(), value.to_owned()))
        });
        match parsed {
            Some(pin) => pins.push(pin),
            None => bail!(
                "{}:{}: `{line}` is not a pin `name = \"x.y.z\"`",
                file.display(),
                index + 1
            ),
        }
    }
    Ok(pins)
}

/// `line` up to a `#` that is not inside a double-quoted string.
fn uncommented(line: &str) -> &str {
    let mut quoted = false;
    for (index, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

/// One row per program of [`PROGRAMS`], with its pin from `pins` when there is one; a forge
/// CLI of [`FORGES`] that `needed` does not name is `NotNeeded`, whether it is there or not.
fn examine(pins: &[(String, String)], needed: &[&str]) -> Vec<Prerequisite> {
    PROGRAMS
        .iter()
        .map(|program| {
            let pin = pins
                .iter()
                .find(|(name, _)| name == program)
                .map(|(_, spec)| spec.clone());
            if FORGES.contains(program) && !needed.contains(program) {
                return Prerequisite {
                    program: (*program).to_owned(),
                    state: PrerequisiteState::NotNeeded,
                    version_line: None,
                    version: None,
                    pin,
                };
            }
            let Some(path) = find(program) else {
                return Prerequisite {
                    program: (*program).to_owned(),
                    state: PrerequisiteState::Missing,
                    version_line: None,
                    version: None,
                    pin,
                };
            };
            let line = version_line(&path);
            let version = line.as_deref().ok().and_then(version_of);
            let state = match (pin.as_deref().map(Pin::parse), version) {
                (None, _) => PrerequisiteState::Present,
                (Some(Some(pin)), Some(version)) if pin.newer_than(version) => {
                    PrerequisiteState::OlderThanPin
                }
                (Some(Some(_)), Some(_)) => PrerequisiteState::Present,
                (Some(_), _) => PrerequisiteState::VersionUnread,
            };
            Prerequisite {
                program: (*program).to_owned(),
                state,
                version_line: Some(line.unwrap_or_else(|why| format!("version unknown: {why}"))),
                version: version.map(|(major, minor, patch)| format!("{major}.{minor}.{patch}")),
                pin,
            }
        })
        .collect()
}

/// The lines `conductor doctor` prints for `row`, whose pin is called `its` (`its pin` for a pin
/// of a `b10x.toml`, `the build's pin` for one of [`build_pins`]).
fn lines(row: &Prerequisite, its: &str) -> Vec<String> {
    let program = &row.program;
    match row.state {
        PrerequisiteState::Missing => return vec![format!("{program}: missing")],
        PrerequisiteState::NotNeeded => return vec![format!("{program}: not needed")],
        _ => {}
    }
    let mut lines = vec![format!(
        "{program}: present, {}",
        row.version_line.as_deref().unwrap_or("")
    )];
    if let Some(pin) = &row.pin {
        let version = row.version.as_deref().unwrap_or("");
        lines.push(match row.state {
            PrerequisiteState::OlderThanPin => {
                format!("{program}: {version} is older than {its} {pin}")
            }
            PrerequisiteState::VersionUnread if Pin::parse(pin).is_none() => {
                format!("{program}: {its} {pin:?} is not a version: write x.y.z or x.y")
            }
            PrerequisiteState::VersionUnread => {
                format!("{program}: no version read, so {its} {pin} cannot be checked")
            }
            _ => format!("{program}: {version} meets {its} {pin}"),
        });
    }
    lines
}

/// The forge CLIs of [`FORGES`] that any instance of the config `flag` (the global `--config`)
/// and the environment name starts ([`forges`]), in [`FORGES`]' order: every instance of the
/// file, not only the one a command would select, since the default instance's conductor serves
/// them all; the built-in instance's when there is no config file. Why the config does not load
/// when it does not, each line of it.
fn needed_forges(flag: Option<&Path>) -> Result<Vec<&'static str>, Vec<String>> {
    let loaded = config::Environment::from_process()
        .and_then(|environment| config::load(flag, &environment));
    match loaded {
        Ok(loaded) => Ok(FORGES
            .into_iter()
            .filter(|forge| {
                loaded
                    .config
                    .instances
                    .iter()
                    .any(|instance| forges(instance).contains(forge))
            })
            .collect()),
        Err(error) => Err(format!("{error:#}")
            .split(['\n', '\r'])
            .filter(|line| !line.trim().is_empty())
            .map(str::to_owned)
            .collect()),
    }
}

/// `conductor doctor` for the config the environment names: [`doctor_with`] without a
/// `--config`.
///
/// # Errors
///
/// What [`doctor_with`] answers.
pub fn doctor() -> Result<ExitCode> {
    doctor_with(None)
}

/// `conductor doctor`: when the config `flag` and the environment name does not load, a
/// `config: ` line for each line of why; one line per program of [`PROGRAMS`], `present` with
/// its version line, `missing`, or `not needed` for a forge CLI that no instance of that config
/// starts; the `b10x.toml` it read, or `pins: none`; one line per pinned program, meeting its pin
/// or older than it, the build's own pin ([`build_pins`]) standing for each program the file
/// does not pin ([`merge_pins`]); and last the verdict, `ready` or `not ready: ` with what failed.
/// Exits 1 when the config does not load, or a program is missing, older than its pin, or pinned
/// with a pin or version that does not read; 0 otherwise. Writes nothing.
///
/// # Errors
///
/// The working directory cannot be read, or the `b10x.toml` found cannot be read or holds a line
/// in `[pins]` that is no pin.
pub fn doctor_with(flag: Option<&Path>) -> Result<ExitCode> {
    const WHAT: &str = "doctor";
    let cwd = env::current_dir().context(WHAT)?;
    let home = env::var_os("HOME").map(PathBuf::from);
    let file = pin_file(&cwd, home.as_deref());
    let project = match &file {
        Some(file) => {
            let text = fs::read_to_string(file)
                .with_context(|| format!("{WHAT}: read {}", file.display()))?;
            pins(&text, file).context(WHAT)?
        }
        None => Vec::new(),
    };
    // The build's own pins stay the minimum for each program the file does not pin
    // (story:doctor-minimum-versions).
    let pins = merge_pins(&project, &build_pins());
    let its = |program: &str| {
        if project.iter().any(|(name, _)| name == program) {
            "its pin"
        } else {
            "the build's pin"
        }
    };
    let (needed, config_unreadable) = match needed_forges(flag) {
        Ok(needed) => (needed, false),
        Err(why) => {
            for line in &why {
                println!("config: {line}");
            }
            println!("config: does not load, so gh and glab are each counted as needed");
            (FORGES.to_vec(), true)
        }
    };
    let rows = examine(&pins, &needed);
    for row in &rows {
        if let Some(first) = lines(row, its(row.program.as_str())).first() {
            println!("{first}");
        }
    }
    match &file {
        Some(file) => println!("pins: {}", file.display()),
        None => println!("pins: none"),
    }
    for row in rows
        .iter()
        .filter(|row| row.pin.is_some() && row.state != PrerequisiteState::Missing)
    {
        for line in lines(row, its(row.program.as_str())).into_iter().skip(1) {
            println!("{line}");
        }
    }
    let verdict = verdict(&rows, config_unreadable);
    println!("{verdict}");
    Ok(if verdict == READY {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The verdict of a run every program of which passed (`story:doctor-verdict`).
const READY: &str = "ready";

/// The last line `conductor doctor` prints (`story:doctor-verdict`): [`READY`] when the config
/// loads and every row is `Present` or `NotNeeded`; otherwise `not ready: ` and what failed,
/// comma-separated: `config (unreadable)` first when `config_unreadable`, then each program in
/// [`PROGRAMS`]' order, as `<program> (missing)`, `<program> (too old)`,
/// `<program> (pin unread)` (its version was read, its pin is no version) or
/// `<program> (version unread)`.
fn verdict(rows: &[Prerequisite], config_unreadable: bool) -> String {
    let config = config_unreadable.then(|| "config (unreadable)".to_owned());
    let failed: Vec<String> = config
        .into_iter()
        .chain(rows.iter().filter_map(|row| {
            let why = match row.state {
                PrerequisiteState::Present | PrerequisiteState::NotNeeded => return None,
                PrerequisiteState::Missing => "missing",
                PrerequisiteState::OlderThanPin => "too old",
                PrerequisiteState::VersionUnread
                    if row.version.is_some()
                        && row
                            .pin
                            .as_deref()
                            .is_some_and(|pin| Pin::parse(pin).is_none()) =>
                {
                    "pin unread"
                }
                PrerequisiteState::VersionUnread => "version unread",
            };
            Some(format!("{} ({why})", row.program))
        }))
        .collect();
    if failed.is_empty() {
        READY.to_owned()
    } else {
        format!("not ready: {}", failed.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_read_from_each_programs_line() {
        for (line, version) in [
            ("git version 2.51.0", (2, 51, 0)),
            ("gh version 2.100.0 (2026-09-04)", (2, 100, 0)),
            ("2.1.295 (Claude Code)", (2, 1, 295)),
            ("jq-1.8.2", (1, 8, 2)),
            ("3.53.1+ff3372fc", (3, 53, 1)),
            ("aep 0.69.1", (0, 69, 1)),
            ("worktree-cli 0.13.0", (0, 13, 0)),
            ("ess 0.56.0", (0, 56, 0)),
            ("Task version: v3.40.0 (h1:abc)", (3, 40, 0)),
            ("tool 1.2", (1, 2, 0)),
        ] {
            assert_eq!(version_of(line), Some(version), "{line}");
        }
        for line in ["no version here", "", "build 7", "1.2.3.4"] {
            assert_eq!(version_of(line), None, "{line}");
        }
    }

    #[test]
    fn each_build_pin_is_a_pin_of_aep_ess_or_worktree() {
        for (program, spec) in build_pins() {
            assert!(
                ["aep", "ess", "worktree"].contains(&program.as_str()),
                "{program}"
            );
            assert!(Pin::parse(&spec).is_some(), "{program} = {spec:?}");
        }
    }

    #[test]
    fn a_pin_is_exact_or_a_line() {
        assert_eq!(Pin::parse("0.32.0"), Some(Pin::Exact(0, 32, 0)));
        assert_eq!(Pin::parse("v0.59"), Some(Pin::Line(0, 59)));
        for refused in ["", "1", "1.2.3.4", "x.y", "1..2", "latest"] {
            assert_eq!(Pin::parse(refused), None, "{refused}");
        }
        assert!(Pin::Exact(0, 32, 1).newer_than((0, 32, 0)));
        assert!(!Pin::Exact(0, 32, 0).newer_than((0, 32, 0)));
        assert!(!Pin::Exact(0, 32, 0).newer_than((0, 33, 0)));
        assert!(Pin::Line(0, 70).newer_than((0, 69, 9)));
        assert!(!Pin::Line(0, 70).newer_than((0, 70, 0)));
        assert!(!Pin::Line(0, 70).newer_than((0, 71, 0)));
    }

    #[test]
    fn a_project_pin_wins_and_the_build_s_stays_for_what_it_leaves_out() {
        let pair = |name: &str, spec: &str| (name.to_owned(), spec.to_owned());
        let project = [pair("worktree", "0.1"), pair("jq", "1.8")];
        let build = [pair("aep", "0.70"), pair("worktree", "0.13.0")];
        assert_eq!(
            merge_pins(&project, &build),
            vec![
                pair("worktree", "0.1"),
                pair("jq", "1.8"),
                pair("aep", "0.70")
            ]
        );
        assert_eq!(merge_pins(&[], &build), build.to_vec());
    }

    #[test]
    fn the_verdict_names_an_unreadable_config_and_a_pin_that_is_no_version() {
        let row = |program: &str, version: Option<&str>, pin: &str| Prerequisite {
            program: program.to_owned(),
            state: PrerequisiteState::VersionUnread,
            version_line: Some(String::new()),
            version: version.map(str::to_owned),
            pin: Some(pin.to_owned()),
        };
        let rows = [
            row("aep", Some("0.70.1"), "latest"),
            row("ess", None, "0.55"),
            row("worktree", None, "latest"),
        ];
        assert_eq!(
            verdict(&rows, true),
            "not ready: config (unreadable), aep (pin unread), ess (version unread), \
             worktree (version unread)"
        );
        assert_eq!(verdict(&[], true), "not ready: config (unreadable)");
        assert_eq!(verdict(&[], false), READY);
    }

    #[test]
    fn pins_are_read_from_the_pins_table_only() {
        let text = "[other]\naep = \"9.9\"\n\n[pins]\n# a comment\naep = \"0.70\" # newest\n\
                    \"ess\" = \"0.55.1\"\n[tail]\nx = 1\n";
        assert_eq!(
            pins(text, Path::new("b10x.toml")).expect("pins"),
            vec![
                ("aep".to_owned(), "0.70".to_owned()),
                ("ess".to_owned(), "0.55.1".to_owned())
            ]
        );
        let refused =
            pins("[pins]\naep = 0.70\n", Path::new("b10x.toml")).expect_err("an unquoted value");
        assert!(refused.to_string().contains("b10x.toml:2"), "{refused}");
    }
}
