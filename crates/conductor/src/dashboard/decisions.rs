//! `/api/decisions`, what the Decisions page reads (`story:decisions-page`).
//!
//! It answers `{open, history, errors}`:
//! - `open`: every decision request escalated as class O or H and not answered or withdrawn, the
//!   one raised first first, then by id, as `conductor status` orders them
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
//! The answer is computed once per write ([`Cache`]): a write to the store is a new commit file
//! under its tree, and a write to the log changes a file's size or time, so the server keeps the
//! last answer with what it read it from ([`Fingerprint`]) and serves it from memory while that
//! is unchanged. Each answer carries an `ETag`, the digest of its body; a request whose
//! `If-None-Match` names it is answered `304` with no body.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash as _, Hasher as _};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use anyhow::Result;
use conductor_model::behaviour::DecisionRequestStorage;
use conductor_model::decision::DecisionRequestData;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::store::Store;

/// The path of the Decisions page's data.
pub const DECISIONS_PATH: &str = "/api/decisions";

/// What an answer was read from: the store's commit files under `state/tree/` (their count and the
/// newest one's time) and each decision log file (its name, size and time).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fingerprint {
    commits: usize,
    newest: Option<SystemTime>,
    log: Vec<(String, u64, Option<SystemTime>)>,
}

impl Fingerprint {
    /// The fingerprint of the store under `state`, if any, and of the decision log under
    /// `records`.
    ///
    /// # Errors
    ///
    /// A directory that is there does not read.
    pub fn of(state: Option<&Path>, records: &Path) -> io::Result<Self> {
        let mut print = Self::default();
        if let Some(state) = state {
            let tenants = state.join("tree").join("tenants");
            match fs::read_dir(&tenants) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry?;
                        if entry.file_type()?.is_dir() {
                            print.walk(&entry.path().join("groups"))?;
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        match fs::read_dir(records.join(crate::records::DECISIONS)) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry?;
                    let metadata = entry.metadata()?;
                    print.log.push((
                        entry.file_name().to_string_lossy().into_owned(),
                        metadata.len(),
                        metadata.modified().ok(),
                    ));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        print.log.sort();
        Ok(print)
    }

    /// Counts the commit files under `directory`, as the store's own mark counts them, and keeps
    /// the newest one's time.
    fn walk(&mut self, directory: &Path) -> io::Result<()> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        for entry in entries {
            let entry = entry?;
            let kind = entry.file_type()?;
            let name = entry.file_name();
            if kind.is_dir() {
                self.walk(&entry.path())?;
            } else if kind.is_file()
                && !name.to_string_lossy().starts_with('.')
                && Path::new(&name)
                    .extension()
                    .is_some_and(|extension| extension == "json")
            {
                self.commits += 1;
                let modified = entry.metadata()?.modified().ok();
                self.newest = self.newest.max(modified);
            }
        }
        Ok(())
    }
}

/// One computed answer: its body and its `ETag`.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The JSON body, pretty-printed, ending in a newline.
    pub body: Arc<String>,
    /// The `ETag`, quoted.
    pub etag: String,
}

/// The last answer and what it was read from.
#[derive(Debug, Default)]
pub struct Cache {
    held: Mutex<Option<(Fingerprint, Answer)>>,
}

impl Cache {
    /// The answer for `fingerprint`: the one held when it was computed for the same fingerprint,
    /// else `compute`'s, which is then held.
    ///
    /// # Errors
    ///
    /// `compute` failed; nothing is held then.
    pub fn answer(
        &self,
        fingerprint: Fingerprint,
        compute: impl FnOnce() -> Result<Value>,
    ) -> Result<Answer> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((print, answer)) = held.as_ref()
            && *print == fingerprint
        {
            return Ok(answer.clone());
        }
        let mut body = serde_json::to_string_pretty(&compute()?).unwrap_or_default();
        body.push('\n');
        let mut hasher = DefaultHasher::new();
        body.hash(&mut hasher);
        let answer = Answer {
            etag: format!("\"d-{:016x}\"", hasher.finish()),
            body: Arc::new(body),
        };
        *held = Some((fingerprint, answer.clone()));
        Ok(answer)
    }
}

/// Whether the request head `head` carries an `If-None-Match` that names `etag` (or `*`).
#[must_use]
pub fn not_modified(head: &str, etag: &str) -> bool {
    head.split("\r\n")
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.trim().eq_ignore_ascii_case("if-none-match"))
        .flat_map(|(_, value)| value.split(','))
        .map(|tag| tag.trim().trim_start_matches("W/"))
        .any(|tag| tag == etag || tag == "*")
}

/// The answer over `store` (none: no open item) and the decision log's lines `log`, in file
/// order, with the log's read problems `errors`.
#[must_use]
pub fn answer(store: Option<&Store>, log: &[Value], errors: Vec<String>) -> Value {
    let open: Vec<Value> = store.map(open_items).unwrap_or_default();
    json!({"open": open, "history": history(log), "errors": errors})
}

/// The open class-O and class-H items of `store`, in the order of [`crate::status::unanswered`].
fn open_items(store: &Store) -> Vec<Value> {
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
