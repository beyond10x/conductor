//! `/api/decisions`, what the Decisions page reads (`story:decisions-page`).
//!
//! It answers `{open, history, errors}`:
//! - `open`: every decision request escalated as class O or H and not answered or withdrawn, in
//!   the store or in the decision log ([`answered`], `story:sessions-page-coherent`), the one
//!   raised first first, then by id, as `conductor status` orders them
//!   ([`crate::status::unanswered`]). Each row is `{request_id, class, repository, question, kind,
//!   options, recommended, default, deadline, raised_at, reason, options_text,
//!   recommendation_text}`: `kind` is `yes-no`, `one-of` or `several-of`; `options` the lettered
//!   options, each `{letter, label, cost}`; `recommended` and `default` lists of letters;
//!   `deadline` RFC 3339 or null; `reason` why it was escalated; `options_text` and
//!   `recommendation_text` the request's own words, as the controller raised it. A request
//!   escalated before the escalation recorded what the operator chooses from reads as `one-of`
//!   with no options and no letters, and the page shows its words alone.
//! - `history`: every line of the records' decision log (`decisions/*.jsonl`), newest first, each
//!   `{at, event, id, repo, class, decided_by, question, choice, reason}`, null where the line has
//!   none. `at` is the line's `at`, else its `decided_at`, else its `raised_at` (a line written by
//!   hand before the commands wrote the log); `repo` its `repo`, else its `repository`; `class` its
//!   `class`, else its `escalation_class`. A line whose instant does not read is listed last.
//! - `errors`: each decision log file that could not be read, or held a line that is no JSON
//!   object.
//!
//! The server computes the answer once per write to the store or to the decision log and serves
//! it from memory with an `ETag`, as it serves every answer that reads the store
//! (`story:dashboard-fast`, `super::cache`); a request whose `If-None-Match` names it is answered
//! `304` with no body.

use conductor_model::behaviour::DecisionRequestStorage;
use conductor_model::decision::DecisionRequestData;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::store::Store;

/// The path of the Decisions page's data.
pub const DECISIONS_PATH: &str = "/api/decisions";

/// The answer over `store` (none: no open item) and the decision log's lines `log`, in file
/// order, with the log's read problems `errors`.
#[must_use]
pub fn answer(store: Option<&Store>, log: &[Value], errors: Vec<String>) -> Value {
    let open: Vec<Value> = store
        .map(open_items)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| {
            row["request_id"]
                .as_str()
                .is_some_and(|id| !answered(log, id))
        })
        .collect();
    json!({"open": open, "history": history(log), "errors": errors})
}

/// Whether the decision log `log` answered the item `id` (`story:sessions-page-coherent`): a line
/// that closed it (`event` `answered` or `withdrawn`, its `id` the item's), a line of its own id
/// with a choice, or an operator's decision (class O or H, with a choice) whose `request` or
/// `rests_on` names it as a whole word. An answer recorded in the log counts whether or not the
/// store's request was ever closed; a class-C decision that merely names the item answers nothing.
#[must_use]
pub fn answered(log: &[Value], id: &str) -> bool {
    let chose = |line: &Value| {
        line["choice"]
            .as_str()
            .is_some_and(|choice| !choice.trim().is_empty())
    };
    log.iter().any(|line| {
        let own = line["id"].as_str() == Some(id);
        if own && (matches!(line["event"].as_str(), Some("answered" | "withdrawn")) || chose(line))
        {
            return true;
        }
        let operator = matches!(
            line["class"]
                .as_str()
                .or_else(|| line["escalation_class"].as_str()),
            Some("O" | "H")
        );
        if own || !operator || !chose(line) {
            return false;
        }
        let mut cited = Vec::new();
        match &line["request"] {
            Value::String(text) => cited.push(text.as_str()),
            Value::Array(items) => cited.extend(items.iter().filter_map(Value::as_str)),
            _ => {}
        }
        cited.extend(line["rests_on"].as_str());
        cited.into_iter().any(|text| names(text, id))
    })
}

/// Whether `text` names `id` as a whole word: not inside a longer id.
fn names(text: &str, id: &str) -> bool {
    let part = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_');
    text.match_indices(id).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(part)
            && !text[at + id.len()..].chars().next().is_some_and(part)
    })
}

/// The open class-O and class-H items of `store`, in the order of [`crate::status::unanswered`].
pub(crate) fn open_items(store: &Store) -> Vec<Value> {
    let held: Vec<DecisionRequestData> = DecisionRequestStorage::list(store)
        .into_iter()
        .map(|held| held.data)
        .collect();
    crate::status::unanswered(store)
        .into_iter()
        .filter_map(|item| {
            let data = held
                .iter()
                .find(|data| data.request_id.0 == item.request_id)?;
            Some(open_row(item.class, data))
        })
        .collect()
}

fn open_row(class: &str, data: &DecisionRequestData) -> Value {
    let letters = |given: &Option<String>| -> Vec<String> {
        given
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|letter| !letter.is_empty())
            .map(str::to_owned)
            .collect()
    };
    json!({
        "request_id": data.request_id.0,
        "class": class,
        "repository": data.repository.0,
        "question": data.question,
        "kind": data.kind.map_or("one-of", crate::decide::kind_name),
        "options": data
            .lettered_options
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(crate::decide::option_value)
            .collect::<Vec<Value>>(),
        "recommended": letters(&data.recommended_letters),
        "default": letters(&data.default_letters),
        "deadline": data.deadline.as_ref().map(|deadline| &deadline.0),
        "raised_at": data.raised_at.0,
        "reason": data.escalation_reason,
        "options_text": data.options,
        "recommendation_text": data.recommendation,
    })
}

/// Every line of the decision log, newest first; a line whose instant does not read last, the
/// later line in the log first among equals.
fn history(log: &[Value]) -> Vec<Value> {
    let first = |line: &Value, keys: &[&str]| -> Value {
        keys.iter()
            .map(|key| &line[*key])
            .find(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null)
    };
    let mut rows: Vec<(Option<OffsetDateTime>, Value)> = log
        .iter()
        .rev()
        .map(|line| {
            let at = first(line, &["at", "decided_at", "raised_at"]);
            let instant = at
                .as_str()
                .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok());
            let row = json!({
                "at": at,
                "event": first(line, &["event"]),
                "id": first(line, &["id"]),
                "repo": first(line, &["repo", "repository"]),
                "class": first(line, &["class", "escalation_class"]),
                "decided_by": first(line, &["decided_by"]),
                "question": first(line, &["question"]),
                "choice": first(line, &["choice"]),
                "reason": first(line, &["reason"]),
            });
            (instant, row)
        })
        .collect();
    rows.sort_by(|(left, _), (right, _)| match (left, right) {
        (Some(left), Some(right)) => right.cmp(left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    rows.into_iter().map(|(_, row)| row).collect()
}
