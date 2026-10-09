//! The `sessions` collector: live Claude Code and Codex sessions (`story:collect-sessions`).
//!
//! It reads two sources, both held by a [`Sources`] value a test replaces, and records one
//! `RecordSession` per live session:
//!
//! | harness | source | live when | `session_ref` | `activity` |
//! |---|---|---|---|---|
//! | Claude | the list `claude agents --json` prints, run through [`run`] under [`Sources::bound`] | the entry has a `pid`; the list keeps a session that exited, without one | `id`, else `sessionId` (an interactive session has no `id`) | `status`, else `state` (`claude_activity`) |
//! | Codex | the first record, `session_meta`, of each `*.jsonl` file under [`Sources::codex`] | the file was written within [`RECENT`] of [`Sources::now`] | `payload.id` | `Background` for a sub-agent thread (`payload.thread_source`), else `Unknown`: the file does not say whether the thread works |
//!
//! A Claude session's `name` is the list's; a Codex thread has none in its file.
//!
//! `place` decides `cwd` and `repository`, against the instance's checkouts root and the
//! directory of their managed trees (`checkouts.root` and `checkouts.trees` of
//! `conductor.config.Checkouts`). `repository` is the repository of a working directory in
//! `<root>/` or in `<trees>/`, the deeper of the two when both hold it: the directory holding
//! `.git` at depth 1 or 2, `<repo>` or `<group>/<repo>` ([`collect::repository_of`],
//! `story:grouped-instance`), and absent for the root itself or a group directory; `cwd` is the
//! directory with the home directory written `~`. A session whose working directory is the
//! instance's records directory or lies under it is conductor's own
//! (`story:sessions-outside-root-explained`): it is placed, its `role` is `conductor`, and its
//! `repository` is absent unless the records directory lies in a checkout too; every other
//! session's `role` is absent. A records directory that is the home directory or above it places
//! no session, nor does the built-in instance's, which is only the process's working directory. A session whose working directory lies in none of the three, or
//! under an `exclude` entry of the instance's sources under the checkouts root, keeps its harness
//! and activity only: its `session_ref` and `cwd` are recorded empty and its `name` absent,
//! because they may name the operator's employer or its customers. Such a redacted row is by
//! design, not a defect. No error names any of them: an error names the command, an entry's
//! position in the list, or a Codex file's path.
//!
//! Whose a session is, its name and directory read together (`story:instance-session-names`,
//! [`collect::session_instance`]: a session named as an instance names the controller of the
//! repository it works in is that instance's, else one whose name carries an instance's
//! `session_prefix` is), decides before its directory alone does: the instance's own session is
//! placed wherever its working directory is but under an `exclude` entry, bound to a repository
//! only where its directory is, and `<prefix>-conductor` is conductor's own; another instance's
//! is kept to its harness and activity wherever it runs, even in this instance's checkouts.
//!
//! [`collect`] places against the instance the process runs ([`config::active`]), its excluded
//! repositories left out and its records directory conductor's, [`collect_in`] against any
//! checkouts, and [`collect_from`] against the built-in instance's under [`Sources::home`]
//! ([`collect::built_in_checkouts`]); the last two place no session as conductor's.
//!
//! Both sources are read whole before anything is recorded, so a collector that fails records no
//! session. A Codex file whose first line is not written to its end yet is left for the next
//! snapshot; one whose first line is complete but no `session_meta` record fails the collector.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead as _, BufReader, ErrorKind, Read as _};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{Checkouts, Instance};
use conductor_model::observation::{
    Harness, RecordSession, RepositoryName, SessionState, SnapshotId,
};
use serde_json::Value;

use super::{BOUND, Under, run};
use crate::collect;
use crate::config;
use crate::snapshot::Recorder;

/// How recently a Codex session file must have been written for its thread to count as live.
pub const RECENT: Duration = Duration::from_secs(60 * 60);

/// The longest first line of a Codex session file that is read. Its `session_meta` record carries
/// the thread's base instructions, about 23 KB on 2026-10-07.
const FIRST_LINE: usize = 1 << 20;

/// What the collector reads, the one seam a test points at fixtures. [`Sources::new`] gives the
/// real ones.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The program and arguments that print the Claude session list as one JSON array.
    pub agents: Vec<OsString>,
    /// Codex's session directory, searched for `*.jsonl` files at any depth.
    pub codex: PathBuf,
    /// The home directory, which a recorded `cwd` writes `~`; [`collect_from`] places against the
    /// built-in instance's checkouts under it.
    pub home: PathBuf,
    /// The time a Codex file's modification time is read against.
    pub now: SystemTime,
    /// How long the session-list command may run before it is stopped.
    pub bound: Duration,
}

impl Sources {
    /// `claude agents --json`, `~/.codex/sessions`, the current time and [`BOUND`], for the home
    /// directory `home`.
    #[must_use]
    pub fn new(home: PathBuf) -> Self {
        Self {
            agents: ["claude", "agents", "--json"].map(OsString::from).to_vec(),
            codex: home.join(".codex/sessions"),
            home,
            now: SystemTime::now(),
            bound: BOUND,
        }
    }
}

/// Records one `RecordSession` per live session into the recorder's snapshot, from
/// [`Sources::new`] over the home directory `HOME` names, placed against the checkouts of the
/// instance the process runs.
///
/// # Errors
///
/// `HOME` is not an absolute path, or what [`collect_in`] answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .context("HOME is not an absolute path; a recorded working directory writes it `~`")?;
    let active = config::active();
    let instance = &active.instance;
    let exclude = collect::excluded_under(instance, Path::new(&instance.checkouts.root));
    // Without a config file the records directory is the process's working directory, which says
    // nothing about where conductor's session runs.
    let records = active.from_file.then(|| Path::new(&instance.records));
    let names = Names {
        own: Some(instance),
        instances: active.all(),
    };
    collect_named(
        &Sources::new(home),
        &instance.checkouts,
        records,
        &exclude,
        &names,
        record,
    )
}

/// [`collect_in`], placed against the built-in instance's checkouts under [`Sources::home`].
///
/// # Errors
///
/// What [`collect_in`] answers.
pub fn collect_from(sources: &Sources, record: &mut Recorder<'_>) -> Result<()> {
    collect_in(sources, &collect::built_in_checkouts(&sources.home), record)
}

/// Records one `RecordSession` per live session `sources` show into the recorder's snapshot, each
/// placed against `checkouts`: the Claude sessions in list order, then the Codex threads in path
/// order.
///
/// # Errors
///
/// The session-list command does not start, runs past its bound, exits unsuccessfully or prints no
/// JSON array; a live entry has no working directory or no reference; a Codex file cannot be read,
/// or its complete first line is no `session_meta` record; or an observation is refused.
pub fn collect_in(
    sources: &Sources,
    checkouts: &Checkouts,
    record: &mut Recorder<'_>,
) -> Result<()> {
    collect_excluding(sources, checkouts, None, &[], record)
}

/// [`collect_in`], a session whose working directory is `records` or lies under it placed as
/// conductor's own, and a session under one of the `exclude` entries under the checkouts root
/// kept to its harness and activity, as one outside the checkouts is.
///
/// # Errors
///
/// What [`collect_in`] answers.
pub fn collect_excluding(
    sources: &Sources,
    checkouts: &Checkouts,
    records: Option<&Path>,
    exclude: &[String],
    record: &mut Recorder<'_>,
) -> Result<()> {
    collect_named(
        sources,
        checkouts,
        records,
        exclude,
        &Names::default(),
        record,
    )
}

/// What a session's name and directory are read against (`story:instance-session-names`): the
/// instance the collector records for, and every instance of its config file.
#[derive(Debug, Default)]
struct Names<'a> {
    own: Option<&'a Instance>,
    instances: Vec<&'a Instance>,
}

/// Whose a session name is, by the prefix it carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    /// It carries the instance's own prefix.
    Own,
    /// It carries another instance's prefix.
    Other,
    /// It carries no instance's prefix: its directory places it.
    Unknown,
}

impl Names<'_> {
    /// Whose the session named `name` working in `cwd` is, by [`collect::session_instance`].
    fn owner(&self, name: Option<&str>, cwd: &str) -> Owner {
        let (Some(own), Some(name)) = (self.own, name) else {
            return Owner::Unknown;
        };
        match collect::session_instance(&self.instances, name, Some(Path::new(cwd))) {
            Some(instance) if instance.name == own.name => Owner::Own,
            Some(_) => Owner::Other,
            None => Owner::Unknown,
        }
    }

    /// Whether `name` is the instance's conductor session, `<prefix>-conductor`; without a prefix
    /// no name says so.
    fn conductor(&self, name: Option<&str>) -> bool {
        self.own
            .filter(|own| own.session_prefix.is_some())
            .zip(name)
            .is_some_and(|(own, name)| config::session_name(own, CONDUCTOR).0 == name)
    }
}

/// [`collect_excluding`], a session's name read against `names` before its directory.
fn collect_named(
    sources: &Sources,
    checkouts: &Checkouts,
    records: Option<&Path>,
    exclude: &[String],
    names: &Names<'_>,
    record: &mut Recorder<'_>,
) -> Result<()> {
    let places = Places {
        home: &sources.home,
        root: Path::new(&checkouts.root),
        trees: Path::new(&checkouts.trees),
        records,
    };
    let mut sessions = claude(sources)?;
    sessions.extend(codex(sources)?);
    let snapshot = record.snapshot().clone();
    for session in sessions {
        record.session(session.observation(snapshot.clone(), &places, exclude, names))?;
    }
    Ok(())
}

/// The `role` of a session placed in the instance's records directory: the instance role
/// `conductor`, whose session runs there.
const CONDUCTOR: &str = "conductor";

/// What a working directory is placed against: the home directory, written `~`, the checkouts
/// root, the directory of their managed trees, and the instance's records directory, if any.
struct Places<'a> {
    home: &'a Path,
    root: &'a Path,
    trees: &'a Path,
    records: Option<&'a Path>,
}

/// One live session as its source shows it, before it is placed.
struct Live {
    harness: Harness,
    reference: String,
    name: Option<String>,
    cwd: String,
    activity: SessionState,
}

impl Live {
    /// The observation of this session for `snapshot`, placed against `places`: in the records
    /// directory, conductor's role; outside the checkouts root, their managed trees and the
    /// records directory, or under one of the `exclude` entries, only its harness and activity.
    /// Its name read against `names` decides first: the instance's prefix places it wherever it
    /// runs, and another instance's keeps it to its harness and activity.
    fn observation(
        self,
        snapshot: SnapshotId,
        places: &Places<'_>,
        exclude: &[String],
        names: &Names<'_>,
    ) -> RecordSession {
        let by_directory =
            || place(&self.cwd, places).filter(|_| !excluded(&self.cwd, places, exclude));
        let placed = match names.owner(self.name.as_deref(), &self.cwd) {
            Owner::Other => None,
            Owner::Unknown => by_directory(),
            // Under an excluded repository it stays redacted whatever its name says: the
            // exclusion keeps that repository out of the instance's records.
            Owner::Own if excluded(&self.cwd, places, exclude) => None,
            Owner::Own => by_directory().or_else(|| {
                let cwd = Path::new(&self.cwd);
                is_plain(cwd)
                    .then(|| shown(cwd, places.home))
                    .flatten()
                    .map(|shown| (shown, None))
            }),
        };
        let (session_ref, name, cwd, repository, role) = match placed {
            Some((cwd, repository)) => {
                let role = (conductors(Path::new(&self.cwd), places)
                    || names.conductor(self.name.as_deref()))
                .then(|| CONDUCTOR.to_owned());
                (self.reference, self.name, cwd, repository, role)
            }
            None => (String::new(), None, String::new(), None, None),
        };
        RecordSession {
            snapshot_id: snapshot,
            harness: self.harness,
            session_ref,
            name,
            cwd,
            repository: repository.map(RepositoryName),
            role,
            activity: self.activity,
        }
    }
}

/// The live sessions of the Claude session list.
fn claude(sources: &Sources) -> Result<Vec<Live>> {
    let shown = command_line(&sources.agents);
    let (program, arguments) = sources
        .agents
        .split_first()
        .context("no session-list command is given")?;
    let output = run(Command::new(program).args(arguments), sources.bound)
        .with_context(|| format!("read the Claude session list `{shown}`"))?;
    match output.status.code() {
        Some(0) => {}
        Some(code) => bail!("`{shown}` exited {code}"),
        None => bail!("`{shown}` was ended by a signal"),
    }
    let Value::Array(entries) = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("`{shown}` printed no JSON"))?
    else {
        bail!("`{shown}` printed no JSON array");
    };
    let mut live = Vec::new();
    for (at, entry) in entries.iter().enumerate() {
        let position = at + 1;
        if !entry.is_object() {
            bail!("entry {position} of `{shown}` is no JSON object");
        }
        if entry["pid"].is_null() {
            continue;
        }
        let text = |key: &str| entry[key].as_str().filter(|text| !text.is_empty());
        let reference = text("id").or_else(|| text("sessionId")).ok_or_else(|| {
            anyhow!(
                "the live session at entry {position} of `{shown}` carries neither `id` nor \
                 `sessionId`"
            )
        })?;
        let cwd = text("cwd").ok_or_else(|| {
            anyhow!("the live session at entry {position} of `{shown}` carries no `cwd`")
        })?;
        live.push(Live {
            harness: Harness::Claude,
            reference: reference.to_owned(),
            name: text("name").map(str::to_owned),
            cwd: cwd.to_owned(),
            activity: claude_activity(entry["status"].as_str(), entry["state"].as_str()),
        });
    }
    Ok(live)
}

/// The activity a Claude session's `status`, else its `state`, names: `busy` and `working` are
/// `Busy`, `idle` is `Idle`, `waiting` and `blocked` are `Waiting`, `background` is `Background`,
/// any other word or none `Unknown`. A `status` decides even when its word is not one of these.
fn claude_activity(status: Option<&str>, state: Option<&str>) -> SessionState {
    match status.or(state).map(str::to_ascii_lowercase).as_deref() {
        Some("busy" | "working") => SessionState::Busy,
        Some("idle") => SessionState::Idle,
        Some("waiting" | "blocked") => SessionState::Waiting,
        Some("background") => SessionState::Background,
        _ => SessionState::Unknown,
    }
}

/// The live Codex threads: each `*.jsonl` file under the session directory written within
/// [`RECENT`] of the clock, in path order. No session directory is no thread.
fn codex(sources: &Sources) -> Result<Vec<Live>> {
    let mut files = Vec::new();
    jsonl_files(&sources.codex, &sources.home, &mut files)?;
    files.sort();
    let since = sources
        .now
        .checked_sub(RECENT)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut live = Vec::new();
    for file in files {
        let shown = tilde(&file, &sources.home);
        let modified = match fs::metadata(&file).and_then(|metadata| metadata.modified()) {
            Ok(modified) => modified,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("read when {shown} was written"));
            }
        };
        if modified < since {
            continue;
        }
        let Some(line) = first_line(&file).with_context(|| format!("read {shown}"))? else {
            continue;
        };
        live.push(session_meta(&line).with_context(|| shown.clone())?);
    }
    Ok(live)
}

/// Every `*.jsonl` file under `dir`, at any depth, into `files`; symbolic links are not followed.
/// A directory that is not there holds none.
fn jsonl_files(dir: &Path, home: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("list {}", tilde(dir, home))),
    };
    for entry in entries {
        let entry = entry.with_context(|| format!("list {}", tilde(dir, home)))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .with_context(|| format!("read the type of {}", tilde(&path, home)))?;
        if kind.is_dir() {
            jsonl_files(&path, home, files)?;
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "jsonl")
        {
            files.push(path);
        }
    }
    Ok(())
}

/// The first line of `file` with its line break, or `None` when the file is gone or its first
/// line is not written to its end yet.
fn first_line(file: &Path) -> Result<Option<Vec<u8>>> {
    let opened = match File::open(file) {
        Ok(opened) => opened,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut line = Vec::new();
    BufReader::new(opened.take(FIRST_LINE as u64)).read_until(b'\n', &mut line)?;
    if line.ends_with(b"\n") {
        return Ok(Some(line));
    }
    if line.len() >= FIRST_LINE {
        bail!("its first line is longer than {FIRST_LINE} bytes");
    }
    Ok(None)
}

/// The thread a Codex file's first line, `line`, describes. An error names nothing the line holds.
fn session_meta(line: &[u8]) -> Result<Live> {
    let record: Value = serde_json::from_slice(line).context("its first line is no JSON")?;
    if record["type"] != "session_meta" {
        bail!("its first record is no `session_meta` record");
    }
    let payload = &record["payload"];
    let text = |key: &str| payload[key].as_str().filter(|text| !text.is_empty());
    let reference = text("id").context("its `session_meta` record carries no `payload.id`")?;
    let cwd = text("cwd").context("its `session_meta` record carries no `payload.cwd`")?;
    let activity = if payload["thread_source"] == "subagent" {
        SessionState::Background
    } else {
        SessionState::Unknown
    };
    Ok(Live {
        harness: Harness::Codex,
        reference: reference.to_owned(),
        name: None,
        cwd: cwd.to_owned(),
        activity,
    })
}

/// Where the working directory `cwd` lies: the `cwd` to record, with the home directory written
/// `~`, and the repository it is bound to ([`collect::repository_of`]) by its steps under the
/// checkouts root or under the managed trees, whichever of the two is deeper when both hold it;
/// in the records directory and neither of those, no repository. `None` when it lies under none
/// of the three, is no absolute path free of `..`, or holds a step that is not UTF-8.
fn place(cwd: &str, places: &Places<'_>) -> Option<(String, Option<String>)> {
    let cwd = Path::new(cwd);
    let repository = match root_of(cwd, places) {
        Some((under, rest)) => {
            collect::repository_of(places.root, places.trees, under, &steps(rest)?)
        }
        None if conductors(cwd, places) => None,
        None => return None,
    };
    Some((shown(cwd, places.home)?, repository))
}

/// The working directory `cwd` as a session row records it: the home directory written `~`, each
/// step after it joined by `/`. `None` when a step is not UTF-8.
fn shown(cwd: &Path, home: &Path) -> Option<String> {
    let (start, under) = match cwd.strip_prefix(home) {
        Ok(under) => ("~", under),
        Err(_) => ("", cwd),
    };
    Some(
        std::iter::once(start)
            .chain(steps(under)?)
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// Whether `cwd` is an absolute path free of `..`, which a session row may record.
fn is_plain(cwd: &Path) -> bool {
    cwd.is_absolute() && !cwd.components().any(|part| part == Component::ParentDir)
}

/// Which root the working directory `cwd` lies under, the deeper of the two when both hold it,
/// and its path under that root; `None` when it lies under neither or is no absolute path free
/// of `..`.
fn root_of<'c>(cwd: &'c Path, places: &Places<'_>) -> Option<(Under, &'c Path)> {
    if !cwd.is_absolute() || cwd.components().any(|part| part == Component::ParentDir) {
        return None;
    }
    let roots = [
        (Under::Checkouts, places.root),
        (Under::Trees, places.trees),
    ];
    roots
        .into_iter()
        .filter_map(|(under, dir)| Some((dir, under, cwd.strip_prefix(dir).ok()?)))
        .max_by_key(|(dir, _, _)| dir.components().count())
        .map(|(_, under, rest)| (under, rest))
}

/// Whether the working directory `cwd` is the instance's records directory or lies under it,
/// step by step: a sibling whose name only begins with the records directory's is not. No
/// records directory, one that is the home directory or above it, or a `cwd` that is no absolute
/// path free of `..`, holds no such session.
fn conductors(cwd: &Path, places: &Places<'_>) -> bool {
    cwd.is_absolute()
        && !cwd.components().any(|part| part == Component::ParentDir)
        && places.records.is_some_and(|records| {
            records.is_absolute() && !places.home.starts_with(records) && cwd.starts_with(records)
        })
}

/// Whether the working directory `cwd` lies under one of the `exclude` entries, each a path
/// under the checkouts root: a repository it names is under it there and in the managed trees.
fn excluded(cwd: &str, places: &Places<'_>, exclude: &[String]) -> bool {
    !exclude.is_empty()
        && root_of(Path::new(cwd), places)
            .and_then(|(_, rest)| steps(rest))
            .is_some_and(|steps| collect::is_excluded(&steps, exclude))
}

/// The steps of `path` past its root, or `None` when one is not UTF-8.
fn steps(path: &Path) -> Option<Vec<&str>> {
    path.components()
        .filter_map(|part| match part {
            Component::Normal(part) => Some(part.to_str()),
            _ => None,
        })
        .collect()
}

/// `path` with `home` written `~`.
fn tilde(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

fn command_line(command: &[OsString]) -> String {
    command
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use conductor_model::observation::SessionState;

    use super::{Places, claude_activity, conductors, place};

    #[test]
    fn a_claude_session_reads_its_status_before_its_state() {
        let cases = [
            (Some("busy"), Some("working"), SessionState::Busy),
            (Some("idle"), Some("blocked"), SessionState::Idle),
            (Some("waiting"), None, SessionState::Waiting),
            (Some("background"), None, SessionState::Background),
            (None, Some("working"), SessionState::Busy),
            (None, Some("blocked"), SessionState::Waiting),
            (Some("Busy"), None, SessionState::Busy),
            (
                Some("something-new"),
                Some("working"),
                SessionState::Unknown,
            ),
            (None, None, SessionState::Unknown),
        ];
        for (status, state, activity) in cases {
            assert_eq!(
                claude_activity(status, state),
                activity,
                "status {status:?}, state {state:?}"
            );
        }
    }

    /// Placed against the checkouts root `/h/src` and the managed trees under
    /// `/h/.local/state/worktree/trees/acme`, the home directory `/h`.
    fn placed(cwd: &str) -> Option<(String, Option<String>)> {
        place(
            cwd,
            &Places {
                home: Path::new("/h"),
                root: Path::new("/h/src"),
                trees: Path::new("/h/.local/state/worktree/trees/acme"),
                records: None,
            },
        )
    }

    fn at(shown: &str, repository: Option<&str>) -> Option<(String, Option<String>)> {
        Some((shown.to_owned(), repository.map(str::to_owned)))
    }

    #[test]
    fn a_working_directory_is_placed_against_the_checkouts_and_their_managed_trees() {
        assert_eq!(placed("/h/src"), at("~/src", None));
        assert_eq!(placed("/h/src/"), at("~/src", None));
        assert_eq!(
            placed("/h/src/conductor"),
            at("~/src/conductor", Some("conductor"))
        );
        assert_eq!(
            placed("/h/src/ess/crates/x"),
            at("~/src/ess/crates/x", Some("ess"))
        );
        assert_eq!(
            placed("/h/.local/state/worktree/trees/acme/ess/batch-0-52"),
            at(
                "~/.local/state/worktree/trees/acme/ess/batch-0-52",
                Some("ess")
            )
        );
        for outside in [
            "/h",
            "/h/elsewhere/project",
            "/h/src-old/x",
            "/h/src/../elsewhere",
            "/h/.local/state/worktree/trees/other/ess",
            "/other/src/conductor",
            "src/conductor",
            "",
        ] {
            assert_eq!(placed(outside), None, "{outside:?}");
        }
    }

    #[test]
    fn checkouts_outside_the_home_directory_are_shown_whole_and_the_deeper_place_wins() {
        let places = Places {
            home: Path::new("/h"),
            root: Path::new("/srv/src"),
            trees: Path::new("/srv/src/.trees"),
            records: None,
        };
        assert_eq!(
            place("/srv/src/alpha/x", &places),
            at("/srv/src/alpha/x", Some("alpha"))
        );
        assert_eq!(
            place("/srv/src/.trees/beta/fix-1", &places),
            at("/srv/src/.trees/beta/fix-1", Some("beta"))
        );
        assert_eq!(place("/h/src/alpha", &places), None);
    }

    /// The places of [`placed`], with the records directory `records`.
    fn with_records(records: &str) -> Places<'_> {
        Places {
            home: Path::new("/h"),
            root: Path::new("/h/src"),
            trees: Path::new("/h/.local/state/worktree/trees/acme"),
            records: Some(Path::new(records)),
        }
    }

    #[test]
    fn the_records_directory_places_conductors_session_and_nothing_beside_it() {
        let places = with_records("/h/.b10x/conductor/acme/records");
        for inside in [
            "/h/.b10x/conductor/acme/records",
            "/h/.b10x/conductor/acme/records/",
            "/h/.b10x/conductor/acme/records/docs/handoff",
        ] {
            assert!(conductors(Path::new(inside), &places), "{inside:?}");
            assert!(place(inside, &places).is_some_and(|(_, repository)| repository.is_none()));
        }
        assert_eq!(
            place("/h/.b10x/conductor/acme/records/docs", &places),
            at("~/.b10x/conductor/acme/records/docs", None)
        );
        for beside in [
            "/h/.b10x/conductor/acme",
            "/h/.b10x/conductor/acme/records-old",
            "/h/.b10x/conductor/acme/state",
            "/h/.b10x/conductor/acme/records/../state",
            "h/.b10x/conductor/acme/records",
            "",
        ] {
            assert!(!conductors(Path::new(beside), &places), "{beside:?}");
            assert_eq!(place(beside, &places), None, "{beside:?}");
        }
        assert!(!conductors(Path::new("/h/src/alpha"), &places));
    }

    #[test]
    fn a_records_directory_at_or_above_the_home_directory_places_nothing() {
        for records in ["/h", "/h/", "/", "h/records"] {
            let places = with_records(records);
            assert!(
                !conductors(Path::new("/h/elsewhere"), &places),
                "{records:?}"
            );
            assert_eq!(place("/h/elsewhere", &places), None, "{records:?}");
        }
    }
}
