//! `conductor init` (`story:first-run-seed`): writes the records directory skeleton of an
//! instance, so a fresh instance has what conductor's start prompt reads — the hand-over
//! committed last in `docs/handoff/` and `NORTHSTAR.md` — and the `rules.md` every session
//! reads its role's section of.
//!
//! It makes the directory a git repository, writes `NORTHSTAR.md`, `docs/handoff/<date>.md`
//! with a first task, `rules.md` with one section per role, a `README.md` saying what the
//! directory is, `.engineering/workspace.yaml` as `conductor config workspace` writes it
//! (`story:workspace-from-config`), and `decisions/`, `dispatches/` and `charters/`, then commits
//! the files it wrote, and only those. It overwrites nothing: when any of those files is there, it writes
//! nothing. A skeleton whose commit failed, in a repository with no commit at all, is committed as
//! it lies by [`commit_lying`], which the daemon's unattended start runs
//! (`story:container-start-without-prompt`). The texts name no organization. It installs no agent
//! adapter under `~/.claude/agents/`: the daemon starts the host roles from their profiles
//! (`story:host-roles-by-daemon`).

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
session reads this file at start and follows the section of its role; the director passes the
other sections on in the charters and briefs it writes.

## Director

## Lead

## Maintainer
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

/// The directories of the skeleton's files, under the records directory.
const FILE_DIRECTORIES: [&str; 2] = ["docs/handoff", ".engineering"];

/// The files of the skeleton, under the records directory, and their text; `workspace` is the
/// text of the workspace file.
fn files(date: &str, workspace: &str) -> Vec<(PathBuf, String)> {
    vec![
        (PathBuf::from("README.md"), README.to_owned()),
        (PathBuf::from("NORTHSTAR.md"), NORTHSTAR.to_owned()),
        (PathBuf::from("rules.md"), RULES.to_owned()),
        (
            Path::new("docs/handoff").join(format!("{date}.md")),
            handoff(date),
        ),
        (PathBuf::from(config::WORKSPACE_FILE), workspace.to_owned()),
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
    let files = files(date, "");
    let there: Vec<String> = files
        .iter()
        .map(|(file, _)| records.join(file))
        .filter(|path| path.symlink_metadata().is_ok())
        .map(|path| path.display().to_string())
        .collect();
    if !there.is_empty() {
        bail!(
            "{} {} there; init overwrites nothing and wrote nothing there",
            there.join(", "),
            if there.len() == 1 { "is" } else { "are" }
        );
    }
    for directory in DIRECTORIES
        .iter()
        .map(Path::new)
        .chain(FILE_DIRECTORIES.iter().map(Path::new))
    {
        let path = records.join(directory);
        if path.exists() && !path.is_dir() {
            bail!(
                "{} is there and is not a directory; init wrote nothing there",
                path.display()
            );
        }
    }
    Ok(())
}

/// Writes the skeleton under `records` for `date`, its workspace file holding `workspace`, and
/// answers the paths it wrote, files then directories. Writes nothing when [`check`] refuses.
///
/// # Errors
///
/// What [`check`] answers, or a write failed.
pub fn write(records: &Path, date: &str, workspace: &str) -> Result<Vec<PathBuf>> {
    check(records, date)?;
    let files = files(date, workspace);
    let mut written = Vec::new();
    for directory in DIRECTORIES
        .iter()
        .map(Path::new)
        .chain(FILE_DIRECTORIES.iter().map(Path::new))
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

/// The files of the skeleton under `records` as they lie, relative to it: each of its files that
/// is there, every file of `docs/handoff/` among them whatever its date.
fn lying(records: &Path) -> Vec<String> {
    let handoff = Path::new("docs/handoff");
    let mut there: Vec<String> = files("", "")
        .into_iter()
        .map(|(file, _)| file)
        .filter(|file| file.parent() != Some(handoff))
        .filter(|file| records.join(file).is_file())
        .map(|file| file.display().to_string())
        .collect();
    let mut handoffs: Vec<String> = fs::read_dir(records.join(handoff))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| handoff.join(entry.file_name()).display().to_string())
        .collect();
    handoffs.sort();
    there.extend(handoffs);
    there
}

/// Runs `git -C <records> <args>` and answers its standard output; a refusal is one line: git's
/// subcommand and the reason git gave, its first line of standard error and, when another, its
/// last.
fn git_line(records: &Path, args: &[&str]) -> Result<String> {
    let shown = args.first().copied().unwrap_or_default();
    let output = crate::instance::command("git")
        .arg("-C")
        .arg(records)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("start `git {shown}`"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stderr
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let why = match (lines.first(), lines.last()) {
            (Some(first), Some(last)) if first != last => format!("{first} ({last})"),
            (Some(first), _) => (*first).to_owned(),
            _ => "no reason given".to_owned(),
        };
        bail!("`git {shown}` failed: {why}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The files of `files` that `git ls-files <which>` lists in `records`, in their order.
fn listed<'a>(records: &Path, files: &'a [String], which: &[&str]) -> Result<Vec<&'a str>> {
    let mut args = vec!["ls-files", "-z"];
    args.extend(which);
    args.push("--");
    args.extend(files.iter().map(String::as_str));
    let output = git_line(records, &args)?;
    let shown: Vec<&str> = output.split('\0').collect();
    Ok(files
        .iter()
        .map(String::as_str)
        .filter(|file| shown.contains(file))
        .collect())
}

/// Commits the skeleton under `records` as it lies, where `records` is a git repository of its
/// own with no commit on any branch or ref: the files of it that are there ([`lying`]) and git
/// does not ignore, with the message `conductor init` commits with. A repository that has a
/// commit anywhere, a directory that is no repository of its own and a skeleton of which nothing
/// is there are left alone. Changes no file. Answers whether it committed.
///
/// # Errors
///
/// A `git` it runs fails, or git ignores every file of the skeleton; one line, naming `records`
/// and the reason.
pub fn commit_lying(records: &Path) -> Result<bool> {
    let not_committed = || {
        format!(
            "the records skeleton in {} is not committed",
            records.display()
        )
    };
    if records.join(".git").symlink_metadata().is_err()
        || !git_line(records, &["rev-list", "-n", "1", "--all"])
            .with_context(not_committed)?
            .trim()
            .is_empty()
    {
        return Ok(false);
    }
    let lying = lying(records);
    if lying.is_empty() {
        return Ok(false);
    }
    // A file of the index is subject to no ignore rule; a file outside it is committed where git
    // does not ignore it. `git add` refuses a file of the index under an ignored directory, so
    // only the second is added.
    let indexed = listed(records, &lying, &["--cached"]).with_context(not_committed)?;
    let new =
        listed(records, &lying, &["--others", "--exclude-standard"]).with_context(not_committed)?;
    let files: Vec<&str> = lying
        .iter()
        .map(String::as_str)
        .filter(|file| indexed.contains(file) || new.contains(file))
        .collect();
    if files.is_empty() {
        bail!("{}: git ignores each file of it", not_committed());
    }
    let mut add = vec!["add", "--"];
    add.extend(&new);
    let mut commit = vec!["commit", "--quiet", "-m", COMMIT_MESSAGE, "--"];
    commit.extend(&files);
    let added = if new.is_empty() {
        Ok(String::new())
    } else {
        git_line(records, &add)
    };
    added
        .and_then(|_| git_line(records, &commit))
        .with_context(|| format!("{}; the next start tries again", not_committed()))?;
    println!("committed: {}", files.join(", "));
    Ok(true)
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
    let index = loaded
        .config
        .instances
        .iter()
        .position(|other| other.name == instance.name)
        .unwrap_or_default();
    let workspace =
        config::workspace_text(&config::workspace_members(instance, index).context(WHAT)?);
    fs::create_dir_all(&records)
        .with_context(|| format!("{WHAT}: create {}", records.display()))?;
    git(&records, &["init", "--quiet"]).context(WHAT)?;
    let written = write(&records, &date, &workspace).context(WHAT)?;
    for path in &written {
        println!("wrote: {}", path.display());
    }
    let files: Vec<String> = files(&date, &workspace)
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
