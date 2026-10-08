//! `conductor repository shipped`: what shipped since an instant (`story:release-digest`, wave 07, U2).
//!
//! Read from the newest complete snapshot's GitHub observations, and from nothing else: a claim
//! about what shipped states only what GitHub shows. The newest complete snapshot is the
//! `Complete` one that started last (`snapshot snapshots`); a collecting or failed one is never
//! read. Its releases (`snapshot releases`) and merged pull requests (`snapshot
//! merged-pull-requests`) are those of the 14 days before it started
//! ([`github::WINDOW`](crate::collect::github::WINDOW)).
//!
//! Per repository, in name order:
//! - each release published at or after `--since`, oldest first: a row with its tag, its
//!   publication instant and its link;
//! - beneath it, each pull request merged at or after `--since` and before it, and after the
//!   release before it, oldest first: a row with the release's tag and instant, and the pull
//!   request's number, title, merge instant and link;
//! - then each pull request merged at or after `--since` and after the newest release the
//!   snapshot holds, whether that release is listed or not: `merged, not released` in the release
//!   column.
//!
//! A repository with nothing since the instant is left out. The rows render as every view's do
//! (`--format text|json|jsonl|markdown`, text by default). An instant older than the window prints
//! one first line, `the window starts at <instant>`: on standard output in `text` and `markdown`,
//! on standard error in `json` and `jsonl`, which keeps standard output one JSON document.
//!
//! The association is by time: a pull request merged before a release is listed under it, as
//! GitHub's timestamps say, not as the release's commits do.

use std::collections::BTreeMap;
use std::io::{self, Write as _};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::Generated;
use conductor_model::observation::obligations::{
    MergedPullRequestsQuery, ReleasesQuery, SnapshotsQuery,
};
use conductor_model::observation::{MergedPullRequests, Releases, SnapshotState};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{Format, RepositoryShippedArgs, ViewArgs};
use crate::collect::github::WINDOW;
use crate::store::Store;

/// The command, as an error names it.
const COMMAND: &str = "repository shipped";

/// The digest's columns, in order.
const COLUMNS: [&str; 7] = [
    "repository",
    "release",
    "published_at",
    "number",
    "title",
    "merged_at",
    "url",
];

/// What the release column says of a pull request merged after the newest release.
const NOT_RELEASED: &str = "merged, not released";

/// `conductor repository shipped --since <instant> [--format <format>]`.
///
/// Exits 0 with the digest, and 1 when a record of the store cannot be read (each named on
/// standard error, after the rows that could).
///
/// # Errors
///
/// `--since` is missing or not an RFC 3339 instant, `--format` is not a view's format, there is no
/// store or no complete snapshot in it, or a stored instant does not read.
pub fn shipped(state: Option<&Path>, args: &RepositoryShippedArgs) -> Result<ExitCode> {
    answer(state, args).context(COMMAND)
}

fn answer(state: Option<&Path>, args: &RepositoryShippedArgs) -> Result<ExitCode> {
    let given = args
        .since
        .as_deref()
        .ok_or_else(|| anyhow!("--since is missing"))?;
    let since = OffsetDateTime::parse(given, &Rfc3339)
        .map_err(|_| anyhow!("--since {given:?} is not an RFC 3339 instant"))?;
    let view = ViewArgs {
        format: format(args.format.as_deref())?,
    };

    let generated = Generated::new(crate::state::open_existing(state)?);
    let snapshots = generated.snapshots().map_err(|unmet| anyhow!("{unmet}"))?;
    let mut newest: Option<(OffsetDateTime, _)> = None;
    for row in snapshots {
        if row.state != SnapshotState::Complete {
            continue;
        }
        let started = instant("a snapshot's start", &row.started_at.0)?;
        if newest.as_ref().is_none_or(|(at, _)| started > *at) {
            newest = Some((started, row.snapshot_id));
        }
    }
    let Some((started, snapshot)) = newest else {
        bail!("no complete snapshot in the store, so nothing has been observed");
    };

    let mut repositories: BTreeMap<String, Repository> = BTreeMap::new();
    for row in generated.releases().map_err(|unmet| anyhow!("{unmet}"))? {
        if row.snapshot_id == snapshot {
            let at = instant("a release's publication", &row.published_at.0)?;
            repositories
                .entry(row.repository.0.clone())
                .or_default()
                .releases
                .push((at, row));
        }
    }
    for row in generated
        .merged_pull_requests()
        .map_err(|unmet| anyhow!("{unmet}"))?
    {
        if row.snapshot_id == snapshot {
            let at = instant("a pull request's merge", &row.merged_at.0)?;
            repositories
                .entry(row.repository.0.clone())
                .or_default()
                .merged
                .push((at, row));
        }
    }
    let rows: Vec<Vec<Value>> = repositories
        .into_values()
        .flat_map(|repository| repository.rows(since))
        .collect();

    let window = started - WINDOW;
    let note = if since < window {
        Some(format!(
            "the window starts at {}",
            window
                .format(&Rfc3339)
                .context("format the window's start")?
        ))
    } else {
        None
    };
    let mut rendered = view.render(&COLUMNS, &rows);
    if let Some(note) = note {
        match view.format {
            Format::Text | Format::Markdown => rendered.insert_str(0, &format!("{note}\n")),
            Format::Json | Format::Jsonl => writeln!(io::stderr().lock(), "{note}")
                .context("write the window's start to standard error")?,
        }
    }
    show(&generated, &rendered)
}

/// The rows through [`crate::store::show`], which names each record it could not read.
fn show(generated: &Generated<Store>, rendered: &str) -> Result<ExitCode> {
    crate::store::show(
        &generated.ports,
        "shipped releases and pull requests",
        rendered,
    )
}

/// One repository's observations of the snapshot, each with its instant.
#[derive(Default)]
struct Repository {
    releases: Vec<(OffsetDateTime, Releases)>,
    merged: Vec<(OffsetDateTime, MergedPullRequests)>,
}

impl Repository {
    /// Its digest rows since `since`; none when nothing was released or merged since then.
    fn rows(mut self, since: OffsetDateTime) -> Vec<Vec<Value>> {
        self.releases
            .sort_by(|(a, one), (b, other)| a.cmp(b).then_with(|| one.tag.cmp(&other.tag)));
        self.merged
            .sort_by(|(a, one), (b, other)| a.cmp(b).then_with(|| one.number.cmp(&other.number)));

        // Each pull request merged since the instant goes under the first release published after
        // it, or, after the newest release, under none.
        let mut under: Vec<Vec<&MergedPullRequests>> = vec![Vec::new(); self.releases.len()];
        let mut unreleased = Vec::new();
        for (merged_at, pull) in &self.merged {
            if *merged_at < since {
                continue;
            }
            match self
                .releases
                .iter()
                .position(|(published_at, _)| published_at > merged_at)
            {
                Some(release) => under[release].push(pull),
                None => unreleased.push(pull),
            }
        }

        let mut rows = Vec::new();
        for ((published_at, release), pulls) in self.releases.iter().zip(&under) {
            if *published_at < since {
                continue;
            }
            rows.push(vec![
                Value::from(release.repository.0.as_str()),
                Value::from(release.tag.as_str()),
                Value::from(release.published_at.0.as_str()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::from(release.url.as_str()),
            ]);
            for pull in pulls {
                rows.push(pull_row(
                    pull,
                    Value::from(release.tag.as_str()),
                    Value::from(release.published_at.0.as_str()),
                ));
            }
        }
        for pull in unreleased {
            rows.push(pull_row(pull, Value::from(NOT_RELEASED), Value::Null));
        }
        rows
    }
}

/// One pull request's row, under `release` published at `published_at`.
fn pull_row(pull: &MergedPullRequests, release: Value, published_at: Value) -> Vec<Value> {
    vec![
        Value::from(pull.repository.0.as_str()),
        release,
        published_at,
        Value::from(pull.number),
        Value::from(pull.title.as_str()),
        Value::from(pull.merged_at.0.as_str()),
        Value::from(pull.url.as_str()),
    ]
}

/// `--format`, as every view spells its formats; text when it is not given.
fn format(name: Option<&str>) -> Result<Format> {
    match name {
        None | Some("text") => Ok(Format::Text),
        Some("json") => Ok(Format::Json),
        Some("jsonl") => Ok(Format::Jsonl),
        Some("markdown") => Ok(Format::Markdown),
        Some(other) => bail!("--format {other:?} is not one of text, json, jsonl, markdown"),
    }
}

/// A stored instant, `what` naming it when it does not read.
fn instant(what: &str, text: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339)
        .with_context(|| format!("{what} {text:?} is not an RFC 3339 instant"))
}
