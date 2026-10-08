//! `conductor init` (`story:first-run-seed`): writes the records directory skeleton of an
//! instance, so a fresh instance has what conductor's start prompt reads — the hand-over
//! committed last in `docs/handoff/` and `NORTHSTAR.md` — and the `rules.md` every session
//! reads its role's section of.
//!
//! It makes the directory a git repository, writes `NORTHSTAR.md`, `docs/handoff/<date>.md`
//! with a first task, `rules.md` with one section per role, a `README.md` saying what the
//! directory is, and `decisions/`, `dispatches/` and `charters/`, then commits the files it
//! wrote, and only those. It overwrites nothing: when any of those files is there, it writes
//! nothing. The texts name no organization.

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context as _, Result, bail};
use time::OffsetDateTime;

use crate::cli::InitArgs;
use crate::config::{self, Environment};

/// The directories conductor writes its records into.
pub const DIRECTORIES: [&str; 3] = ["decisions", "dispatches", "charters"];

/// The message of the commit `conductor init` makes.
const COMMIT_MESSAGE: &str = "Seed the records directory with conductor init";

const NORTHSTAR: &str = "\
# North star

What this organization works towards. Every cycle, conductor holds each repository's active work
against it: which goal the work serves, and what serves none.

Not written yet. With the operator, write one section per goal: what it is, the objective it
serves, and how its progress is measured.
";

const RULES: &str = "\
# Rules

The rules of this organization, each with its source: a decision, a message or a document. Every
session reads this file at start and follows the section of its role; conductor passes the other
sections on in the charters and briefs it writes.

## Conductor

## Repository controller

## conductor-dev
";

const README: &str = "\
# Records of a conductor instance

This directory is the records directory of one conductor instance: the `records` value that
`conductor config show` prints. Conductor starts here, and keeps its own records here, each
committed by the session that writes it:

| path | holds | written by |
|---|---|---|
| `NORTHSTAR.md` | what the organization works towards | conductor, with the operator |
| `rules.md` | the organization's rules, one section per role | conductor-dev |
| `docs/handoff/` | hand-overs; conductor starts from the one committed last | conductor |
| `decisions/` | the decision log | conductor |
| `dispatches/` | the briefs sent to repository controllers and their reports | conductor |
| `charters/` | one charter per repository controller | conductor |
| `STATUS.md` | the newest status | conductor |

`conductor init` wrote the first version of these files.
";

/// The first hand-over, dated `date`.
fn handoff(date: &str) -> String {
    format!(
        "\
# Hand-over {date}

Written by `conductor init`: the first hand-over of this instance. No cycle has run yet.

## First task

1. Read `rules.md`; with the operator, write the rules each role follows in its section, each
   with its source.
2. Read `NORTHSTAR.md`; with the operator, write the goals the work serves.
3. Run `conductor doctor`, then one cycle; report to the operator what the first snapshot shows,
   and which repositories have no goal.

## Open

- Dispatches: none.
- Decision requests: none.
- Waits: none.
"
    )
}

/// Today, UTC, as `YYYY-MM-DD`.
fn today() -> String {
    let date = OffsetDateTime::now_utc().date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

/// The files of the skeleton, under the records directory, and their text.
fn files(date: &str) -> Vec<(PathBuf, String)> {
    vec![
        (PathBuf::from("README.md"), README.to_owned()),
        (PathBuf::from("NORTHSTAR.md"), NORTHSTAR.to_owned()),
        (PathBuf::from("rules.md"), RULES.to_owned()),
        (
            Path::new("docs/handoff").join(format!("{date}.md")),
            handoff(date),
        ),
    ]
}

/// Runs `git -C <records> <args>`, its standard error kept for the refusal.
fn git(records: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(records)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("start `git {}`", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "`git {}` in {} failed: {}",
            args.join(" "),
            records.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Refuses when a file of the skeleton for `date` is under `records` (a link included), or a
/// directory of it is something else.
///
/// # Errors
///
/// As said.
pub fn check(records: &Path, date: &str) -> Result<()> {
    let files = files(date);
    let there: Vec<String> = files
        .iter()
        .map(|(file, _)| records.join(file))
        .filter(|path| path.symlink_metadata().is_ok())
        .map(|path| path.display().to_string())
        .collect();
    if !there.is_empty() {
        bail!(
            "{} {} there; init overwrites nothing and wrote nothing",
            there.join(", "),
            if there.len() == 1 { "is" } else { "are" }
        );
    }
    for directory in DIRECTORIES
        .iter()
        .map(Path::new)
        .chain([Path::new("docs/handoff")])
    {
        let path = records.join(directory);
        if path.exists() && !path.is_dir() {
            bail!(
                "{} is there and is not a directory; init wrote nothing",
                path.display()
            );
        }
    }
    Ok(())
}

/// Writes the skeleton under `records` for `date`, and answers the paths it wrote, files then
/// directories. Writes nothing when [`check`] refuses.
///
/// # Errors
///
/// What [`check`] answers, or a write failed.
pub fn write(records: &Path, date: &str) -> Result<Vec<PathBuf>> {
    check(records, date)?;
    let files = files(date);
    let mut written = Vec::new();
    for directory in DIRECTORIES
        .iter()
        .map(Path::new)
        .chain([Path::new("docs/handoff")])
    {
        fs::create_dir_all(records.join(directory))
            .with_context(|| format!("create {}", records.join(directory).display()))?;
    }
    for (file, text) in &files {
        let path = records.join(file);
        let mut out = File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("create {}", path.display()))?;
        out.write_all(text.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        written.push(path);
    }
    written.extend(DIRECTORIES.iter().map(|directory| records.join(directory)));
    Ok(written)
}

/// `conductor init`: writes and commits the active instance's records skeleton, printing one
/// `wrote:` line per file and directory and one `committed:` line. The instance is selected as
/// for `config show`, and a config file must name it.
///
/// # Errors
///
/// The config does not load or names no such instance, there is no config file, a file of the
/// skeleton is there, or a write, `git init` or the commit failed; a failed commit names what was
/// written.
pub fn init(config_flag: Option<&Path>, args: &InitArgs) -> Result<ExitCode> {
    const WHAT: &str = "init";
    let environment = Environment::from_process().context(WHAT)?;
    let loaded = config::load(config_flag, &environment).context(WHAT)?;
    if !loaded.read {
        bail!(
            "{WHAT}: no config file at {}; init writes the records directory an instance of one \
             names",
            loaded.path.display()
        );
    }
    let instance = config::select(&loaded, args.instance.as_deref(), &environment).context(WHAT)?;
    let records = PathBuf::from(&instance.records);
    let date = today();
    // Refuse before anything is made, `git init` included.
    check(&records, &date).context(WHAT)?;
    fs::create_dir_all(&records)
        .with_context(|| format!("{WHAT}: create {}", records.display()))?;
    git(&records, &["init", "--quiet"]).context(WHAT)?;
    let written = write(&records, &date).context(WHAT)?;
    for path in &written {
        println!("wrote: {}", path.display());
    }
    let files: Vec<String> = files(&date)
        .into_iter()
        .map(|(file, _)| file.display().to_string())
        .collect();
    let mut add = vec!["add", "--"];
    add.extend(files.iter().map(String::as_str));
    let mut commit = vec!["commit", "--quiet", "-m", COMMIT_MESSAGE, "--"];
    commit.extend(files.iter().map(String::as_str));
    git(&records, &add)
        .and_then(|()| git(&records, &commit))
        .with_context(|| {
            format!(
                "{WHAT}: wrote the files above but did not commit them; commit them in {} \
                 (conductor starts from the hand-over committed last)",
                records.display()
            )
        })?;
    println!("committed: {}", files.join(", "));
    Ok(ExitCode::SUCCESS)
}
