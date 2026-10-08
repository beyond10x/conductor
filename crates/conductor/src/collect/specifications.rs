//! The `specifications` collector, which feeds the `snapshot specifications` view through
//! `snapshot record-specification`'s command (both in [`crate::snapshot`]).
//!
//! The collector is filled by `story:collect-specifications`, in this file only. It records one
//! `RecordSpecification` per repository each GitHub owner of the instance lists that is not
//! archived (`gh repo list`), read in the export of its `origin/main` that [`blockers::read`]
//! makes under `<exports>/<repo>/`: `<cache>/exports` of the instance when a config file names it,
//! else `<state>/exports`, `<state>` being the recorder's state directory ([`Recorder::state`]).
//! A local source has no GitHub list, so its repositories are not listed here. A name two owners
//! list fails the collector naming it and both owners. Run after the blockers collector in one
//! snapshot, it reads that collector's exports and fetches nothing. The export holds what
//! [`read_paths`] names. It never runs in a checkout, and never builds.
//!
//! - `presence`: `Present` when the export holds a file named `ess-inputs.yaml` or `system.yaml`
//!   outside every `target/` and `node_modules/` directory; else `OptedOut` when its `AGENTS.md`
//!   holds one of [`OPT_OUT_PHRASES`]; else `Missing`.
//! - `path`: the specification root, relative to the repository, `.` for its top: the shallowest
//!   directory holding an `ess-inputs.yaml`, else the shallowest holding a `system.yaml`, the first
//!   by name among equals.
//! - `format`: the top-level `format:` of the root's `system.yaml`, or else of the `system.yaml`
//!   the root's `ess-inputs.yaml` lists under `specification:`. `required_ess`: the top-level
//!   `requires:` of the root's `ess-inputs.yaml`. Both are read as `key: value` lines; there is no
//!   YAML parser.
//! - `validation`, `validation_refusals`: `ess specify validate --path <root>`. Exit 0 is `Valid`;
//!   exit 1 is `Refused`, with one refusal per problem line it printed, and at least one. `NotRun`
//!   when not `Present`.
//! - `scenarios`, `synthesis_refusals`: when `Valid`, the last line of
//!   `ess verify conform synthesize --path <root> --out <state>/specs/<repo>.json`,
//!   `N scenario(s) (A authored), M refusal(s), written to …`. A refused specification does not
//!   synthesize, so both are absent then.
//! - `conformance_status`: the status of the repository's `executable-system-specification`
//!   artifacts in the workspace listing; when they differ, each status in id order, joined by `, `.
//!   Absent when it has none.
//!
//! A command that fails (another exit status, no answer in time, no summary line) is tried once
//! more; failing again, the collector answers an error naming the repository and the command. A
//! repository that is not archived and is no member of the workspace is an error naming it, raised
//! before anything is recorded.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use conductor_model::observation::{
    RecordSpecification, RepositoryName, SnapshotId, SpecificationPresence, ValidationResult,
};
use serde_json::Value;

use crate::collect::blockers::{self, Member, Workspace};
use crate::collect::repositories::one_of_each;
use crate::collect::{self, Origins};
use crate::config::{self, Active};
use crate::snapshot::Recorder;

/// The phrases an `AGENTS.md` records an ESS opt-out with. Each is matched in lower case, with
/// every run of white space read as one space, as whole words: no letter or digit right before or
/// after it.
pub const OPT_OUT_PHRASES: [&str; 3] = ["ess opt-out", "opts out of ess", "opted out of ess"];

/// An input manifest's file name.
const INPUTS: &str = "ess-inputs.yaml";

/// A system file's name.
const SYSTEM: &str = "system.yaml";

/// The file whose opt-out phrases are read.
const AGENTS: &str = "AGENTS.md";

/// The kind of the artifact whose status is a repository's conformance status.
const SPECIFICATION_ARTIFACT: &str = "executable-system-specification";

/// What the collector reads, the one seam a test points at fixtures. [`Sources::of`] gives an
/// instance's.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The workspace, its exports and its listing, read as the blockers collector reads them.
    pub workspace: blockers::Sources,
    /// The program and arguments that print one owner's repositories as a JSON array of
    /// `{"name", "isArchived"}`; `{organization}` in a word is filled with the owner.
    pub repositories: Vec<OsString>,
    /// The owners whose repositories are listed, each once through [`Sources::repositories`], in
    /// this order.
    pub owners: Vec<String>,
    /// The program, and any arguments, that `specify validate …` and
    /// `verify conform synthesize …` are appended to.
    pub ess: Vec<OsString>,
    /// The directory synthesis writes `<repo>.json` into.
    pub specs: PathBuf,
    /// How long one command may run before it is stopped.
    pub bound: Duration,
}

impl Sources {
    /// [`Sources::over`] the blockers collector's sources over `root` and `state` and the
    /// built-in instance's owners ([`Origins::built_in`]). `state` is an absolute path.
    #[must_use]
    pub fn new(root: PathBuf, state: &Path) -> Self {
        Self::over(
            blockers::Sources::new(root, state),
            Origins::built_in().owners,
            state,
        )
    }

    /// [`Sources::over`] the blockers collector's sources of the instance `active` names
    /// ([`blockers::Sources::of`]) and its owners. `state` is an absolute path.
    ///
    /// # Errors
    ///
    /// What [`blockers::Sources::of`] answers.
    pub fn of(active: &Active, state: &Path) -> Result<Self> {
        Ok(Self::over(
            blockers::Sources::of(active, state)?,
            Origins::of(&active.instance).owners,
            state,
        ))
    }

    /// `workspace`; each of `owners` through
    /// `gh repo list {organization} --limit 200 --json name,isArchived`; the installed `ess`;
    /// synthesis under `state/specs/`; and [`collect::BOUND`] for each command.
    #[must_use]
    pub fn over(workspace: blockers::Sources, owners: Vec<String>, state: &Path) -> Self {
        Self {
            workspace,
            repositories: blockers::words(&[
                "gh",
                "repo",
                "list",
                "{organization}",
                "--limit",
                "200",
                "--json",
                "name,isArchived",
            ]),
            owners,
            ess: blockers::words(&["ess"]),
            specs: state.join("specs"),
            bound: collect::BOUND,
        }
    }
}

/// Records one `RecordSpecification` per non-archived repository of each GitHub owner of the
/// instance the process runs into the recorder's snapshot, over that instance's workspace
/// ([`Sources::of`] over [`config::active`]), writing under the recorder's state directory
/// ([`Recorder::state`]).
///
/// # Errors
///
/// What [`Sources::of`] answers, the recorder has no state directory, or what [`collect_from`]
/// answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    let sources = Sources::of(config::active(), record.state()?)?;
    collect_from(&sources, record)
}

/// Records one `RecordSpecification` per repository the owners of `sources` list as not
/// archived, read in the export of its `origin/main`.
///
/// # Errors
///
/// An owner's repository list failed twice or is not the JSON `gh` prints; two owners list one
/// name; a repository is no member of the workspace; what [`blockers::read`] answers; a command
/// that failed twice, naming the repository and the command; or an observation the recorder
/// refused.
pub fn collect_from(sources: &Sources, record: &mut Recorder<'_>) -> Result<()> {
    // Each repository as `(name, owner)`, by name.
    let mut listed: Vec<(String, String)> = Vec::new();
    for owner in &sources.owners {
        let command = filled(&sources.repositories, owner);
        let answered = blockers::twice(|| {
            let stdout = blockers::succeed(&command, None, sources.bound)?;
            serde_json::from_slice::<Value>(&stdout)
                .with_context(|| format!("`{}` printed no JSON", blockers::shown(&command)))
        })
        .with_context(|| format!("list the repositories of {owner}, tried twice"))?;
        listed.extend(
            not_archived(&answered)?
                .into_iter()
                .map(|name| (name, owner.clone())),
        );
    }
    listed.sort();
    one_of_each(
        &listed,
        |(name, _)| name,
        |(_, owner)| format!("owner {owner}"),
    )?;
    let repositories: Vec<String> = listed.into_iter().map(|(name, _)| name).collect();
    let snapshot = record.snapshot().clone();
    let workspace = blockers::read(&sources.workspace, &snapshot)?;
    let members = repositories
        .iter()
        .map(|repository| {
            workspace
                .members
                .iter()
                .find(|member| &member.repository == repository)
                .ok_or_else(|| {
                    anyhow!(
                        "repository {repository} is not archived, and no member of {} has it as \
                         its source",
                        sources.workspace.root.join(blockers::WORKSPACE).display()
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;
    fs::create_dir_all(&sources.specs)
        .with_context(|| format!("create {}", sources.specs.display()))?;
    for member in members {
        let observed = observe(sources, &workspace, member, &snapshot)?;
        record
            .specification(observed)
            .with_context(|| format!("record the specification of {}", member.repository))?;
    }
    Ok(())
}

/// The names of the repositories `listed` holds that are not archived, sorted.
fn not_archived(listed: &Value) -> Result<Vec<String>> {
    let mut names = listed
        .as_array()
        .ok_or_else(|| anyhow!("the repository list is no JSON array"))?
        .iter()
        .filter(|repository| repository["isArchived"] != Value::Bool(true))
        .map(|repository| {
            repository["name"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("a listed repository has no name: {repository}"))
        })
        .collect::<Result<Vec<_>>>()?;
    names.sort();
    Ok(names)
}

/// `template` with `{organization}` in each word that holds it replaced by `owner`.
fn filled(template: &[OsString], owner: &str) -> Vec<OsString> {
    template
        .iter()
        .map(|word| match word.to_str() {
            Some(text) if text.contains("{organization}") => {
                OsString::from(text.replace("{organization}", owner))
            }
            _ => word.clone(),
        })
        .collect()
}

/// The specification status of `member`'s export.
fn observe(
    sources: &Sources,
    workspace: &Workspace,
    member: &Member,
    snapshot: &SnapshotId,
) -> Result<RecordSpecification> {
    let repository = &member.repository;
    let conformance_status = conformance(workspace, &member.name);
    let files = specification_files(&member.export)?;
    let Some(root) = root_of(&files) else {
        let presence = if opted_out(&member.export.join(AGENTS)) {
            SpecificationPresence::OptedOut
        } else {
            SpecificationPresence::Missing
        };
        return Ok(RecordSpecification {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(repository.clone()),
            presence,
            path: None,
            format: None,
            required_ess: None,
            validation: ValidationResult::NotRun,
            validation_refusals: 0,
            scenarios: None,
            synthesis_refusals: None,
            conformance_status,
        });
    };
    let dir = member.export.join(&root);
    let (validation, validation_refusals) = blockers::twice(|| validate(sources, &dir))
        .with_context(|| format!("repository {repository}, tried twice"))?;
    let (scenarios, synthesis_refusals) = if validation == ValidationResult::Valid {
        let out = sources.specs.join(format!("{repository}.json"));
        let (scenarios, refusals) = blockers::twice(|| synthesize(sources, &dir, &out))
            .with_context(|| format!("repository {repository}, tried twice"))?;
        (Some(scenarios), Some(refusals))
    } else {
        (None, None)
    };
    Ok(RecordSpecification {
        snapshot_id: snapshot.clone(),
        repository: RepositoryName(repository.clone()),
        presence: SpecificationPresence::Present,
        path: Some(shown_path(&root)),
        format: format_of(&member.export, &root),
        required_ess: top_level(&dir.join(INPUTS), "requires"),
        validation,
        validation_refusals,
        scenarios,
        synthesis_refusals,
        conformance_status,
    })
}

/// Every file named `ess-inputs.yaml` or `system.yaml` under `export`, relative to it, outside
/// every directory named `target` or `node_modules`. A symbolic link is not followed.
fn specification_files(export: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let dir = export.join(&relative);
        for entry in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
            let entry = entry.with_context(|| format!("read {}", dir.display()))?;
            let name = entry.file_name();
            let path = relative.join(&name);
            let kind = entry
                .file_type()
                .with_context(|| format!("read the type of {}", path.display()))?;
            if kind.is_dir() {
                if name != "target" && name != "node_modules" {
                    pending.push(path);
                }
            } else if name == INPUTS || name == SYSTEM {
                found.push(path);
            }
        }
    }
    Ok(found)
}

/// The specification root among `files`: the shallowest directory holding an input manifest,
/// else the shallowest holding a system file, the first by name among equals.
fn root_of(files: &[PathBuf]) -> Option<PathBuf> {
    let shallowest = |name: &str| {
        files
            .iter()
            .filter(|file| file.file_name() == Some(OsStr::new(name)))
            .map(|file| file.parent().map(Path::to_path_buf).unwrap_or_default())
            .min_by(|a, b| {
                (a.components().count(), a.as_path()).cmp(&(b.components().count(), b.as_path()))
            })
    };
    shallowest(INPUTS).or_else(|| shallowest(SYSTEM))
}

/// `root` as the observation shows a path: `/`-separated, `.` for the repository's top.
fn shown_path(root: &Path) -> String {
    if root.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        root.components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// The `format:` of the root's system file, or else of the system file the root's input manifest
/// lists under `specification:`, when that lies inside the export.
fn format_of(export: &Path, root: &Path) -> Option<String> {
    let system = export.join(root).join(SYSTEM);
    if system.is_file() {
        return top_level(&system, "format");
    }
    let inputs = fs::read_to_string(export.join(root).join(INPUTS)).ok()?;
    let listed = listed(&inputs, "specification")
        .into_iter()
        .find(|entry| Path::new(entry).file_name() == Some(OsStr::new(SYSTEM)))?;
    top_level(&export.join(inside(root, &listed)?), "format")
}

/// `entry`, a path relative to `root`, as a path relative to the export; `None` when it leaves it.
fn inside(root: &Path, entry: &str) -> Option<PathBuf> {
    let mut parts: Vec<&OsStr> = root.iter().collect();
    for part in Path::new(entry).components() {
        match part {
            Component::Normal(part) => parts.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.iter().collect())
}

/// The value of the top-level `key: value` line of `file`, unquoted, without a trailing comment;
/// `None` when the file or the line is not there, or the value is empty.
fn top_level(file: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(file).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(':').map(scalar))
        .filter(|value| !value.is_empty())
}

/// The entries of the top-level block list `key:` in `text`, each unquoted.
fn listed(text: &str, key: &str) -> Vec<String> {
    let heading = format!("{key}:");
    let mut lines = text
        .lines()
        .skip_while(|line| scalar_of_line(line) != heading);
    if lines.next().is_none() {
        return Vec::new();
    }
    lines
        .take_while(|line| line.is_empty() || line.starts_with([' ', '\t', '-', '#']))
        .filter_map(|line| line.trim_start().strip_prefix("- "))
        .map(scalar)
        .collect()
}

/// `line` without a trailing comment or trailing white space.
fn scalar_of_line(line: &str) -> &str {
    line.find(" #").map_or(line, |at| &line[..at]).trim_end()
}

/// A YAML scalar as a plain value: unquoted when quoted, else without a trailing comment.
fn scalar(value: &str) -> String {
    let value = value.trim();
    for quote in ['"', '\''] {
        if let Some(rest) = value.strip_prefix(quote)
            && let Some(end) = rest.find(quote)
        {
            return rest[..end].to_owned();
        }
    }
    if value.starts_with('#') {
        return String::new();
    }
    scalar_of_line(value).to_owned()
}

/// Whether the `AGENTS.md` at `file` holds one of [`OPT_OUT_PHRASES`].
fn opted_out(file: &Path) -> bool {
    let Ok(bytes) = fs::read(file) else {
        return false;
    };
    let text = String::from_utf8_lossy(&bytes)
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    OPT_OUT_PHRASES.iter().any(|phrase| mentions(&text, phrase))
}

/// Whether `text` holds `phrase` with no letter or digit right before or after it.
fn mentions(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + phrase.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// The `ess` command line with `arguments`, then `path`, then `more` appended.
fn ess(sources: &Sources, arguments: &[&str], path: &Path, more: &[&OsStr]) -> Vec<OsString> {
    let mut words = sources.ess.clone();
    words.extend(arguments.iter().map(OsString::from));
    words.push(path.as_os_str().to_owned());
    words.extend(more.iter().map(|word| (*word).to_owned()));
    words
}

/// `ess specify validate --path <dir>`: `Valid` on exit 0, `Refused` with its problem lines on
/// exit 1.
fn validate(sources: &Sources, dir: &Path) -> Result<(ValidationResult, i64)> {
    let words = ess(sources, &["specify", "validate", "--path"], dir, &[]);
    let output = blockers::run(&words, None, sources.bound)?;
    match output.status.code() {
        Some(0) => Ok((ValidationResult::Valid, 0)),
        Some(1) => Ok((ValidationResult::Refused, refusals(&output))),
        _ => Err(blockers::failed(&words, &output)),
    }
}

/// The refusals a refused validation printed: each problem line (`  - …`, but not a rendered
/// diagnostic, `  - error[…]` and the like), and each `error:` line, and at least one.
fn refusals(output: &Output) -> i64 {
    let text = [
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ]
    .concat();
    let problems = text
        .lines()
        .filter(|line| match line.strip_prefix("  - ") {
            Some(problem) => !["error[", "warning[", "note[", "help["]
                .iter()
                .any(|diagnostic| problem.starts_with(diagnostic)),
            None => line.starts_with("Error: ") || line.starts_with("error: "),
        })
        .count();
    i64::try_from(problems.max(1)).unwrap_or(i64::MAX)
}

/// `ess verify conform synthesize --path <dir> --out <out>`: the scenarios and refusals its last
/// line names.
fn synthesize(sources: &Sources, dir: &Path, out: &Path) -> Result<(i64, i64)> {
    let words = ess(
        sources,
        &["verify", "conform", "synthesize", "--path"],
        dir,
        &[OsStr::new("--out"), out.as_os_str()],
    );
    let stdout = blockers::succeed(&words, None, sources.bound)?;
    summary(&String::from_utf8_lossy(&stdout)).ok_or_else(|| {
        anyhow!(
            "`{}` printed no last line `N scenario(s) … M refusal(s)`",
            blockers::shown(&words)
        )
    })
}

/// The counts of `N scenario(s) (A authored), M refusal(s), written to …`, the last line of
/// `text` that is not empty.
fn summary(text: &str) -> Option<(i64, i64)> {
    let last = text.lines().rev().find(|line| !line.trim().is_empty())?;
    let words: Vec<&str> = last.split_whitespace().collect();
    let before = |unit: &str| -> Option<i64> {
        let at = words.iter().position(|word| word.starts_with(unit))?;
        words.get(at.checked_sub(1)?)?.parse().ok()
    };
    Some((before("scenario(s)")?, before("refusal(s)")?))
}

/// The status of the `executable-system-specification` artifacts of the member `name`: each
/// distinct status, in id order, joined by `, `; `None` when it has none.
fn conformance(workspace: &Workspace, name: &str) -> Option<String> {
    let mut artifacts: Vec<_> = workspace
        .artifacts
        .iter()
        .filter(|artifact| artifact.member == name && artifact.kind() == SPECIFICATION_ARTIFACT)
        .collect();
    artifacts.sort_by(|a, b| a.id.cmp(&b.id));
    let mut statuses: Vec<&str> = Vec::new();
    for artifact in artifacts {
        if !statuses.contains(&artifact.status.as_str()) {
            statuses.push(&artifact.status);
        }
    }
    (!statuses.is_empty()).then(|| statuses.join(", "))
}

/// What this collector reads in a repository whose `origin/main` holds `files` (`/`-separated
/// paths from its top), sorted and none inside another: `AGENTS.md`, when it has one; each
/// specification root, a directory holding an `ess-inputs.yaml` or `system.yaml` outside every
/// `target/` and `node_modules/` directory, the rule [`specification_files`] applies to an
/// export; and each path an input manifest lists under `specification:` or `scenarios:`,
/// resolved from its directory, that lies inside the repository and is held. `manifest` answers a
/// manifest's text by its path. `None` when a root, or a listed path, is the repository's top: the
/// whole tree is read then.
///
/// # Errors
///
/// What `manifest` answers for a manifest it cannot read.
pub(crate) fn read_paths(
    files: &[String],
    mut manifest: impl FnMut(&str) -> Result<String>,
) -> Result<Option<BTreeSet<String>>> {
    let held = |path: &str| {
        files.iter().any(|file| {
            file == path
                || file
                    .strip_prefix(path)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    };
    let mut paths = BTreeSet::new();
    if held(AGENTS) {
        paths.insert(AGENTS.to_owned());
    }
    for file in files {
        let path = Path::new(file);
        let Some(name) = path.file_name() else {
            continue;
        };
        if (name != INPUTS && name != SYSTEM)
            || path
                .components()
                .any(|part| part.as_os_str() == "target" || part.as_os_str() == "node_modules")
        {
            continue;
        }
        let root = path.parent().unwrap_or(Path::new(""));
        if root.as_os_str().is_empty() {
            return Ok(None);
        }
        paths.insert(shown_path(root));
        if name != INPUTS {
            continue;
        }
        let text = manifest(file)?;
        for entry in listed(&text, "specification")
            .into_iter()
            .chain(listed(&text, "scenarios"))
        {
            let Some(listed_path) = inside(root, &entry) else {
                continue;
            };
            if listed_path.as_os_str().is_empty() {
                return Ok(None);
            }
            let shown = shown_path(&listed_path);
            if held(&shown) {
                paths.insert(shown);
            }
        }
    }
    // A path inside another one read is read with it.
    let within = |path: &String| {
        paths.iter().any(|outer| {
            path.strip_prefix(outer.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        })
    };
    Ok(Some(
        paths.iter().filter(|path| !within(path)).cloned().collect(),
    ))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{inside, listed, mentions, read_paths, root_of, scalar, shown_path, summary};

    #[test]
    fn the_root_is_the_shallowest_manifest_then_the_shallowest_system_file() {
        let files = |paths: &[&str]| paths.iter().map(PathBuf::from).collect::<Vec<_>>();
        let root = |paths: &[&str]| root_of(&files(paths)).map(|root| shown_path(&root));
        assert_eq!(
            root(&["spec/system.yaml", "spec/ess-inputs.yaml"]),
            Some("spec".to_owned())
        );
        assert_eq!(
            root(&["ess/system.yaml", "contracts/deep/ess-inputs.yaml"]),
            Some("contracts/deep".to_owned())
        );
        assert_eq!(
            root(&["b/system.yaml", "a/system.yaml", "system.yaml"]),
            Some(".".to_owned())
        );
        assert_eq!(
            root(&["b/system.yaml", "a/system.yaml"]),
            Some("a".to_owned())
        );
        assert_eq!(root(&[]), None);
    }

    #[test]
    fn a_scalar_is_unquoted_and_loses_its_comment() {
        assert_eq!(scalar(" ess/22"), "ess/22");
        assert_eq!(scalar(" ess/22  # the newest"), "ess/22");
        assert_eq!(scalar(" \"ess 0.55\""), "ess 0.55");
        assert_eq!(scalar(" 'ess # 1'"), "ess # 1");
        assert_eq!(scalar("  # nothing"), "");
    }

    #[test]
    fn a_block_list_is_read_until_the_next_top_level_key() {
        let text = "format: ess-inputs/1\nspecification:\n  - ess/system.yaml\n  # a note\n\
                    - \"ess/domains/a.yaml\"\nscenarios:\n  - s.yaml\n";
        assert_eq!(
            listed(text, "specification"),
            vec![
                "ess/system.yaml".to_owned(),
                "ess/domains/a.yaml".to_owned()
            ]
        );
        assert!(listed("specification: []\n", "specification").is_empty());
        assert!(listed("format: x\n", "specification").is_empty());
    }

    #[test]
    fn a_listed_path_is_resolved_inside_the_export_or_not_at_all() {
        assert_eq!(
            inside(Path::new("model"), "../shared/system.yaml"),
            Some(PathBuf::from("shared/system.yaml"))
        );
        assert_eq!(
            inside(Path::new(""), "./ess/system.yaml"),
            Some(PathBuf::from("ess/system.yaml"))
        );
        assert_eq!(inside(Path::new("model"), "../../system.yaml"), None);
        assert_eq!(inside(Path::new(""), "/etc/system.yaml"), None);
    }

    #[test]
    fn the_summary_is_the_counts_of_the_last_line() {
        assert_eq!(
            summary("note: x\n250 scenario(s) (0 authored), 3 refusal(s), written to /x.json\n"),
            Some((250, 3))
        );
        assert_eq!(summary("250 scenario(s) (0 authored)\nwritten\n"), None);
        assert_eq!(summary(""), None);
    }

    #[test]
    fn a_phrase_is_matched_as_whole_words() {
        assert!(mentions("ess opt-out: none", "ess opt-out"));
        assert!(!mentions("the ess opt-outs", "ess opt-out"));
        assert!(!mentions("bless opt-out", "ess opt-out"));
        assert!(mentions("**opted out of ess**", "opted out of ess"));
        assert!(!mentions("opts out of essential", "opts out of ess"));
    }

    #[test]
    fn what_is_read_is_agents_md_each_root_and_each_held_path_a_manifest_lists() {
        let files = |paths: &[&str]| {
            paths
                .iter()
                .map(|&path| path.to_owned())
                .collect::<Vec<_>>()
        };
        let manifest = "format: ess-inputs/2\nspecification:\n  - system.yaml\n  - \
                        ../shared/x.yaml\n  - ../../outside.yaml\n  - ../missing.yaml\n\
                        scenarios:\n  - ../scen\n";
        let read = read_paths(
            &files(&[
                "AGENTS.md",
                "README.md",
                "spec/ess-inputs.yaml",
                "spec/system.yaml",
                "shared/x.yaml",
                "shared/y.yaml",
                "scen/a.yaml",
                "target/system.yaml",
                "web/node_modules/p/ess-inputs.yaml",
            ]),
            |path| {
                assert_eq!(path, "spec/ess-inputs.yaml");
                Ok(manifest.to_owned())
            },
        )
        .expect("the manifest reads");
        assert_eq!(
            read.map(|paths| paths.into_iter().collect::<Vec<_>>()),
            Some(vec![
                "AGENTS.md".to_owned(),
                "scen".to_owned(),
                "shared/x.yaml".to_owned(),
                "spec".to_owned(),
            ])
        );

        let none = |_: &str| -> anyhow::Result<String> { panic!("no manifest is read") };
        assert_eq!(
            read_paths(&files(&["system.yaml", "src/lib.rs"]), none).expect("no manifest"),
            None,
            "a root at the top reads the whole tree"
        );
        assert_eq!(
            read_paths(&files(&["model/ess-inputs.yaml", "lib/a.yaml"]), |_| Ok(
                "specification:\n  - ..\n".to_owned()
            ))
            .expect("the manifest reads"),
            None,
            "a manifest that lists the top reads the whole tree"
        );
        assert_eq!(
            read_paths(&files(&["README.md", "src/lib.rs"]), none)
                .expect("no manifest")
                .map(|paths| paths.len()),
            Some(0)
        );
    }
}
