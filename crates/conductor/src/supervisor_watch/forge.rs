//! The two forge triggers of `conductor supervisor watch` (story:supervisor-watch-forge-triggers,
//! issues #254 and #247). Each instance's forge is read with the host's `gh` and `glab`
//! ([`Sources::gh`](super::Sources::gh), [`Sources::glab`](super::Sources::glab)) over the
//! instance's own sources, with no credential of the command's own, at most every
//! [`FORGE_EVERY`] per instance, and only for the repositories with an open request.
//!
//! - **Reads.** Per `github` owner, `gh search prs --owner <owner> --state open` names the
//!   repositories with an open pull request (those an `include` names, when the source names
//!   any), and `gh pr list -R <owner>/<repo>` reads each one's pull requests: head, draft,
//!   auto-merge and check rollup with each check's end. Per `gitlab` source,
//!   `groups/<group>/merge_requests?state=opened` names its open merge requests (head, draft,
//!   `merge_when_pipeline_succeeds`); when there is one, the group's projects name their
//!   repositories ([`Forge::projects`]), and `projects/<id>/merge_requests/<iid>` reads a
//!   request's head pipeline, again only once its head changed or while its pipeline ran at the
//!   last read ([`GITLAB_CACHE`]).
//! - **`GREEN`**: an open request that is no draft and has no auto-merge, whose checks all ended
//!   green over [`GREEN_AFTER`] ago (the newest end on its head), once per head sha
//!   ([`GREEN_CURSOR`]); one a `--hold` names is passed over and not kept.
//! - **`CIEND`**: the newest report in the mailbox of a repository a `Running` controller works
//!   on is `blocked ci <ref>`, and the checks of that open request ended after the report's ULID
//!   time; once per ref and report ([`CIEND_CURSOR`]).
//! - **Refusals.** A read that fails (`gh` or `glab` exits non-zero: HTTP 403 or 429, no login)
//!   is one note naming the instance and what was read, and doubles the instance's interval up
//!   to [`FORGE_EVERY_MAX`] until a pass reads every source ([`FORGE_CURSOR`]). What it could not
//!   read loses nothing: the kept heads and reports stay. An instance with `forge_access:
//!   anonymous` is not read, one note per run.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::ControllerStorage;
use conductor_model::config::Instance;
use conductor_model::dispatch::ControllerState;
use conductor_model::host::SupervisorTriggerKind as Kind;
use conductor_model::observation::RunConclusion;
use conductor_model::primitives::Timestamp;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::local::{first_line, instant, newest_reports, report_words, ulid_ms};
use super::{Pass, Watch, put, read};
use crate::collect::Origins;
use crate::collect::gitlab::{self, Forge};
use crate::watch::{Check, concluded};

/// The shortest time between two reads of one instance's forge.
pub const FORGE_EVERY: Duration = Duration::from_secs(2 * 60);

/// The longest interval a refused forge backs its instance off to.
pub const FORGE_EVERY_MAX: Duration = Duration::from_secs(16 * 60);

/// How long a request's checks have ended green before `GREEN` names it.
pub const GREEN_AFTER: Duration = Duration::from_secs(15 * 60);

/// How long one `gh` or `glab` read may take before it is stopped.
const FORGE_BOUND: Duration = Duration::from_secs(60);

/// The entries of one page of a listing, and the most pages read.
const PER_PAGE: usize = 100;
const PAGES: usize = 10;

/// The forge cursor of an instance: `<next read, Unix seconds> <interval, seconds>`.
const FORGE_CURSOR: &str = "forge";

/// The heads `GREEN` named of the requests still open: `<subject> <head sha>`, one per line.
const GREEN_CURSOR: &str = "green";

/// The reports `CIEND` named: `<subject> <report file>`, one per line.
const CIEND_CURSOR: &str = "ciend";

/// What the last read of each GitLab merge request found: `<host> <project id> <iid> <head sha>
/// <pipeline status, else -> <finished at, else ->`, one per line.
const GITLAB_CACHE: &str = "gitlab";

/// One open request of an instance, as either forge answers it.
#[derive(Debug, Clone)]
struct Request {
    /// The repository, as the instance's records name it.
    repository: String,
    /// `<repo>#<n>` on GitHub, `<repo>!<iid>` on GitLab.
    subject: String,
    number: u64,
    /// The head commit's sha.
    head: String,
    draft: bool,
    /// Auto-merge (GitHub) or merge when the pipeline succeeds (GitLab) is set.
    auto_merge: bool,
    /// Once every check of its head ended: green or not, and the newest end.
    ended: Option<(bool, OffsetDateTime)>,
}

/// A report `blocked ci <ref>` that is its repository's newest.
#[derive(Debug)]
struct Blocked {
    /// The report's mail file.
    file: String,
    /// The ref as the report wrote it.
    reference: String,
    number: u64,
    /// When it was written: its ULID time.
    written: OffsetDateTime,
}

/// `GREEN` and `CIEND` of `instance`, when its forge is due a read.
///
/// # Errors
///
/// The forge cursor or the GitLab cache cannot be read or written.
pub(super) fn forge(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
    now: OffsetDateTime,
    at: &Timestamp,
    notes: &mut dyn Write,
) -> Result<()> {
    let name = instance.name.0.clone();
    let origins = Origins::of(instance);
    let mut forges = Forge::all(instance);
    if origins.owners.is_empty() && forges.is_empty() {
        return Ok(());
    }
    if crate::config::anonymous(instance) {
        watch.note(
            notes,
            format!("{name}: GREEN, CIEND: forge_access: anonymous, so its forge is not read"),
        );
        return Ok(());
    }
    let cursors = watch.cursors(&name);
    let cursor_file = cursors.join(FORGE_CURSOR);
    let (next, interval) = read(&cursor_file)?
        .and_then(|text| {
            let mut words = text.split_whitespace();
            let next: i64 = words.next()?.parse().ok()?;
            let interval: u64 = words.next()?.parse().ok()?;
            Some((next, interval))
        })
        .unwrap_or((i64::MIN, FORGE_EVERY.as_secs()));
    if now.unix_timestamp() < next {
        return Ok(());
    }
    for forge in &mut forges {
        forge.glab.clone_from(&watch.sources.glab);
        forge.bound = FORGE_BOUND;
    }
    let mut refusals = Vec::new();
    let mut requests = Vec::new();
    for owner in &origins.owners {
        match github(watch, owner, origins.include.get(owner)) {
            Ok((read, refused)) => {
                requests.extend(read);
                refusals.extend(refused);
            }
            Err(error) => refusals.push(error),
        }
    }
    let cache_file = cursors.join(GITLAB_CACHE);
    let cache = read(&cache_file)?.unwrap_or_default();
    let mut cached: BTreeMap<String, String> = cache
        .lines()
        .filter_map(|line| {
            let mut words = line.splitn(4, ' ');
            let key = format!("{} {} {}", words.next()?, words.next()?, words.next()?);
            Some((key, words.next()?.to_owned()))
        })
        .collect();
    let mut fresh = BTreeMap::new();
    for forge in &forges {
        match gitlab_requests(watch, forge, &cached, &mut fresh) {
            Ok((read, refused)) => {
                requests.extend(read);
                refusals.extend(refused);
            }
            Err(error) => refusals.push(error),
        }
    }
    let refused = !refusals.is_empty();
    if refused {
        // A request a refused read did not reach keeps what was cached of it.
        fresh.append(&mut cached);
    }
    let text: String = fresh
        .iter()
        .map(|(key, value)| format!("{key} {value}\n"))
        .collect();
    if text != cache {
        put(&cache_file, &text)?;
    }
    for error in &refusals {
        watch.note(notes, format!("{name}: GREEN, CIEND: {error:#}"));
    }
    let interval = if refused {
        interval
            .max(FORGE_EVERY.as_secs())
            .saturating_mul(2)
            .min(FORGE_EVERY_MAX.as_secs())
    } else {
        FORGE_EVERY.as_secs()
    };
    let next = now
        .unix_timestamp()
        .saturating_add(i64::try_from(interval).unwrap_or(i64::MAX));
    put(&cursor_file, &format!("{next} {interval}\n"))?;
    green(watch, &name, &requests, refused, pass, now, at)?;
    let blocked = blocked(watch, instance);
    ciend(watch, &name, &requests, &blocked, refused, pass, at)
}

/// The lines of the cursor `before` a pass keeps whatever it reads: all of them when a read was
/// `refused`, so what it could not read loses nothing; else none.
fn kept_of(before: &str, refused: bool) -> BTreeSet<String> {
    if refused {
        before.lines().map(str::to_owned).collect()
    } else {
        BTreeSet::new()
    }
}

/// `GREEN` over the requests read: see the module.
fn green(
    watch: &Watch,
    name: &str,
    requests: &[Request],
    refused: bool,
    pass: &mut Pass,
    now: OffsetDateTime,
    at: &Timestamp,
) -> Result<()> {
    let file = watch.cursors(name).join(GREEN_CURSOR);
    let before = read(&file)?.unwrap_or_default();
    let named: BTreeSet<&str> = before.lines().collect();
    let mut kept = kept_of(&before, refused);
    let after = i64::try_from(GREEN_AFTER.as_secs()).unwrap_or(i64::MAX);
    for request in requests {
        let key = format!("{} {}", request.subject, request.head);
        if named.contains(key.as_str()) {
            kept.insert(key);
            continue;
        }
        let Some((true, ended)) = request.ended else {
            continue;
        };
        let green_for = (now - ended).whole_seconds();
        if request.draft
            || request.auto_merge
            || green_for <= after
            || watch.holds().iter().any(|hold| holds(hold, request))
        {
            continue;
        }
        pass.fire(
            Kind::Green,
            name,
            &request.subject,
            &format!("green {}m, head {}", green_for / 60, request.head),
            at,
        );
        kept.insert(key);
    }
    let text: String = kept.iter().map(|line| format!("{line}\n")).collect();
    if text != before {
        pass.write(file, text);
    }
    Ok(())
}

/// `CIEND` over the requests read and the repositories' newest `blocked ci` reports: see the
/// module.
fn ciend(
    watch: &Watch,
    name: &str,
    requests: &[Request],
    blocked: &BTreeMap<String, Blocked>,
    refused: bool,
    pass: &mut Pass,
    at: &Timestamp,
) -> Result<()> {
    let file = watch.cursors(name).join(CIEND_CURSOR);
    let before = read(&file)?.unwrap_or_default();
    let named: BTreeSet<&str> = before.lines().collect();
    let mut kept = kept_of(&before, refused);
    for (repository, report) in blocked {
        let Some(request) = requests
            .iter()
            .find(|request| &request.repository == repository && request.number == report.number)
        else {
            continue;
        };
        let key = format!("{} {}", request.subject, report.file);
        if named.contains(key.as_str()) {
            kept.insert(key);
            continue;
        }
        let Some((green, ended)) = request.ended else {
            continue;
        };
        if ended <= report.written {
            continue;
        }
        pass.fire(
            Kind::CiEnd,
            name,
            &request.subject,
            &format!(
                "{} {}, ended {}",
                report.reference,
                if green { "green" } else { "red" },
                ended.format(&Rfc3339).unwrap_or_default()
            ),
            at,
        );
        kept.insert(key);
    }
    let text: String = kept.iter().map(|line| format!("{line}\n")).collect();
    if text != before {
        pass.write(file, text);
    }
    Ok(())
}

/// The newest report of each repository a `Running` controller of the instance's store works on,
/// when it is `blocked ci <ref>` with a ref that names a request. A store that is not there or
/// does not open gives none: `IDLE` names it.
fn blocked(watch: &Watch, instance: &Instance) -> BTreeMap<String, Blocked> {
    let name = &instance.name.0;
    let state = Path::new(&instance.state);
    let Ok(store) =
        crate::store::open_reader_of(state, name, watch.sources.daemon_store.as_deref())
    else {
        return BTreeMap::new();
    };
    let repositories: BTreeSet<String> = ControllerStorage::list(&store)
        .into_iter()
        .filter(|controller| controller.state == ControllerState::Running)
        .map(|controller| controller.data.repository.0)
        .collect();
    drop(store);
    let repositories: BTreeSet<&str> = repositories.iter().map(String::as_str).collect();
    let mut found = BTreeMap::new();
    for (repository, (file, text)) in newest_reports(instance, &repositories) {
        let Some(words) = report_words(first_line(&text)) else {
            continue;
        };
        if !words.starts_with(&["blocked", "ci"]) {
            continue;
        }
        let Some(reference) = words.get(2) else {
            continue;
        };
        let Some((_, number)) = reference_of(reference) else {
            continue;
        };
        let Some(written) = ulid_ms(&file)
            .and_then(|ms| i128::from(ms).checked_mul(1_000_000))
            .and_then(|nanos| OffsetDateTime::from_unix_timestamp_nanos(nanos).ok())
        else {
            continue;
        };
        found.insert(
            repository,
            Blocked {
                reference: (*reference).to_owned(),
                file,
                number,
                written,
            },
        );
    }
    found
}

/// The repository a ref names, if any, and its request's number: `<repo>#<n>`, `<repo>!<n>`,
/// `<owner>/<repo>#<n>`, `#<n>`, `!<n>`, `<n>`, or a request's URL (`…/<repo>/pull/<n>`,
/// `…/<repo>/-/merge_requests/<n>`).
fn reference_of(text: &str) -> Option<(Option<String>, u64)> {
    let text = text.trim().trim_end_matches(['.', ',', ';', ':', ')']);
    let last = |path: &str| {
        path.trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|step| !step.is_empty())
            .map(str::to_owned)
    };
    for marker in ["/-/merge_requests/", "/pull/"] {
        if let Some((before, after)) = text.split_once(marker) {
            let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
            return Some((last(before), digits.parse().ok()?));
        }
    }
    let (repository, number) = match text.rfind(['#', '!']) {
        Some(split) => (last(&text[..split]), &text[split + 1..]),
        None => (None, text),
    };
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((repository, number.parse().ok()?))
}

/// Whether the `--hold` ref `hold` names `request`.
fn holds(hold: &str, request: &Request) -> bool {
    let Some((repository, number)) = reference_of(hold) else {
        return false;
    };
    let own = request
        .repository
        .rsplit('/')
        .next()
        .unwrap_or(&request.repository);
    number == request.number && repository.is_none_or(|repository| repository == own)
}

/// What `program`, then `words`, printed, or why not: it ran over [`FORGE_BOUND`] or exited
/// non-zero, with the start of its standard error.
fn output(program: &[String], words: &[&str], what: &str) -> Result<String> {
    let (first, rest) = program
        .split_first()
        .ok_or_else(|| anyhow!("{what}: no program is given"))?;
    let shown = program
        .iter()
        .map(String::as_str)
        .chain(words.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let mut command = Command::new(first);
    command
        .args(rest)
        .args(words)
        .env(gitlab::TELEMETRY_VARIABLE, "false")
        .env("GH_PROMPT_DISABLED", "1");
    let output = crate::collect::run(&mut command, FORGE_BOUND)
        .with_context(|| format!("{what}: `{shown}`"))?;
    if !output.status.success() {
        let status = output.status.code().map_or_else(
            || "was stopped by a signal".to_owned(),
            |code| format!("exited {code}"),
        );
        let stderr: String = String::from_utf8_lossy(&output.stderr)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect();
        if stderr.is_empty() {
            bail!("{what}: `{shown}` {status}");
        }
        bail!("{what}: `{shown}` {status}: {stderr}");
    }
    String::from_utf8(output.stdout)
        .with_context(|| format!("{what}: `{shown}` printed text that is not UTF-8"))
}

/// [`output`] read as JSON.
fn json<T: DeserializeOwned>(program: &[String], words: &[&str], what: &str) -> Result<T> {
    let text = output(program, words, what)?;
    serde_json::from_str(&text).with_context(|| format!("{what}: it printed no JSON it answers"))
}

/// One open pull request as `gh search prs --json repository,number` prints it.
#[derive(Deserialize)]
struct Found {
    repository: FoundRepository,
}

/// The repository of a [`Found`].
#[derive(Deserialize)]
struct FoundRepository {
    name: String,
}

/// One open pull request as `gh pr list --json
/// number,headRefOid,isDraft,autoMergeRequest,statusCheckRollup` prints it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    number: u64,
    head_ref_oid: String,
    #[serde(default)]
    is_draft: bool,
    #[serde(default)]
    auto_merge_request: Option<serde_json::Value>,
    #[serde(default)]
    status_check_rollup: Option<Vec<Check>>,
}

/// The open pull requests of the repositories of `owner` with one (those `include` names, when
/// it is given), and each repository's read that was refused.
///
/// # Errors
///
/// The search was refused.
fn github(
    watch: &Watch,
    owner: &str,
    include: Option<&Vec<String>>,
) -> Result<(Vec<Request>, Vec<anyhow::Error>)> {
    let program = &watch.sources.gh;
    let limit = (PER_PAGE * PAGES).to_string();
    let found: Vec<Found> = json(
        program,
        &[
            "search",
            "prs",
            "--owner",
            owner,
            "--state",
            "open",
            "--limit",
            &limit,
            "--json",
            "repository,number",
        ],
        &format!("the open pull requests of {owner} not read"),
    )?;
    let repositories: BTreeSet<String> = found
        .into_iter()
        .map(|found| found.repository.name)
        .filter(|name| include.is_none_or(|include| include.iter().any(|entry| entry == name)))
        .collect();
    let mut requests = Vec::new();
    let mut refused = Vec::new();
    for repository in repositories {
        let named = format!("{owner}/{repository}");
        let pulls: Vec<Pull> = match json(
            program,
            &[
                "pr",
                "list",
                "-R",
                &named,
                "--state",
                "open",
                "--limit",
                &limit,
                "--json",
                "number,headRefOid,isDraft,autoMergeRequest,statusCheckRollup",
            ],
            &format!("the pull requests of {named} not read"),
        ) {
            Ok(pulls) => pulls,
            Err(error) => {
                refused.push(error);
                continue;
            }
        };
        for pull in pulls {
            let checks = pull.status_check_rollup.unwrap_or_default();
            let ended = concluded(&checks).and_then(|green| {
                let newest = checks
                    .iter()
                    .filter_map(|check| check.ended_at().and_then(instant))
                    .max()?;
                Some((green, newest))
            });
            requests.push(Request {
                subject: format!("{repository}#{}", pull.number),
                repository: repository.clone(),
                number: pull.number,
                head: pull.head_ref_oid.split_whitespace().collect(),
                draft: pull.is_draft,
                auto_merge: pull.auto_merge_request.is_some_and(|set| !set.is_null()),
                ended,
            });
        }
    }
    Ok((requests, refused))
}

/// One open merge request of a group, as `groups/<group>/merge_requests` answers it.
#[derive(Deserialize)]
struct MergeRequest {
    iid: u64,
    project_id: u64,
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    work_in_progress: bool,
    #[serde(default)]
    merge_when_pipeline_succeeds: bool,
}

/// One merge request, as `projects/<id>/merge_requests/<iid>` answers it, in the part read.
#[derive(Deserialize)]
struct Detail {
    #[serde(default)]
    head_pipeline: Option<Pipeline>,
}

/// A merge request's head pipeline.
#[derive(Deserialize)]
struct Pipeline {
    status: String,
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    finished_at: Option<String>,
}

/// Every entry of the GitLab list at `path` on `host`, page by page.
fn pages<T: DeserializeOwned>(
    program: &[String],
    host: &str,
    path: &str,
    what: &str,
) -> Result<Vec<T>> {
    let mut entries = Vec::new();
    for page in 1..=PAGES {
        let paged = format!("{path}&per_page={PER_PAGE}&page={page}");
        let answered: Vec<T> = json(program, &["api", "--hostname", host, &paged], what)?;
        let full = answered.len() >= PER_PAGE;
        entries.extend(answered);
        if !full {
            break;
        }
    }
    Ok(entries)
}

/// The open merge requests of `forge`'s group, each with its head pipeline, read again only
/// when `cached` (by `<host> <project id> <iid>`: `<sha> <status> <finished at>`) holds no
/// pipeline that ended on its head; what each read found goes into `fresh`. And each request's
/// read that was refused.
///
/// # Errors
///
/// The group's list or its projects were refused.
fn gitlab_requests(
    watch: &Watch,
    forge: &Forge,
    cached: &BTreeMap<String, String>,
    fresh: &mut BTreeMap<String, String>,
) -> Result<(Vec<Request>, Vec<anyhow::Error>)> {
    let program = &watch.sources.glab;
    let group = forge.named();
    let opened: Vec<MergeRequest> = pages(
        program,
        &forge.host,
        &format!(
            "groups/{}/merge_requests?state=opened&scope=all",
            gitlab::encoded(forge.group.trim_matches('/'))
        ),
        &format!("the open merge requests of {group} not read"),
    )?;
    if opened.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let projects: BTreeMap<u64, String> = forge
        .projects()
        .with_context(|| format!("the projects of {group} not read"))?
        .into_iter()
        .filter(|project| !project.archived)
        .map(|project| (project.id, project.name))
        .collect();
    let mut requests = Vec::new();
    let mut refused = Vec::new();
    for request in opened {
        let Some(repository) = projects.get(&request.project_id) else {
            continue;
        };
        let sha: String = request.sha.unwrap_or_default().split_whitespace().collect();
        let key = format!("{} {} {}", forge.host, request.project_id, request.iid);
        let known = cached.get(&key).and_then(|value| {
            let mut words = value.split(' ');
            let (head, status, finished) = (words.next()?, words.next()?, words.next()?);
            (head == sha && ended(status)).then(|| (status.to_owned(), finished.to_owned()))
        });
        let (status, finished) = if let Some(known) = known {
            known
        } else {
            let path = format!(
                "projects/{}/merge_requests/{}",
                request.project_id, request.iid
            );
            let detail: Detail = match json(
                program,
                &["api", "--hostname", &forge.host, &path],
                &format!("the merge request {repository}!{} not read", request.iid),
            ) {
                Ok(detail) => detail,
                Err(error) => {
                    refused.push(error);
                    continue;
                }
            };
            detail
                .head_pipeline
                .filter(|pipeline| pipeline.sha.as_deref().is_none_or(|on| on == sha))
                .map_or_else(
                    || ("-".to_owned(), "-".to_owned()),
                    |pipeline| {
                        let status: String = pipeline.status.split_whitespace().collect();
                        let finished = pipeline
                            .finished_at
                            .map(|at| at.split_whitespace().collect::<String>())
                            .filter(|at| !at.is_empty())
                            .unwrap_or_else(|| "-".to_owned());
                        (
                            if status.is_empty() {
                                "-".to_owned()
                            } else {
                                status
                            },
                            finished,
                        )
                    },
                )
        };
        fresh.insert(key, format!("{sha} {status} {finished}"));
        let green = match gitlab::conclusion(&status) {
            Some(RunConclusion::Success | RunConclusion::Skipped) => Some(true),
            Some(RunConclusion::Failure | RunConclusion::Cancelled) => Some(false),
            _ => None,
        };
        requests.push(Request {
            subject: format!("{repository}!{}", request.iid),
            repository: repository.clone(),
            number: request.iid,
            head: sha,
            draft: request.draft || request.work_in_progress,
            auto_merge: request.merge_when_pipeline_succeeds,
            ended: green.zip(instant(&finished)),
        });
    }
    Ok((requests, refused))
}

/// Whether a pipeline of `status` ended, as GitLab's mapping decides it
/// ([`gitlab::conclusion`]).
fn ended(status: &str) -> bool {
    matches!(
        gitlab::conclusion(status),
        Some(
            RunConclusion::Success
                | RunConclusion::Skipped
                | RunConclusion::Failure
                | RunConclusion::Cancelled
        )
    )
}
