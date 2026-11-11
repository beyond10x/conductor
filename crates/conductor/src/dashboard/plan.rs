//! The Plan page (`story:dashboard-plan-board`): `GET /v1/plan` answers the board of one
//! repository's planning store (`conductor.direction.PlanBoard`), `GET /v1/plan/story` one story
//! as the drawer shows it (`conductor.direction.PlanStory`), and `POST /v1/plan` does what the
//! operator chose to one story (`conductor.direction.WritePlan`).
//!
//! The planning store is the `aep` CLI's: every read and every write here is one `aep plan
//! artifact` verb, run in the repository's checkout with `--store <checkout>/.engineering/planning`,
//! so the store's lifecycle decides each move and the store keeps its own journal. A repository is
//! a checkout under the instance's checkouts root, or one level below it in a group, that holds
//! `.engineering/planning`; a request naming any other is refused before `aep` runs.
//!
//! The write is admitted by the dashboard's one write path, the Configuration page's
//! (`super::post_write`): the server's own host and origin on 127.0.0.1, the page's token, JSON
//! of a bounded length, one write at a time. A verb the store refuses changes nothing and
//! records nothing; one that succeeds records one class-C decision by the operator, recorded by
//! `dashboard`, with the verb and the store's answer as its evidence.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_model::decision::DecisionClass;
use conductor_model::direction::{PlanAction, PlanColumn, PlanWriteRefusal};
use serde::Deserialize;
use serde_json::{Value, json};
use time::OffsetDateTime;

use super::Sources;
use super::answers::{RECORDED_BY, Refused};
use crate::decide;

/// The board's data, and the write.
pub const PLAN_PATH: &str = "/v1/plan";

/// One story as the drawer shows it.
pub const STORY_PATH: &str = "/v1/plan/story";

/// The tag that makes a story important: it goes first, in the lane `next`.
pub const IMPORTANT: &str = "p1";

/// The tag that says an accepted story is being worked on.
pub const STARTED: &str = "started";

/// The planning store under a checkout.
const STORE: [&str; 2] = [".engineering", "planning"];

/// What runs a move for the operator, as the store's journal names it: the move is on his
/// behalf, executed by the dashboard.
const EXECUTOR: &str = "agent:dashboard";

/// The repositories under `checkouts` that hold a planning store, by name relative to it
/// (`alpha`, or `group/alpha` one level down in a directory that is no checkout), in name order.
#[must_use]
pub fn repositories(checkouts: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let holds_store = |dir: &Path| dir.join(STORE[0]).join(STORE[1]).is_dir();
    let children = |dir: &Path| -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| path.is_dir())
                    .collect()
            })
            .unwrap_or_default();
        dirs.sort();
        dirs
    };
    for child in children(checkouts) {
        let Some(name) = child
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if holds_store(&child) {
            found.push((name, child));
        } else if !child.join(".git").exists() {
            for inner in children(&child) {
                let Some(leaf) = inner
                    .file_name()
                    .map(|leaf| leaf.to_string_lossy().into_owned())
                else {
                    continue;
                };
                if !leaf.starts_with('.') && holds_store(&inner) {
                    found.push((format!("{name}/{leaf}"), inner));
                }
            }
        }
    }
    found
}

/// One run of `aep plan artifact <args> --store <store> --format json` in `checkout`.
fn aep(checkout: &Path, args: &[&str], stdin: Option<&str>) -> std::io::Result<Output> {
    let store = checkout.join(STORE[0]).join(STORE[1]);
    let mut command = Command::new("aep");
    command
        .env_remove(crate::instance::SLOTS_VARIABLE)
        .args(["plan", "artifact"])
        .args(args)
        .arg("--store")
        .arg(&store)
        .args(["--format", "json"])
        .current_dir(checkout)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = command.spawn()?;
    if let (Some(text), Some(mut input)) = (stdin, child.stdin.take()) {
        input.write_all(text.as_bytes())?;
    }
    child.wait_with_output()
}

/// The JSON `aep` printed for `args`, or what it said when it printed none.
fn read(checkout: &Path, args: &[&str]) -> Result<Value, String> {
    let output = aep(checkout, args, None)
        .map_err(|error| format!("aep plan artifact {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "aep plan artifact {}: {}",
            args.join(" "),
            said(&output)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("aep plan artifact {}: no JSON: {error}", args.join(" ")))
}

/// What a run of `aep` said, on standard output and standard error, on one line each.
fn said(output: &Output) -> String {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    text.trim().to_owned()
}

/// The column of a story at `status`, started or not.
#[must_use]
pub fn column(status: &str, started: bool) -> Option<PlanColumn> {
    match status {
        "draft" => Some(PlanColumn::Draft),
        "proposed" => Some(PlanColumn::Proposed),
        "active" if started => Some(PlanColumn::InProgress),
        "active" => Some(PlanColumn::Accepted),
        "implemented" => Some(PlanColumn::Done),
        _ => None,
    }
}

/// The word of `column` as the page reads it.
fn column_word(column: PlanColumn) -> &'static str {
    match column {
        PlanColumn::Draft => "Draft",
        PlanColumn::Proposed => "Proposed",
        PlanColumn::Accepted => "Accepted",
        PlanColumn::InProgress => "InProgress",
        PlanColumn::Done => "Done",
    }
}

/// The lane of each story `aep plan artifact waves --status <status>` places: `next` for the
/// first wave, `wave <n>` for a later one, `unplanned` for a story it could not place.
fn lanes(checkout: &Path, status: &str, errors: &mut Vec<String>) -> Vec<(String, String)> {
    let waves = match read(checkout, &["waves", "--kind", "story", "--status", status]) {
        Ok(waves) => waves,
        Err(error) => {
            errors.push(error);
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for (position, wave) in waves["waves"].as_array().into_iter().flatten().enumerate() {
        let lane = if position == 0 {
            "next".to_owned()
        } else {
            format!(
                "wave {}",
                wave["wave"].as_i64().unwrap_or(position as i64 + 1)
            )
        };
        for artifact in wave["artifacts"].as_array().into_iter().flatten() {
            if let Some(id) = artifact["id"].as_str() {
                out.push((id.to_owned(), lane.clone()));
            }
        }
    }
    for id in waves["unassessed"].as_array().into_iter().flatten() {
        if let Some(id) = id.as_str() {
            out.push((id.to_owned(), "unplanned".to_owned()));
        }
    }
    out
}

/// The tags of `show`'s answer.
fn tags(shown: &Value) -> Vec<String> {
    shown["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tag| tag.as_str().map(str::to_owned))
        .collect()
}

/// Whether evidence was recorded about a story, from its `history`.
fn has_evidence(history: &Value) -> bool {
    history
        .as_array()
        .into_iter()
        .flatten()
        .any(|entry| entry["change"]["change"] == "evidence")
}

/// The answer of `GET /v1/plan` for the repository `asked`, else the first.
#[must_use]
pub fn board(sources: &Sources, asked: Option<&str>) -> Value {
    let found = repositories(&sources.checkouts);
    let mut errors = Vec::new();
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    let chosen = match asked {
        Some(asked) => found.iter().find(|(name, _)| name == asked),
        None => found.first(),
    };
    let Some((name, checkout)) = chosen else {
        if let Some(asked) = asked {
            errors.push(format!(
                "no checkout named {asked:?} holds a planning store"
            ));
        } else {
            errors.push(format!(
                "no checkout under {} holds a planning store (.engineering/planning)",
                sources.checkouts.display()
            ));
        }
        return json!({"repositories": names, "repository": "", "cards": [], "errors": errors});
    };
    let listed = match read(checkout, &["list", "--kind", "story"]) {
        Ok(listed) => listed,
        Err(error) => {
            errors.push(error);
            Value::Array(Vec::new())
        }
    };
    let mut lane_of = lanes(checkout, "proposed", &mut errors);
    lane_of.extend(lanes(checkout, "active", &mut errors));
    let mut cards = Vec::new();
    for story in listed.as_array().into_iter().flatten() {
        let (Some(id), Some(status)) = (story["id"].as_str(), story["status"].as_str()) else {
            continue;
        };
        if column(status, false).is_none() {
            continue;
        }
        // The tags are read for the stories a tag places: proposed and active ones.
        let (important, started) = if matches!(status, "proposed" | "active") {
            let tags = read(checkout, &["show", id])
                .map(|shown| tags(&shown))
                .unwrap_or_default();
            let started = status == "active"
                && (tags.iter().any(|tag| tag == STARTED)
                    || read(checkout, &["history", id])
                        .is_ok_and(|history| has_evidence(&history)));
            (tags.iter().any(|tag| tag == IMPORTANT), started)
        } else {
            (false, false)
        };
        let Some(column) = column(status, started) else {
            continue;
        };
        let lane = if important && matches!(column, PlanColumn::Accepted | PlanColumn::Proposed) {
            "next".to_owned()
        } else {
            lane_of
                .iter()
                .find(|(known, _)| known == id)
                .map(|(_, lane)| lane.clone())
                .unwrap_or_default()
        };
        cards.push(json!({
            "id": id,
            "title": story["title"].as_str().unwrap_or_default(),
            "status": status,
            "column": column_word(column),
            "lane": lane,
            "important": important,
            "started": started,
        }));
    }
    json!({"repositories": names, "repository": name, "cards": cards, "errors": errors})
}

/// The prose under the `## <heading>` of `body`, without the heading; empty when it has none.
fn section(body: &str, heading: &str) -> String {
    let mut out = Vec::new();
    let mut inside = false;
    for line in body.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            inside = title.trim().eq_ignore_ascii_case(heading);
            continue;
        }
        if inside {
            out.push(line);
        }
    }
    out.join("\n").trim().to_owned()
}

/// The items of an Acceptance section: each bullet with its continuation lines, and each
/// paragraph that is no bullet, on one line each.
#[must_use]
pub fn items(text: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    let mut open = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            open = false;
            continue;
        }
        let bullet = ["- ", "* "]
            .iter()
            .find_map(|mark| line.strip_prefix(mark))
            .or_else(|| {
                let digits = line.chars().take_while(char::is_ascii_digit).count();
                (digits > 0)
                    .then(|| line[digits..].strip_prefix(". "))
                    .flatten()
            });
        match (bullet, open) {
            (Some(rest), _) => {
                items.push(rest.trim().to_owned());
                open = true;
            }
            (None, true) => {
                if let Some(last) = items.last_mut() {
                    last.push(' ');
                    last.push_str(trimmed);
                }
            }
            (None, false) => {
                items.push(trimmed.to_owned());
                open = true;
            }
        }
    }
    items
}

/// The code spans of `line`: the text between each pair of backticks.
fn spans(line: &str) -> Vec<&str> {
    line.split('`')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, span)| span.trim())
        .filter(|span| span.len() >= 3)
        .collect()
}

/// Whether the evidence `sources` name `line`: one of them holds one of its code spans.
#[must_use]
pub fn evidenced(line: &str, sources: &[String]) -> bool {
    spans(line)
        .iter()
        .any(|span| sources.iter().any(|source| source.contains(span)))
}

/// One history entry as one line: a move `draft → proposed`, an evidence record with its kind
/// and source, else what changed.
fn event(entry: &Value) -> Value {
    let change = &entry["change"];
    let what = match change["change"].as_str().unwrap_or_default() {
        "moved" => format!(
            "{} → {}",
            change["from"].as_str().unwrap_or_default(),
            change["to"].as_str().unwrap_or_default()
        ),
        "evidence" => format!(
            "evidence: {} ({})",
            change["kind"].as_str().unwrap_or_default(),
            change["source"].as_str().unwrap_or_default()
        ),
        other => other.to_owned(),
    };
    json!({
        "at": entry["at"].as_str().unwrap_or_default(),
        "actor": entry["actor"].as_str().unwrap_or_default(),
        "what": what,
    })
}

/// Whether `id` is a story id the page may pass to `aep`: `story:` and a name of lower-case
/// letters, digits and dashes.
#[must_use]
pub fn story_id(id: &str) -> bool {
    id.strip_prefix("story:").is_some_and(|name| {
        !name.is_empty()
            && !name.starts_with('-')
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    })
}

/// The statuses the store's story lifecycle lets a story at `status` move to.
fn moves(checkout: &Path, status: &str) -> Vec<String> {
    read(checkout, &["lifecycle", "story"])
        .ok()
        .and_then(|lifecycle| {
            lifecycle["transitions"][status].as_array().map(|to| {
                to.iter()
                    .filter_map(|to| to.as_str().map(str::to_owned))
                    .filter(|to| to != status)
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// The answer of `GET /v1/plan/story` for `id` in the repository `repository`; `Err` with the
/// status and why when there is no such repository or story.
///
/// # Errors
///
/// No checkout of that name holds a planning store (404), `id` is no story id or the store
/// holds no such story (404).
pub fn story(sources: &Sources, repository: &str, id: &str) -> Result<Value, (u16, String)> {
    let found = repositories(&sources.checkouts);
    let Some((_, checkout)) = found.iter().find(|(name, _)| name == repository) else {
        return Err((
            404,
            format!("no checkout named {repository:?} holds a planning store"),
        ));
    };
    if !story_id(id) {
        return Err((404, format!("{id:?} is no story id")));
    }
    let shown = read(checkout, &["show", id]).map_err(|error| (404, error))?;
    let history = read(checkout, &["history", id]).unwrap_or(Value::Array(Vec::new()));
    let sources_of: Vec<String> = history
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["change"]["change"] == "evidence")
        .map(|entry| {
            format!(
                "{} {}",
                entry["change"]["source"].as_str().unwrap_or_default(),
                entry["change"]["ref"].as_str().unwrap_or_default()
            )
        })
        .collect();
    let body = shown["body"].as_str().unwrap_or_default();
    let acceptance: Vec<Value> = items(&section(body, "Acceptance"))
        .into_iter()
        .map(|line| {
            let evidence = evidenced(&line, &sources_of);
            json!({"text": line, "evidence": evidence})
        })
        .collect();
    let mut scope: Vec<String> = shown["scope"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            entry["path"]
                .as_str()
                .map(|path| match entry["confidence"].as_str() {
                    Some(confidence) => format!("{path} ({confidence})"),
                    None => path.to_owned(),
                })
        })
        .collect();
    if scope.is_empty() {
        scope = items(&section(body, "Scope"));
    }
    let status = shown["status"].as_str().unwrap_or_default().to_owned();
    let tags = tags(&shown);
    let started =
        status == "active" && (tags.iter().any(|tag| tag == STARTED) || !sources_of.is_empty());
    Ok(json!({
        "id": id,
        "title": shown["title"].as_str().unwrap_or_default(),
        "status": status,
        "column": column(&status, started).map(column_word),
        "why": section(body, "Why"),
        "acceptance": acceptance,
        "scope": scope,
        "history": history.as_array().into_iter().flatten().map(event).collect::<Vec<_>>(),
        "moves": moves(checkout, &status),
        "comments": section(body, "Comments"),
        "important": tags.iter().any(|tag| tag == IMPORTANT),
        "started": started,
    }))
}

/// The write as the page sends it (`conductor.direction.WritePlan`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Write {
    repository: String,
    id: String,
    action: String,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

/// The word of `refusal` as the answer names it.
fn refusal_word(refusal: PlanWriteRefusal) -> &'static str {
    match refusal {
        PlanWriteRefusal::NotThisPage => "NotThisPage",
        PlanWriteRefusal::TokenMissing => "TokenMissing",
        PlanWriteRefusal::UnknownRepository => "UnknownRepository",
        PlanWriteRefusal::UnknownStory => "UnknownStory",
        PlanWriteRefusal::MoveRefused => "MoveRefused",
        PlanWriteRefusal::ActionInvalid => "ActionInvalid",
        PlanWriteRefusal::StoreFailed => "StoreFailed",
    }
}

/// A refusal of the write, as `conductor.direction.PlanWriteRefused`.
fn refused(refusal: PlanWriteRefusal, detail: impl Into<String>) -> Refused {
    let (status, reason) = match refusal {
        PlanWriteRefusal::NotThisPage | PlanWriteRefusal::TokenMissing => (403, "Forbidden"),
        PlanWriteRefusal::UnknownRepository | PlanWriteRefusal::UnknownStory => (404, "Not Found"),
        PlanWriteRefusal::MoveRefused => (409, "Conflict"),
        PlanWriteRefusal::ActionInvalid => (422, "Unprocessable Content"),
        PlanWriteRefusal::StoreFailed => (500, "Internal Server Error"),
    };
    let body = json!({"refusal": refusal_word(refusal), "detail": detail.into()});
    Refused {
        status,
        reason,
        why: format!("{body}\n"),
    }
}

/// The action the page names.
fn action(word: &str) -> Option<PlanAction> {
    match word {
        "Move" => Some(PlanAction::Move),
        "Start" => Some(PlanAction::Start),
        "Important" => Some(PlanAction::Important),
        "Unimportant" => Some(PlanAction::Unimportant),
        "Comment" => Some(PlanAction::Comment),
        _ => None,
    }
}

/// What the operator asked, in his words as the decision quotes them.
fn words(action: PlanAction, id: &str, to: &str, text: &str) -> String {
    match action {
        PlanAction::Move => match to {
            "active" => format!("Approve {id}"),
            "rejected" => format!("Reject {id}"),
            "proposed" => format!("Propose {id}"),
            "draft" => format!("Back to draft: {id}"),
            "implemented" => format!("Mark {id} done"),
            "archived" => format!("Archive {id}"),
            other => format!("Move {id} to {other}"),
        },
        PlanAction::Start => format!("Start {id}"),
        PlanAction::Important => format!("Make {id} important (P1)"),
        PlanAction::Unimportant => format!("Clear the priority of {id}"),
        PlanAction::Comment => text.to_owned(),
    }
}

/// Does the write `body` asks for to a story of a planning store under `sources`' checkouts, and
/// records it as the operator's decision; the answer (`conductor.direction.PlanWritten`) or the
/// refusal.
pub(super) fn write(
    sources: &Sources,
    state: Option<&Path>,
    body: &[u8],
) -> Result<Value, Refused> {
    let asked: Write = serde_json::from_slice(body).map_err(|error| Refused {
        status: 400,
        reason: "Bad Request",
        why: format!(
            "the body is no write {{\"repository\", \"id\", \"action\", \"to\", \"text\"}}: \
             {error}\n"
        ),
    })?;
    let Some(act) = action(&asked.action) else {
        return Err(refused(
            PlanWriteRefusal::ActionInvalid,
            format!(
                "action {:?} is not Move, Start, Important, Unimportant or Comment",
                asked.action
            ),
        ));
    };
    let found = repositories(&sources.checkouts);
    let Some((_, checkout)) = found.iter().find(|(name, _)| *name == asked.repository) else {
        return Err(refused(
            PlanWriteRefusal::UnknownRepository,
            format!(
                "no checkout named {:?} holds a planning store",
                asked.repository
            ),
        ));
    };
    let id = asked.id.as_str();
    if !story_id(id) {
        return Err(refused(
            PlanWriteRefusal::UnknownStory,
            format!("{id:?} is no story id"),
        ));
    }
    let shown = read(checkout, &["show", id])
        .map_err(|error| refused(PlanWriteRefusal::UnknownStory, error))?;
    let before = shown["status"].as_str().unwrap_or_default().to_owned();
    let to = asked.to.as_deref().map(str::trim).unwrap_or_default();
    let text = asked.text.as_deref().map(str::trim).unwrap_or_default();
    let invalid = |why: &str| Err(refused(PlanWriteRefusal::ActionInvalid, why.to_owned()));
    let (args, stdin): (Vec<String>, Option<String>) = match act {
        PlanAction::Move => {
            if to.is_empty()
                || !to
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
            {
                return invalid("a move names the status to move to, as `to`");
            }
            (
                ["move", id, "--to", to, "--executor", EXECUTOR]
                    .map(str::to_owned)
                    .to_vec(),
                None,
            )
        }
        PlanAction::Start => {
            if before != "active" || tags(&shown).iter().any(|tag| tag == STARTED) {
                return invalid("only an accepted story that is not started yet is started");
            }
            (
                ["set", id, "--tag", STARTED].map(str::to_owned).to_vec(),
                None,
            )
        }
        PlanAction::Important => (
            ["set", id, "--tag", IMPORTANT].map(str::to_owned).to_vec(),
            None,
        ),
        PlanAction::Unimportant => (
            ["set", id, "--untag", IMPORTANT]
                .map(str::to_owned)
                .to_vec(),
            None,
        ),
        PlanAction::Comment => {
            if text.is_empty() {
                return invalid("a comment needs its text");
            }
            let at = OffsetDateTime::now_utc();
            let line = format!(
                "- {:04}-{:02}-{:02} {:02}:{:02}Z, the operator: {}",
                at.year(),
                u8::from(at.month()),
                at.day(),
                at.hour(),
                at.minute(),
                text.split_whitespace().collect::<Vec<_>>().join(" ")
            );
            let old = section(shown["body"].as_str().unwrap_or_default(), "Comments");
            let new = if old.is_empty() {
                format!("{line}\n")
            } else {
                format!("{old}\n{line}\n")
            };
            (
                ["body", id, "--section", "Comments", "--from", "-"]
                    .map(str::to_owned)
                    .to_vec(),
                Some(new),
            )
        }
    };
    let words_of: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = aep(checkout, &words_of, stdin.as_deref()).map_err(|error| {
        refused(
            PlanWriteRefusal::StoreFailed,
            format!("aep plan artifact {}: {error}", args.join(" ")),
        )
    })?;
    let answered = said(&output);
    if !output.status.success() {
        // The store answers a move its lifecycle refuses on standard output, with the statuses
        // the story may move to; a failure of `aep` itself is an `error:` on standard error.
        let failed = String::from_utf8_lossy(&output.stderr)
            .trim_start()
            .starts_with("error");
        let refusal = if act == PlanAction::Move && !failed {
            PlanWriteRefusal::MoveRefused
        } else {
            PlanWriteRefusal::StoreFailed
        };
        return Err(refused(refusal, answered));
    }
    let after = match act {
        PlanAction::Move => to.to_owned(),
        _ => before.clone(),
    };
    let operator_words = words(act, id, to, text);
    let (question, options, choice) = match act {
        PlanAction::Move => (
            format!("Move {id} from {before} to {to}?"),
            format!("move it to {to}; leave it {before}"),
            format!("move it to {to}"),
        ),
        PlanAction::Start => (
            format!("Start {id} now?"),
            "start it; leave it accepted".to_owned(),
            "start it".to_owned(),
        ),
        PlanAction::Important => (
            format!("Make {id} important?"),
            "put it first in the next wave; keep its place".to_owned(),
            "put it first in the next wave".to_owned(),
        ),
        PlanAction::Unimportant => (
            format!("Clear the priority of {id}?"),
            "clear it; keep it first".to_owned(),
            "clear it".to_owned(),
        ),
        PlanAction::Comment => (
            format!("Comment on {id}?"),
            "record the comment on the story; leave it".to_owned(),
            "record the comment on the story".to_owned(),
        ),
    };
    let decision = decide::record_as_operator(
        state,
        &decide::OperatorDecision {
            decision_class: DecisionClass::C,
            question,
            options,
            choice,
            reason: format!("the operator chose it on the dashboard's Plan page: {operator_words}"),
            evidence: format!(
                "aep plan artifact {} in {}: {answered}",
                args.join(" "),
                asked.repository
            ),
            recorded_by: RECORDED_BY.to_owned(),
            operator_words: operator_words.clone(),
        },
    )
    .map_err(|error| Refused {
        status: 500,
        reason: "Internal Server Error",
        why: format!(
            "the store took {operator_words:?}, but the decision was not recorded: {error:#}\n"
        ),
    })?;
    Ok(json!({
        "id": id,
        "status": after,
        "decision_id": decision,
        "detail": answered,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_acceptance_section_is_one_item_per_bullet() {
        let text = "Spec first.\n\n- one `a_case` line\n  continued\n- two\n";
        assert_eq!(
            items(text),
            vec!["Spec first.", "one `a_case` line continued", "two"]
        );
    }

    #[test]
    fn a_line_is_evidenced_when_a_source_holds_its_code_span() {
        let sources = vec!["cargo test --test a_case: 3 passed".to_owned()];
        assert!(evidenced("the `a_case` test", &sources));
        assert!(!evidenced("the `other` test", &sources));
        assert!(!evidenced("no span", &sources));
    }

    #[test]
    fn only_a_story_id_reaches_aep() {
        assert!(story_id("story:dashboard-plan-board"));
        assert!(!story_id("story:--store=/x"));
        assert!(!story_id("epic:a"));
        assert!(!story_id("story:A b"));
    }

    #[test]
    fn active_is_accepted_until_started() {
        assert_eq!(column("active", false), Some(PlanColumn::Accepted));
        assert_eq!(column("active", true), Some(PlanColumn::InProgress));
        assert_eq!(column("rejected", false), None);
    }
}
