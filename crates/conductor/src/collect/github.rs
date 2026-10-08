//! The `github` collector: open pull requests and `main`'s workflow runs over every non-archived
//! repository each GitHub owner of the instance lists (`story:collect-prs-and-ci`), and what
//! shipped in the [`WINDOW`] before the snapshot started: releases and merged pull requests
//! (`story:release-digest`).
//!
//! [`collect`] reads the live sources of the instance the process runs ([`Sources::of`],
//! [`Shipments::live`] at the snapshot's start, and the owners of [`Origins::of`] over
//! [`config::active`]); [`collect_over`] reads any [`Sources`], [`Shipments`] and owners, and
//! [`collect_from`] and [`collect_all_from`] the one owner [`Sources::organization`], which are
//! the seams a test points at fixtures. A local source has no GitHub, so this collector reads
//! none. A name two owners list fails the collector naming it and both owners.
//!
//! - One `RecordPullRequest` per open pull request ([`Sources::pull_requests`]). Its
//!   `failing_checks` counts the rollup entries whose `conclusion` is `FAILURE`, `TIMED_OUT` or
//!   `ACTION_REQUIRED`; a status context, which carries a `state` and no `conclusion`, is not
//!   counted, as the reference loop in `docs/analysis/check-prs-ci.md` does not count it.
//! - One `RecordWorkflowRun` per workflow: the latest by `createdAt` of the last 20 `main` runs
//!   ([`Sources::workflow_runs`]) over [`ANSWERS`] answers of that list. Its `conclusion` is
//!   GitHub's, `Pending` when it is empty; `timed_out` and `startup_failure` are a `Failure`,
//!   `action_required` is `Pending` and `stale` is `Cancelled`, which the specification's six
//!   values leave to this collector.
//! - One `RecordRelease` per release ([`Shipments::releases`], the latest 20, drafts aside)
//!   published at or after the window's start, prereleases aside, with its link
//!   `https://github.com/<owner>/<repository>/releases/tag/<tag>`.
//! - One `RecordMergedPullRequest` per pull request merged at or after the window's start
//!   ([`Shipments::merged_pull_requests`], up to 200, which searches from the window's first day),
//!   with GitHub's link.
//!
//! GitHub answers the run list with an older list at times (`story:ci-runs-stale-answer`). An
//! older list only lacks newer runs, so the latest run of a workflow over both answers is the
//! latest GitHub has. When the newest complete snapshot holds a later run of a workflow than every
//! answer holds, the list is asked up to [`MORE_ANSWERS`] more times. A workflow still behind
//! after that is recorded as the answers give it, with one line on standard error naming the
//! repository, the workflow, the latest instant answered and the instant stored, and the snapshot
//! completes: a run GitHub no longer answers (a deleted run, a recreated workflow) never fails a
//! snapshot. A workflow no answer holds is not compared. The newest complete snapshot is read through the
//! store's ports, as [`repository::activity`](crate::repository::activity) reads it, over a handle
//! on the store under the recorder's state directory ([`Recorder::state`]), and only when an answer
//! holds a run: without a complete snapshot nothing is compared.
//!
//! A list that comes back full with none of it older than the window may be cut, so it is asked
//! for once more with a larger limit ([`Shipments::release_limits`], 100 releases, and
//! [`Shipments::merged_limits`], 1000 pull requests). When that one is full too, the collector
//! fails naming the repository, the list and the limit: nothing is cut silently.
//!
//! A source that does not answer fails the collector as in
//! [`repositories`](crate::collect::repositories): asked once more, then an error naming the
//! repository and the command, and nothing recorded. Repositories are read up to
//! [`Sources::width`] at once and recorded in name order, pull requests by number, runs by
//! workflow name, releases by publication and merged pull requests by number. Every instant is
//! recorded in UTC.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{SnapshotStorage, WorkflowRunObservationStorage};
use conductor_model::config::Instance;
use conductor_model::observation::{
    Mergeability, RecordMergedPullRequest, RecordPullRequest, RecordRelease, RecordWorkflowRun,
    RepositoryName, RunConclusion, SnapshotId, SnapshotSnapshot, SnapshotState,
};
use conductor_model::primitives::Timestamp;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use super::repositories::{WIDTH, checked, each, filled, json, one_of_each, utc, words};
use crate::collect::{self, Origins};
use crate::config;
use crate::snapshot::Recorder;

/// The rollup conclusions that make a check failing.
const FAILING: [&str; 3] = ["FAILURE", "TIMED_OUT", "ACTION_REQUIRED"];

/// How many times each repository's run list is asked: GitHub answers it with an older list at
/// times, and an older list only lacks newer runs.
pub const ANSWERS: usize = 2;

/// How many more times a run list is asked while a workflow's latest run in its answers is older
/// than the newest complete snapshot's.
pub const MORE_ANSWERS: usize = 2;

/// How far back what shipped is recorded: the 14 days before the collector's clock, which is the
/// snapshot's start when the collector runs live.
pub const WINDOW: time::Duration = time::Duration::days(14);

/// What the collector reads: the one seam a test replaces. [`Sources::of`] gives an instance's.
///
/// Each command line is a template: in every word, `{organization}` is filled with the owner
/// whose repository it asks about and `{repository}` with the repository's name.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The GitHub owner [`collect_from`] and [`collect_all_from`] list; [`collect_over`] lists
    /// the owners it is given instead.
    pub organization: String,
    /// Prints one owner's repositories as one JSON array of objects with `name` and
    /// `isArchived`.
    pub repository_list: Vec<String>,
    /// Prints one repository's open pull requests as one JSON array of objects with `number`,
    /// `title`, `isDraft`, `mergeable`, `statusCheckRollup`, `createdAt` and `updatedAt`.
    pub pull_requests: Vec<String>,
    /// Prints one repository's latest `main` runs as one JSON array of objects with
    /// `workflowName`, `conclusion` and `createdAt`. Asked [`ANSWERS`] times, and up to
    /// [`MORE_ANSWERS`] more.
    pub workflow_runs: Vec<String>,
    /// How many repositories are read at once.
    pub width: usize,
    /// How long one command may run before it is stopped.
    pub bound: Duration,
}

impl Sources {
    /// `instance`'s: its first `github:` owner as [`Sources::organization`] (none when it has
    /// none), every command through `gh`, [`WIDTH`] repositories at once, and [`collect::BOUND`]
    /// on each command.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        Self {
            organization: Origins::of(instance)
                .owners
                .into_iter()
                .next()
                .unwrap_or_default(),
            repository_list: words(&[
                "gh",
                "repo",
                "list",
                "{organization}",
                "--limit",
                "200",
                "--json",
                "name,isArchived",
            ]),
            pull_requests: words(&[
                "gh",
                "pr",
                "list",
                "-R",
                "{organization}/{repository}",
                "--state",
                "open",
                "--limit",
                "1000",
                "--json",
                "number,title,isDraft,mergeable,statusCheckRollup,createdAt,updatedAt",
            ]),
            workflow_runs: words(&[
                "gh",
                "run",
                "list",
                "-R",
                "{organization}/{repository}",
                "--branch",
                "main",
                "--limit",
                "20",
                "--json",
                "workflowName,conclusion,status,createdAt",
            ]),
            width: WIDTH,
            bound: collect::BOUND,
        }
    }
}

/// What the collector reads of what shipped, beside [`Sources`]: the seam a test replaces.
/// [`Shipments::live`] gives the real ones.
///
/// Each command line is a template, filled as [`Sources`]'s are, `{since}` with the window's first
/// day (`YYYY-MM-DD`, UTC), and `{limit}` with the number of entries asked for: [`Limits::first`],
/// then, when that list is cut, [`Limits::larger`].
#[derive(Debug, Clone)]
pub struct Shipments {
    /// Prints one repository's latest `{limit}` releases, drafts aside, as one JSON array of
    /// objects with `tagName`, `publishedAt` and `isPrerelease`.
    pub releases: Vec<String>,
    /// How many releases are asked for.
    pub release_limits: Limits,
    /// Prints up to `{limit}` of one repository's pull requests merged on or after `{since}` as one
    /// JSON array of objects with `number`, `title`, `mergedAt` and `url`.
    pub merged_pull_requests: Vec<String>,
    /// How many merged pull requests are asked for.
    pub merged_limits: Limits,
    /// The collector's clock: the window is the [`WINDOW`] before it.
    pub now: OffsetDateTime,
}

/// How many entries a shipped list asks for. A list is cut when it holds `{limit}` entries and
/// none of them is older than the window, since an older one may then be missing from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The limit the list is asked for with first.
    pub first: usize,
    /// The limit it is asked for with once more when the first answer is cut. An answer at this
    /// limit that is cut too fails the collector.
    pub larger: usize,
}

impl Shipments {
    /// The latest 20 releases (100 when 20 are cut) and up to 200 merged pull requests (1000 when
    /// 200 are cut) of a repository through `gh`, in the [`WINDOW`] before `now`.
    #[must_use]
    pub fn live(now: OffsetDateTime) -> Self {
        Self {
            releases: words(&[
                "gh",
                "release",
                "list",
                "-R",
                "{organization}/{repository}",
                "--exclude-drafts",
                "--limit",
                "{limit}",
                "--json",
                "tagName,publishedAt,isPrerelease",
            ]),
            merged_pull_requests: words(&[
                "gh",
                "pr",
                "list",
                "-R",
                "{organization}/{repository}",
                "--state",
                "merged",
                "--search",
                "merged:>={since}",
                "--limit",
                "{limit}",
                "--json",
                "number,title,mergedAt,url",
            ]),
            release_limits: Limits {
                first: 20,
                larger: 100,
            },
            merged_limits: Limits {
                first: 200,
                larger: 1000,
            },
            now,
        }
    }
}

/// Records, over every non-archived repository of each GitHub owner of the instance the process
/// runs, one `RecordPullRequest` per open pull request, one `RecordWorkflowRun` per workflow on
/// `main`, and the releases and merged pull requests of the [`WINDOW`] before the snapshot
/// started, into the recorder's snapshot, over that instance's live sources.
///
/// # Errors
///
/// The snapshot's start does not read, or what [`collect_over`] answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    let started = record.started_at()?;
    let now = OffsetDateTime::parse(&started.0, &Rfc3339)
        .with_context(|| format!("the snapshot's start {:?} is not RFC 3339", started.0))?;
    let instance = &config::active().instance;
    collect_over(
        &Sources::of(instance),
        &Shipments::live(now),
        &Origins::of(instance),
        record,
    )
}

/// Records the open pull requests and the latest `main` run of each workflow of every
/// non-archived repository [`Sources::organization`] lists.
///
/// # Errors
///
/// A source did not answer (named with its repository and command) or answered something this
/// collector cannot read, and nothing is recorded; or an observation was refused.
pub fn collect_from(sources: &Sources, record: &mut Recorder<'_>) -> Result<()> {
    collect_with(
        sources,
        None,
        std::slice::from_ref(&sources.organization),
        record,
    )
}

/// What [`collect_from`] records, and, for each repository, the releases and merged pull requests
/// `shipments` shows in the [`WINDOW`] before [`Shipments::now`].
///
/// # Errors
///
/// What [`collect_from`] answers, for the shipments' sources too.
pub fn collect_all_from(
    sources: &Sources,
    shipments: &Shipments,
    record: &mut Recorder<'_>,
) -> Result<()> {
    collect_with(
        sources,
        Some(shipments),
        std::slice::from_ref(&sources.organization),
        record,
    )
}

/// What [`collect_all_from`] records, over every non-archived repository each owner of `origins`
/// lists; its local sources have no GitHub, and are not read.
///
/// # Errors
///
/// What [`collect_all_from`] answers, or two owners list one name.
pub fn collect_over(
    sources: &Sources,
    shipments: &Shipments,
    origins: &Origins,
    record: &mut Recorder<'_>,
) -> Result<()> {
    collect_with(sources, Some(shipments), &origins.owners, record)
}

/// The repositories each of `owners` lists, each read whole (with `shipments` when given) and its
/// runs compared with the newest complete snapshot's before anything is recorded.
fn collect_with(
    sources: &Sources,
    shipments: Option<&Shipments>,
    owners: &[String],
    record: &mut Recorder<'_>,
) -> Result<()> {
    let shipments = shipments.map(|shipments| (shipments, Window::of(shipments)));
    // Each repository as `(name, owner)`, by name.
    let mut repositories: Vec<(String, String)> = Vec::new();
    for owner in owners {
        let what = format!("organization {owner}");
        let command = filled(&sources.repository_list, &[("organization", owner)]);
        let listed: Vec<Listed> = json(&what, &command, None, sources.bound)?;
        let mut names = listed
            .into_iter()
            .filter(|listed| !listed.is_archived)
            .map(|listed| checked(&what, listed.name))
            .collect::<Result<Vec<_>>>()?;
        names.sort();
        names.dedup();
        repositories.extend(names.into_iter().map(|name| (name, owner.clone())));
    }
    repositories.sort_by(|one, other| one.0.cmp(&other.0));
    one_of_each(
        &repositories,
        |(name, _)| name,
        |(_, owner)| format!("owner {owner}"),
    )?;
    let snapshot = record.snapshot().clone();
    let mut read = each(&repositories, sources.width, |(name, owner)| {
        let open = read(sources, &snapshot, owner, name)?;
        let shipped = match &shipments {
            Some((shipments, window)) => {
                shipped(sources, shipments, window, &snapshot, owner, name)?
            }
            None => Shipped::default(),
        };
        Ok((open, shipped))
    })?;
    let answered = read.iter().any(|((_, runs), _)| !runs.latest.is_empty());
    if answered && let Some(stored) = stored(record)? {
        for ((name, owner), ((_, runs), _)) in repositories.iter().zip(&mut read) {
            caught_up(sources, &stored, owner, name, runs)?;
        }
    }
    let mut recorded = Vec::with_capacity(read.len());
    for ((name, _), ((pulls, runs), shipped)) in repositories.iter().zip(read) {
        let observed = Observed {
            what: format!("repository {name}"),
            snapshot: &snapshot,
            name,
        };
        let runs = runs
            .latest
            .into_values()
            .map(|(_, run)| observed.workflow_run(run))
            .collect::<Result<Vec<_>>>()?;
        recorded.push((pulls, runs, shipped));
    }
    for (pulls, runs, shipped) in recorded {
        for pull in pulls {
            record.pull_request(pull)?;
        }
        for run in runs {
            record.workflow_run(run)?;
        }
        for release in shipped.releases {
            record.release(release)?;
        }
        for merged in shipped.merged {
            record.merged_pull_request(merged)?;
        }
    }
    Ok(())
}

/// Where the window opens: the instant, and its day as `{since}` names it.
struct Window {
    start: OffsetDateTime,
    day: String,
}

impl Window {
    /// The [`WINDOW`] before `shipments`' clock, in UTC.
    fn of(shipments: &Shipments) -> Self {
        let start = (shipments.now - WINDOW).to_offset(UtcOffset::UTC);
        let date = start.date();
        Self {
            start,
            day: format!(
                "{:04}-{:02}-{:02}",
                date.year(),
                u8::from(date.month()),
                date.day()
            ),
        }
    }
}

/// One repository's releases, by publication, and merged pull requests, by number, in the window.
#[derive(Default)]
struct Shipped {
    releases: Vec<RecordRelease>,
    merged: Vec<RecordMergedPullRequest>,
}

/// One release, as `gh release list --json` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Release {
    tag_name: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    is_prerelease: bool,
}

/// One merged pull request, as `gh pr list --json` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Merged {
    number: i64,
    title: String,
    #[serde(default)]
    merged_at: Option<String>,
    url: String,
}

/// What `owner`'s `name` shipped in `window`, as observations of `snapshot`. A release without a
/// publication instant, or a merged pull request without a merge instant, is an answer this
/// collector cannot read.
fn shipped(
    sources: &Sources,
    shipments: &Shipments,
    window: &Window,
    snapshot: &SnapshotId,
    owner: &str,
    name: &str,
) -> Result<Shipped> {
    let what = format!("repository {name}");
    let asked = Asked {
        what: &what,
        fill: [
            ("organization", owner),
            ("repository", name),
            ("since", window.day.as_str()),
        ],
        bound: sources.bound,
        window,
    };
    let listed: Vec<Release> = asked.list(
        "releases",
        &shipments.releases,
        shipments.release_limits,
        |release: &Release| release.published_at.as_deref(),
    )?;
    let mut releases = Vec::new();
    for release in listed {
        if release.is_prerelease {
            continue;
        }
        let published = release.published_at.as_deref().unwrap_or_default();
        let at = OffsetDateTime::parse(published.trim(), &Rfc3339).map_err(|_| {
            anyhow!(
                "{what}: release {:?} has no RFC 3339 publication instant ({published:?})",
                release.tag_name
            )
        })?;
        if at < window.start {
            continue;
        }
        releases.push((
            at,
            RecordRelease {
                snapshot_id: snapshot.clone(),
                repository: RepositoryName(name.to_owned()),
                url: format!(
                    "https://github.com/{owner}/{name}/releases/tag/{}",
                    release.tag_name
                ),
                tag: release.tag_name,
                published_at: Timestamp(utc(&what, published)?),
            },
        ));
    }
    releases.sort_by(|(a, one), (b, other)| a.cmp(b).then_with(|| one.tag.cmp(&other.tag)));

    let listed: Vec<Merged> = asked.list(
        "merged pull requests",
        &shipments.merged_pull_requests,
        shipments.merged_limits,
        |pull: &Merged| pull.merged_at.as_deref(),
    )?;
    let mut merged = Vec::new();
    for pull in listed {
        let merged_at = pull.merged_at.as_deref().unwrap_or_default();
        let at = OffsetDateTime::parse(merged_at.trim(), &Rfc3339).map_err(|_| {
            anyhow!(
                "{what}: pull request {} has no RFC 3339 merge instant ({merged_at:?})",
                pull.number
            )
        })?;
        if at < window.start {
            continue;
        }
        merged.push(RecordMergedPullRequest {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(name.to_owned()),
            number: pull.number,
            title: pull.title,
            merged_at: Timestamp(utc(&what, merged_at)?),
            url: pull.url,
        });
    }
    merged.sort_by_key(|pull| pull.number);

    Ok(Shipped {
        releases: releases.into_iter().map(|(_, release)| release).collect(),
        merged,
    })
}

/// How one repository's shipped lists are asked for: what an error names, the template's fill
/// besides `{limit}`, the bound on each run, and the window an answer must reach past.
struct Asked<'a> {
    what: &'a str,
    fill: [(&'static str, &'a str); 3],
    bound: Duration,
    window: &'a Window,
}

impl Asked<'_> {
    /// The list `command` prints, `list` naming it, asked for at [`Limits::first`] and, when that
    /// answer is cut, once more at [`Limits::larger`]. Each run goes through [`collect::run`].
    ///
    /// An answer is cut when it holds its limit of entries and none of them, by the instant `at`
    /// reads, is older than the window: an entry inside the window may then be missing. An entry
    /// whose instant does not read is not taken as older.
    ///
    /// # Errors
    ///
    /// A run did not answer or printed what this collector cannot read, or the larger answer is
    /// cut too, which names the repository, the list and the limit.
    fn list<T: DeserializeOwned>(
        &self,
        list: &str,
        command: &[String],
        limits: Limits,
        at: fn(&T) -> Option<&str>,
    ) -> Result<Vec<T>> {
        let cut = |entries: &[T], limit: usize| {
            entries.len() >= limit
                && !entries.iter().any(|entry| {
                    at(entry)
                        .and_then(|text| OffsetDateTime::parse(text.trim(), &Rfc3339).ok())
                        .is_some_and(|instant| instant < self.window.start)
                })
        };
        let entries = self.ask(command, limits.first)?;
        if !cut(&entries, limits.first) {
            return Ok(entries);
        }
        let entries = self.ask(command, limits.larger)?;
        if cut(&entries, limits.larger) {
            bail!(
                "{}: the {list} fill `--limit {}` and none of them is older than the {}-day \
                 window, so some may be missing",
                self.what,
                limits.larger,
                WINDOW.whole_days()
            );
        }
        Ok(entries)
    }

    /// What `command`, filled with `limit`, prints.
    fn ask<T: DeserializeOwned>(&self, command: &[String], limit: usize) -> Result<Vec<T>> {
        let limit = limit.to_string();
        let mut fill = self.fill.to_vec();
        fill.push(("limit", &limit));
        json(self.what, &filled(command, &fill), None, self.bound)
    }
}

/// One entry of an owner's list.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    name: String,
    is_archived: bool,
}

/// One open pull request, as `gh pr list --json` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    number: i64,
    title: String,
    is_draft: bool,
    mergeable: String,
    #[serde(default)]
    status_check_rollup: Option<Vec<Check>>,
    created_at: String,
    updated_at: String,
}

/// One entry of a pull request's check rollup: a check run's conclusion, or none for a status
/// context.
#[derive(Deserialize)]
struct Check {
    #[serde(default)]
    conclusion: Option<String>,
}

/// One run, as `gh run list --json` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Run {
    workflow_name: String,
    #[serde(default)]
    conclusion: Option<String>,
    created_at: String,
}

/// `owner`'s repository `name`: its open pull requests, by number, as observations of
/// `snapshot`, and its `main` runs over [`ANSWERS`] answers of the run list.
fn read(
    sources: &Sources,
    snapshot: &SnapshotId,
    owner: &str,
    name: &str,
) -> Result<(Vec<RecordPullRequest>, Runs)> {
    let observed = Observed {
        what: format!("repository {name}"),
        snapshot,
        name,
    };
    let what = observed.what.as_str();
    let fill = [("organization", owner), ("repository", name)];
    let mut pulls: Vec<Pull> = json(
        what,
        &filled(&sources.pull_requests, &fill),
        None,
        sources.bound,
    )?;
    pulls.sort_by_key(|pull| pull.number);
    let pulls = pulls
        .into_iter()
        .map(|pull| observed.pull_request(pull))
        .collect::<Result<Vec<_>>>()?;

    let command = filled(&sources.workflow_runs, &fill);
    let mut runs = Runs::default();
    for _ in 0..ANSWERS {
        runs.ask(what, &command, sources.bound)?;
    }
    Ok((pulls, runs))
}

/// One repository's `main` runs over every answer of its run list so far: the latest run of each
/// workflow by `createdAt`, by workflow name, and how many answers were read.
#[derive(Default)]
struct Runs {
    latest: BTreeMap<String, (OffsetDateTime, Run)>,
    answers: usize,
}

impl Runs {
    /// Reads one more answer of `command`, through [`collect::run`], keeping each workflow's run
    /// when it is later than the one kept.
    fn ask(&mut self, what: &str, command: &[String], bound: Duration) -> Result<()> {
        let runs: Vec<Run> = json(what, command, None, bound)?;
        for run in runs {
            let at = OffsetDateTime::parse(run.created_at.trim(), &Rfc3339)
                .map_err(|_| anyhow!("{what}: {:?} is not an RFC 3339 instant", run.created_at))?;
            if self
                .latest
                .get(&run.workflow_name)
                .is_none_or(|(kept, _)| at > *kept)
            {
                self.latest.insert(run.workflow_name.clone(), (at, run));
            }
        }
        self.answers += 1;
        Ok(())
    }

    /// Each workflow, by name, whose latest run here started before the one `stored` holds for
    /// it: its name, the instant here and the instant stored. A workflow `stored` does not hold,
    /// or no answer holds, is not behind.
    fn behind(
        &self,
        stored: &BTreeMap<String, OffsetDateTime>,
    ) -> Vec<(&str, OffsetDateTime, OffsetDateTime)> {
        self.latest
            .iter()
            .filter_map(|(workflow, (at, _))| {
                stored
                    .get(workflow)
                    .filter(|held| **held > *at)
                    .map(|held| (workflow.as_str(), *at, *held))
            })
            .collect()
    }
}

/// The `main` runs of the newest complete snapshot: the latest instant of each workflow, by
/// repository and workflow name.
struct Stored {
    snapshot: SnapshotId,
    runs: BTreeMap<String, BTreeMap<String, OffsetDateTime>>,
}

/// What the newest complete snapshot holds of `main`'s runs, or `None` when the store holds no
/// complete snapshot. Read through the store's ports over a handle of its own on the store under
/// the recorder's state directory, as [`crate::repository::activity`] reads its snapshot; the
/// snapshot being collected is not complete, so it is never the one read. A record that does not
/// read is left out, as every port leaves it out; a run whose instant does not read is too.
///
/// # Errors
///
/// The snapshot is taken over a store opened without its state directory, the store does not
/// open, or a read of it failed for a reason other than a record this build cannot read.
fn stored(record: &Recorder<'_>) -> Result<Option<Stored>> {
    const WHAT: &str = "read the newest complete snapshot's runs";
    let store = crate::store::open_existing(record.state().context(WHAT)?).context(WHAT)?;
    let newest = newest_complete(SnapshotStorage::list(&store));
    store.unreadable().context(WHAT)?;
    let Some(snapshot) = newest else {
        return Ok(None);
    };
    let mut runs: BTreeMap<String, BTreeMap<String, OffsetDateTime>> = BTreeMap::new();
    for run in WorkflowRunObservationStorage::list(&store) {
        let data = run.data;
        if data.snapshot_id != snapshot {
            continue;
        }
        let Ok(at) = OffsetDateTime::parse(&data.run_at.0, &Rfc3339) else {
            continue;
        };
        let held = runs
            .entry(data.repository.0)
            .or_default()
            .entry(data.workflow)
            .or_insert(at);
        *held = (*held).max(at);
    }
    store.unreadable().context(WHAT)?;
    Ok(Some(Stored { snapshot, runs }))
}

/// The `Complete` snapshot of `snapshots` that started latest; of equal starts, the later one in
/// the store's order. A start that does not read as RFC 3339 counts as earlier than any that does.
/// The rule [`crate::repository::activity`] finds its snapshot by.
fn newest_complete(snapshots: Vec<SnapshotSnapshot>) -> Option<SnapshotId> {
    snapshots
        .into_iter()
        .enumerate()
        .filter(|(_, snapshot)| snapshot.state == SnapshotState::Complete)
        .max_by_key(|(stored, snapshot)| {
            (
                OffsetDateTime::parse(&snapshot.data.started_at.0, &Rfc3339).ok(),
                *stored,
            )
        })
        .map(|(_, snapshot)| snapshot.data.snapshot_id)
}

/// Asks the run list of `owner`'s `name` up to [`MORE_ANSWERS`] more times while a workflow's
/// latest run in `runs` started before the one `stored` holds for it, each time through
/// [`collect::run`]. A workflow still behind after the last answer keeps what the answers give,
/// and one line on standard error names the repository, the workflow, the latest instant
/// answered, how many answers, the instant stored and the snapshot holding it: a run GitHub no
/// longer answers (a deleted run, a recreated workflow) never fails a snapshot.
///
/// # Errors
///
/// A run did not answer or printed what this collector cannot read.
fn caught_up(
    sources: &Sources,
    stored: &Stored,
    owner: &str,
    name: &str,
    runs: &mut Runs,
) -> Result<()> {
    let Some(held) = stored.runs.get(name) else {
        return Ok(());
    };
    let what = format!("repository {name}");
    let command = filled(
        &sources.workflow_runs,
        &[("organization", owner), ("repository", name)],
    );
    for _ in 0..MORE_ANSWERS {
        if runs.behind(held).is_empty() {
            return Ok(());
        }
        runs.ask(&what, &command, sources.bound)?;
    }
    for (workflow, answered, kept) in runs.behind(held) {
        eprintln!(
            "{what}: workflow {workflow:?}: GitHub's run list answered {} in {} answers, older \
             than {} in snapshot {}; recorded what it answered",
            shown(answered),
            runs.answers,
            shown(kept),
            stored.snapshot.0.0
        );
    }
    Ok(())
}

/// `at` in UTC, RFC 3339, as a line shows it.
fn shown(at: OffsetDateTime) -> String {
    let at = at.to_offset(UtcOffset::UTC);
    at.format(&Rfc3339).unwrap_or_else(|_| at.to_string())
}

/// What one repository's observations are filed under: the snapshot and the repository, and what
/// an error names.
struct Observed<'a> {
    what: String,
    snapshot: &'a SnapshotId,
    name: &'a str,
}

impl Observed<'_> {
    fn pull_request(&self, pull: Pull) -> Result<RecordPullRequest> {
        let what = &self.what;
        let mergeability = match pull.mergeable.as_str() {
            "MERGEABLE" => Mergeability::Mergeable,
            "CONFLICTING" => Mergeability::Conflicting,
            "UNKNOWN" | "" => Mergeability::Unknown,
            other => bail!(
                "{what}: pull request {} is {other:?}, a mergeability the specification does not \
                 name",
                pull.number
            ),
        };
        let failing_checks = pull
            .status_check_rollup
            .unwrap_or_default()
            .iter()
            .filter(|check| {
                check
                    .conclusion
                    .as_deref()
                    .is_some_and(|conclusion| FAILING.contains(&conclusion))
            })
            .count();
        Ok(RecordPullRequest {
            snapshot_id: self.snapshot.clone(),
            repository: RepositoryName(self.name.to_owned()),
            number: pull.number,
            title: pull.title,
            draft: pull.is_draft,
            mergeability,
            failing_checks: i64::try_from(failing_checks).unwrap_or(i64::MAX),
            opened_at: Timestamp(utc(what, &pull.created_at)?),
            updated_at: Timestamp(utc(what, &pull.updated_at)?),
        })
    }

    fn workflow_run(&self, run: Run) -> Result<RecordWorkflowRun> {
        let what = &self.what;
        let conclusion = match run.conclusion.as_deref().unwrap_or_default() {
            "success" => RunConclusion::Success,
            "failure" | "timed_out" | "startup_failure" => RunConclusion::Failure,
            "cancelled" | "stale" => RunConclusion::Cancelled,
            "skipped" => RunConclusion::Skipped,
            "neutral" => RunConclusion::Neutral,
            "" | "action_required" => RunConclusion::Pending,
            other => bail!(
                "{what}: workflow {:?} concluded {other:?}, a conclusion this collector does not \
                 know",
                run.workflow_name
            ),
        };
        Ok(RecordWorkflowRun {
            snapshot_id: self.snapshot.clone(),
            repository: RepositoryName(self.name.to_owned()),
            workflow: run.workflow_name,
            conclusion,
            run_at: Timestamp(utc(what, &run.created_at)?),
        })
    }
}
