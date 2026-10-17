//! The records committed through the bot route (`story:records-committed-by-daemon`,
//! docs/design/conductor-run.md § 3): [`commit_records`], which the daily tick runs after `metric
//! compute` and the brief, and `conductor run --once --commit-records` runs alone.
//!
//! In order, each step once:
//! 1. `records check`, a run of the conductor binary with the daemon's global flags; a check that
//!    does not pass refuses the commit with what it printed (its first [`SHOWN`] lines).
//! 2. The records directory is the top of a git repository of its own, so nothing of a repository
//!    around it is staged.
//! 3. Of the paths the records commit ([`PATHS`]), those with changes (`git status --porcelain`)
//!    are staged (`git add --all`); none changed is nothing to commit, and no line is written.
//! 4. The bot route commits them, `b10x-gates bot -- commit -F <file> -- <paths>`, with the
//!    message `Conductor: records of <day>: <n> dispatch lines, <m> decision lines`, which counts
//!    the lines of each log whose `at` falls on the UTC day `day` ([`message`]). The commit's id
//!    is the one the route printed (git's `[<branch> <id>] <title>`), else `HEAD`'s.
//! 5. With an `origin`, the bot route pushes: `b10x-gates bot -- push origin main`.
//!
//! The answer ([`Committed`]) is one dispatch line ([`Committed::line`]): `records-committed`
//! with the commit and whether it was pushed, or `records-commit-refused` with the step that
//! refused and the last line the route, git or the check printed. Nothing is retried.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use anyhow::Result;
use conductor_model::dispatch::{DispatchLogEvent, DispatchLogLine};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::one_line;

/// The paths of the records directory the route commits, in the order it names them.
pub const PATHS: [&str; 6] = [
    "decisions",
    "dispatches",
    "charters",
    "docs/handoff",
    "REPORT.md",
    "STATUS.md",
];

/// How many lines of a failing `records check` the refusal carries.
pub const SHOWN: usize = 10;

/// How long one step may run: the bot route scans before it commits and pushes over the network.
const BOUND: Duration = Duration::from_secs(600);

/// What commits the records: the bot route, and the conductor binary that checks them first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// The bot route's program: `b10x-gates`, found on `PATH`.
    pub gates: PathBuf,
    /// The conductor binary `records check` runs.
    pub program: PathBuf,
    /// The global `--config` it runs with, when given.
    pub config: Option<PathBuf>,
    /// The global `--state-dir` it runs with, when given; the commit message's file is written
    /// under its `run/`.
    pub state: Option<PathBuf>,
}

impl Route {
    /// The route of `b10x-gates` on `PATH`, checking with `program` and the global flags given.
    #[must_use]
    pub fn new(program: &Path, config: Option<&Path>, state: Option<&Path>) -> Self {
        Self {
            gates: PathBuf::from(crate::gates::PROGRAM),
            program: program.to_owned(),
            config: config.map(Path::to_owned),
            state: state.map(Path::to_owned),
        }
    }
}

/// What [`commit_records`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Committed {
    /// No path the route commits had changes: nothing ran past the check, and no line is
    /// written.
    Nothing,
    /// Committed, and pushed when `pushed`.
    Committed {
        /// The commit's id as the route printed it.
        commit: String,
        /// The commit's message.
        message: String,
        /// Whether it was pushed to `origin`.
        pushed: bool,
    },
    /// A step refused.
    Refused {
        /// The step: `records check`, `repository`, `commit` or `push`.
        reason: &'static str,
        /// What it printed last, or the check's findings.
        line: String,
        /// The commit, when it was made and its push refused.
        commit: Option<String>,
    },
}

impl Committed {
    /// The dispatch log line it is recorded as, with no `tick_id`; none for [`Committed::Nothing`].
    ///
    /// # Errors
    ///
    /// The clock cannot be formatted.
    pub fn line(&self) -> Result<Option<DispatchLogLine>> {
        match self {
            Self::Nothing => Ok(None),
            Self::Committed {
                commit,
                message,
                pushed,
            } => {
                let mut line =
                    crate::records::dispatch_line(DispatchLogEvent::RecordsCommitted, "-")?;
                line.commit = Some(commit.clone());
                line.line = Some(message.clone());
                line.evidence = Some(
                    if *pushed {
                        "pushed origin main"
                    } else {
                        "no origin"
                    }
                    .to_owned(),
                );
                Ok(Some(line))
            }
            Self::Refused {
                reason,
                line: said,
                commit,
            } => {
                let mut line =
                    crate::records::dispatch_line(DispatchLogEvent::RecordsCommitRefused, "-")?;
                line.reason = Some((*reason).to_owned());
                line.line = Some(said.clone());
                line.commit.clone_from(commit);
                Ok(Some(line))
            }
        }
    }
}

/// Commits the records directory `records` through `route` for the UTC day `day`
/// (`YYYY-MM-DD`), as the module documentation says.
#[must_use]
pub fn commit_records(records: &Path, route: &Route, day: &str) -> Committed {
    let refused = |reason: &'static str, line: String| Committed::Refused {
        reason,
        line,
        commit: None,
    };
    if let Err(said) = check(route) {
        return refused("records check", said);
    }
    if let Err(said) = own_repository(records) {
        return refused("repository", said);
    }
    let changed = match changed(records) {
        Ok(changed) => changed,
        Err(said) => return refused("repository", said),
    };
    if changed.is_empty() {
        return Committed::Nothing;
    }
    let mut add = vec!["add", "--all", "--"];
    add.extend(changed.iter().copied());
    if let Err(said) = git(records, &add) {
        return refused("commit", said);
    }
    let message = message(records, day);
    let file = match message_file(route, &message) {
        Ok(file) => file,
        Err(error) => return refused("commit", one_line(&format!("{error:#}"))),
    };
    let mut words = vec!["bot".to_owned(), "--".to_owned(), "commit".to_owned()];
    words.push("-F".to_owned());
    words.push(file.display().to_string());
    words.push("--".to_owned());
    words.extend(changed.iter().map(|path| (*path).to_owned()));
    let said = bot(records, route, &words);
    let _ = fs::remove_file(&file);
    let commit = match said {
        Ok(output) => commit_id(&output.stdout)
            .or_else(|| {
                git(records, &["rev-parse", "--short", "HEAD"])
                    .ok()
                    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            })
            .unwrap_or_default(),
        Err(said) => return refused("commit", said),
    };
    if !has_origin(records) {
        return Committed::Committed {
            commit,
            message,
            pushed: false,
        };
    }
    let push: Vec<String> = ["bot", "--", "push", "origin", "main"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    match bot(records, route, &push) {
        Ok(_) => Committed::Committed {
            commit,
            message,
            pushed: true,
        },
        Err(said) => Committed::Refused {
            reason: "push",
            line: said,
            commit: Some(commit),
        },
    }
}

/// The commit message for the UTC day `day`: `Conductor: records of <day>: <n> dispatch lines,
/// <m> decision lines`, `n` and `m` the lines of `dispatches/` and `decisions/` under `records`
/// whose `at` falls on that day.
#[must_use]
pub fn message(records: &Path, day: &str) -> String {
    format!(
        "Conductor: records of {day}: {} dispatch lines, {} decision lines",
        lines_of(records, crate::records::DISPATCHES, day),
        lines_of(records, crate::records::DECISIONS, day)
    )
}

/// The lines of the log `log` under `records` whose `at` falls on the UTC day `day`: those of its
/// month's file, where every line of that day is written.
fn lines_of(records: &Path, log: &str, day: &str) -> usize {
    let Some(month) = day.get(..7) else {
        return 0;
    };
    let Ok(text) = fs::read_to_string(records.join(log).join(format!("{month}.jsonl"))) else {
        return 0;
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|line| {
            line["at"]
                .as_str()
                .is_some_and(|at| utc_day(at).as_deref() == Some(day))
        })
        .count()
}

/// The UTC day of the RFC 3339 instant `at`, `YYYY-MM-DD`.
fn utc_day(at: &str) -> Option<String> {
    let at = OffsetDateTime::parse(at, &Rfc3339)
        .ok()?
        .to_offset(time::UtcOffset::UTC);
    Some(format!(
        "{:04}-{:02}-{:02}",
        at.year(),
        u8::from(at.month()),
        at.day()
    ))
}

/// `records check`, as step 1 says: what it printed when it does not pass.
fn check(route: &Route) -> std::result::Result<(), String> {
    let mut command = Command::new(&route.program);
    if let Some(config) = &route.config {
        command.arg("--config").arg(config);
    }
    if let Some(state) = &route.state {
        command.arg("--state-dir").arg(state);
    }
    command.args(["records", "check"]);
    let output = crate::collect::run(&mut command, BOUND)
        .map_err(|error| format!("did not run: {}", one_line(&format!("{error:#}"))))?;
    if output.status.success() {
        return Ok(());
    }
    let lines = |bytes: &[u8]| -> Vec<String> {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()
    };
    let mut said = lines(&output.stdout);
    if said.is_empty() {
        said = lines(&output.stderr);
    }
    if said.is_empty() {
        return Err(crate::confine::exited(&output));
    }
    if said.len() > SHOWN {
        let more = said.len() - SHOWN;
        said.truncate(SHOWN);
        said.push(format!("and {more} more"));
    }
    Err(said.join("; "))
}

/// Whether `records` is the top of a git repository of its own; why not, when it is not.
fn own_repository(records: &Path) -> std::result::Result<(), String> {
    let output = git(records, &["rev-parse", "--show-toplevel"])?;
    let top = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let same = match (top.canonicalize(), records.canonicalize()) {
        (Ok(top), Ok(records)) => top == records,
        _ => false,
    };
    if same {
        Ok(())
    } else {
        Err(format!(
            "{} is not the top of a git repository of its own; {} is",
            records.display(),
            top.display()
        ))
    }
}

/// The paths of [`PATHS`] that have changes, in that order.
fn changed(records: &Path) -> std::result::Result<Vec<&'static str>, String> {
    let mut changed = Vec::new();
    for path in PATHS {
        let output = git(
            records,
            &["status", "--porcelain", "--untracked-files=all", "--", path],
        )?;
        if !output.stdout.iter().all(u8::is_ascii_whitespace) {
            changed.push(path);
        }
    }
    Ok(changed)
}

/// Whether the repository at `records` has a remote named `origin`.
fn has_origin(records: &Path) -> bool {
    git(records, &["remote"]).is_ok_and(|output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|remote| remote.trim() == "origin")
    })
}

/// Writes `message` to a file of its own under the state directory's `run/`.
fn message_file(route: &Route, message: &str) -> Result<PathBuf> {
    let file = super::run_dir(route.state.as_deref())?
        .join(format!("records-commit.{}.txt", std::process::id()));
    super::write_file(&file, &format!("{message}\n"))?;
    Ok(file)
}

/// `git <args>` in `records`: its output when it succeeds, else the last line it printed.
fn git(records: &Path, args: &[&str]) -> std::result::Result<Output, String> {
    let mut command = Command::new("git");
    command.args(args).current_dir(records);
    succeeded(crate::collect::run(&mut command, BOUND), "git")
}

/// `<route> <words>` in `records`: its output when it succeeds, else the last line it printed.
fn bot(records: &Path, route: &Route, words: &[String]) -> std::result::Result<Output, String> {
    let mut command = Command::new(&route.gates);
    command.args(words).current_dir(records);
    succeeded(
        crate::collect::run(&mut command, BOUND),
        &route.gates.display().to_string(),
    )
}

/// `ran`'s output when the program succeeded, else the last line it printed: of its standard
/// error, else of its standard output, else how it ended.
fn succeeded(ran: Result<Output>, program: &str) -> std::result::Result<Output, String> {
    let output =
        ran.map_err(|error| format!("{program} did not run: {}", one_line(&format!("{error:#}"))))?;
    if output.status.success() {
        return Ok(output);
    }
    let last = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .rfind(|line| !line.is_empty())
            .map(str::to_owned)
    };
    Err(last(&output.stderr)
        .or_else(|| last(&output.stdout))
        .unwrap_or_else(|| format!("{program} {}", crate::confine::exited(&output))))
}

/// The commit id in what `git commit` printed: the last word inside the brackets of its first
/// line that opens with `[`, as in `[main 0123abc] <title>` or `[main (root-commit) 0123abc]`.
fn commit_id(stdout: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stdout).lines().find_map(|line| {
        let inside = line.trim().strip_prefix('[')?;
        let (inside, _) = inside.split_once(']')?;
        inside.split_whitespace().last().map(str::to_owned)
    })
}
