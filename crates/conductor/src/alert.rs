//! The one route by which the loop and the daemon tell the operator of a condition they cannot act
//! on (`story:loop-alerts-when-it-cannot-act`, issue #207).
//!
//! [`raise`] writes one `alert` line (`conductor.dispatch.DispatchLogEvent::Alert`) with the
//! condition, the repository and the reason, and sends `<instance>: alert` with that reason to the
//! operator's desktop notifier ([`notify`], `notify-send`, which `conductor doctor` names on its
//! notifier row). It does so once per condition and subject: the pairs alerted are kept in the
//! seen file ([`SEEN`]) of the caller's directory (`<state>/run/` for the loop, the daemon's own
//! directory for the daemon), and [`clear`] forgets a subject once its condition no longer holds,
//! so a second occurrence alerts again. A notifier that is missing or fails is a journal line on
//! standard error, never an error: the alert is recorded all the same.
//!
//! No config key chooses the route, and no condition the loop acts on itself alerts.

use std::io::{self, Write as _};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use conductor_model::dispatch::{DispatchLogEvent, DispatchLogLine};
use serde_json::{Value, json};

/// The seen file: one JSON object, each condition with the subjects alerted while it holds.
pub const SEEN: &str = "alerts.seen";

/// The condition of a unit of work whose start failed at the launch
/// [`crate::dispatch::FAILED_STARTS_MOST`] times, which the cycle tries no more; its subject is
/// the dispatch.
pub const UNIT_LEFT_TO_OPERATOR: &str = "unit-left-to-operator";

/// How long the desktop notifier may run.
const NOTIFY_BOUND: Duration = Duration::from_secs(10);

/// An `alert` line of `condition` about `repo` (the instance's name for a condition of the
/// instance), its subject the dispatch `id` when there is one, saying `reason`.
///
/// # Errors
///
/// The clock cannot be formatted.
pub fn line(
    condition: &str,
    repo: &str,
    id: Option<&str>,
    reason: &str,
) -> Result<DispatchLogLine> {
    let mut line = crate::records::dispatch_line(DispatchLogEvent::Alert, repo)?;
    line.condition = Some(condition.to_owned());
    line.id = id.map(str::to_owned);
    line.reason = Some(crate::run::one_line(reason));
    Ok(line)
}

/// The subject of an alert line: its `id`, else its `repo`.
fn subject(line: &DispatchLogLine) -> &str {
    line.id.as_deref().unwrap_or(&line.repo)
}

/// Raises the alert `line` (made by [`line`]) of the instance `instance` once ([`mark`]), and
/// sends it to the notifier when it was raised. Answers whether it was.
///
/// # Errors
///
/// As [`mark`].
pub fn raise(
    dir: &Path,
    instance: &str,
    line: &DispatchLogLine,
    record: impl FnOnce(&DispatchLogLine) -> Result<()>,
) -> Result<bool> {
    if !mark(dir, line, record)? {
        return Ok(false);
    }
    let text = format!(
        "{} {}{}: {}",
        line.condition.as_deref().unwrap_or_default(),
        line.repo,
        line.id
            .as_deref()
            .map(|id| format!(" {id}"))
            .unwrap_or_default(),
        line.reason.as_deref().unwrap_or_default()
    );
    if let Err(error) = notify(instance, "alert", &text) {
        let _ = writeln!(
            io::stderr().lock(),
            "conductor alert: notify-send: {error:#}"
        );
    }
    Ok(true)
}

/// Unless the condition and subject of `line` are in the seen file of `dir`: records it with
/// `record` and keeps the pair in the seen file. Answers whether it recorded it.
///
/// # Errors
///
/// `record` fails (the pair is then not kept, so the next call tries again), or the seen file
/// cannot be written.
pub fn mark(
    dir: &Path,
    line: &DispatchLogLine,
    record: impl FnOnce(&DispatchLogLine) -> Result<()>,
) -> Result<bool> {
    let condition = line.condition.as_deref().unwrap_or_default();
    let subject = subject(line);
    let mut seen = crate::run::read_json(dir, SEEN, json!({}));
    if held(&seen, condition, subject) {
        return Ok(false);
    }
    record(line)?;
    let subjects = seen
        .as_object_mut()
        .map(|conditions| {
            conditions
                .entry(condition.to_owned())
                .or_insert_with(|| json!([]))
        })
        .and_then(Value::as_array_mut);
    match subjects {
        Some(subjects) => subjects.push(json!(subject)),
        None => match seen.as_object_mut() {
            Some(conditions) => {
                conditions.insert(condition.to_owned(), json!([subject]));
            }
            None => seen = json!({condition: [subject]}),
        },
    }
    crate::run::write_json(dir, SEEN, &seen)?;
    Ok(true)
}

/// Forgets each subject of `condition` in the seen file of `dir` for which `holds` answers
/// false: its condition cleared, so a second occurrence alerts again.
///
/// # Errors
///
/// The seen file cannot be written.
pub fn clear(dir: &Path, condition: &str, holds: impl Fn(&str) -> bool) -> Result<()> {
    let mut seen = crate::run::read_json(dir, SEEN, json!({}));
    let Some(conditions) = seen.as_object_mut() else {
        return Ok(());
    };
    let Some(subjects) = conditions.get_mut(condition).and_then(Value::as_array_mut) else {
        return Ok(());
    };
    let before = subjects.len();
    subjects.retain(|subject| subject.as_str().is_some_and(&holds));
    if subjects.len() == before {
        return Ok(());
    }
    if subjects.is_empty() {
        conditions.remove(condition);
    }
    let seen = Value::Object(std::mem::take(conditions));
    crate::run::write_json(dir, SEEN, &seen)
}

/// Whether the seen file `seen` holds `subject` under `condition`.
fn held(seen: &Value, condition: &str, subject: &str) -> bool {
    seen.get(condition)
        .and_then(Value::as_array)
        .is_some_and(|subjects| subjects.iter().any(|held| held == subject))
}

/// A critical desktop notification of `text`, titled `<instance>: <what>`, through `notify-send`.
///
/// # Errors
///
/// The notifier does not start or does not end within its bound.
pub fn notify(instance: &str, what: &str, text: &str) -> Result<()> {
    let title = format!("{instance}: {what}");
    crate::collect::run(
        crate::instance::command("notify-send").args(["-u", "critical", &title, text]),
        NOTIFY_BOUND,
    )
    .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen_of(dir: &Path) -> Value {
        crate::run::read_json(dir, SEEN, json!({}))
    }

    /// A directory of its own for `name`, empty.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("conductor-alert-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory");
        dir
    }

    #[test]
    fn a_condition_and_subject_is_raised_once_until_it_clears() {
        let dir = scratch("once");
        let mut recorded = Vec::new();
        let first = line(UNIT_LEFT_TO_OPERATOR, "alpha", Some("DSP-1"), "left").expect("a line");
        for _ in 0..3 {
            mark(&dir, &first, |line| {
                recorded.push(line.clone());
                Ok(())
            })
            .expect("raise");
        }
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].event, DispatchLogEvent::Alert);
        let other = line(UNIT_LEFT_TO_OPERATOR, "alpha", Some("DSP-2"), "left").expect("a line");
        assert!(
            mark(&dir, &other, |_| Ok(())).expect("raise"),
            "another subject alerts"
        );
        clear(&dir, UNIT_LEFT_TO_OPERATOR, |subject| subject == "DSP-2").expect("clear");
        assert_eq!(seen_of(&dir), json!({UNIT_LEFT_TO_OPERATOR: ["DSP-2"]}));
        assert!(
            mark(&dir, &first, |_| Ok(())).expect("raise"),
            "a cleared subject alerts again"
        );
        clear(&dir, UNIT_LEFT_TO_OPERATOR, |_| false).expect("clear");
        assert_eq!(seen_of(&dir), json!({}));
    }

    #[test]
    fn a_refused_record_keeps_nothing() {
        let dir = scratch("refused");
        let alert = line(UNIT_LEFT_TO_OPERATOR, "alpha", None, "left").expect("a line");
        assert!(mark(&dir, &alert, |_| anyhow::bail!("refused")).is_err());
        assert_eq!(seen_of(&dir), json!({}));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
