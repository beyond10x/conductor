//! The `secrets` collector (`story:policy-secret-drift`): for each secret sync of the instance
//! (`secret_syncs`), when its source was last changed and when the secrets it feeds were last
//! written, so the board can name a secret its source has moved past.
//!
//! One `RecordSecretSync` per sync, in the config's order:
//!
//! | field | read from |
//! |---|---|
//! | `source_committed_at` | `git log -1 --format=%cI origin/<branch> -- <path>` in `<checkouts.root>/<repository>`, the committer time of the last commit of the branch that touched the file, as the `repositories` collector's fetch left `origin/<branch>` |
//! | `secrets_updated_at` | the oldest `updated_at` of the sync's secrets, each read with `gh api <endpoint>` ([`endpoint`]): `orgs/<owner>/actions/secrets/<name>` for an organization's secret, `repos/<owner>/<repository>/actions/secrets/<name>` for a repository's |
//! | `error_line` | the first line of the first read that failed |
//!
//! Every read is a GET: GitHub answers a secret's name and times, never its value, and the
//! collector writes nothing anywhere but its observations. Conductor never runs a sync's
//! `command`. A read that fails is asked once more ([`answer`]); failing again, the time it would
//! have read is left absent and the failure is the row's `error_line`, and the collector goes on:
//! a sync it cannot read never fails a snapshot. A collector stopped by its budget stops there.
//! Every instant is recorded in UTC.
//!
//! [`collect`] reads the instance the process runs ([`Sources::of`] over [`config::active`]);
//! [`collect_from`] reads any [`Sources`], the seam a test points at a fixture checkout and a
//! fake `gh`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, anyhow};
use conductor_model::config::{ForgeSecret, Instance, SecretSync};
use conductor_model::observation::{RecordSecretSync, RepositoryName};
use conductor_model::primitives::Timestamp;
use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::repositories::{answer, filled, json, utc, words};
use crate::collect;
use crate::config;
use crate::snapshot::Recorder;

/// What the collector reads: the one seam a test replaces. [`Sources::of`] gives an instance's.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The syncs whose drift is read, in the config's order.
    pub syncs: Vec<SecretSync>,
    /// The checkouts root a sync's `source.repository` lies under.
    pub root: PathBuf,
    /// Prints one secret as a JSON object with `updated_at`; `{endpoint}` is filled with the
    /// secret's [`endpoint`].
    pub secret: Vec<String>,
    /// How long one command may run before it is stopped.
    pub bound: Duration,
}

impl Sources {
    /// `instance`'s: its `secret_syncs`, its checkouts root, each secret through `gh api`, and
    /// [`collect::BOUND`] on each command.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        Self {
            syncs: instance.secret_syncs.clone(),
            root: PathBuf::from(&instance.checkouts.root),
            secret: words(&["gh", "api", "{endpoint}"]),
            bound: collect::BOUND,
        }
    }
}

/// The GitHub REST path of `secret`: its organization's, or its repository's when it names one.
#[must_use]
pub fn endpoint(secret: &ForgeSecret) -> String {
    match &secret.repository {
        Some(repository) => format!(
            "repos/{}/{repository}/actions/secrets/{}",
            secret.owner, secret.name
        ),
        None => format!("orgs/{}/actions/secrets/{}", secret.owner, secret.name),
    }
}

/// Records one `RecordSecretSync` per secret sync of the instance the process runs.
///
/// # Errors
///
/// What [`collect_from`] answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    collect_from(&Sources::of(&config::active().instance), record)
}

/// Records one `RecordSecretSync` per sync of `sources`.
///
/// # Errors
///
/// The collector's budget is spent, or an observation was refused. A read that fails is
/// recorded, not answered.
pub fn collect_from(sources: &Sources, record: &mut Recorder<'_>) -> Result<()> {
    for sync in &sources.syncs {
        let what = format!("sync {}", sync.name);
        let mut errors = Vec::new();
        let source_committed_at = kept(committed_at(sources, sync, &what), &mut errors)?;
        let secrets_updated_at = kept(oldest_write(sources, sync, &what), &mut errors)?;
        let snapshot = record.snapshot().clone();
        record.secret_sync(RecordSecretSync {
            snapshot_id: snapshot,
            sync: sync.name.clone(),
            repository: RepositoryName(sync.source.repository.clone()),
            branch: sync.source.branch.clone(),
            path: sync.source.path.clone(),
            source_committed_at: source_committed_at.map(Timestamp),
            secrets_updated_at: secrets_updated_at.map(Timestamp),
            error_line: errors.into_iter().next(),
        })?;
    }
    Ok(())
}

/// The value `read` answered, or `None` with its error's first line kept in `errors`; an
/// over-budget error is answered.
fn kept<T>(read: Result<T>, errors: &mut Vec<String>) -> Result<Option<T>> {
    match read {
        Ok(value) => Ok(Some(value)),
        Err(error) if collect::is_over_budget(&error) => Err(error),
        Err(error) => {
            let text = format!("{error:#}");
            errors.push(text.lines().next().unwrap_or_default().to_owned());
            Ok(None)
        }
    }
}

/// The committer time of the last commit on `origin/<branch>` that touched the sync's file, in
/// UTC.
fn committed_at(sources: &Sources, sync: &SecretSync, what: &str) -> Result<String> {
    let checkout: PathBuf = sources.root.join(&sync.source.repository);
    if !checkout.join(".git").exists() {
        return Err(anyhow!(
            "{what}: no checkout of {} under {}",
            sync.source.repository,
            sources.root.display()
        ));
    }
    let branch = format!("origin/{}", sync.source.branch);
    let command = words(&[
        "git",
        "log",
        "-1",
        "--format=%cI",
        &branch,
        "--",
        &sync.source.path,
    ]);
    let printed = answer(what, &command, Some(Path::new(&checkout)), sources.bound)?;
    let printed = printed.trim();
    if printed.is_empty() {
        return Err(anyhow!(
            "{what}: no commit on {branch} touches {}",
            sync.source.path
        ));
    }
    utc(what, printed)
}

/// One secret, as `gh api` prints it.
#[derive(Deserialize)]
struct Secret {
    updated_at: String,
}

/// The oldest `updated_at` of the sync's secrets, in UTC.
fn oldest_write(sources: &Sources, sync: &SecretSync, what: &str) -> Result<String> {
    let mut oldest: Option<(OffsetDateTime, String)> = None;
    for secret in &sync.secrets {
        let command = filled(&sources.secret, &[("endpoint", &endpoint(secret))]);
        let read: Secret = json(what, &command, None, sources.bound)?;
        let at = utc(what, &read.updated_at)?;
        let instant = OffsetDateTime::parse(&at, &Rfc3339)
            .map_err(|_| anyhow!("{what}: {at:?} is not an RFC 3339 instant"))?;
        if oldest.as_ref().is_none_or(|(held, _)| instant < *held) {
            oldest = Some((instant, at));
        }
    }
    oldest
        .map(|(_, at)| at)
        .ok_or_else(|| anyhow!("{what}: names no secret"))
}
