//! The requests Blocked dispatches wait on (`story:watch-unblocks-merged-requests`).
//!
//! At each pass the watch reads the dispatch log (the month before the clock's through the
//! clock's, [`dispatch_states`]) for the dispatches whose newest line of their own is `blocked`,
//! and of those whose reason's first word is `ci`, `review` or `mr-route` ([`REASONS`]) and that
//! name a pull or merge request ([`Request::of`]) it reads that request on the forge:
//!
//! - a GitHub pull request, named by its URL (`https://<host>/<owner>/<repo>/pull/<n>`), or by
//!   `#<n>` or `<repo>#<n>` on an instance with a GitHub owner, through the reader
//!   [`Watch::with_pull`] gives (`gh pr view <n> -R <owner>/<repo> --json …`; with no owner in
//!   the URL, under each of the instance's owners until one answers);
//! - a GitLab merge request, named by its URL (`https://<host>/<path>/-/merge_requests/<n>`),
//!   read through the forge of that host, or by `!<n>`, or `#<n>` on an instance with no GitHub
//!   owner, read through the first forge under `<group>/<repo>`; its failed jobs, when its head
//!   pipeline failed, through `pipelines/<id>/jobs?scope=failed`.
//!
//! Merged: `request merged: <repo> <dispatch> <url> at <t>`; closed unmerged: `request closed:
//! <repo> <dispatch> <url> at <t>`; open with a failed check (a GitLab head pipeline that
//! failed): `request red: <repo> <dispatch> <url> head <sha> job <job url> <job name>`, the
//! first failed job. The dispatches are read in the order of their ids. Each once per block (`<dispatch> <blocked at> merged|closed`) and a red one
//! once per head commit (`<dispatch> <blocked at> red <sha>`), kept in `requests.printed` while
//! the dispatch stays blocked. A request that does not answer is named on standard error as the
//! source `request <repo> <dispatch>`, once until it answers, and keeps what was printed of it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::process::Command;

use anyhow::{Context as _, Result, anyhow, bail};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use time::{OffsetDateTime, UtcOffset};

use conductor_model::observation::RunConclusion;

use super::{Check, EXCERPT, Watch, months, one_line, read_optional, replace};
use crate::collect;
use crate::collect::gitlab::{self, Forge};
use crate::collect::repositories::filled;

/// The first words of a block reason whose request the watch reads.
pub const REASONS: [&str; 3] = ["ci", "review", "mr-route"];

/// The blocks printed, one per line: `<dispatch> <blocked at> merged|closed`, or `<dispatch>
/// <blocked at> red <head sha>`.
const REQUESTS_PRINTED: &str = "requests.printed";

/// The events of the dispatch log that move a dispatch, and so say its state.
const MOVES: [&str; 7] = [
    "sent",
    "started",
    "blocked",
    "unblocked",
    "done",
    "failed",
    "cancelled",
];

/// The events after which a dispatch is open.
const OPEN: [&str; 4] = ["sent", "started", "blocked", "unblocked"];

/// One dispatch as the newest line of the dispatch log that moved it leaves it.
#[derive(Debug, Clone)]
pub struct DispatchState {
    pub id: String,
    pub repo: String,
    /// That line's event.
    pub event: String,
    /// That line's `reason`, for a `blocked` one.
    pub reason: String,
    /// That line's `at`.
    pub at: String,
}

impl DispatchState {
    /// Whether the dispatch is open: sent, started or blocked.
    #[must_use]
    pub fn open(&self) -> bool {
        OPEN.contains(&self.event.as_str())
    }
}

/// Each dispatch of the dispatch log under `dispatches`, read from the month before `now`'s
/// through `now`'s, by its id, as the newest line that moved it ([`MOVES`]) leaves it.
pub fn dispatch_states(
    dispatches: &std::path::Path,
    now: OffsetDateTime,
) -> BTreeMap<String, DispatchState> {
    let now_utc = now.to_offset(UtcOffset::UTC);
    let earlier = now_utc
        .date()
        .replace_day(1)
        .ok()
        .and_then(time::Date::previous_day)
        .map_or(now_utc, |day| day.midnight().assume_utc());
    let mut states = BTreeMap::new();
    for month in months(earlier, now) {
        let Ok(text) = fs::read_to_string(dispatches.join(format!("{month}.jsonl"))) else {
            continue;
        };
        for line in text.lines() {
            let Ok(record) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let text = |key: &str| record[key].as_str().unwrap_or_default().to_owned();
            let event = text("event");
            let id = text("id");
            if id.is_empty() || !MOVES.contains(&event.as_str()) {
                continue;
            }
            states.insert(
                id.clone(),
                DispatchState {
                    id,
                    repo: text("repo"),
                    event,
                    reason: text("reason"),
                    at: text("at"),
                },
            );
        }
    }
    states
}

/// The request a block reason names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// A GitHub pull request: its owner when the reason names one, its repository and number.
    GitHub {
        owner: Option<String>,
        repository: String,
        number: u64,
    },
    /// A GitLab merge request: the host and project path when the reason names them, else the
    /// first forge under `<group>/<repository>`.
    GitLab {
        host: Option<String>,
        path: Option<String>,
        repository: String,
        iid: u64,
    },
    /// `#<n>` or `<repo>#<n>`: GitHub on an instance with an owner, else GitLab.
    Number { repository: String, number: u64 },
}

impl Request {
    /// The request the block reason `reason` of a dispatch of `repository` names: none unless its
    /// first word is one of [`REASONS`] and a later word is a pull or merge request's URL,
    /// `#<n>`, `<repo>#<n>` or `!<n>`.
    #[must_use]
    pub fn of(reason: &str, repository: &str) -> Option<Self> {
        let mut words = reason.split_whitespace();
        let first = words.next()?.trim_end_matches([':', ',', ';']);
        if !REASONS.contains(&first) {
            return None;
        }
        words.find_map(|word| {
            let word = word
                .trim_start_matches(['(', '<'])
                .trim_end_matches([',', '.', ';', ':', ')', '>']);
            Self::named(word, repository)
        })
    }

    fn named(word: &str, repository: &str) -> Option<Self> {
        let number = |text: &str| text.parse::<u64>().ok().filter(|number| *number > 0);
        if let Some(rest) = word
            .strip_prefix("https://")
            .or_else(|| word.strip_prefix("http://"))
        {
            let (host, path) = rest.split_once('/')?;
            let path = path.trim_end_matches('/');
            if let Some((project, tail)) = path.split_once("/-/merge_requests/") {
                return Some(Self::GitLab {
                    host: Some(host.to_owned()),
                    path: Some(project.to_owned()),
                    repository: project.rsplit('/').next()?.to_owned(),
                    iid: number(tail.split('/').next()?)?,
                });
            }
            let parts: Vec<&str> = path.split('/').collect();
            if let [owner, name, "pull", n, ..] = parts.as_slice() {
                return Some(Self::GitHub {
                    owner: Some((*owner).to_owned()),
                    repository: (*name).to_owned(),
                    number: number(n)?,
                });
            }
            return None;
        }
        if let Some(iid) = word.strip_prefix('!') {
            return Some(Self::GitLab {
                host: None,
                path: None,
                repository: repository.to_owned(),
                iid: number(iid)?,
            });
        }
        let (name, n) = word.split_once('#')?;
        Some(Self::Number {
            repository: if name.is_empty() { repository } else { name }.to_owned(),
            number: number(n)?,
        })
    }
}

/// What the forge answered of a request.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Read {
    Merged {
        url: String,
        at: String,
    },
    Closed {
        url: String,
        at: String,
    },
    /// Open: its head, and its first failed job (name, URL) when one failed.
    Open {
        url: String,
        head: String,
        failed: Option<(String, String)>,
    },
}

/// A pull request as `gh pr view --json state,url,mergedAt,closedAt,headRefOid,statusCheckRollup`
/// prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullView {
    state: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    merged_at: Option<String>,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    head_ref_oid: Option<String>,
    #[serde(default)]
    status_check_rollup: Option<Vec<Check>>,
}

/// A merge request as GitLab's `merge_requests/<iid>` answers it.
#[derive(Deserialize)]
struct MergeRequestView {
    state: String,
    #[serde(default)]
    web_url: Option<String>,
    #[serde(default)]
    merged_at: Option<String>,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    head_pipeline: Option<PipelineView>,
}

/// A merge request's head pipeline.
#[derive(Deserialize)]
struct PipelineView {
    id: i64,
    status: String,
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    web_url: Option<String>,
}

/// A job of a pipeline.
#[derive(Deserialize)]
struct Job {
    name: String,
    #[serde(default)]
    web_url: Option<String>,
}

impl Watch {
    /// The request lines of one pass at `now` (the module documentation).
    pub(super) fn requests(
        &mut self,
        now: OffsetDateTime,
        lines: &mut Vec<String>,
        notes: &mut dyn Write,
    ) -> Result<()> {
        let blocked: Vec<(DispatchState, Request)> = dispatch_states(&self.sources.dispatches, now)
            .into_values()
            .filter(|state| state.event == "blocked")
            .filter_map(|state| {
                let request = Request::of(&state.reason, &state.repo)?;
                Some((state, request))
            })
            .collect();
        let file = self.state(REQUESTS_PRINTED);
        let before = read_optional(&file)?.unwrap_or_default();
        let printed: BTreeSet<&str> = before.lines().collect();
        let mut kept: BTreeSet<String> = BTreeSet::new();
        for (state, request) in blocked {
            let block = format!("{} {}", state.id, state.at);
            let source = format!("request {} {}", state.repo, state.id);
            let read = match self.read_request(&request) {
                Ok(Some(read)) => {
                    self.answered(&source);
                    read
                }
                Ok(None) => continue,
                Err(error) => {
                    self.failed(&source, &error, notes);
                    let mine = format!("{block} ");
                    kept.extend(
                        printed
                            .iter()
                            .filter(|line| line.starts_with(&mine))
                            .map(|line| (*line).to_owned()),
                    );
                    continue;
                }
            };
            let (key, line) = match read {
                Read::Merged { url, at } => (
                    format!("{block} merged"),
                    format!("request merged: {} {} {url} at {at}", state.repo, state.id),
                ),
                Read::Closed { url, at } => (
                    format!("{block} closed"),
                    format!("request closed: {} {} {url} at {at}", state.repo, state.id),
                ),
                Read::Open {
                    url,
                    head,
                    failed: Some((name, job)),
                } => (
                    format!("{block} red {head}"),
                    format!(
                        "request red: {} {} {url} head {head} job {job} {name}",
                        state.repo, state.id
                    ),
                ),
                Read::Open { failed: None, .. } => continue,
            };
            if !printed.contains(key.as_str()) {
                lines.push(line);
            }
            kept.insert(key);
        }
        let text: String = kept.iter().map(|line| format!("{line}\n")).collect();
        if text != before {
            replace(&file, &text)?;
        }
        Ok(())
    }

    /// What the forge answers of `request`; none when the instance has no reader for it.
    fn read_request(&self, request: &Request) -> Result<Option<Read>> {
        let github = !self.sources.organization.is_empty() || !self.sources.owners.is_empty();
        match request {
            Request::GitHub {
                owner,
                repository,
                number,
            } => self.read_pull(owner.as_deref(), repository, *number),
            Request::GitLab {
                host,
                path,
                repository,
                iid,
            } => self.read_merge_request(host.as_deref(), path.as_deref(), repository, *iid),
            Request::Number { repository, number } if github => {
                self.read_pull(None, repository, *number)
            }
            Request::Number { repository, number } => {
                self.read_merge_request(None, None, repository, *number)
            }
        }
    }

    /// The pull request `<owner>/<repository>#<number>` through [`Watch::with_pull`]'s reader;
    /// without `owner`, under each of the instance's owners until one answers.
    fn read_pull(
        &self,
        owner: Option<&str>,
        repository: &str,
        number: u64,
    ) -> Result<Option<Read>> {
        if self.pull.is_empty() {
            return Ok(None);
        }
        let owners: Vec<String> = match owner {
            Some(owner) => vec![owner.to_owned()],
            None if self.sources.owners.is_empty() => vec![self.sources.organization.clone()],
            None => self.sources.owners.clone(),
        };
        let mut first = None;
        for owner in &owners {
            let what = format!("pull request {owner}/{repository}#{number} not read");
            let number = number.to_string();
            let command = filled(
                &self.pull,
                &[
                    ("organization", owner.as_str()),
                    ("repository", repository),
                    ("number", number.as_str()),
                ],
            );
            match run_json::<PullView>(&command, &what, self.sources.bound) {
                Ok(view) => {
                    let url = view.url.map_or_else(
                        || format!("https://github.com/{owner}/{repository}/pull/{number}"),
                        |url| one_line(&url),
                    );
                    return Ok(Some(pull_read(
                        url,
                        &view.state,
                        view.merged_at,
                        view.closed_at,
                        view.head_ref_oid,
                        &view.status_check_rollup.unwrap_or_default(),
                    )));
                }
                Err(error) => {
                    first.get_or_insert(error);
                }
            }
        }
        Err(first.unwrap_or_else(|| anyhow!("pull request {repository}#{number}: no GitHub owner")))
    }

    /// The merge request `iid` of the project `path` on the forge of `host`, else of
    /// `<group>/<repository>` on the first forge; its first failed job when its head pipeline
    /// failed.
    fn read_merge_request(
        &self,
        host: Option<&str>,
        path: Option<&str>,
        repository: &str,
        iid: u64,
    ) -> Result<Option<Read>> {
        let forge: Option<&Forge> = match host {
            Some(host) => self.gitlab.iter().find(|forge| forge.host == host),
            None => self.gitlab.first(),
        };
        let Some(forge) = forge else {
            return Ok(None);
        };
        let project = path.map_or_else(|| format!("{}/{repository}", forge.group), str::to_owned);
        let encoded = gitlab::encoded(&project);
        let what = format!("merge request {project}!{iid} not read");
        let view: MergeRequestView =
            forge.get(&what, &format!("projects/{encoded}/merge_requests/{iid}"))?;
        let url = view.web_url.map_or_else(
            || format!("https://{}/{project}/-/merge_requests/{iid}", forge.host),
            |url| one_line(&url),
        );
        let at = |at: Option<String>| one_line(&at.unwrap_or_else(|| "-".to_owned()));
        match view.state.as_str() {
            "merged" => {
                return Ok(Some(Read::Merged {
                    url,
                    at: at(view.merged_at),
                }));
            }
            "closed" => {
                return Ok(Some(Read::Closed {
                    url,
                    at: at(view.closed_at),
                }));
            }
            _ => {}
        }
        let head = one_line(view.sha.as_deref().unwrap_or("-"));
        let failed = match view.head_pipeline {
            Some(pipeline)
                if pipeline.sha.as_deref().is_none_or(|sha| sha == head)
                    && gitlab::conclusion(&pipeline.status) == Some(RunConclusion::Failure) =>
            {
                let jobs: Vec<Job> = forge.get(
                    &what,
                    &format!(
                        "projects/{encoded}/pipelines/{}/jobs?scope=failed",
                        pipeline.id
                    ),
                )?;
                Some(jobs.into_iter().next().map_or_else(
                    || {
                        (
                            "pipeline".to_owned(),
                            one_line(pipeline.web_url.as_deref().unwrap_or("-")),
                        )
                    },
                    |job| {
                        (
                            one_line(&job.name),
                            one_line(job.web_url.as_deref().unwrap_or("-")),
                        )
                    },
                ))
            }
            _ => None,
        };
        Ok(Some(Read::Open { url, head, failed }))
    }
}

/// A GitHub pull request read: merged on `MERGED`, closed on `CLOSED`, else open, its first
/// failed check ([`Check::passed`]) by its name (or context) and its URL.
fn pull_read(
    url: String,
    state: &str,
    merged_at: Option<String>,
    closed_at: Option<String>,
    head: Option<String>,
    checks: &[Check],
) -> Read {
    let at = |at: Option<String>| one_line(&at.unwrap_or_else(|| "-".to_owned()));
    match state {
        "MERGED" => Read::Merged {
            url,
            at: at(merged_at),
        },
        "CLOSED" => Read::Closed {
            url,
            at: at(closed_at),
        },
        _ => Read::Open {
            url,
            head: one_line(head.as_deref().unwrap_or("-")),
            failed: checks
                .iter()
                .find(|check| check.passed() == Some(false))
                .map(|check| {
                    (
                        one_line(
                            check
                                .name
                                .as_deref()
                                .or(check.context.as_deref())
                                .unwrap_or("check"),
                        ),
                        one_line(
                            check
                                .details_url
                                .as_deref()
                                .or(check.target_url.as_deref())
                                .unwrap_or("-"),
                        ),
                    )
                }),
        },
    }
}

/// What `command` prints, read as JSON; `what` names what an error is about.
fn run_json<T: DeserializeOwned>(
    command: &[String],
    what: &str,
    bound: std::time::Duration,
) -> Result<T> {
    let shown = command.join(" ");
    let (program, arguments) = command
        .split_first()
        .with_context(|| format!("{what}: no command is given"))?;
    let output = collect::run(Command::new(program).args(arguments), bound)
        .with_context(|| format!("{what}: `{shown}`"))?;
    if !output.status.success() {
        let status = output.status.code().map_or_else(
            || "was ended by a signal".to_owned(),
            |code| format!("exited {code}"),
        );
        let stderr: String = one_line(&String::from_utf8_lossy(&output.stderr))
            .chars()
            .take(EXCERPT)
            .collect();
        if stderr.is_empty() {
            bail!("{what}: `{shown}` {status}");
        }
        bail!("{what}: `{shown}` {status}: {stderr}");
    }
    serde_json::from_slice(&output.stdout)
        .with_context(|| format!("{what}: `{shown}` printed no JSON it reads"))
}

#[cfg(test)]
mod tests {
    use super::Request;

    #[test]
    fn a_reason_names_its_request_by_url_or_number() {
        let github = |owner: Option<&str>, repository: &str, number| Request::GitHub {
            owner: owner.map(str::to_owned),
            repository: repository.to_owned(),
            number,
        };
        assert_eq!(
            Request::of("ci https://github.com/example-org/alpha/pull/7", "alpha"),
            Some(github(Some("example-org"), "alpha", 7))
        );
        assert_eq!(
            Request::of(
                "review: (https://github.com/example-org/beta/pull/9/files).",
                "x"
            ),
            Some(github(Some("example-org"), "beta", 9))
        );
        assert_eq!(
            Request::of(
                "mr-route https://gitlab.example.com/example-org/sub/gamma/-/merge_requests/4",
                "x"
            ),
            Some(Request::GitLab {
                host: Some("gitlab.example.com".to_owned()),
                path: Some("example-org/sub/gamma".to_owned()),
                repository: "gamma".to_owned(),
                iid: 4,
            })
        );
        assert_eq!(
            Request::of("ci beta#12", "alpha"),
            Some(Request::Number {
                repository: "beta".to_owned(),
                number: 12
            })
        );
        assert_eq!(
            Request::of("ci #12", "alpha"),
            Some(Request::Number {
                repository: "alpha".to_owned(),
                number: 12
            })
        );
        assert_eq!(Request::of("disk: #12", "alpha"), None);
        assert_eq!(Request::of("ci waiting on the pipeline", "alpha"), None);
        assert_eq!(Request::of("ci #0", "alpha"), None);
    }
}
