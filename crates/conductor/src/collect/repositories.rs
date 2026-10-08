//! The `repositories` collector: one `RecordRepository` per repository of the instance, archived
//! ones included (`story:collect-repositories`).
//!
//! [`collect`] reads the live sources of the instance the process runs ([`Sources::of`] and
//! [`Origins::of`] over [`config::active`]); [`collect_over`] reads any, and [`collect_from`] any
//! with the one owner [`Sources::organization`], which are the seams a test points at fixtures and
//! at repositories it builds.
//!
//! The repositories are the instance's ([`Origins`]): each `github:` owner's list, each read from
//! its checkout `<workspace>/<repository>` when there is one and from GitHub when not; and each
//! git repository under a `local:` source's directory, at depth 1 or 2 (`<repo>` or
//! `<group>/<repo>`, `story:grouped-instance`) and not under one of its `exclude` entries, read as
//! it is: at `HEAD`, with no fetch, and with nothing asked of GitHub, so it is `Private`, not
//! archived, and has no issue and no GitHub Release. A `gitlab:` source's repositories are not
//! read here. A name two origins list is one checkout and one recorded repository, so the
//! collector fails naming it and both origins. The specifications collector lists a local
//! source's repositories through this collector's own listing ([`local_names`]). Each field has
//! one source:
//!
//! | fields | source |
//! |---|---|
//! | `visibility`, `archived` | the owner's list ([`Sources::repository_list`]) |
//! | `open_issues`, `oldest_open_issue_at` | the open issues, pull requests not among them ([`Sources::open_issues`]); none, unasked, when the repository has issues turned off |
//! | `local_checkout` | whether `<workspace>/<repository>/.git` exists |
//! | `main_head`, `main_committed_at`, `main_commits_7d` | `origin/main` after `git fetch --quiet origin` in the checkout; GitHub's `main` without one |
//! | `real_commits_7d` | the window's commits on every `origin` branch (on GitHub's `main` without a checkout), less those real activity excludes (below) |
//! | `behind_main`, `dirty_files`, `worktrees` | `git rev-list --count HEAD..origin/main`, the lines of `git status --porcelain`, the entries of `git worktree list`; 0 without a checkout |
//! | `latest_release`, `latest_release_at` | the latest GitHub Release, drafts aside, and its publication date ([`Sources::latest_release`]); without one, the newest tag on `origin/main`; the tag's commit date when there is no publication date |
//! | `unreleased_commits` | `git rev-list --count <tag>..origin/main`, when the checkout holds the tag |
//! | `planning_store_version` | `version` in `.engineering/project.yaml` on `origin/main` (YAML or JSON) |
//! | `in_catalog` | whether the instance's catalog ([`Sources::catalog`], `story:catalog-source`) names the repository: a `<stem>.json` entry of the catalog's directory on its repository's `origin/main`, never that checkout's working tree, whose stem spells the name as the catalog's `names` says; absent for every repository when the instance names no catalog or `<workspace>/<catalog repository>` is no checkout |
//!
//! A local source's repository reads `HEAD` where the table reads `origin/main`, its own branches
//! and `HEAD` for `real_commits_7d`, and 0 for `behind_main`.
//!
//! # Real activity
//!
//! Design § 10. The window is the 7 days before [`Sources::now`]. A commit in it is real unless
//! it is a merge; or its normalised subject appears in the window in at least 3 repositories (an
//! org-wide sweep); or its subject is a maintenance subject (lint, format, CI, docs, planning,
//! pins, release chores, retrofit, publish); or every path it touches is a docs, CI, planning,
//! markdown or lockfile path. A commit on several branches counts once.
//!
//! # Failures
//!
//! Every command runs through [`crate::collect::run`] under [`Sources::bound`]. One that exits
//! non-zero is run once more; failing again, the collector answers an error naming the repository
//! and the command and records nothing, so no field reads 0 for a source that did not answer. Up
//! to [`Sources::width`] repositories are read at once, and they are recorded in name order.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{Catalog, CatalogNames, Instance};
use conductor_model::observation::{CommitSha, RecordRepository, RepositoryName, Visibility};
use conductor_model::primitives::Timestamp;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::collect::{self, Origins};
use crate::config;
use crate::snapshot::Recorder;

/// The commit a fetched checkout is read at.
const MAIN: &str = "origin/main";

/// How many repositories the live sources read at once.
pub const WIDTH: usize = 8;

/// How far the window reaches back from the clock.
const WINDOW: time::Duration = time::Duration::days(7);

/// The repositories a normalised subject must appear in, in the window, to be an org-wide sweep.
const SWEEP_REPOSITORIES: usize = 3;

/// The suffix of a catalog entry's file name; its stem spells a cataloged repository's name.
const CATALOG_ENTRY: &str = ".json";

/// The planning store's project file.
const PROJECT_FILE: &str = ".engineering/project.yaml";

/// Conventional-commit types whose commits are maintenance.
const MAINTENANCE_KINDS: [&str; 12] = [
    "ci", "doc", "docs", "fmt", "format", "lint", "plan", "planning", "publish", "release",
    "retrofit", "style",
];

/// First words (after any conventional-commit type) of maintenance subjects.
const MAINTENANCE_WORDS: [&str; 17] = [
    "bump", "ci", "doc", "docs", "document", "fmt", "format", "lint", "pin", "plan", "publish",
    "reformat", "release", "repin", "retrofit", "rustfmt", "unpin",
];

/// Lockfiles whose names do not end in `.lock`.
const LOCKFILES: [&str; 5] = [
    "bun.lockb",
    "go.sum",
    "npm-shrinkwrap.json",
    "package-lock.json",
    "pnpm-lock.yaml",
];

/// What the collector reads: the one seam a test replaces. [`Sources::of`] gives an instance's.
///
/// Each command line is a template: in every word, `{organization}` is filled with the owner
/// whose repository it asks about, `{repository}` with the repository's name, `{since}` with the
/// window's start (RFC 3339) and `{sha}` with a commit. Local git commands are not templates: they
/// run as `git` in the checkout.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The GitHub owner [`collect_from`] lists; [`collect_over`] lists the owners its
    /// [`Origins`] name instead.
    pub organization: String,
    /// The checkouts root: the directory holding one checkout per repository an owner lists,
    /// named as the repository.
    pub workspace: PathBuf,
    /// The instance's catalog, when it names one: its repository's `origin/main` is read when
    /// that repository has a checkout in [`Sources::workspace`].
    pub catalog: Option<Catalog>,
    /// The current time; the window is the 7 days before it.
    pub now: OffsetDateTime,
    /// Prints one owner's repositories as one JSON array of objects with `name`, `visibility`
    /// (`PUBLIC` or `PRIVATE`), `isArchived` and `hasIssuesEnabled`.
    pub repository_list: Vec<String>,
    /// Prints one repository's open issues, pull requests not among them, as one JSON array of
    /// objects with `createdAt`.
    pub open_issues: Vec<String>,
    /// Prints one repository's latest release, drafts aside, as a JSON array of at most one object
    /// with `tagName` and `publishedAt`.
    pub latest_release: Vec<String>,
    /// Prints `main`'s head commit as GitHub's REST API answers it (`sha`, `commit.committer.date`),
    /// for a repository without a checkout.
    pub main_commit: Vec<String>,
    /// Prints `main`'s commits since `{since}` as one or more JSON arrays (pages) of GitHub's
    /// commit objects (`sha`, `parents`, `commit.message`), for a repository without a checkout.
    pub main_commits: Vec<String>,
    /// Prints the commit `{sha}` with its `files` as GitHub's REST API answers it, for a
    /// repository without a checkout.
    pub commit: Vec<String>,
    /// How many repositories are read at once.
    pub width: usize,
    /// How long one command may run before it is stopped.
    pub bound: Duration,
}

impl Sources {
    /// `instance`'s: its first `github:` owner as [`Sources::organization`] (none when it has
    /// none), its checkouts root as the workspace, its catalog when it names one, every command
    /// through `gh`, the 7 days before `now`, [`WIDTH`] repositories at once, and
    /// [`collect::BOUND`] on each command.
    #[must_use]
    pub fn of(instance: &Instance, now: OffsetDateTime) -> Self {
        Self {
            organization: Origins::of(instance)
                .owners
                .into_iter()
                .next()
                .unwrap_or_default(),
            workspace: PathBuf::from(&instance.checkouts.root),
            catalog: instance.catalog.clone(),
            now,
            repository_list: words(&[
                "gh",
                "repo",
                "list",
                "{organization}",
                "--limit",
                "200",
                "--json",
                "name,visibility,isArchived,hasIssuesEnabled",
            ]),
            open_issues: words(&[
                "gh",
                "issue",
                "list",
                "-R",
                "{organization}/{repository}",
                "--state",
                "open",
                "--limit",
                "1000",
                "--json",
                "number,createdAt",
            ]),
            latest_release: words(&[
                "gh",
                "release",
                "list",
                "-R",
                "{organization}/{repository}",
                "--limit",
                "1",
                "--exclude-drafts",
                "--json",
                "tagName,publishedAt",
            ]),
            main_commit: words(&[
                "gh",
                "api",
                "repos/{organization}/{repository}/commits/main",
            ]),
            main_commits: words(&[
                "gh",
                "api",
                "--paginate",
                "repos/{organization}/{repository}/commits?sha=main&since={since}&per_page=100",
            ]),
            commit: words(&[
                "gh",
                "api",
                "repos/{organization}/{repository}/commits/{sha}",
            ]),
            width: WIDTH,
            bound: collect::BOUND,
        }
    }
}

/// Records one `RecordRepository` per repository of the instance the process runs, archived ones
/// included, into the recorder's snapshot, over that instance's live sources.
///
/// # Errors
///
/// What [`collect_over`] answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    let instance = &config::active().instance;
    let origins = Origins::of(instance);
    let exclude = origins
        .local
        .iter()
        .map(|dir| (dir.clone(), collect::excluded_under(instance, dir)))
        .collect();
    collect_excluding(
        &Sources::of(instance, OffsetDateTime::now_utc()),
        &origins,
        &exclude,
        record,
    )
}

/// Records one `RecordRepository` per repository [`Sources::organization`] lists, in name order.
///
/// # Errors
///
/// What [`collect_over`] answers.
pub fn collect_from(sources: &Sources, record: &mut Recorder<'_>) -> Result<()> {
    collect_over(sources, &Origins::owner(&sources.organization), record)
}

/// Records one `RecordRepository` per repository of `origins`, in name order: every repository
/// each of its owners lists, and every one under each of its local directories.
///
/// # Errors
///
/// A source did not answer (named with its repository and command) or answered something this
/// collector cannot read, or two origins list one name, and nothing is recorded; or an
/// observation was refused.
pub fn collect_over(sources: &Sources, origins: &Origins, record: &mut Recorder<'_>) -> Result<()> {
    collect_excluding(sources, origins, &HashMap::new(), record)
}

/// [`collect_over`], leaving out of each local directory the repositories its `exclude` entries
/// in `exclude`, by directory, name or lie under (`story:grouped-instance`).
///
/// # Errors
///
/// What [`collect_over`] answers.
pub fn collect_excluding(
    sources: &Sources,
    origins: &Origins,
    exclude: &HashMap<PathBuf, Vec<String>>,
    record: &mut Recorder<'_>,
) -> Result<()> {
    let since = instant((sources.now - WINDOW).replace_nanosecond(0)?)?;
    let listed = listed(sources, origins, exclude)?;
    let read = each(&listed, sources.width, |listed| {
        read(sources, listed, &since)
    })?;
    let cataloged = cataloged(sources, &listed)?;
    let sweeps = sweeps(&listed, &read);
    for (listed, read) in listed.iter().zip(read) {
        let real = read
            .commits
            .iter()
            .filter(|commit| real(commit, &listed.name, &sweeps))
            .count();
        let snapshot_id = record.snapshot().clone();
        record.repository(RecordRepository {
            snapshot_id,
            repository: RepositoryName(listed.name.clone()),
            visibility: listed.visibility,
            archived: listed.archived,
            local_checkout: read.local_checkout,
            main_head: CommitSha(read.main_head),
            main_committed_at: Timestamp(read.main_committed_at),
            main_commits_7d: read.main_commits_7d,
            real_commits_7d: number(real),
            behind_main: read.behind_main,
            dirty_files: read.dirty_files,
            worktrees: read.worktrees,
            open_issues: read.open_issues,
            oldest_open_issue_at: read.oldest_open_issue_at.map(Timestamp),
            latest_release: read.release.tag,
            latest_release_at: read.release.at.map(Timestamp),
            unreleased_commits: read.release.unreleased,
            planning_store_version: read.planning_store_version,
            in_catalog: cataloged.as_ref().map(|names| names.contains(&listed.name)),
        })?;
    }
    Ok(())
}

/// One repository of the instance, as its origin lists it.
struct Listed {
    name: String,
    visibility: Visibility,
    archived: bool,
    issues: bool,
    origin: Origin,
}

/// Where a listed repository comes from.
enum Origin {
    /// An owner's list: its checkout, when it has one, is `<workspace>/<name>`, fetched.
    Owner(String),
    /// A local source's directory, which holds its checkout `<directory>/<name>`, read as it is.
    Local(PathBuf),
}

impl Listed {
    /// The owner whose list names it; none for a repository of a local source.
    fn owner(&self) -> Option<&str> {
        match &self.origin {
            Origin::Owner(owner) => Some(owner),
            Origin::Local(_) => None,
        }
    }

    /// Where its checkout is, or would be.
    fn checkout(&self, sources: &Sources) -> PathBuf {
        match &self.origin {
            Origin::Owner(_) => sources.workspace.join(&self.name),
            Origin::Local(dir) => dir.join(&self.name),
        }
    }

    /// Its origin, as an error names it.
    fn origin_named(&self) -> String {
        match &self.origin {
            Origin::Owner(owner) => format!("owner {owner}"),
            Origin::Local(dir) => format!("the local source {}", dir.display()),
        }
    }
}

/// One entry of an owner's list, as `gh repo list --json` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedEntry {
    name: String,
    visibility: String,
    is_archived: bool,
    has_issues_enabled: bool,
}

/// The repositories of `origins`, by name: each owner's, then each local directory's.
fn listed(
    sources: &Sources,
    origins: &Origins,
    exclude: &HashMap<PathBuf, Vec<String>>,
) -> Result<Vec<Listed>> {
    let mut listed = Vec::new();
    for owner in &origins.owners {
        listed.extend(owned(sources, owner)?);
    }
    for dir in &origins.local {
        let exclude = exclude.get(dir).map_or(&[][..], Vec::as_slice);
        listed.extend(local_repositories(dir, exclude)?);
    }
    listed.sort_by(|one, other| one.name.cmp(&other.name));
    one_of_each(&listed, |listed| &listed.name, Listed::origin_named)?;
    Ok(listed)
}

/// `owner`'s repositories, by name.
fn owned(sources: &Sources, owner: &str) -> Result<Vec<Listed>> {
    let what = format!("organization {owner}");
    let command = filled(&sources.repository_list, &[("organization", owner)]);
    let entries: Vec<ListedEntry> = json(&what, &command, None, sources.bound)?;
    let mut listed = entries
        .into_iter()
        .map(|entry| {
            let name = checked(&what, entry.name)?;
            let visibility = match entry.visibility.as_str() {
                "PUBLIC" => Visibility::Public,
                "PRIVATE" => Visibility::Private,
                other => bail!(
                    "repository {name}: visibility {other:?} is not one the specification names"
                ),
            };
            Ok(Listed {
                name,
                visibility,
                archived: entry.is_archived,
                issues: entry.has_issues_enabled,
                origin: Origin::Owner(owner.to_owned()),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    listed.sort_by(|one, other| one.name.cmp(&other.name));
    listed.dedup_by(|one, other| one.name == other.name);
    Ok(listed)
}

/// The repositories of the local source `dir`, but those `exclude` names or holds: each
/// directory that holds a `.git` directly under it, named as the directory, or one level down in
/// a group, a directory under it that holds no `.git`, named `<group>/<directory>`
/// (`story:grouped-instance`). No host publishes them, so each is `Private`, not archived, and
/// has issues turned off. A checkout with no commit yet ([`born`]) is no repository yet: it has
/// no `HEAD` to read, and one such checkout failed a whole snapshot (2026-10-08).
fn local_repositories(dir: &Path, exclude: &[String]) -> Result<Vec<Listed>> {
    let what = format!("the local source {}", dir.display());
    let mut listed = Vec::new();
    let local = |name: String| Listed {
        name,
        visibility: Visibility::Private,
        archived: false,
        issues: false,
        origin: Origin::Local(dir.to_owned()),
    };
    for path in directories(dir, &what)? {
        let name = path.file_name().and_then(OsStr::to_str);
        if name.is_some_and(|name| collect::is_excluded(&[name], exclude)) {
            continue;
        }
        if path.join(".git").exists() {
            if born(&path) {
                listed.push(local(checked(&what, step(&what, &path)?.to_owned())?));
            }
            continue;
        }
        for inner in directories(&path, &what)? {
            if !inner.join(".git").exists() || !born(&inner) {
                continue;
            }
            let group = step(&what, &path)?;
            let repository = step(&what, &inner)?;
            if collect::is_excluded(&[group, repository], exclude) {
                continue;
            }
            let group = checked(&what, group.to_owned())?;
            let repository = checked(&what, repository.to_owned())?;
            listed.push(local(format!("{group}/{repository}")));
        }
    }
    Ok(listed)
}

/// The names of the repositories of the local source `dir`, but those `exclude` names or holds,
/// as [`collect_over`] lists them: the one listing the specifications collector reads too
/// (`story:specifications-every-source`). A name is `<repo>` or `<group>/<repo>`, the checkout
/// `<dir>/<name>`.
///
/// # Errors
///
/// `dir` or a group under it cannot be listed, or a directory's name is no repository name.
pub(crate) fn local_names(dir: &Path, exclude: &[String]) -> Result<Vec<String>> {
    Ok(local_repositories(dir, exclude)?
        .into_iter()
        .map(|listed| listed.name)
        .collect())
}

/// Whether the checkout at `dir` has a commit at `HEAD`: `HEAD` names a commit, or a branch whose
/// ref is a loose file or a line of `packed-refs`. A checkout whose `.git` is not a directory (a
/// linked worktree) is taken to have one; git answers for it when it is read.
fn born(dir: &Path) -> bool {
    let git = dir.join(".git");
    if !git.is_dir() {
        return true;
    }
    let Ok(head) = fs::read_to_string(git.join("HEAD")) else {
        return false;
    };
    let Some(reference) = head.trim().strip_prefix("ref: ") else {
        return !head.trim().is_empty();
    };
    git.join(reference).is_file()
        || fs::read_to_string(git.join("packed-refs")).is_ok_and(|packed| {
            packed.lines().any(|line| {
                line.split_once(' ')
                    .is_some_and(|(_, name)| name == reference)
            })
        })
}

/// The directories directly under `dir`, `what` naming the source an error names.
fn directories(dir: &Path, what: &str) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dir).with_context(|| format!("{what}: list {}", dir.display()))?;
    let mut directories = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("{what}: list {}", dir.display()))?
            .path();
        if path.is_dir() {
            directories.push(path);
        }
    }
    Ok(directories)
}

/// The last step of `path`, as a repository or group name is written.
fn step<'p>(what: &str, path: &'p Path) -> Result<&'p str> {
    path.file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("{what}: {} is no repository name", path.display()))
}

/// What one repository's sources answered.
struct Read {
    local_checkout: bool,
    main_head: String,
    main_committed_at: String,
    main_commits_7d: i64,
    /// The window's commits that are not merges, each once.
    commits: Vec<Commit>,
    behind_main: i64,
    dirty_files: i64,
    worktrees: i64,
    open_issues: i64,
    oldest_open_issue_at: Option<String>,
    release: Release,
    planning_store_version: Option<String>,
}

/// A commit of the window that is not a merge.
struct Commit {
    subject: String,
    /// The paths it touches. Without a checkout, the files of a commit whose subject is a
    /// maintenance subject are not asked for, and this is empty: the subject excludes it already.
    files: Vec<String>,
}

#[derive(Default)]
struct Release {
    tag: Option<String>,
    at: Option<String>,
    unreleased: Option<i64>,
}

/// One repository's sources: an owner's from its checkout, fetched, when it has one and from
/// GitHub when not; a local source's from its checkout as it is.
fn read(sources: &Sources, listed: &Listed, since: &str) -> Result<Read> {
    let what = format!("repository {}", listed.name);
    let dir = listed.checkout(sources);
    let checkout = Checkout {
        what: &what,
        dir: &dir,
        bound: sources.bound,
    };
    let mut read = match &listed.origin {
        Origin::Owner(_) if dir.join(".git").exists() => {
            local(sources, listed, &checkout, since, Reading::Fetched)?
        }
        Origin::Owner(owner) => remote(sources, listed, owner, &what, since)?,
        Origin::Local(_) => local(sources, listed, &checkout, since, Reading::AsIs)?,
    };
    (read.open_issues, read.oldest_open_issue_at) = issues(sources, listed, &what)?;
    Ok(read)
}

/// How a checkout is read.
#[derive(Clone, Copy)]
enum Reading {
    /// Fetched from `origin`, then read at `origin/main` and across `origin`'s branches.
    Fetched,
    /// Read as it is, with no fetch: at `HEAD` and across its own branches and `HEAD`.
    AsIs,
}

impl Reading {
    /// The commit read as `main`.
    fn main(self) -> &'static str {
        match self {
            Self::Fetched => MAIN,
            Self::AsIs => "HEAD",
        }
    }

    /// The revisions whose commits of the window are read for real activity.
    fn branches(self) -> &'static [&'static str] {
        match self {
            Self::Fetched => &["--remotes=origin"],
            Self::AsIs => &["--branches", "HEAD"],
        }
    }
}

/// A repository's local checkout, and what its git commands are named by when they fail.
struct Checkout<'a> {
    what: &'a str,
    dir: &'a Path,
    bound: Duration,
}

impl Checkout<'_> {
    /// What `git <args>` printed in the checkout.
    fn git(&self, args: &[&str]) -> Result<String> {
        let command: Vec<String> = std::iter::once("git")
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect();
        answer(self.what, &command, Some(self.dir), self.bound)
    }

    /// The count `git <args>` printed.
    fn count(&self, args: &[&str]) -> Result<i64> {
        let text = self.git(args)?;
        text.trim().parse().with_context(|| {
            format!(
                "{}: `git {}` printed no count: {text:?}",
                self.what,
                args.join(" ")
            )
        })
    }
}

/// A repository with a checkout, read as `reading` says: fetched, then read from `origin` and the
/// working tree; or as it is, from `HEAD` and the working tree.
fn local(
    sources: &Sources,
    listed: &Listed,
    checkout: &Checkout<'_>,
    since: &str,
    reading: Reading,
) -> Result<Read> {
    let what = checkout.what;
    let main = reading.main();
    let after = format!("--since={since}");
    if matches!(reading, Reading::Fetched) {
        checkout.git(&["fetch", "--quiet", "origin"])?;
    }
    let head = checkout.git(&["log", "-1", "--format=%H%x09%cI", main])?;
    let (main_head, main_committed_at) = head
        .trim()
        .split_once('\t')
        .ok_or_else(|| anyhow!("{what}: `git log -1 {main}` printed no commit: {head:?}"))?;
    let main_commits_7d = checkout.count(&["rev-list", "--count", &after, main])?;
    let mut log = vec!["-c", "core.quotepath=off", "log"];
    log.extend(reading.branches());
    log.extend([
        "--no-merges",
        &after,
        "--format=%x1e%H%x09%s",
        "--name-only",
    ]);
    let commits = logged(&checkout.git(&log)?);
    let dirty_files = checkout
        .git(&["status", "--porcelain"])?
        .lines()
        .filter(|line| !line.is_empty())
        .count();
    let behind_main = match reading {
        Reading::Fetched => checkout.count(&["rev-list", "--count", "HEAD..origin/main"])?,
        Reading::AsIs => 0,
    };
    let worktrees = checkout
        .git(&["worktree", "list", "--porcelain"])?
        .lines()
        .filter(|line| line.starts_with("worktree "))
        .count();
    let release = release(sources, listed, what, Some((checkout, main)))?;
    let planning_store_version = if checkout
        .git(&["ls-tree", "--name-only", main, "--", PROJECT_FILE])?
        .trim()
        .is_empty()
    {
        None
    } else {
        store_version(&checkout.git(&["show", &format!("{main}:{PROJECT_FILE}")])?)
    };
    Ok(Read {
        local_checkout: true,
        main_head: main_head.to_owned(),
        main_committed_at: utc(what, main_committed_at)?,
        main_commits_7d,
        commits,
        behind_main,
        dirty_files: number(dirty_files),
        worktrees: number(worktrees),
        open_issues: 0,
        oldest_open_issue_at: None,
        release,
        planning_store_version,
    })
}

/// The commits `git log --format=%x1e%H%x09%s --name-only` printed, each once.
fn logged(text: &str) -> Vec<Commit> {
    let mut seen = HashSet::new();
    text.split('\x1e')
        .filter_map(|entry| {
            let mut lines = entry.lines();
            let (sha, subject) = lines.next()?.split_once('\t')?;
            seen.insert(sha.to_owned()).then(|| Commit {
                subject: subject.to_owned(),
                files: lines
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect()
}

/// A commit as GitHub's REST API answers it, in the parts this collector reads.
#[derive(Deserialize)]
struct ApiCommit {
    sha: String,
    commit: ApiCommitBody,
    #[serde(default)]
    parents: Vec<Value>,
    #[serde(default)]
    files: Vec<ApiFile>,
}

#[derive(Deserialize)]
struct ApiCommitBody {
    #[serde(default)]
    message: String,
    committer: Option<ApiSignature>,
}

#[derive(Deserialize)]
struct ApiSignature {
    date: String,
}

#[derive(Deserialize)]
struct ApiFile {
    filename: String,
}

/// A repository `owner` lists that has no checkout: `main` from GitHub, and 0 for what only a
/// checkout shows.
fn remote(
    sources: &Sources,
    listed: &Listed,
    owner: &str,
    what: &str,
    since: &str,
) -> Result<Read> {
    let fill = [
        ("organization", owner),
        ("repository", listed.name.as_str()),
        ("since", since),
    ];
    let command = filled(&sources.main_commit, &fill);
    let head: ApiCommit = json(what, &command, None, sources.bound)?;
    let committed_at = head
        .commit
        .committer
        .map(|committer| committer.date)
        .ok_or_else(|| anyhow!("{what}: `{}` printed no committer date", command.join(" ")))?;
    let command = filled(&sources.main_commits, &fill);
    let pages = answer(what, &command, None, sources.bound)?;
    let listed_commits = serde_json::Deserializer::from_str(&pages)
        .into_iter::<Vec<ApiCommit>>()
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("{what}: `{}` printed no commit list", command.join(" ")))?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let main_commits_7d = number(listed_commits.len());
    let mut seen = HashSet::new();
    let mut commits = Vec::new();
    for listed_commit in listed_commits {
        if listed_commit.parents.len() > 1 || !seen.insert(listed_commit.sha.clone()) {
            continue;
        }
        let subject = listed_commit
            .commit
            .message
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned();
        let files = if maintenance(&subject) {
            Vec::new()
        } else {
            let command = filled(
                &sources.commit,
                &[
                    ("organization", owner),
                    ("repository", listed.name.as_str()),
                    ("sha", listed_commit.sha.as_str()),
                ],
            );
            let detail: ApiCommit = json(what, &command, None, sources.bound)?;
            detail.files.into_iter().map(|file| file.filename).collect()
        };
        commits.push(Commit { subject, files });
    }
    Ok(Read {
        local_checkout: false,
        main_head: head.sha,
        main_committed_at: utc(what, &committed_at)?,
        main_commits_7d,
        commits,
        behind_main: 0,
        dirty_files: 0,
        worktrees: 0,
        open_issues: 0,
        oldest_open_issue_at: None,
        release: release(sources, listed, what, None)?,
        planning_store_version: None,
    })
}

/// One open issue, as `gh issue list --json createdAt` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Issue {
    created_at: String,
}

/// How many issues are open, and when the oldest was opened (UTC); 0 and none, unasked, for a
/// repository with issues turned off and for one of a local source.
fn issues(sources: &Sources, listed: &Listed, what: &str) -> Result<(i64, Option<String>)> {
    let Some(owner) = listed.owner().filter(|_| listed.issues) else {
        return Ok((0, None));
    };
    let command = filled(
        &sources.open_issues,
        &[
            ("organization", owner),
            ("repository", listed.name.as_str()),
        ],
    );
    let issues: Vec<Issue> = json(what, &command, None, sources.bound)?;
    let mut oldest: Option<OffsetDateTime> = None;
    for issue in &issues {
        let opened = parsed_instant(what, &issue.created_at)?;
        if oldest.is_none_or(|oldest| opened < oldest) {
            oldest = Some(opened);
        }
    }
    Ok((number(issues.len()), oldest.map(instant).transpose()?))
}

/// The latest release, as `gh release list --json tagName,publishedAt` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Published {
    tag_name: String,
    #[serde(default)]
    published_at: Option<String>,
}

/// The latest release: GitHub's, for a repository an owner lists; else the newest tag on `main` in
/// the checkout, given with the commit it reads as `main`; its date; and the commits on `main`
/// since its tag when the checkout holds the tag.
fn release(
    sources: &Sources,
    listed: &Listed,
    what: &str,
    checkout: Option<(&Checkout<'_>, &str)>,
) -> Result<Release> {
    let published: Vec<Published> = match listed.owner() {
        Some(owner) => {
            let command = filled(
                &sources.latest_release,
                &[
                    ("organization", owner),
                    ("repository", listed.name.as_str()),
                ],
            );
            json(what, &command, None, sources.bound)?
        }
        None => Vec::new(),
    };
    let main = checkout.map_or(MAIN, |(_, main)| main);
    let checkout = checkout.map(|(checkout, _)| checkout);
    let (tag, at) = match published.into_iter().next() {
        Some(release) => {
            let at = release
                .published_at
                .filter(|at| !at.trim().is_empty())
                .map(|at| utc(what, &at))
                .transpose()?;
            (release.tag_name, at)
        }
        None => {
            let Some(checkout) = checkout else {
                return Ok(Release::default());
            };
            let newest = checkout.git(&[
                "for-each-ref",
                &format!("--merged={main}"),
                "--sort=-creatordate",
                "--count=1",
                "--format=%(refname:lstrip=2)",
                "refs/tags",
            ])?;
            match newest.lines().next().map(str::trim) {
                Some(tag) if !tag.is_empty() => (tag.to_owned(), None),
                _ => return Ok(Release::default()),
            }
        }
    };
    let reference = format!("refs/tags/{tag}");
    let held = match checkout {
        Some(checkout) => checkout
            .git(&["for-each-ref", "--format=%(refname)", &reference])?
            .lines()
            .any(|line| line == reference),
        None => false,
    };
    let Some(checkout) = checkout.filter(|_| held) else {
        return Ok(Release {
            tag: Some(tag),
            at,
            unreleased: None,
        });
    };
    let at = match at {
        Some(at) => at,
        None => utc(
            what,
            &checkout.git(&["log", "-1", "--format=%cI", &reference])?,
        )?,
    };
    let unreleased = checkout.count(&["rev-list", "--count", &format!("{reference}..{main}")])?;
    Ok(Release {
        tag: Some(tag),
        at: Some(at),
        unreleased: Some(unreleased),
    })
}

/// `version` in a planning store's project file, YAML or JSON; none when it names none.
fn store_version(text: &str) -> Option<String> {
    if let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(text) {
        return fields
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    text.lines()
        .find_map(|line| line.strip_prefix("version:"))
        .map(|value| {
            value
                .split(" #")
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_owned()
        })
        .filter(|version| !version.is_empty())
}

/// The repositories the instance's catalog names on its repository's `origin/main`, which the
/// read before this fetched (or this fetches, when no owner lists that repository): the stem of
/// each `.json` entry of the catalog's directory, spelled as its `names` says. None, unread, when
/// the instance names no catalog or the checkouts root holds no checkout of its repository.
fn cataloged(sources: &Sources, listed: &[Listed]) -> Result<Option<HashSet<String>>> {
    let Some(catalog) = &sources.catalog else {
        return Ok(None);
    };
    let what = format!("repository {}", catalog.repository);
    let dir = sources.workspace.join(&catalog.repository);
    if !dir.join(".git").exists() {
        return Ok(None);
    }
    let checkout = Checkout {
        what: &what,
        dir: &dir,
        bound: sources.bound,
    };
    if !listed
        .iter()
        .any(|listed| listed.name == catalog.repository && listed.owner().is_some())
    {
        checkout.git(&["fetch", "--quiet", "origin"])?;
    }
    let directory = format!("{}/", catalog.path.trim_end_matches('/'));
    let names: HashSet<String> = checkout
        .git(&["ls-tree", "--name-only", MAIN, &directory])?
        .lines()
        .filter_map(|path| path.rsplit('/').next()?.strip_suffix(CATALOG_ENTRY))
        .filter_map(|stem| match catalog.names {
            CatalogNames::Plain => Some(stem.to_owned()),
            CatalogNames::Hex => unhex(stem),
        })
        .collect();
    if names.is_empty() {
        bail!("{what}: {MAIN} catalogs no repository under {directory}");
    }
    Ok(Some(names))
}

/// The normalised subjects of the window found in at least [`SWEEP_REPOSITORIES`] repositories.
fn sweeps(listed: &[Listed], read: &[Read]) -> HashSet<String> {
    let mut found: HashMap<String, HashSet<&str>> = HashMap::new();
    for (listed, read) in listed.iter().zip(read) {
        for commit in &read.commits {
            found
                .entry(normalised(&commit.subject, &listed.name))
                .or_default()
                .insert(&listed.name);
        }
    }
    found
        .into_iter()
        .filter(|(_, repositories)| repositories.len() >= SWEEP_REPOSITORIES)
        .map(|(subject, _)| subject)
        .collect()
}

/// Whether `commit`, in `repository`, is real activity.
fn real(commit: &Commit, repository: &str, sweeps: &HashSet<String>) -> bool {
    !sweeps.contains(&normalised(&commit.subject, repository))
        && !maintenance(&commit.subject)
        && commit.files.iter().any(|path| !excluded_path(path))
}

/// `subject` as the sweep rule compares it across repositories: lower case, without a trailing
/// pull-request reference such as `(#12)`, the repository's own name as `<repository>`, every
/// word holding a digit (a version, a number, a date) as `<n>`, its words single-spaced.
fn normalised(subject: &str, repository: &str) -> String {
    let lower = subject.trim().to_lowercase();
    let mut subject = lower.as_str();
    if let Some(open) = subject.rfind("(#")
        && let Some(reference) = subject[open + 2..].strip_suffix(')')
        && !reference.is_empty()
        && reference.chars().all(|c| c.is_ascii_digit())
    {
        subject = subject[..open].trim_end();
    }
    let repository = repository.to_lowercase();
    subject
        .split_whitespace()
        .map(|word| {
            if word.trim_matches(|c: char| !c.is_alphanumeric()) == repository {
                "<repository>"
            } else if word.chars().any(|c| c.is_ascii_digit()) {
                "<n>"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `subject` is a maintenance subject: lint, format, CI, docs, planning, pins, release
/// chores, retrofit, publish. Its conventional-commit type decides when it is one of
/// `MAINTENANCE_KINDS`; otherwise its first word (after any type) when it is one of
/// `MAINTENANCE_WORDS`; and a `release` scope, or `prepare`, `cut` or `tag` with the word
/// `release`, is a release chore.
fn maintenance(subject: &str) -> bool {
    let lower = subject.trim().to_lowercase();
    let (kind, scope, rest) = conventional(&lower);
    if kind.is_some_and(|kind| MAINTENANCE_KINDS.contains(&kind)) || scope == Some("release") {
        return true;
    }
    let words: Vec<&str> = rest
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect();
    let first = words.first().copied().unwrap_or_default();
    MAINTENANCE_WORDS.contains(&first)
        || (matches!(first, "prepare" | "cut" | "tag") && words.contains(&"release"))
}

/// A lower-case subject split as a conventional commit, `type(scope)!: rest`; no type and the
/// whole subject when it is not one.
fn conventional(subject: &str) -> (Option<&str>, Option<&str>, &str) {
    if let Some((head, rest)) = subject.split_once(':') {
        let head = head.strip_suffix('!').unwrap_or(head);
        let (kind, scope) = match head.split_once('(') {
            Some((kind, scope)) => match scope.strip_suffix(')') {
                Some(scope) => (kind, Some(scope)),
                None => return (None, None, subject),
            },
            None => (head, None),
        };
        if !kind.is_empty() && kind.chars().all(|c| c.is_ascii_lowercase()) {
            return (Some(kind), scope, rest.trim());
        }
    }
    (None, None, subject)
}

/// Whether touching `path` alone is no real activity: a docs, CI, planning, markdown or lockfile
/// path.
fn excluded_path(path: &str) -> bool {
    let path = path.to_lowercase();
    let (dirs, name) = path.rsplit_once('/').unwrap_or(("", path.as_str()));
    let mut dirs = dirs.split('/');
    let docs = dirs.clone().any(|dir| dir == "docs" || dir == "doc");
    let ci = path.starts_with(".github/")
        || path.starts_with(".gitlab/")
        || path.starts_with(".circleci/")
        || name == ".gitlab-ci.yml";
    let planning = path.starts_with(".engineering/") || dirs.any(|dir| dir == "planning");
    let markdown = [".md", ".mdx", ".markdown"]
        .iter()
        .any(|extension| name.ends_with(extension));
    let lockfile = name.ends_with(".lock") || LOCKFILES.contains(&name);
    docs || ci || planning || markdown || lockfile
}

// ---------------------------------------------------------------------------------------------
// Shared with the `github` collector
// ---------------------------------------------------------------------------------------------

/// `words` as a command line.
pub(crate) fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

/// `template` with `{name}` replaced by its value from `fill`, in every word.
pub(crate) fn filled(template: &[String], fill: &[(&str, &str)]) -> Vec<String> {
    template
        .iter()
        .map(|word| {
            fill.iter().fold(word.clone(), |word, (name, value)| {
                word.replace(&format!("{{{name}}}"), value)
            })
        })
        .collect()
}

/// Fails naming the first name `items`, sorted by name, holds from two origins, and both: the
/// checkouts root holds one checkout of a name, and a snapshot records one repository of it.
pub(crate) fn one_of_each<T>(
    items: &[T],
    name: impl Fn(&T) -> &str,
    from: impl Fn(&T) -> String,
) -> Result<()> {
    if let Some(pair) = items
        .windows(2)
        .find(|pair| name(&pair[0]) == name(&pair[1]) && from(&pair[0]) != from(&pair[1]))
    {
        bail!(
            "repository {} is listed by {} and by {}; a snapshot records one repository of a name",
            name(&pair[0]),
            from(&pair[0]),
            from(&pair[1])
        );
    }
    Ok(())
}

/// `name`, when it can name a repository: GitHub's letters, digits, `-`, `_` and `.`, and not a
/// path step such as `..`.
pub(crate) fn checked(what: &str, name: String) -> Result<String> {
    let valid = !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if valid {
        Ok(name)
    } else {
        bail!("{what}: {name:?} is not a repository name")
    }
}

/// What `command` printed, run in `dir` when one is given, through [`collect::run`] under
/// `bound`. A run that exits non-zero is run once more; failing again, the answer is an error
/// naming `what` (the repository or the organization), the command and what it printed on
/// standard error. Git takes no optional locks, so reading a checkout never rewrites its index,
/// and never prompts.
pub(crate) fn answer(
    what: &str,
    command: &[String],
    dir: Option<&Path>,
    bound: Duration,
) -> Result<String> {
    let shown = command.join(" ");
    let Some((program, arguments)) = command.split_first() else {
        bail!("{what}: an empty command line");
    };
    let attempt = || {
        let mut command = Command::new(program);
        command
            .args(arguments)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0");
        if let Some(dir) = dir {
            command.current_dir(dir);
        }
        collect::run(&mut command, bound).with_context(|| format!("{what}: `{shown}`"))
    };
    let mut output = attempt()?;
    if !output.status.success() {
        output = attempt()?;
    }
    if !output.status.success() {
        let status = output.status.code().map_or_else(
            || "was stopped by a signal".to_owned(),
            |code| format!("exited {code}"),
        );
        let stderr = excerpt(&String::from_utf8_lossy(&output.stderr));
        if stderr.is_empty() {
            bail!("{what}: `{shown}` {status}");
        }
        bail!("{what}: `{shown}` {status}: {stderr}");
    }
    String::from_utf8(output.stdout)
        .with_context(|| format!("{what}: `{shown}` printed text that is not UTF-8"))
}

/// What `command` printed, read as JSON.
pub(crate) fn json<T: DeserializeOwned>(
    what: &str,
    command: &[String],
    dir: Option<&Path>,
    bound: Duration,
) -> Result<T> {
    let text = answer(what, command, dir, bound)?;
    serde_json::from_str(&text).with_context(|| {
        format!(
            "{what}: `{}` printed what this collector cannot read: {}",
            command.join(" "),
            excerpt(&text)
        )
    })
}

/// Runs `work` over each of `items`, up to `width` at once, and answers the results in the
/// items' order, or the first error in that order. Once one item has failed, no further item is
/// started.
pub(crate) fn each<T: Sync, R: Send>(
    items: &[T],
    width: usize,
    work: impl Fn(&T) -> Result<R> + Sync,
) -> Result<Vec<R>> {
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let slots: Vec<Mutex<Option<Result<R>>>> = items.iter().map(|_| Mutex::new(None)).collect();
    thread::scope(|scope| {
        for _ in 0..width.clamp(1, items.len().max(1)) {
            scope.spawn(|| {
                while !failed.load(Ordering::SeqCst) {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(item) = items.get(index) else {
                        break;
                    };
                    let answered = work(item);
                    if answered.is_err() {
                        failed.store(true, Ordering::SeqCst);
                    }
                    *slots[index].lock().unwrap_or_else(PoisonError::into_inner) = Some(answered);
                }
            });
        }
    });
    let mut results = Vec::with_capacity(items.len());
    for slot in slots {
        match slot.into_inner().unwrap_or_else(PoisonError::into_inner) {
            Some(Ok(result)) => results.push(result),
            Some(Err(error)) => return Err(error),
            None => {}
        }
    }
    if results.len() == items.len() {
        Ok(results)
    } else {
        bail!("stopped before every repository was read")
    }
}

/// `text`, an RFC 3339 instant, as the same instant in UTC (`…Z`), which the store reads back.
pub(crate) fn utc(what: &str, text: &str) -> Result<String> {
    instant(parsed_instant(what, text)?)
}

fn parsed_instant(what: &str, text: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(text.trim(), &Rfc3339)
        .with_context(|| format!("{what}: {text:?} is not an RFC 3339 instant"))
}

/// `at` in UTC, RFC 3339.
fn instant(at: OffsetDateTime) -> Result<String> {
    at.to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .context("format an instant")
}

/// `text` on one line, at most 300 characters of it.
fn excerpt(text: &str) -> String {
    let line = text
        .split(['\n', '\r'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    match line.char_indices().nth(300) {
        Some((end, _)) => format!("{}…", &line[..end]),
        None => line,
    }
}

/// `count` as the specification's integer.
fn number(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// `text`'s bytes as lower-case hex, the spelling [`unhex`] reads: a catalog whose `names` is
/// `hex` names each repository so.
#[cfg(test)]
fn hex(text: &str) -> String {
    use std::fmt::Write as _;
    text.bytes().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// The text whose bytes `hex` spells; none when it spells no UTF-8 text.
fn unhex(hex: &str) -> Option<String> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::{conventional, excluded_path, hex, maintenance, normalised, store_version, unhex};

    #[test]
    fn a_subject_is_normalised_for_the_sweep_rule() {
        assert_eq!(
            normalised("Raise the toolchain to 1.99 (#12)", "alpha"),
            "raise the toolchain to <n>"
        );
        assert_eq!(
            normalised("  Adopt the   SHARED error type ", "alpha"),
            "adopt the shared error type"
        );
        assert_eq!(
            normalised("Publish alpha docs", "alpha"),
            normalised("Publish beta docs", "beta")
        );
        assert_ne!(
            normalised("Publish alpha docs", "beta"),
            normalised("Publish beta docs", "beta")
        );
        assert_eq!(normalised("Fix (#x)", "alpha"), "fix (#x)");
    }

    #[test]
    fn maintenance_subjects_are_recognised_and_others_are_not() {
        for subject in [
            "ci: repin the Pages facade",
            "docs: explain the board",
            "style: format the board",
            "chore(release): 0.4.0",
            "chore: release 0.4.0",
            "build(deps): bump serde from 1.0.1 to 1.0.2",
            "Bump eventlog to 0.4.0",
            "Pin the facade to 3.1.0",
            "Repin the Pages facade",
            "Publish the site",
            "Retrofit the order specification",
            "Prepare release 0.4.0",
            "fix: lint the board",
            "Format the sources",
            "Plan wave 06",
            "Release 0.68.0",
            "feat!: ci the world",
        ] {
            assert!(maintenance(subject), "{subject:?} is a maintenance subject");
        }
        for subject in [
            "feat: read the board header",
            "fix: handle an empty board",
            "Adopt the shared logger",
            "Record the 10:47Z cycle",
            "chore: tidy the board",
            "Merge branch 'feature'",
            "Prepare the board",
        ] {
            assert!(
                !maintenance(subject),
                "{subject:?} is not a maintenance subject"
            );
        }
    }

    #[test]
    fn a_conventional_subject_is_split() {
        assert_eq!(
            conventional("feat(board)!: draw it"),
            (Some("feat"), Some("board"), "draw it")
        );
        assert_eq!(
            conventional("record the 10:47z cycle"),
            (None, None, "record the 10:47z cycle")
        );
        assert_eq!(conventional("fix(: x"), (None, None, "fix(: x"));
    }

    #[test]
    fn docs_ci_planning_markdown_and_lockfile_paths_are_excluded() {
        for path in [
            "docs/board.md",
            "docs/api.txt",
            "crates/x/docs/guide.html",
            ".github/workflows/ci.yml",
            ".gitlab-ci.yml",
            ".engineering/planning/wave.yaml",
            ".engineering/project.yaml",
            "README.md",
            "notes/Plan.MD",
            "Cargo.lock",
            "web/package-lock.json",
            "go.sum",
        ] {
            assert!(excluded_path(path), "{path:?} is excluded");
        }
        for path in [
            "src/board.rs",
            "Cargo.toml",
            "rust-toolchain.toml",
            "spec/order.yaml",
            "docsite.rs",
            "src/docs.rs",
            "lock.rs",
        ] {
            assert!(!excluded_path(path), "{path:?} is not excluded");
        }
    }

    #[test]
    fn the_planning_store_version_is_read_from_yaml_or_json() {
        assert_eq!(
            store_version("version: aep.project/5\n\n# A comment.\nprotocol: adp/1\n").as_deref(),
            Some("aep.project/5")
        );
        assert_eq!(
            store_version("version: \"aep.project/5\" # pinned\n").as_deref(),
            Some("aep.project/5")
        );
        assert_eq!(
            store_version("{\"version\": \"aep.project/4\"}").as_deref(),
            Some("aep.project/4")
        );
        assert_eq!(store_version("protocol: adp/1\n  version: nested\n"), None);
    }

    #[test]
    fn a_hex_catalog_name_spells_its_bytes() {
        assert_eq!(
            hex("catalog.repository"),
            "636174616c6f672e7265706f7369746f7279"
        );
        assert_eq!(
            unhex(&hex("catalog.repository")).as_deref(),
            Some("catalog.repository")
        );
        assert_eq!(
            unhex("6165702d73657276696365").as_deref(),
            Some("aep-service")
        );
        assert_eq!(unhex("6"), None);
        assert_eq!(unhex("zz"), None);
    }
}
