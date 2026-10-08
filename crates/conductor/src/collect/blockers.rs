//! The `blockers` collector: open blockers across every planning store of the instance, read at
//! each repository's `origin/main`.
//!
//! Filled by `story:collect-blockers`, in this file only. The instance's repositories are the
//! members of a workspace file `.engineering/workspace.yaml` (`aep.workspace/1`): the one under the
//! instance's `records` when a config file names the instance, else conductor's own, the nearest
//! at or above the working directory, whose members are checkouts beside conductor's,
//! `source: ../../<repo>` ([`Sources::of`]). A checkout's working tree and `HEAD` can lag
//! `origin/main` (on 2026-10-07, two of the eight decision-blockers read from one checkout had been
//! cleared on `origin/main`), so the collector reads neither. [`read`] fetches each checkout's
//! `origin`, exports what the collectors read of its `origin/main` with `git archive` into
//! `<exports>/<repo>/`, writes `<exports>/.engineering/workspace.yaml` naming the exports, and lists
//! that workspace with `aep plan workspace list`. `<exports>` is `<cache>/exports` of the instance
//! when a config file names it, else `<state>/exports`, which git ignores, `<state>` being the
//! state directory the recorder gives ([`Recorder::state`]); never the working directory. Nothing
//! runs in a checkout's working tree, and nothing builds.
//!
//! **What is exported**, found with `git ls-tree -r --name-only origin/main`: the planning store
//! `.engineering/`, and what the specifications collector reads
//! ([`specifications::read_paths`]): `AGENTS.md`, each specification root and each path an input
//! manifest lists. A repository with none of these exports to an empty directory.
//!
//! **One fetch and one export per repository per snapshot.** Once every export is written,
//! [`read`] marks them with the snapshot's id in `<exports>/.snapshot`. A later [`read`] in
//! the same snapshot (the specifications collector, after this one) finds its own snapshot's mark
//! and an export for every member, and lists those exports again without fetching or exporting.
//! Any other mark, or none, and it fetches and exports itself; the mark is removed before the
//! first export, so a half-written set never carries one.
//!
//! A blocker is an artifact at status `open` whose kind, its id up to the first `:`, is lower-case
//! letters and hyphens ending in `blocker` (`blocker`, `decision-blocker`, `operator-blocker`, …):
//! the selection `jq '.id | test("^[a-z-]*blocker:")'` makes. Each is recorded with its
//! repository, the checkout's directory name (a member's name cannot always be it:
//! `acme.github.io` is the member `acme-github-io`), its id as the reference, its kind,
//! and its title, empty when it has none.
//!
//! A repository whose fetch or export fails, and a member list or listing that fails, is tried once
//! more; failing again, the collector answers an error naming the repository and the command. The
//! specifications collector reads the same exports and listing through [`read`].

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::observation::{RecordBlocker, RepositoryName, SnapshotId};
use serde_json::Value;

use crate::collect::{self, specifications};
use crate::config::{self, Active};
use crate::snapshot::Recorder;

/// The workspace file, under the repository that holds it.
pub const WORKSPACE: &str = ".engineering/workspace.yaml";

/// The file under the exports that names the snapshot whose exports they are.
pub const MARK: &str = ".snapshot";

/// The planning store's directory, which this collector reads in each export.
const STORE: &str = ".engineering";

/// The commit every store and specification is read at.
const MAIN: &str = "origin/main";

/// The status of a blocker that still blocks.
const OPEN: &str = "open";

/// What the collector reads, the one seam a test points at fixtures. [`Sources::new`] gives the
/// real ones.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The directory holding [`WORKSPACE`], whose members are the checkouts.
    pub root: PathBuf,
    /// The directory each member's `origin/main` is exported under, as `<repo>/`, beside the
    /// exports' own [`WORKSPACE`]. An absolute path.
    pub exports: PathBuf,
    /// The program and arguments that bring a checkout's `origin/main` up to date, run in the
    /// checkout.
    pub fetch: Vec<OsString>,
    /// The program and arguments that print the members of `root`'s workspace as JSON
    /// (`aep plan workspace members`).
    pub members: Vec<OsString>,
    /// The program and arguments that print every artifact of the exports' workspace as JSON
    /// (`aep plan workspace list`).
    pub list: Vec<OsString>,
    /// How long one command may run before it is stopped.
    pub bound: Duration,
}

impl Sources {
    /// [`Sources::exporting`] under `state/exports/`. `state` is an absolute path.
    #[must_use]
    pub fn new(root: PathBuf, state: &Path) -> Self {
        Self::exporting(root, state.join("exports"))
    }

    /// The sources of the instance `active` names, exporting under the state directory `state`
    /// only without a config file. With a file, the workspace file under the instance's
    /// `records` and the exports under `<cache>/exports/`; without one, today's: the workspace of
    /// the nearest directory at or above the working directory that holds [`WORKSPACE`] ([`root`])
    /// and the exports under `state/exports/`.
    ///
    /// # Errors
    ///
    /// Without a config file, what [`root`] answers.
    pub fn of(active: &Active, state: &Path) -> Result<Self> {
        if active.from_file {
            Ok(Self::exporting(
                PathBuf::from(&active.instance.records),
                PathBuf::from(&active.instance.cache).join("exports"),
            ))
        } else {
            Ok(Self::new(root()?, state))
        }
    }

    /// `git fetch --quiet origin` in each checkout; `aep plan workspace members` over `root` and
    /// `aep plan workspace list` over the exports, as JSON; the exports under `exports`, an
    /// absolute path; and [`collect::BOUND`] for each command.
    #[must_use]
    pub fn exporting(root: PathBuf, exports: PathBuf) -> Self {
        let aep = |verb: &str, over: &Path| {
            let mut words = words(&[
                "aep",
                "plan",
                "workspace",
                verb,
                "--format",
                "json",
                "--root",
            ]);
            words.push(over.as_os_str().to_owned());
            words
        };
        Self {
            fetch: words(&["git", "fetch", "--quiet", "origin"]),
            members: aep("members", &root),
            list: aep("list", &exports),
            bound: collect::BOUND,
            exports,
            root,
        }
    }
}

/// Records one `RecordBlocker` per open blocker artifact into the recorder's snapshot, over the
/// workspace of the instance the process runs ([`Sources::of`] over [`config::active`]),
/// exporting under its cache, or under the recorder's state directory ([`Recorder::state`])
/// without a config file.
///
/// # Errors
///
/// What [`Sources::of`] answers, the recorder has no state directory, or what [`collect_from`]
/// answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    let sources = Sources::of(config::active(), record.state()?)?;
    collect_from(&sources, record)
}

/// Records one `RecordBlocker` per open blocker artifact the stores `sources` name hold at
/// `origin/main`.
///
/// # Errors
///
/// What [`read`] answers, or a blocker the recorder refused.
pub fn collect_from(sources: &Sources, record: &mut Recorder<'_>) -> Result<()> {
    let snapshot = record.snapshot().clone();
    let workspace = read(sources, &snapshot)?;
    for artifact in &workspace.artifacts {
        let kind = artifact.kind();
        if artifact.status != OPEN || !is_blocker(kind) {
            continue;
        }
        let repository = workspace.repository(&artifact.member)?;
        record
            .blocker(RecordBlocker {
                snapshot_id: record.snapshot().clone(),
                repository: RepositoryName(repository.to_owned()),
                reference: artifact.id.clone(),
                blocker_kind: kind.to_owned(),
                title: artifact.title.clone().unwrap_or_default(),
            })
            .with_context(|| format!("record {} of {repository}", artifact.id))?;
    }
    Ok(())
}

/// Whether `kind` names a blocker: lower-case letters and hyphens, ending in `blocker`.
fn is_blocker(kind: &str) -> bool {
    kind.ends_with("blocker")
        && kind
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
}

/// The nearest directory at or above the working directory that holds [`WORKSPACE`].
///
/// # Errors
///
/// The working directory cannot be read, or no such directory is there.
pub(crate) fn root() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("read the working directory")?;
    cwd.ancestors()
        .find(|dir| dir.join(WORKSPACE).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("no {WORKSPACE} in {} or above it", cwd.display()))
}

/// The organization's workspace, read at each member's `origin/main`.
#[derive(Debug)]
pub(crate) struct Workspace {
    /// Every member, in the workspace file's order.
    pub(crate) members: Vec<Member>,
    /// Every artifact of every member's store, as `aep plan workspace list` orders them.
    pub(crate) artifacts: Vec<Artifact>,
}

/// One member of the workspace.
#[derive(Debug)]
pub(crate) struct Member {
    /// Its name in the workspace.
    pub(crate) name: String,
    /// The repository: the last component of its `source`, the checkout's directory name.
    pub(crate) repository: String,
    /// Where its `origin/main` is exported.
    pub(crate) export: PathBuf,
}

/// One artifact of a member's store.
#[derive(Debug)]
pub(crate) struct Artifact {
    /// The member whose store holds it.
    pub(crate) member: String,
    /// Its id, `<kind>:<slug>`.
    pub(crate) id: String,
    /// Its status.
    pub(crate) status: String,
    /// Its title, when it has one.
    pub(crate) title: Option<String>,
}

impl Artifact {
    /// Its kind: its id up to the first `:`.
    pub(crate) fn kind(&self) -> &str {
        self.id.split_once(':').map_or("", |(kind, _)| kind)
    }
}

impl Workspace {
    /// The repository of the member `name`.
    fn repository(&self, name: &str) -> Result<&str> {
        self.members
            .iter()
            .find(|member| member.name == name)
            .map(|member| member.repository.as_str())
            .ok_or_else(|| anyhow!("the listing names a member {name} the workspace does not hold"))
    }
}

/// The workspace of `sources.root` at `origin/main`, for `snapshot`: each member fetched and
/// exported, unless the exports already carry `snapshot`'s mark and hold every member; the
/// exports' workspace file written; and that workspace listed.
///
/// # Errors
///
/// A member list, export or listing that failed twice, naming the repository and the command; an
/// answer that is not the JSON `aep` prints; or a file that cannot be written or removed.
pub(crate) fn read(sources: &Sources, snapshot: &SnapshotId) -> Result<Workspace> {
    let listed = twice(|| json(&sources.members, None, sources.bound)).with_context(|| {
        format!(
            "list the members of {}, tried twice",
            sources.root.join(WORKSPACE).display()
        )
    })?;
    let mut members = Vec::new();
    let mut checkouts = Vec::new();
    for (name, repository, checkout) in members_of(&listed)? {
        if let Some(other) = members
            .iter()
            .find(|member: &&Member| member.repository == repository)
        {
            bail!(
                "the members {} and {name} both name the repository {repository}",
                other.name
            );
        }
        members.push(Member {
            name,
            export: sources.exports.join(&repository),
            repository,
        });
        checkouts.push(checkout);
    }
    if members.is_empty() {
        // No repository has a planning store: there is nothing to export or list, and `aep`
        // refuses a workspace of no members (an instance with no store, 2026-10-08).
        return Ok(Workspace {
            artifacts: Vec::new(),
            members,
        });
    }
    let mark = sources.exports.join(MARK);
    let marked = fs::read_to_string(&mark).is_ok_and(|held| held == snapshot.0.0);
    if !marked || !members.iter().all(|member| member.export.is_dir()) {
        remove(&mark)?;
        for (member, checkout) in members.iter().zip(&checkouts) {
            twice(|| export(sources, &member.repository, checkout))
                .with_context(|| format!("repository {}, tried twice", member.repository))?;
        }
        fs::create_dir_all(&sources.exports)
            .with_context(|| format!("create {}", sources.exports.display()))?;
        fs::write(&mark, &snapshot.0.0).with_context(|| format!("write {}", mark.display()))?;
    }
    write_workspace(&sources.exports, &members)?;
    let listing = twice(|| json(&sources.list, None, sources.bound)).with_context(|| {
        format!(
            "list the workspace of {}, tried twice",
            sources.exports.display()
        )
    })?;
    Ok(Workspace {
        artifacts: artifacts_of(&listing)?,
        members,
    })
}

/// Each member `aep plan workspace members` printed: its name, its repository, and its checkout.
fn members_of(listed: &Value) -> Result<Vec<(String, String, PathBuf)>> {
    listed["members"]
        .as_array()
        .ok_or_else(|| anyhow!("the member list has no `members` array"))?
        .iter()
        .map(|member| {
            let name = field(member, "name")?;
            let source = field(member, "source")?;
            let repository = Path::new(source)
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| anyhow!("the source {source:?} of {name} names no repository"))?;
            Ok((
                name.to_owned(),
                repository.to_owned(),
                PathBuf::from(field(member, "tree")?),
            ))
        })
        .collect()
}

/// Every artifact `aep plan workspace list` printed.
fn artifacts_of(listing: &Value) -> Result<Vec<Artifact>> {
    listing["artifacts"]
        .as_array()
        .ok_or_else(|| anyhow!("the workspace listing has no `artifacts` array"))?
        .iter()
        .map(|artifact| {
            Ok(Artifact {
                member: field(artifact, "member")?.to_owned(),
                id: field(artifact, "id")?.to_owned(),
                status: field(artifact, "status")?.to_owned(),
                title: artifact["title"].as_str().map(str::to_owned),
            })
        })
        .collect()
}

/// The text of `value`'s field `name`.
fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value[name]
        .as_str()
        .ok_or_else(|| anyhow!("an entry has no text `{name}`: {value}"))
}

/// Fetches `checkout`'s `origin` and exports what the collectors read of its `origin/main`
/// ([`exported`]) into `<exports>/<repository>/`, replacing what an earlier export left there.
fn export(sources: &Sources, repository: &str, checkout: &Path) -> Result<PathBuf> {
    succeed(&sources.fetch, Some(checkout), sources.bound)?;
    let mut list = words(&["git", "ls-tree", "-r", "-z", "--name-only"]);
    list.push(OsString::from(MAIN));
    let files: Vec<String> =
        String::from_utf8_lossy(&succeed(&list, Some(checkout), sources.bound)?)
            .split('\0')
            .filter(|file| !file.is_empty())
            .map(str::to_owned)
            .collect();
    let paths = exported(&files, |manifest| {
        let mut show = words(&["git", "show"]);
        show.push(OsString::from(format!("{MAIN}:{manifest}")));
        let text = succeed(&show, Some(checkout), sources.bound)?;
        Ok(String::from_utf8_lossy(&text).into_owned())
    })?;

    fs::create_dir_all(&sources.exports)
        .with_context(|| format!("create {}", sources.exports.display()))?;
    let export = sources.exports.join(repository);
    if fs::symlink_metadata(&export).is_ok() {
        fs::remove_dir_all(&export)
            .with_context(|| format!("remove the earlier export {}", export.display()))?;
    }
    fs::create_dir_all(&export).with_context(|| format!("create {}", export.display()))?;
    if paths.as_ref().is_some_and(Vec::is_empty) {
        return Ok(export);
    }
    let tar = sources.exports.join(format!("{repository}.tar"));
    let mut archive = words(&["git", "archive", "--format=tar", "-o"]);
    archive.push(tar.clone().into_os_string());
    archive.push(OsString::from(MAIN));
    if let Some(paths) = paths {
        archive.push(OsString::from("--"));
        archive.extend(paths.into_iter().map(OsString::from));
    }
    succeed(&archive, Some(checkout), sources.bound)?;
    let mut extract = words(&["tar", "-x", "-f"]);
    extract.push(tar.clone().into_os_string());
    extract.push(OsString::from("-C"));
    extract.push(export.clone().into_os_string());
    let extracted = succeed(&extract, None, sources.bound);
    fs::remove_file(&tar).with_context(|| format!("remove {}", tar.display()))?;
    extracted?;
    Ok(export)
}

/// The paths of a repository whose `origin/main` holds `files` that the collectors read, sorted:
/// the planning store [`STORE`] when it has one, and what [`specifications::read_paths`] names,
/// whose input manifests `manifest` reads. `None` when the whole tree is read: a specification
/// root at the repository's top.
fn exported(
    files: &[String],
    manifest: impl FnMut(&str) -> Result<String>,
) -> Result<Option<Vec<String>>> {
    let Some(mut paths) = specifications::read_paths(files, manifest)? else {
        return Ok(None);
    };
    if files.iter().any(|file| {
        file.strip_prefix(STORE)
            .is_some_and(|rest| rest.starts_with('/'))
    }) {
        paths.insert(STORE.to_owned());
    }
    Ok(Some(paths.into_iter().collect()))
}

/// Removes `file`; one that is not there is removed already.
fn remove(file: &Path) -> Result<()> {
    match fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", file.display())),
    }
}

/// Writes the exports' workspace file: each member under its own name, its source the export.
fn write_workspace(exports: &Path, members: &[Member]) -> Result<()> {
    let mut text = String::from("version: aep.workspace/1\n");
    if members.is_empty() {
        text.push_str("members: []\n");
    } else {
        text.push_str("members:\n");
        for member in members {
            text.push_str(&format!(
                "  - name: {}\n    source: {}\n",
                Value::from(member.name.as_str()),
                Value::from(format!("../{}", member.repository))
            ));
        }
    }
    let file = exports.join(WORKSPACE);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(&file, text).with_context(|| format!("write {}", file.display()))
}

/// What `attempt` answers, or, when it fails, what it answers the second time.
pub(crate) fn twice<T>(mut attempt: impl FnMut() -> Result<T>) -> Result<T> {
    attempt().or_else(|_| attempt())
}

/// `words` as the program and arguments of a command.
pub(crate) fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// The command line `words`, as an error shows it.
pub(crate) fn shown(words: &[OsString]) -> String {
    words
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs the command `words`, in `dir` when it is given, through [`collect::run`] within `bound`.
///
/// # Errors
///
/// `words` is empty, or the command did not start or answer in time; the error names it.
pub(crate) fn run(words: &[OsString], dir: Option<&Path>, bound: Duration) -> Result<Output> {
    let (program, arguments) = words.split_first().context("an empty command line")?;
    let mut command = Command::new(program);
    command.args(arguments);
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    collect::run(&mut command, bound).with_context(|| format!("`{}`", shown(words)))
}

/// What [`run`] answers, as its standard output, when the command exited 0.
///
/// # Errors
///
/// What [`run`] answers, or the exit status was not 0; the error names the command.
pub(crate) fn succeed(words: &[OsString], dir: Option<&Path>, bound: Duration) -> Result<Vec<u8>> {
    let output = run(words, dir, bound)?;
    if !output.status.success() {
        return Err(failed(words, &output));
    }
    Ok(output.stdout)
}

/// The error for the command `words`, which answered `output` and not as it should.
pub(crate) fn failed(words: &[OsString], output: &Output) -> anyhow::Error {
    anyhow!(
        "`{}` {}: {}",
        shown(words),
        exited(output.status),
        one_line(&String::from_utf8_lossy(&output.stderr))
    )
}

/// What [`succeed`] answers, read as JSON.
fn json(words: &[OsString], dir: Option<&Path>, bound: Duration) -> Result<Value> {
    let stdout = succeed(words, dir, bound)?;
    serde_json::from_slice(&stdout).with_context(|| format!("`{}` printed no JSON", shown(words)))
}

fn exited(status: ExitStatus) -> String {
    status.code().map_or_else(
        || "was stopped by a signal".to_owned(),
        |code| format!("exited {code}"),
    )
}

/// `text` on one line: every line break a space.
fn one_line(text: &str) -> String {
    text.split(['\n', '\r'])
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::is_blocker;

    #[test]
    fn a_blocker_kind_is_lower_case_letters_and_hyphens_ending_in_blocker() {
        for kind in [
            "blocker",
            "decision-blocker",
            "operator-blocker",
            "credential-blocker",
        ] {
            assert!(is_blocker(kind), "{kind}");
        }
        for kind in [
            "",
            "story",
            "blockers",
            "Decision-blocker",
            "v2-blocker",
            "blocker-note",
        ] {
            assert!(!is_blocker(kind), "{kind}");
        }
    }
}
