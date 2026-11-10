//! What a snapshot reads of a GitLab forge (`story:gitlab-repositories`): each `gitlab` source's
//! projects, and which checkout under the checkouts root holds each.
//!
//! A forge is read only through `glab api --hostname <host> <path>` ([`Forge::get`]), GitLab's
//! REST API v4, each command through [`crate::collect::run`] under the bound, asked once more when
//! it fails, as the other collectors ask theirs ([`repositories::json`]). A list is read page by
//! page, [`PER_PAGE`] entries a page, until a page is not full ([`Forge::pages`]).
//!
//! - A source's projects ([`Forge::projects`]) are `groups/<group>/projects` with its subgroups,
//!   projects shared into the group aside, archived ones included, but a project with no commit
//!   yet (no default branch), which has nothing to read.
//! - A project's checkout ([`checkouts`]) is each git checkout under the checkouts root, at depth
//!   1 or 2 (`<root>/<repo>` or `<root>/<group>/<repo>`) and not under one of the source's
//!   `exclude` entries, whose `origin` remote, read from its git config with no network read and
//!   no git command, is on the source's host and names the project. `conductor config workspace`
//!   writes the same checkouts as the source's members.
//! - The checkout is the repository and its path under the root the repository's name
//!   (`story:checkout-name-differs-from-project`), as for every other source: the checkout
//!   `checks` of the project `example-checks` is `checks` in every record, and the project's id
//!   and path are read only to call the forge. A project without a checkout is named by its path
//!   under the group (`alpha`, `sub/gamma`), but one whose name an `exclude` entry names or holds.
//!
//! The repositories collector reads each project's row from the forge ([`read`]), the
//! specifications collector each project's checkout.
//!
//! # The `gitlab` collector
//!
//! [`collect`] (`story:gitlab-merge-requests`) records, for each project of each `gitlab` source
//! that is not archived, what the `github` collector records for a GitHub repository, over the
//! same [`WINDOW`] before the snapshot started; [`collect_over`] reads any forges at any clock.
//! Projects are read up to [`WIDTH`] at once and recorded in name order; a name two sources list
//! fails the collector naming it and both.
//!
//! - One `RecordPullRequest` per open merge request (`merge_requests?state=opened`), by its `iid`:
//!   a draft when GitLab marks it one; `Conflicting` when it has conflicts or cannot be merged,
//!   `Mergeable` when it can, else `Unknown`; and one failing check when its head pipeline
//!   (`merge_requests/<iid>`) concluded a `Failure` ([`conclusion`]), named `pipeline <id>`
//!   (`story:pr-check-names`), else none; with its page
//!   (`web_url`) as its link (`story:dashboard-overview`).
//! - One `RecordMergedPullRequest` per merge request merged at or after the window's start
//!   (`merge_requests?state=merged&updated_after=<start>`), with its page as its link.
//! - One `RecordRelease` per release published at or after the window's start, upcoming ones aside
//!   (`releases`, newest first, read until a page reaches past the window), with the link
//!   `<project page>/-/releases/<tag>`; and one per semver tag (`v<major>.<minor>.<patch>`) dated
//!   at or after the window's start that no release names (`repository/tags`, by commit date,
//!   newest first, read until a page reaches past the window), dated by the tag's `created_at`
//!   (an annotated tag), else its commit's `committed_date`, with the link
//!   `<project page>/-/tags/<tag>` (`story:k5-tags-and-no-release-repositories`).
//! - One `RecordWorkflowRun` [`PIPELINE`] for the newest pipeline of the default branch
//!   (`pipelines?ref=<default branch>&per_page=1`), its status mapped by [`conclusion`], which the
//!   watch reads `main` CI by too. A project without one records none.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{GitLabSource, Instance, Source};
use conductor_model::observation::{
    Mergeability, RecordMergedPullRequest, RecordPullRequest, RecordRelease, RecordWorkflowRun,
    RepositoryName, RunConclusion, SnapshotId, Visibility,
};
use conductor_model::primitives::Timestamp;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use super::github::WINDOW;
use super::repositories::{
    self, Commit, Read, Release, WIDTH, checked, each, json, maintenance, one_of_each, utc, words,
};
use crate::collect;
use crate::config;
use crate::snapshot::Recorder;

/// The workflow a project's default-branch pipeline is recorded as.
pub const PIPELINE: &str = "pipeline";

/// The conclusion, of those the `github` collector records, of a GitLab pipeline whose `status`
/// is `status`: `success` is a `Success`; `failed` a `Failure`; `canceled` (and `canceling`) a
/// `Cancelled`; `skipped` a `Skipped`; and a pipeline still to run or decide (`created`,
/// `waiting_for_resource`, `preparing`, `pending`, `running`, `manual`, `scheduled`,
/// `waiting_for_callback`) `Pending`, as GitHub's `action_required` is. `None` for a status GitLab
/// adds that this mapping does not know. The one mapping the `gitlab` collector and the watch read
/// a pipeline by.
#[must_use]
pub(crate) fn conclusion(status: &str) -> Option<RunConclusion> {
    Some(match status {
        "success" => RunConclusion::Success,
        "failed" => RunConclusion::Failure,
        "canceled" | "canceling" => RunConclusion::Cancelled,
        "skipped" => RunConclusion::Skipped,
        "created"
        | "waiting_for_resource"
        | "preparing"
        | "pending"
        | "running"
        | "manual"
        | "scheduled"
        | "waiting_for_callback" => RunConclusion::Pending,
        _ => return None,
    })
}

/// Records, over every project of each `gitlab` source of the instance the process runs that is
/// not archived, its open merge requests, its newest default-branch pipeline, and the releases and
/// merged merge requests of the [`WINDOW`] before the snapshot started.
///
/// # Errors
///
/// The snapshot's start does not read, or what [`collect_over`] answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    let started = record.started_at()?;
    let now = OffsetDateTime::parse(&started.0, &Rfc3339)
        .with_context(|| format!("the snapshot's start {:?} is not RFC 3339", started.0))?;
    collect_over(&Forge::all(&config::active().instance), now, record)
}

/// What [`collect`] records, over the projects of `forges`, in the [`WINDOW`] before `now`.
///
/// # Errors
///
/// A command did not answer (named with its project and command) or answered something this
/// collector cannot read, two sources list one name, or a pipeline's status is one
/// [`conclusion`] does not know, and nothing is recorded; or an observation was refused.
pub fn collect_over(
    forges: &[Forge],
    now: OffsetDateTime,
    record: &mut Recorder<'_>,
) -> Result<()> {
    let start = (now - WINDOW)
        .to_offset(UtcOffset::UTC)
        .replace_nanosecond(0)?;
    let since = start
        .format(&Rfc3339)
        .context("format the window's start")?;
    let mut projects: Vec<(Forge, Project)> = Vec::new();
    for forge in forges {
        for project in forge.projects()? {
            if !project.archived {
                projects.push((forge.clone(), project));
            }
        }
    }
    projects.sort_by(|(_, one), (_, other)| one.name.cmp(&other.name));
    one_of_each(
        &projects,
        |(_, project)| &project.name,
        |(forge, _)| forge.named(),
    )?;
    let snapshot = record.snapshot().clone();
    let window = Window { start, since };
    let observed = each(&projects, WIDTH, |(forge, project)| {
        observe(forge, project, &snapshot, &window)
    })?;
    for observed in observed {
        for pull in observed.pulls {
            record.pull_request(pull)?;
        }
        if let Some(run) = observed.run {
            record.workflow_run(run)?;
        }
        for release in observed.releases {
            record.release(release)?;
        }
        for merged in observed.merged {
            record.merged_pull_request(merged)?;
        }
    }
    Ok(())
}

/// Where the window opens: the instant, and as `updated_after` writes it (RFC 3339, UTC).
struct Window {
    start: OffsetDateTime,
    since: String,
}

/// What one project shows, as observations of a snapshot.
struct Observed {
    pulls: Vec<RecordPullRequest>,
    run: Option<RecordWorkflowRun>,
    releases: Vec<RecordRelease>,
    merged: Vec<RecordMergedPullRequest>,
}

/// One open merge request, as `merge_requests?state=opened` answers it.
#[derive(Deserialize)]
struct ApiMergeRequest {
    iid: i64,
    title: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    work_in_progress: bool,
    #[serde(default)]
    has_conflicts: bool,
    #[serde(default)]
    merge_status: Option<String>,
    created_at: String,
    updated_at: String,
    /// The merge request's page; `None` from a forge that does not answer it.
    #[serde(default)]
    web_url: Option<String>,
}

/// One merge request, as `merge_requests/<iid>` answers it, in the part the list lacks.
#[derive(Deserialize)]
struct ApiMergeRequestDetail {
    #[serde(default)]
    head_pipeline: Option<ApiPipeline>,
}

/// One merged merge request, as `merge_requests?state=merged` answers it.
#[derive(Deserialize)]
struct ApiMerged {
    iid: i64,
    title: String,
    #[serde(default)]
    merged_at: Option<String>,
    web_url: String,
}

/// One pipeline.
#[derive(Deserialize)]
struct ApiPipeline {
    id: i64,
    status: String,
    created_at: String,
}

/// `project` of `forge`, as observations of `snapshot` in `window`.
fn observe(
    forge: &Forge,
    project: &Project,
    snapshot: &SnapshotId,
    window: &Window,
) -> Result<Observed> {
    let what = format!("repository {}", project.name);
    let what = what.as_str();
    let id = project.id;
    let repository = || RepositoryName(project.name.clone());
    let instant = |text: &str| {
        OffsetDateTime::parse(text.trim(), &Rfc3339)
            .map_err(|_| anyhow!("{what}: {text:?} is not an RFC 3339 instant"))
    };

    let mut opened: Vec<ApiMergeRequest> =
        forge.pages(what, &format!("projects/{id}/merge_requests?state=opened"))?;
    opened.sort_by_key(|request| request.iid);
    let mut pulls = Vec::with_capacity(opened.len());
    for request in opened {
        let detail: ApiMergeRequestDetail = forge.get(
            what,
            &format!("projects/{id}/merge_requests/{}", request.iid),
        )?;
        // A failed head pipeline is the one failing check, named for its id
        // (`story:pr-check-names`).
        let failing_check_names: Vec<String> = detail
            .head_pipeline
            .filter(|pipeline| conclusion(&pipeline.status) == Some(RunConclusion::Failure))
            .map(|pipeline| format!("pipeline {}", pipeline.id))
            .into_iter()
            .collect();
        let mergeability = if request.has_conflicts {
            Mergeability::Conflicting
        } else {
            match request.merge_status.as_deref() {
                Some("can_be_merged") => Mergeability::Mergeable,
                Some("cannot_be_merged") => Mergeability::Conflicting,
                _ => Mergeability::Unknown,
            }
        };
        pulls.push(RecordPullRequest {
            actor: crate::actor::daemon(),
            snapshot_id: snapshot.clone(),
            repository: repository(),
            number: request.iid,
            title: request.title,
            draft: request.draft || request.work_in_progress,
            mergeability,
            failing_checks: i64::try_from(failing_check_names.len()).unwrap_or(i64::MAX),
            failing_check_names,
            opened_at: Timestamp(utc(what, &request.created_at)?),
            updated_at: Timestamp(utc(what, &request.updated_at)?),
            url: request.web_url,
        });
    }

    let listed: Vec<ApiMerged> = forge.pages(
        what,
        &format!(
            "projects/{id}/merge_requests?state=merged&updated_after={}",
            encoded(&window.since)
        ),
    )?;
    let mut merged = Vec::new();
    for request in listed {
        let Some(merged_at) = request.merged_at.filter(|at| !at.trim().is_empty()) else {
            continue;
        };
        if instant(&merged_at)? < window.start {
            continue;
        }
        merged.push(RecordMergedPullRequest {
            actor: crate::actor::daemon(),
            snapshot_id: snapshot.clone(),
            repository: repository(),
            number: request.iid,
            title: request.title,
            merged_at: Timestamp(utc(what, &merged_at)?),
            url: request.web_url,
        });
    }
    merged.sort_by_key(|request| request.number);

    let mut releases = Vec::new();
    // Every tag a release read names, upcoming ones' included: such a tag is no release of its own.
    let mut named = std::collections::BTreeSet::new();
    for page in 1..=PAGES {
        let listed: Vec<ApiRelease> = forge.get(
            what,
            &format!("projects/{id}/releases?per_page={PER_PAGE}&page={page}"),
        )?;
        let full = listed.len() >= PER_PAGE;
        let mut past = false;
        for release in listed {
            named.insert(release.tag_name.clone());
            let Some(published) = release.released_at.filter(|at| !at.trim().is_empty()) else {
                continue;
            };
            let at = instant(&published)?;
            if at < window.start {
                past = true;
                continue;
            }
            if release.upcoming_release {
                continue;
            }
            releases.push((
                at,
                RecordRelease {
                    actor: crate::actor::daemon(),
                    snapshot_id: snapshot.clone(),
                    repository: repository(),
                    url: format!(
                        "{}/-/releases/{}",
                        project.web_url.trim_end_matches('/'),
                        release.tag_name
                    ),
                    tag: release.tag_name,
                    published_at: Timestamp(utc(what, &published)?),
                },
            ));
        }
        if !full || past {
            break;
        }
    }
    // A semver tag no release names is a release of its own, dated by GitLab's tag date (an
    // annotated tag's `created_at`), else by its commit's (story:k5-tags-and-no-release-repositories).
    for page in 1..=PAGES {
        let listed: Vec<ApiTag> = forge.get(
            what,
            &format!(
                "projects/{id}/repository/tags?order_by=updated&sort=desc&per_page={PER_PAGE}&page={page}"
            ),
        )?;
        let full = listed.len() >= PER_PAGE;
        let mut past = false;
        for tag in listed {
            let committed = tag
                .commit
                .and_then(|commit| commit.committed_date)
                .filter(|at| !at.trim().is_empty());
            if committed
                .as_deref()
                .map(instant)
                .transpose()?
                .is_some_and(|at| at < window.start)
            {
                past = true;
            }
            let Some(dated) = tag
                .created_at
                .filter(|at| !at.trim().is_empty())
                .or(committed)
            else {
                continue;
            };
            if !super::github::semver_tag(&tag.name)
                || tag.release.is_some()
                || named.contains(&tag.name)
            {
                continue;
            }
            let at = instant(&dated)?;
            if at < window.start {
                continue;
            }
            releases.push((
                at,
                RecordRelease {
                    actor: crate::actor::daemon(),
                    snapshot_id: snapshot.clone(),
                    repository: repository(),
                    url: format!(
                        "{}/-/tags/{}",
                        project.web_url.trim_end_matches('/'),
                        tag.name
                    ),
                    tag: tag.name,
                    published_at: Timestamp(utc(what, &dated)?),
                },
            ));
        }
        if !full || past {
            break;
        }
    }
    releases.sort_by(|(a, one), (b, other)| a.cmp(b).then_with(|| one.tag.cmp(&other.tag)));

    let pipelines: Vec<ApiPipeline> = forge.get(
        what,
        &format!(
            "projects/{id}/pipelines?ref={}&per_page=1",
            encoded(&project.default_branch)
        ),
    )?;
    let run = match pipelines.into_iter().next() {
        None => None,
        Some(pipeline) => Some(RecordWorkflowRun {
            actor: crate::actor::daemon(),
            snapshot_id: snapshot.clone(),
            repository: repository(),
            workflow: PIPELINE.to_owned(),
            conclusion: conclusion(&pipeline.status).ok_or_else(|| {
                anyhow!(
                    "{what}: pipeline {} is {:?}, a status this collector does not know",
                    pipeline.id,
                    pipeline.status
                )
            })?,
            run_at: Timestamp(utc(what, &pipeline.created_at)?),
        }),
    };

    Ok(Observed {
        pulls,
        run,
        releases: releases.into_iter().map(|(_, release)| release).collect(),
        merged,
    })
}

/// The entries a page of a list is asked for with.
pub const PER_PAGE: usize = 100;

/// The pages a list is read to at most: a list longer than this many full pages fails the
/// collector rather than being read for ever.
pub const PAGES: usize = 100;

/// One `gitlab` source, as the collectors read it: its forge, its group and its `exclude`
/// entries, and the program that reads the forge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forge {
    /// The program and any arguments `api --hostname <host> <path>` is appended to: `glab`.
    pub glab: Vec<String>,
    /// The forge's host name, `gitlab.com` when the source names none.
    pub host: String,
    /// The group whose projects are read, as the source writes it.
    pub group: String,
    /// The source's `exclude` entries: paths under the checkouts root, each a repository's name, a
    /// checkout's path or, for a project without a checkout, its path under the group.
    pub exclude: Vec<String>,
    /// The checkouts root, whose checkouts name the projects they hold ([`Forge::projects`]).
    pub checkouts: PathBuf,
    /// How long one command may run before it is stopped.
    pub bound: Duration,
}

impl Forge {
    /// `source`'s forge, read through `glab` under [`collect::BOUND`], its projects' checkouts
    /// under the checkouts root `checkouts`.
    #[must_use]
    pub fn of(source: &GitLabSource, checkouts: &Path) -> Self {
        Self {
            glab: words(&["glab"]),
            host: source.host.clone(),
            group: source.group.clone(),
            exclude: source.exclude.clone(),
            checkouts: checkouts.to_owned(),
            bound: collect::BOUND,
        }
    }

    /// Each `gitlab` source of `instance`, in its order, each once.
    #[must_use]
    pub fn all(instance: &Instance) -> Vec<Self> {
        let mut forges: Vec<Self> = Vec::new();
        for source in &instance.sources {
            if let Source::GitLab(gitlab) = source {
                let forge = Self::of(gitlab, Path::new(&instance.checkouts.root));
                if !forges.contains(&forge) {
                    forges.push(forge);
                }
            }
        }
        forges
    }

    /// The source, as an error names it.
    #[must_use]
    pub fn named(&self) -> String {
        format!("the gitlab group {} on {}", self.group, self.host)
    }

    /// The command line that reads the API path `path`.
    fn command(&self, path: &str) -> Vec<String> {
        let mut command = self.glab.clone();
        command.extend(words(&["api", "--hostname", &self.host, path]));
        command
    }

    /// What the API answers at `path`, read as JSON; `what` names the project or source an error
    /// names.
    ///
    /// # Errors
    ///
    /// `glab` failed twice, or printed what this collector cannot read.
    pub(crate) fn get<T: DeserializeOwned>(&self, what: &str, path: &str) -> Result<T> {
        json(what, &self.command(path), None, self.bound)
    }

    /// Every entry of the list at `path`, read page by page until a page is not full.
    ///
    /// # Errors
    ///
    /// What [`Forge::get`] answers, or the list is longer than [`PAGES`] full pages.
    pub(crate) fn pages<T: DeserializeOwned>(&self, what: &str, path: &str) -> Result<Vec<T>> {
        let joint = if path.contains('?') { '&' } else { '?' };
        let mut entries = Vec::new();
        for page in 1..=PAGES {
            let answered: Vec<T> = self.get(
                what,
                &format!("{path}{joint}per_page={PER_PAGE}&page={page}"),
            )?;
            let full = answered.len() >= PER_PAGE;
            entries.extend(answered);
            if !full {
                return Ok(entries);
            }
        }
        bail!(
            "{what}: `{}` holds more than {PAGES} pages of {PER_PAGE}; it is not read whole",
            path
        )
    }

    /// The source's repositories, by name: each project of the group and its subgroups, archived
    /// ones included, but those with no default branch, once per checkout of it under
    /// [`Forge::checkouts`] ([`found`]).
    ///
    /// The directory is the repository (`story:checkout-name-differs-from-project`), as it is
    /// for every other source: a checkout is named by its path under the checkouts root,
    /// whatever its project is called, so the checkout `checks` of the project `example-checks`
    /// is `checks`, and two checkouts of one project are two repositories. A project without a
    /// checkout is named by its path under the group, the path its checkout would have. An
    /// `exclude` entry names a repository by that name: a checkout it names or holds is none,
    /// and its project is not listed under its own name beside it.
    ///
    /// # Errors
    ///
    /// The list did not answer, a project's path is not under the group, a step of a name is no
    /// repository name, or a project without a checkout has the name of another project's
    /// checkout.
    pub fn projects(&self) -> Result<Vec<Project>> {
        let what = self.named();
        let listed: Vec<ApiProject> = self.pages(
            &what,
            &format!(
                "groups/{}/projects?include_subgroups=true&with_shared=false",
                encoded(self.group.trim_matches('/'))
            ),
        )?;
        let checkouts = found(&self.checkouts, &self.host, &self.group);
        let mut seen = std::collections::BTreeSet::new();
        let mut projects = Vec::new();
        for project in listed {
            let Some(path) = under(&self.group, &project.path_with_namespace) else {
                bail!(
                    "{what}: the project {} is not under the group",
                    project.path_with_namespace
                );
            };
            let steps: Vec<&str> = path.split('/').collect();
            for step in &steps {
                checked(&what, (*step).to_owned())?;
            }
            let excluded = collect::is_excluded(steps.as_slice(), &self.exclude);
            let Some(default_branch) = project
                .default_branch
                .filter(|branch| !branch.is_empty() && !project.empty_repo)
            else {
                continue;
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            let listed = Project {
                id: project.id,
                name: path.clone(),
                path,
                checkout: None,
                archived: project.archived,
                visibility: match project.visibility.as_deref() {
                    Some("public") => Visibility::Public,
                    _ => Visibility::Private,
                },
                default_branch,
                open_issues: project.open_issues_count.unwrap_or(0).max(0),
                web_url: project.web_url,
            };
            let mut held = checkouts
                .iter()
                .filter(|checkout| checkout.project.eq_ignore_ascii_case(&listed.path))
                .peekable();
            if held.peek().is_none() {
                if !excluded {
                    projects.push(listed);
                }
                continue;
            }
            for checkout in held {
                let steps: Vec<&str> = checkout.path.split('/').collect();
                if collect::is_excluded(steps.as_slice(), &self.exclude) {
                    continue;
                }
                for step in &steps {
                    checked(&what, (*step).to_owned())?;
                }
                projects.push(Project {
                    name: checkout.path.clone(),
                    checkout: Some(checkout.dir.clone()),
                    ..listed.clone()
                });
            }
        }
        projects.sort_by(|one, other| one.name.cmp(&other.name));
        // Two checkouts have two paths, and two listed projects two: a name held twice is a
        // checkout's and that of a project without one.
        if let Some(pair) = projects
            .windows(2)
            .find(|pair| pair[0].name == pair[1].name)
        {
            let (held, bare) = if pair[0].checkout.is_some() {
                (&pair[0], &pair[1])
            } else {
                (&pair[1], &pair[0])
            };
            bail!(
                "{what}: repository {} is the checkout of the project {} and the project {}, \
                 which has no checkout; a snapshot records one repository of a name",
                held.name,
                held.path,
                bare.path
            );
        }
        Ok(projects)
    }

    /// The source's repositories ([`Forge::projects`]), each with its forge.
    ///
    /// # Errors
    ///
    /// What [`Forge::projects`] answers.
    pub fn mapped(&self) -> Result<Vec<Mapped>> {
        Ok(self
            .projects()?
            .into_iter()
            .map(|project| Mapped {
                forge: self.clone(),
                checkout: project.checkout.clone(),
                project,
            })
            .collect())
    }
}

/// One repository of a group, as [`Forge::projects`] lists it: a project, in one checkout of it
/// or in none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// GitLab's id of the project, which every further API path names.
    pub id: u64,
    /// The repository's name: its checkout's path under the checkouts root (`checks`, or
    /// `sub/gamma` in a group directory), or without a checkout the project's path under the
    /// group. What every record names it by.
    pub name: String,
    /// The project's path under the group: `example-checks`, or `sub/gamma` in a subgroup. With
    /// [`Project::id`] the project's address on the forge, and no repository's name.
    pub path: String,
    /// The checkout this repository is, when the project has one.
    pub checkout: Option<PathBuf>,
    pub archived: bool,
    /// `Public` for a public project; `Private` for an internal or a private one.
    pub visibility: Visibility,
    pub default_branch: String,
    /// Its open issues, as the project list counts them; 0 with issues turned off.
    pub open_issues: i64,
    /// The project's page, which a release's link starts with.
    pub web_url: String,
}

/// A project, its forge, and its checkout under the checkouts root when one names it.
#[derive(Debug, Clone)]
pub struct Mapped {
    pub forge: Forge,
    pub project: Project,
    pub checkout: Option<PathBuf>,
}

/// One project of `groups/<group>/projects`, in the parts this collector reads.
#[derive(Deserialize)]
struct ApiProject {
    id: u64,
    path_with_namespace: String,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    visibility: Option<String>,
    #[serde(default)]
    default_branch: Option<String>,
    #[serde(default)]
    open_issues_count: Option<i64>,
    #[serde(default)]
    empty_repo: bool,
    #[serde(default)]
    web_url: String,
}

/// One commit, as `repository/commits` answers it.
#[derive(Deserialize)]
struct ApiCommit {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    parent_ids: Vec<String>,
    #[serde(default)]
    committed_date: Option<String>,
}

impl ApiCommit {
    fn subject(&self) -> String {
        if self.title.is_empty() {
            self.message.lines().next().unwrap_or_default().to_owned()
        } else {
            self.title.clone()
        }
    }
}

/// One file of a commit's diff.
#[derive(Deserialize)]
struct ApiDiff {
    new_path: String,
}

/// One release, as `releases` answers it.
#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    released_at: Option<String>,
    #[serde(default)]
    upcoming_release: bool,
}

/// One tag, as `repository/tags` answers it, in the parts this collector reads.
#[derive(Deserialize)]
struct ApiTag {
    name: String,
    /// An annotated tag's date; `null` for a lightweight one.
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    commit: Option<ApiTagCommit>,
    /// The release that names the tag, if one does.
    #[serde(default)]
    release: Option<serde_json::Value>,
}

/// The commit a tag points at, in the part this collector reads.
#[derive(Deserialize)]
struct ApiTagCommit {
    #[serde(default)]
    committed_date: Option<String>,
}

/// What `repository/compare` answers, in the part this collector reads.
#[derive(Deserialize)]
struct ApiCompare {
    #[serde(default)]
    commits: Vec<serde_json::Value>,
}

/// One issue, as `issues` answers it.
#[derive(Deserialize)]
struct ApiIssue {
    created_at: String,
}

/// One project's repository row, read from the forge, `since` being the 7-day window's start
/// (RFC 3339, UTC); what only a checkout shows is read from `mapped`'s checkout when it has one
/// ([`repositories::checkout_state`]), with no fetch.
///
/// | fields | source |
/// |---|---|
/// | `main_head`, `main_committed_at` | `repository/commits/<default branch>` |
/// | `main_commits_7d`, `real_commits_7d` | `repository/commits?ref_name=<default branch>&since=<since>`, and `repository/commits/<sha>/diff` of each commit real activity needs the paths of |
/// | `latest_release`, `latest_release_at` | the newest of `releases` that is not upcoming, and its `released_at` |
/// | `unreleased_commits` | the commits of `repository/compare?from=<tag>&to=<default branch>` |
/// | `open_issues` | the project list's `open_issues_count` |
/// | `oldest_open_issue_at` | the oldest of `issues?state=opened`, asked only when one is open |
///
/// # Errors
///
/// A command failed twice or printed what this collector cannot read, or a git command in the
/// checkout failed.
pub(super) fn read(mapped: &Mapped, what: &str, since: &str) -> Result<Read> {
    let forge = &mapped.forge;
    let project = &mapped.project;
    let id = project.id;
    let branch = encoded(&project.default_branch);
    let head: ApiCommit = forge.get(what, &format!("projects/{id}/repository/commits/{branch}"))?;
    let Some(committed_at) = head.committed_date.as_deref() else {
        bail!(
            "{what}: the head of {} has no commit date",
            project.default_branch
        );
    };
    let main_committed_at = utc(what, committed_at)?;
    let listed: Vec<ApiCommit> = forge.pages(
        what,
        &format!(
            "projects/{id}/repository/commits?ref_name={branch}&since={}",
            encoded(since)
        ),
    )?;
    let main_commits_7d = count(listed.len());
    let mut seen = std::collections::HashSet::new();
    let mut commits = Vec::new();
    for commit in &listed {
        if commit.parent_ids.len() > 1 || !seen.insert(commit.id.clone()) {
            continue;
        }
        let subject = commit.subject();
        let files = if maintenance(&subject) {
            Vec::new()
        } else {
            let diff: Vec<ApiDiff> = forge.pages(
                what,
                &format!(
                    "projects/{id}/repository/commits/{}/diff",
                    encoded(&commit.id)
                ),
            )?;
            diff.into_iter().map(|file| file.new_path).collect()
        };
        commits.push(Commit { subject, files });
    }

    let releases: Vec<ApiRelease> =
        forge.get(what, &format!("projects/{id}/releases?per_page=20"))?;
    let release = match releases
        .into_iter()
        .find(|release| !release.upcoming_release)
    {
        None => Release::default(),
        Some(release) => {
            let compare: ApiCompare = forge.get(
                what,
                &format!(
                    "projects/{id}/repository/compare?from={}&to={branch}",
                    encoded(&release.tag_name)
                ),
            )?;
            Release {
                at: release
                    .released_at
                    .filter(|at| !at.trim().is_empty())
                    .map(|at| utc(what, &at))
                    .transpose()?,
                unreleased: Some(count(compare.commits.len())),
                tag: Some(release.tag_name),
            }
        }
    };

    let oldest_open_issue_at = if project.open_issues > 0 {
        let oldest: Vec<ApiIssue> = forge.get(
            what,
            &format!("projects/{id}/issues?state=opened&order_by=created_at&sort=asc&per_page=1"),
        )?;
        oldest
            .first()
            .map(|issue| utc(what, &issue.created_at))
            .transpose()?
    } else {
        None
    };

    let (behind_main, dirty_files, worktrees, planning_store_version) = match &mapped.checkout {
        Some(dir) => repositories::checkout_state(what, dir, &project.default_branch, forge.bound)?,
        None => (0, 0, 0, None),
    };
    Ok(Read {
        local_checkout: mapped.checkout.is_some(),
        main_head: head.id,
        main_committed_at,
        main_commits_7d,
        commits,
        behind_main,
        dirty_files,
        worktrees,
        open_issues: project.open_issues,
        oldest_open_issue_at,
        release,
        planning_store_version,
    })
}

/// `count` as the specification's integer.
fn count(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// `text` as one step of an API path or a query value: every byte but an ASCII letter, a digit,
/// `-`, `.`, `_` and `~` percent-encoded, so a group `example-group/platform` is
/// `example-group%2Fplatform`.
#[must_use]
pub fn encoded(text: &str) -> String {
    use std::fmt::Write as _;
    text.bytes().fold(String::new(), |mut out, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
        out
    })
}

/// The path of `full` under the group `group`, compared without regard to case: `alpha` of
/// `example-group/alpha` under `example-group`; every path is under the root, written as an empty
/// group. `None` when `full` is not under the group, or is the group itself.
#[must_use]
pub fn under(group: &str, full: &str) -> Option<String> {
    let group = group.trim_matches('/');
    let full = full.trim_matches('/');
    if group.is_empty() {
        return (!full.is_empty()).then(|| full.to_owned());
    }
    let prefix = full.get(..group.len())?;
    let rest = full.get(group.len()..)?.strip_prefix('/')?;
    (prefix.eq_ignore_ascii_case(group) && !rest.is_empty()).then(|| rest.to_owned())
}

// ---------------------------------------------------------------------------------------------
// Checkouts
// ---------------------------------------------------------------------------------------------

/// A git checkout under the checkouts root whose `origin` names a project of a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkout {
    /// The checkout.
    pub dir: PathBuf,
    /// Its path under the checkouts root: `<repo>` or `<group>/<repo>`.
    pub path: String,
    /// The project its `origin` names, by its path under the group.
    pub project: String,
}

/// The git checkouts under `root` whose `origin` remote is on `host` and names a project under
/// `group`, by their path under `root`: each directory at depth 1 holding `.git`, and each at
/// depth 2 holding one in a directory at depth 1 that holds none, but those an entry of `exclude`
/// names or holds. The `origin` is read from each checkout's git config with no network read and
/// no git command ([`origin`]); hosts are compared without their port and without regard to case.
/// A directory that cannot be read, or whose name is not UTF-8, holds none.
#[must_use]
pub fn checkouts(root: &Path, host: &str, group: &str, exclude: &[String]) -> Vec<Checkout> {
    let mut found = found(root, host, group);
    found.retain(|checkout| {
        let steps: Vec<&str> = checkout.path.split('/').collect();
        !collect::is_excluded(steps.as_slice(), exclude)
    });
    found
}

/// What [`checkouts`] answers with no `exclude` entry: every checkout of a project of the group,
/// those a source leaves out included, which [`Forge::projects`] reads so that a project whose
/// checkout is excluded is not taken to have none.
fn found(root: &Path, host: &str, group: &str) -> Vec<Checkout> {
    let host = host_name(host);
    let mut found = Vec::new();
    let mut consider = |dir: PathBuf, steps: &[&str]| {
        let Some((origin_host, origin_path)) = origin(&dir) else {
            return;
        };
        if host_name(&origin_host) != host {
            return;
        }
        if let Some(project) = under(group, &origin_path) {
            found.push(Checkout {
                dir,
                path: steps.join("/"),
                project,
            });
        }
    };
    for first in directories(root) {
        let Some(one) = first.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let one = one.to_owned();
        if first.join(".git").exists() {
            consider(first, &[one.as_str()][..]);
            continue;
        }
        for second in directories(&first) {
            let Some(two) = second.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let two = two.to_owned();
            if second.join(".git").exists() {
                consider(second, &[one.as_str(), two.as_str()][..]);
            }
        }
    }
    found.sort_by(|one, other| one.path.cmp(&other.path));
    found
}

/// The directories directly under `dir`, by name; none when it cannot be read.
fn directories(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// `host` without a port, in lower case.
fn host_name(host: &str) -> String {
    host.split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// The host and the path the `origin` remote of the checkout `dir` names, read from its git
/// config: `<dir>/.git/config`, or, for a `.git` file (a linked worktree), the config of the
/// repository it names. `None` without an `origin` URL, or for one that names no host (a local
/// path, `file://`).
#[must_use]
pub fn origin(dir: &Path) -> Option<(String, String)> {
    let git = dir.join(".git");
    let config = if git.is_dir() {
        git.join("config")
    } else {
        let pointer = fs::read_to_string(&git).ok()?;
        let gitdir = dir.join(pointer.trim().strip_prefix("gitdir:")?.trim());
        let common = fs::read_to_string(gitdir.join("commondir"))
            .ok()
            .map_or_else(|| gitdir.clone(), |common| gitdir.join(common.trim()));
        common.join("config")
    };
    remote(&origin_url(&fs::read_to_string(config).ok()?)?)
}

/// The `url` of `[remote "origin"]` in the git config `text`, the first one written.
fn origin_url(text: &str) -> Option<String> {
    let mut in_origin = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix('[') {
            let header = header.split(']').next().unwrap_or_default().trim();
            in_origin = header
                .split_once(char::is_whitespace)
                .is_some_and(|(section, name)| {
                    section.eq_ignore_ascii_case("remote") && name.trim() == "\"origin\""
                });
            continue;
        }
        if !in_origin {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim().eq_ignore_ascii_case("url")
        {
            let value = value.trim().trim_matches('"').trim();
            return (!value.is_empty()).then(|| value.to_owned());
        }
    }
    None
}

/// The host (without port, in lower case) and the path, without `.git` and slashes at either
/// end, of the remote URL `url`: `scheme://[user@]host[:port]/path` or `[user@]host:path`.
/// `None` for a local path or a URL without a host or a path.
#[must_use]
pub fn remote(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let (authority, path) = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?,
        None => {
            let (authority, path) = url.split_once(':')?;
            if authority.contains('/') {
                return None;
            }
            (authority, path)
        }
    };
    let host = host_name(authority.rsplit('@').next().unwrap_or_default());
    let path = path.trim_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some((host, path.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{encoded, origin_url, remote, under};

    #[test]
    fn a_group_path_is_encoded_as_one_step() {
        assert_eq!(
            encoded("example-group/platform"),
            "example-group%2Fplatform"
        );
        assert_eq!(encoded("v1.2.0"), "v1.2.0");
        assert_eq!(encoded("2026-09-30T12:00:00Z"), "2026-09-30T12%3A00%3A00Z");
    }

    #[test]
    fn a_project_is_named_by_its_path_under_the_group() {
        assert_eq!(
            under("example-group", "example-group/alpha").as_deref(),
            Some("alpha")
        );
        assert_eq!(
            under("Example-Group/", "example-group/sub/gamma").as_deref(),
            Some("sub/gamma")
        );
        assert_eq!(under("example-group", "example-groups/alpha"), None);
        assert_eq!(under("example-group", "example-group"), None);
        assert_eq!(
            under("", "example-group/alpha").as_deref(),
            Some("example-group/alpha")
        );
    }

    #[test]
    fn a_remote_url_gives_its_host_and_path() {
        let alpha = Some((
            "gitlab.example.com".to_owned(),
            "example-group/alpha".to_owned(),
        ));
        assert_eq!(
            remote("git@gitlab.example.com:example-group/alpha.git"),
            alpha
        );
        assert_eq!(
            remote("https://gitlab.example.com/example-group/alpha.git"),
            alpha
        );
        assert_eq!(
            remote("https://user@GitLab.Example.com:8443/example-group/alpha/"),
            alpha
        );
        assert_eq!(
            remote("ssh://git@gitlab.example.com:2222/example-group/alpha"),
            alpha
        );
        assert_eq!(remote("/srv/git/alpha.git"), None);
        assert_eq!(remote("file:///srv/git/alpha.git"), None);
        assert_eq!(remote("../alpha"), None);
    }

    #[test]
    fn the_origin_url_is_read_from_its_section() {
        let config = "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = git@a:x/y.git\n\
                      [remote \"origin\"]\n\turl = git@gitlab.example.com:example-group/alpha.git\n\
                      \tfetch = +refs/heads/*:refs/remotes/origin/*\n";
        assert_eq!(
            origin_url(config).as_deref(),
            Some("git@gitlab.example.com:example-group/alpha.git")
        );
        assert_eq!(origin_url("[core]\n\turl = x\n"), None);
    }
}
