//! The records' two logs, written by the commands that record their events
//! (`story:records-from-commands`), and `conductor records check`, which reads them.
//!
//! **Writing.** Once the store has kept what a command recorded, the command appends exactly one
//! line to `<records>/dispatches/YYYY-MM.jsonl` ([`DispatchLogLine`]) or
//! `<records>/decisions/YYYY-MM.jsonl` ([`DecisionLogLine`]): the month of the line's `at`, in
//! UTC. `<records>` is the `records` of the instance this process runs ([`config::active`]). A
//! line is one JSON object: `at`, `event` and `repo` (the repository, under that key only), then
//! the fields its event declares in `spec/domains/dispatch.yaml` and
//! `spec/domains/decision.yaml`; an absent field is left out, but for a decision line's `repo`,
//! written null. With a config file the log's directory is created when it is not there. Without
//! one the records are the working directory, and a line is appended only where `conductor init`
//! made the log's directory, so a command run elsewhere plants no log. A line that cannot be
//! appended fails the command, naming the file, after the store kept the record.
//!
//! **Checking.** `records check` reads every `*.jsonl` of both logs, in name order, and the
//! decisions of the store when there is one (it opens only an existing store and writes nothing).
//! It reports, per line, in file and line order: a line of `dispatches/` whose `event` is a
//! decision log's event, or that has no `event` and whose `id` is a decision id
//! (`DecisionInDispatchLog`, and nothing else for that line); a line of either log that is no JSON
//! object, misses a key every line of its log holds or holds a key its log does not declare
//! (`UnknownKeySet`); and each decision id (`DEC-…`) a line of `dispatches/` cites that no line of
//! `decisions/` and no decision of the store holds (`UnknownDecision`). It prints one finding per
//! line, `<file>:<line>: <finding>: <detail>`, or with `--format json` an array of
//! `RecordsFinding` objects, and exits 1 when it found any.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::DecisionStorage;
use conductor_model::decision::{DecisionLogEvent, DecisionLogLine};
use conductor_model::dispatch::{
    CheckRecords, DispatchLogEvent, DispatchLogLine, RecordsFinding, RecordsFindingKind,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::RecordsCheckArgs;
use crate::config;
use crate::decide::{class_name, decider_name};

/// The directory of the dispatch log under the records directory.
pub const DISPATCHES: &str = "dispatches";

/// The directory of the decision log under the records directory.
pub const DECISIONS: &str = "decisions";

/// The keys a line of the dispatch log may hold: the fields of `conductor.dispatch.DispatchLogLine`
/// (`spec/domains/dispatch.yaml`), in its order.
pub const DISPATCH_KEYS: [&str; 19] = [
    "at",
    "event",
    "repo",
    "id",
    "to",
    "goal",
    "brief",
    "priority",
    "waits_on",
    "reason",
    "evidence",
    "controller_id",
    "session_id",
    "charter_revision",
    "from",
    "kind",
    "first_line",
    "resource",
    "amount",
];

/// The keys every line of the dispatch log holds.
const DISPATCH_REQUIRED: [&str; 3] = ["at", "event", "repo"];

/// The keys a line of the decision log may hold: the fields of `conductor.decision.DecisionLogLine`
/// (`spec/domains/decision.yaml`), in its order.
pub const DECISION_KEYS: [&str; 17] = [
    "at",
    "event",
    "repo",
    "id",
    "class",
    "decided_by",
    "question",
    "options",
    "choice",
    "reason",
    "evidence",
    "recorded_by",
    "operator_words",
    "recommendation",
    "blocker",
    "decision_id",
    "escalation_class",
];

/// The keys every line of the decision log holds.
const DECISION_REQUIRED: [&str; 4] = ["at", "event", "repo", "id"];

/// The `event` of a dispatch line, as the specification spells it in the log. Exhaustive, so an
/// event the specification adds fails to compile here rather than being written under a guess.
#[must_use]
pub const fn dispatch_event(event: DispatchLogEvent) -> &'static str {
    match event {
        DispatchLogEvent::Sent => "sent",
        DispatchLogEvent::Started => "started",
        DispatchLogEvent::Blocked => "blocked",
        DispatchLogEvent::Unblocked => "unblocked",
        DispatchLogEvent::Done => "done",
        DispatchLogEvent::Failed => "failed",
        DispatchLogEvent::Cancelled => "cancelled",
        DispatchLogEvent::MessageReceived => "message-received",
        DispatchLogEvent::ResourceGranted => "resource-granted",
        DispatchLogEvent::ResourceRefused => "resource-refused",
        DispatchLogEvent::CharterRevised => "charter-revised",
    }
}

/// The `event` of a decision line, as the specification spells it in the log.
#[must_use]
pub const fn decision_event(event: DecisionLogEvent) -> &'static str {
    match event {
        DecisionLogEvent::Decided => "decided",
        DecisionLogEvent::Requested => "requested",
        DecisionLogEvent::Answered => "answered",
        DecisionLogEvent::Escalated => "escalated",
        DecisionLogEvent::Withdrawn => "withdrawn",
        DecisionLogEvent::Reversed => "reversed",
    }
}

/// Every event of the decision log, for telling a decision line in the dispatch log.
const DECISION_EVENTS: [DecisionLogEvent; 6] = [
    DecisionLogEvent::Decided,
    DecisionLogEvent::Requested,
    DecisionLogEvent::Answered,
    DecisionLogEvent::Escalated,
    DecisionLogEvent::Withdrawn,
    DecisionLogEvent::Reversed,
];

/// The instant a line is written, RFC 3339 in UTC.
///
/// # Errors
///
/// The clock's instant cannot be formatted.
pub fn now() -> Result<Timestamp> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map(Timestamp)
        .context("format the time a records line is written")
}

/// A dispatch line with `at` now, `event` and `repo`, and every other field absent.
///
/// # Errors
///
/// [`now`] fails.
pub fn dispatch_line(event: DispatchLogEvent, repo: &str) -> Result<DispatchLogLine> {
    Ok(DispatchLogLine {
        at: now()?,
        event,
        repo: repo.to_owned(),
        id: None,
        to: None,
        goal: None,
        brief: None,
        priority: None,
        waits_on: None,
        reason: None,
        evidence: None,
        controller_id: None,
        session_id: None,
        charter_revision: None,
        from: None,
        kind: None,
        first_line: None,
        resource: None,
        amount: None,
    })
}

/// A decision line with `at` now, `event`, `repo` and `id`, and every other field absent.
///
/// # Errors
///
/// [`now`] fails.
pub fn decision_line(
    event: DecisionLogEvent,
    repo: Option<&str>,
    id: &str,
) -> Result<DecisionLogLine> {
    Ok(DecisionLogLine {
        at: now()?,
        event,
        repo: repo.map(str::to_owned),
        id: id.to_owned(),
        class: None,
        decided_by: None,
        question: None,
        options: None,
        choice: None,
        reason: None,
        evidence: None,
        recorded_by: None,
        operator_words: None,
        recommendation: None,
        blocker: None,
        decision_id: None,
        escalation_class: None,
    })
}

/// Puts `value` under `key` when it is there.
fn put<T: Into<Value>>(object: &mut Map<String, Value>, key: &str, value: Option<T>) {
    if let Some(value) = value {
        object.insert(key.to_owned(), value.into());
    }
}

/// The JSON object a dispatch line is written as.
#[must_use]
pub fn dispatch_object(line: &DispatchLogLine) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("at".to_owned(), Value::from(line.at.0.clone()));
    object.insert("event".to_owned(), Value::from(dispatch_event(line.event)));
    object.insert("repo".to_owned(), Value::from(line.repo.clone()));
    put(&mut object, "id", line.id.clone());
    put(&mut object, "to", line.to.clone());
    put(&mut object, "goal", line.goal.clone());
    put(&mut object, "brief", line.brief.clone());
    put(&mut object, "priority", line.priority);
    put(&mut object, "waits_on", line.waits_on.clone());
    put(&mut object, "reason", line.reason.clone());
    put(&mut object, "evidence", line.evidence.clone());
    put(&mut object, "controller_id", line.controller_id.clone());
    put(&mut object, "session_id", line.session_id.clone());
    put(&mut object, "charter_revision", line.charter_revision);
    put(&mut object, "from", line.from.clone());
    put(&mut object, "kind", line.kind.clone());
    put(&mut object, "first_line", line.first_line.clone());
    put(&mut object, "resource", line.resource.clone());
    put(&mut object, "amount", line.amount);
    object
}

/// The JSON object a decision line is written as: `repo` always, null for a decision about none.
#[must_use]
pub fn decision_object(line: &DecisionLogLine) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("at".to_owned(), Value::from(line.at.0.clone()));
    object.insert("event".to_owned(), Value::from(decision_event(line.event)));
    object.insert("repo".to_owned(), Value::from(line.repo.clone()));
    object.insert("id".to_owned(), Value::from(line.id.clone()));
    put(&mut object, "class", line.class.map(class_name));
    put(&mut object, "decided_by", line.decided_by.map(decider_name));
    put(&mut object, "question", line.question.clone());
    put(&mut object, "options", line.options.clone());
    put(&mut object, "choice", line.choice.clone());
    put(&mut object, "reason", line.reason.clone());
    put(&mut object, "evidence", line.evidence.clone());
    put(&mut object, "recorded_by", line.recorded_by.clone());
    put(&mut object, "operator_words", line.operator_words.clone());
    put(&mut object, "recommendation", line.recommendation.clone());
    put(&mut object, "blocker", line.blocker.clone());
    put(
        &mut object,
        "decision_id",
        line.decision_id.as_ref().map(|id| id.0.clone()),
    );
    put(
        &mut object,
        "escalation_class",
        line.escalation_class.map(class_name),
    );
    object
}

/// Appends `line` to the records' dispatch log, as the module documentation says.
///
/// # Errors
///
/// The log's directory cannot be made, or the line cannot be appended; each names the file.
pub fn append_dispatch(line: &DispatchLogLine) -> Result<()> {
    append(DISPATCHES, &line.at, &dispatch_object(line))
}

/// Appends `line` to the records' decision log, as the module documentation says.
///
/// # Errors
///
/// The log's directory cannot be made, or the line cannot be appended; each names the file.
pub fn append_decision(line: &DecisionLogLine) -> Result<()> {
    append(DECISIONS, &line.at, &decision_object(line))
}

/// The directory of `log` under the active instance's records, or none where a line is not
/// written: without a config file, a directory `conductor init` did not make.
fn log_dir(log: &str) -> Result<Option<PathBuf>> {
    let active = config::active();
    let dir = Path::new(&active.instance.records).join(log);
    if dir.is_dir() {
        return Ok(Some(dir));
    }
    if !active.from_file {
        return Ok(None);
    }
    fs::create_dir_all(&dir)
        .with_context(|| format!("create the records log {}", dir.display()))?;
    Ok(Some(dir))
}

/// The log file of the month of `at`: `YYYY-MM.jsonl`.
fn month_file(at: &Timestamp) -> Result<String> {
    let at = OffsetDateTime::parse(&at.0, &Rfc3339)
        .with_context(|| format!("the records line's at {:?} is no RFC 3339 instant", at.0))?
        .to_offset(time::UtcOffset::UTC);
    Ok(format!(
        "{:04}-{:02}.jsonl",
        at.year(),
        u8::from(at.month())
    ))
}

/// Appends `object` as one line, in one write, to the month's file of `log`.
fn append(log: &str, at: &Timestamp, object: &Map<String, Value>) -> Result<()> {
    let Some(dir) = log_dir(log)? else {
        return Ok(());
    };
    let file = dir.join(month_file(at)?);
    let mut text = serde_json::to_string(object).context("encode a records line")?;
    text.push('\n');
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .and_then(|mut open| open.write_all(text.as_bytes()))
        .with_context(|| {
            format!(
                "the store kept the record, but its line was not appended to {}",
                file.display()
            )
        })
}

/// `conductor records check`: the view the module documentation describes, over the binding's
/// `local` callable `records-check` (input `conductor.dispatch.CheckRecords`).
///
/// # Errors
///
/// `--format` is neither text nor json, the records directory or a log file cannot be read, the
/// store is there and does not open or read, or standard output cannot be written.
pub fn records_check(state: Option<&Path>, args: &RecordsCheckArgs) -> Result<ExitCode> {
    const COMMAND: &str = "records check";
    let input = CheckRecords {
        format: args.format.clone(),
    };
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is not one of text, json"),
    };
    let records = PathBuf::from(&config::active().instance.records);
    let dispatches = read_log(&records, DISPATCHES).with_context(|| COMMAND.to_owned())?;
    let decisions = read_log(&records, DECISIONS).with_context(|| COMMAND.to_owned())?;
    let mut known: BTreeSet<String> = decisions
        .iter()
        .flat_map(|line| decision_ids(&line.text))
        .collect();
    known.extend(stored_decisions(state).with_context(|| COMMAND.to_owned())?);
    let findings = check(&dispatches, &decisions, &known);
    let legacy_lines = legacy(&dispatches, &DISPATCH_KEYS, &DISPATCH_REQUIRED)
        + legacy(&decisions, &DECISION_KEYS, &DECISION_REQUIRED);
    if legacy_lines > 0 {
        let _ = writeln!(
            io::stderr().lock(),
            "{COMMAND}: {legacy_lines} line(s) written before the first a command wrote, judged only on the decisions they cite"
        );
    }

    let mut out = io::stdout().lock();
    let written = if json {
        let rows: Vec<Value> = findings
            .iter()
            .map(|finding| {
                json!({
                    "file": finding.file,
                    "line": finding.line,
                    "finding": finding_name(finding.finding),
                    "detail": finding.detail,
                })
            })
            .collect();
        writeln!(out, "{}", Value::Array(rows))
    } else {
        findings.iter().try_for_each(|finding| {
            writeln!(
                out,
                "{}:{}: {}: {}",
                finding.file,
                finding.line,
                finding_name(finding.finding),
                finding.detail
            )
        })
    };
    written.with_context(|| format!("{COMMAND}: write to standard output"))?;
    if findings.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    let mut err = io::stderr().lock();
    let _ = writeln!(
        err,
        "{COMMAND}: {} finding(s) in {}",
        findings.len(),
        records.display()
    );
    Ok(ExitCode::FAILURE)
}

/// The finding's name as the specification spells it. Exhaustive, so a finding the
/// specification adds fails to compile here rather than rendering under a guessed name.
const fn finding_name(finding: RecordsFindingKind) -> &'static str {
    match finding {
        RecordsFindingKind::UnknownDecision => "UnknownDecision",
        RecordsFindingKind::UnknownKeySet => "UnknownKeySet",
        RecordsFindingKind::DecisionInDispatchLog => "DecisionInDispatchLog",
    }
}

/// One line of a log: its file under the records directory, its 1-based number and its text.
struct Line {
    file: String,
    number: i64,
    text: String,
}

/// Every non-blank line of every `*.jsonl` file of `log`, files in name order. A log that is not
/// there holds no line.
fn read_log(records: &Path, log: &str) -> Result<Vec<Line>> {
    let dir = records.join(log);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).with_context(|| format!("read {}", dir.display())),
    };
    let mut files: Vec<PathBuf> = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<_>>()
        .with_context(|| format!("read {}", dir.display()))?;
    files.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "jsonl")
    });
    files.sort();
    let mut lines = Vec::new();
    for path in files {
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (index, text) in text.lines().enumerate() {
            if text.trim().is_empty() {
                continue;
            }
            lines.push(Line {
                file: format!("{log}/{name}"),
                number: i64::try_from(index + 1).unwrap_or(i64::MAX),
                text: text.to_owned(),
            });
        }
    }
    Ok(lines)
}

/// The ids of the decisions the store holds, or none when there is no store: a view creates
/// nothing.
fn stored_decisions(state: Option<&Path>) -> Result<Vec<String>> {
    let dir = crate::state::dir(state)?;
    if !dir.join("tree").exists() {
        return Ok(Vec::new());
    }
    let store = crate::state::open_existing(state)?;
    let held = DecisionStorage::list(&store);
    store
        .check()
        .map_err(|failure| anyhow!("read the store's decisions: {failure}"))?;
    Ok(held
        .into_iter()
        .map(|decision| decision.data.decision_id.0)
        .collect())
}

/// Every decision id `DEC-…` in `text`: `DEC-` not inside a longer word, then letters, digits and
/// `-`, the first a digit, without a trailing `-`.
fn decision_ids(text: &str) -> Vec<String> {
    const PREFIX: &str = "DEC-";
    let mut ids = Vec::new();
    for (at, _) in text.match_indices(PREFIX) {
        if text[..at]
            .chars()
            .next_back()
            .is_some_and(|before| before.is_ascii_alphanumeric() || before == '-')
        {
            continue;
        }
        let rest = &text[at + PREFIX.len()..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(rest.len());
        let body = rest[..end].trim_end_matches('-');
        if body.starts_with(|c: char| c.is_ascii_digit()) {
            ids.push(format!("{PREFIX}{body}"));
        }
    }
    ids
}

/// How many lines of `lines` come before the first one a command wrote, one whose key set is the
/// log's (`keys`, `required`): the hand-written lines from before the commands wrote the log,
/// which the key-set and misfiled-decision checks do not judge. Every line, when none is a
/// command's yet.
fn legacy(lines: &[Line], keys: &[&str], required: &[&str]) -> usize {
    lines
        .iter()
        .position(|line| {
            let object = serde_json::from_str::<Value>(&line.text).ok();
            object.as_ref().is_some_and(Value::is_object)
                && key_set(object.as_ref(), keys, required).is_none()
        })
        .unwrap_or(lines.len())
}

/// The findings over both logs, in file and line order: the dispatch log's, then the decision
/// log's. A line before the first a command wrote in its log ([`legacy`]) is judged only on the
/// decision ids it cites.
fn check(dispatches: &[Line], decisions: &[Line], known: &BTreeSet<String>) -> Vec<RecordsFinding> {
    let old_dispatches = legacy(dispatches, &DISPATCH_KEYS, &DISPATCH_REQUIRED);
    let old_decisions = legacy(decisions, &DECISION_KEYS, &DECISION_REQUIRED);
    let mut findings = Vec::new();
    let finding = |line: &Line, kind: RecordsFindingKind, detail: String| RecordsFinding {
        file: line.file.clone(),
        line: line.number,
        finding: kind,
        detail,
    };
    for (index, line) in dispatches.iter().enumerate() {
        let object = serde_json::from_str::<Value>(&line.text).ok();
        let judged = index >= old_dispatches;
        if judged
            && let Some(Value::Object(object)) = &object
            && let Some(detail) = decision_in_dispatch_log(object)
        {
            findings.push(finding(
                line,
                RecordsFindingKind::DecisionInDispatchLog,
                detail,
            ));
            continue;
        }
        if judged && let Some(detail) = key_set(object.as_ref(), &DISPATCH_KEYS, &DISPATCH_REQUIRED)
        {
            findings.push(finding(line, RecordsFindingKind::UnknownKeySet, detail));
        }
        let mut cited = decision_ids(&line.text);
        cited.sort();
        cited.dedup();
        for id in cited {
            if !known.contains(&id) {
                findings.push(finding(
                    line,
                    RecordsFindingKind::UnknownDecision,
                    format!(
                        "cites {id}, which no line of {DECISIONS}/ and no store decision holds"
                    ),
                ));
            }
        }
    }
    for line in &decisions[old_decisions..] {
        let object = serde_json::from_str::<Value>(&line.text).ok();
        if let Some(detail) = key_set(object.as_ref(), &DECISION_KEYS, &DECISION_REQUIRED) {
            findings.push(finding(line, RecordsFindingKind::UnknownKeySet, detail));
        }
    }
    findings
}

/// Why a dispatch line is a decision's: its `event` is a decision log's, or it has no `event`
/// and its `id` is a decision id.
fn decision_in_dispatch_log(object: &Map<String, Value>) -> Option<String> {
    match object.get("event") {
        Some(Value::String(event)) => DECISION_EVENTS
            .iter()
            .any(|known| decision_event(*known) == event.as_str())
            .then(|| {
                format!("the event {event:?} is the decision log's; it belongs in {DECISIONS}/")
            }),
        None => object
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| {
                decision_ids(id)
                    .first()
                    .is_some_and(|first| first.as_str() == *id)
            })
            .map(|id| format!("decision {id} has no event; it belongs in {DECISIONS}/")),
        Some(_) => None,
    }
}

/// Why a line's keys are not its log's: it is no JSON object, misses a key of `required`, or holds
/// a key `keys` does not declare.
fn key_set(object: Option<&Value>, keys: &[&str], required: &[&str]) -> Option<String> {
    let Some(Value::Object(object)) = object else {
        return Some("the line is no JSON object".to_owned());
    };
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|key| !object.contains_key(*key))
        .collect();
    let unknown: Vec<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|key| !keys.contains(key))
        .collect();
    match (missing.is_empty(), unknown.is_empty()) {
        (true, true) => None,
        (false, true) => Some(format!("misses {}", missing.join(", "))),
        (true, false) => Some(format!("holds undeclared {}", unknown.join(", "))),
        (false, false) => Some(format!(
            "misses {}, and holds undeclared {}",
            missing.join(", "),
            unknown.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_ids_are_read_out_of_any_text() {
        assert_eq!(
            decision_ids("waits on DEC-1-02, and (DEC-3) but not XDEC-4 or DEC-x"),
            vec!["DEC-1-02".to_owned(), "DEC-3".to_owned()]
        );
    }

    #[test]
    fn the_key_set_names_what_is_missing_and_what_is_undeclared() {
        let line = json!({"at": "t", "event": "started", "repository": "alpha"});
        assert_eq!(
            key_set(Some(&line), &DISPATCH_KEYS, &DISPATCH_REQUIRED).as_deref(),
            Some("misses repo, and holds undeclared repository")
        );
        let line = json!({"at": "t", "event": "started", "repo": "alpha", "id": "x"});
        assert_eq!(
            key_set(Some(&line), &DISPATCH_KEYS, &DISPATCH_REQUIRED),
            None
        );
    }
}
