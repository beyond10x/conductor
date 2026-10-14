//! The `usage` collector and the pool forecast (`story:usage-probe`).
//!
//! **The probe.** The instance's `usage.probe` is a command, run with `sh -c` within [`BOUND`],
//! that prints the harness accounts as one JSON object. Conductor names no tool: any command that
//! prints this shape serves.
//!
//! ```json
//! {"accounts": [
//!   {"id": "account-a", "active": true, "health": "ready",
//!    "usage": {"windows": [{"name": "seven_day", "used_percent": 40.0,
//!                           "resets_at": "2026-10-16T11:00:00Z"}]}}
//! ]}
//! ```
//!
//! `id` names the account and is all conductor keeps of it: every other key of an account (an
//! e-mail address, a name, a plan) is never read. `active` is whether it is the account in use
//! (absent: not); `health` how the probe judges it, `exhausted` for one with nothing left
//! (absent: `unknown`); `usage` may be `null` or leave out `windows`, and then the account is not
//! recorded. A window's `used_percent` is a number, kept as a whole percent, rounded and held to
//! 0 through 100; `resets_at` is RFC 3339 or `null`. An id, a health or a window's name is a word
//! of letters, digits, `.`, `_`, `:` and `-`, at most [`WORD`] long; a probe that prints another
//! is refused whole, and the value is never echoed, so an address printed where an id belongs is
//! never kept.
//!
//! **The collector.** [`collect`] runs the probe of the instance the process runs, when it names
//! one, and records one `RecordUsage` per window of each account, every one with the instant the
//! probe answered. A probe that does not start, runs past its bound, exits unsuccessfully or
//! prints another shape records nothing of usage: it is one member failure, member and collector
//! `usage`, whose line names the exit status or the shape's problem and never what the probe
//! printed. The snapshot goes on.
//!
//! **The forecast** ([`Forecast::of`]). The readings are those of the newest complete snapshot
//! holding any and of the one before it. Per window name, the pool's headroom is the percent left
//! (100 less the percent used) summed over the newest reading's accounts whose health is not
//! `exhausted`; the burn rate is what the accounts used between the two readings (each rise of
//! one account's percent, summed; a fall is a reset and counts nothing) per hour between them.
//! It is summed over every account rather than the active one alone, so a rotation between the
//! readings does not hide what the account rotated away from used. A window runs out at the
//! newest reading plus its headroom over its burn rate, at once when nothing is left; the pool
//! runs out when its first window does. Without an earlier reading, or with nothing used, there is
//! no forecast. The next reset is the earliest `resets_at` after the newest reading of a window
//! any account has used.
//!
//! [`Forecast::line`] is the line the watch and `conductor status` print, `usage: pool runs out
//! ~<hour>, next reset <minute>`, the hour as `YYYY-MM-DD HH:00 UTC` (the forecast's hour) and
//! the minute as `YYYY-MM-DD HH:MM UTC`; `unknown` for either one that is not known.
//! [`Forecast::hour`] is what the watch keeps to print it once per change of that hour.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use conductor_model::behaviour::{SnapshotStorage, UsageObservationStorage};
use conductor_model::observation::{
    RecordMemberFailure, RecordUsage, RepositoryName, SnapshotState, UsageObservationData,
};
use conductor_model::primitives::Timestamp;
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::collect::{self, BOUND};
use crate::config;
use crate::snapshot::Recorder;
use crate::store::Store;

/// The collector's name, and the member a probe that does not answer is recorded under.
pub const COLLECTOR: &str = "usage";

/// The health of an account with nothing left: its windows add nothing to the pool.
pub const EXHAUSTED: &str = "exhausted";

/// The health of an account the probe gives none.
const UNKNOWN: &str = "unknown";

/// The longest id, health or window name kept.
pub const WORD: usize = 128;

/// What the probe prints: only these keys are read.
#[derive(Deserialize)]
struct Printed {
    accounts: Vec<PrintedAccount>,
}

#[derive(Deserialize)]
struct PrintedAccount {
    id: String,
    #[serde(default)]
    active: Option<bool>,
    #[serde(default)]
    health: Option<String>,
    #[serde(default)]
    usage: Option<PrintedUsage>,
}

#[derive(Deserialize)]
struct PrintedUsage {
    #[serde(default)]
    windows: Vec<PrintedWindow>,
}

#[derive(Deserialize)]
struct PrintedWindow {
    name: String,
    used_percent: f64,
    #[serde(default)]
    resets_at: Option<String>,
}

/// One window of one account, as the probe printed it and as it is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The account's id.
    pub account: String,
    /// Whether it is the account in use.
    pub active: bool,
    /// Its health, `exhausted` for one with nothing left.
    pub health: String,
    /// The window's name.
    pub window: String,
    /// The whole percent of the window used, 0 through 100.
    pub used_percent: i64,
    /// When the window resets, RFC 3339.
    pub resets_at: Option<String>,
}

/// The windows of every account `text` holds, in its order, an account's in its order.
///
/// # Errors
///
/// `text` is not the probe's shape, or holds an id, a health or a window name that is no word,
/// a percent that is no number, or a reset that is no RFC 3339 instant. The error names the place
/// (`accounts[1].id`), never the value.
pub fn parse(text: &str) -> Result<Vec<Reading>> {
    let printed: Printed = serde_json::from_str(text).map_err(|error| {
        anyhow::anyhow!(
            "the probe printed no JSON object of accounts (line {}, column {})",
            error.line(),
            error.column()
        )
    })?;
    let mut readings = Vec::new();
    for (index, account) in printed.accounts.into_iter().enumerate() {
        let at = |field: &str| format!("accounts[{index}].{field}");
        let id = word(account.id, &at("id"))?;
        let health = match account.health {
            Some(health) => word(health, &at("health"))?,
            None => UNKNOWN.to_owned(),
        };
        let active = account.active.unwrap_or(false);
        let windows = account.usage.map(|usage| usage.windows).unwrap_or_default();
        for (number, window) in windows.into_iter().enumerate() {
            let at = |field: &str| format!("accounts[{index}].usage.windows[{number}].{field}");
            let name = word(window.name, &at("name"))?;
            if !window.used_percent.is_finite() {
                bail!("{} is not a percent", at("used_percent"));
            }
            #[allow(clippy::cast_possible_truncation)]
            let used_percent = window.used_percent.round().clamp(0.0, 100.0) as i64;
            let resets_at = match window.resets_at {
                Some(text) => Some(instant(&text, &at("resets_at"))?),
                None => None,
            };
            readings.push(Reading {
                account: id.clone(),
                active,
                health: health.clone(),
                window: name,
                used_percent,
                resets_at,
            });
        }
    }
    Ok(readings)
}

/// `value` when it is a word ([`WORD`] characters at most, of letters, digits, `.`, `_`, `:`
/// and `-`); otherwise an error naming `at` alone.
fn word(value: String, at: &str) -> Result<String> {
    let valid = !value.is_empty()
        && value.chars().count() <= WORD
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'));
    if valid {
        Ok(value)
    } else {
        bail!("{at} is not a word of letters, digits, '.', '_', ':' and '-'")
    }
}

/// `text` as the RFC 3339 instant it names, written back as RFC 3339.
fn instant(text: &str, at: &str) -> Result<String> {
    OffsetDateTime::parse(text, &Rfc3339)
        .ok()
        .and_then(|instant| instant.format(&Rfc3339).ok())
        .with_context(|| format!("{at} is not an RFC 3339 instant"))
}

/// Runs the `usage.probe` of the instance the process runs, when it names one, and records what
/// it printed ([`collect_with`]) at the instant it answered.
///
/// # Errors
///
/// What [`collect_with`] answers.
pub fn collect(record: &mut Recorder<'_>) -> Result<()> {
    let active = config::active();
    let Some(probe) = active.instance.usage.probe.clone() else {
        return Ok(());
    };
    collect_with(&probe, None, record)
}

/// Runs `probe` with `sh -c` and records one `RecordUsage` per window it printed, each read at
/// `read_at`, else at the instant the probe answered. A probe that does not answer, or prints
/// another shape, is recorded as one member failure ([`COLLECTOR`]) and nothing else.
///
/// # Errors
///
/// The collector's budget is spent, or an observation is refused.
pub fn collect_with(
    probe: &str,
    read_at: Option<OffsetDateTime>,
    record: &mut Recorder<'_>,
) -> Result<()> {
    let printed = collect::run(Command::new("sh").args(["-c", probe]), BOUND);
    let readings = match printed {
        Err(error) if collect::is_over_budget(&error) => return Err(error),
        Err(error) => Err(format!("the usage probe did not answer: {error}")),
        Ok(output) if !output.status.success() => Err(format!(
            "the usage probe exited {}",
            output
                .status
                .code()
                .map_or_else(|| "on a signal".to_owned(), |code| code.to_string())
        )),
        Ok(output) => match std::str::from_utf8(&output.stdout) {
            Err(_) => Err("the usage probe printed no UTF-8 text".to_owned()),
            Ok(text) => parse(text).map_err(|error| format!("{error}")),
        },
    };
    let snapshot = record.snapshot().clone();
    let readings = match readings {
        Ok(readings) => readings,
        Err(line) => {
            record.member_failure(RecordMemberFailure {
                snapshot_id: snapshot,
                member: RepositoryName(COLLECTOR.to_owned()),
                collector: COLLECTOR.to_owned(),
                error_line: line,
            })?;
            return Ok(());
        }
    };
    let read_at = read_at
        .unwrap_or_else(OffsetDateTime::now_utc)
        .replace_nanosecond(0)
        .unwrap_or_else(|_| OffsetDateTime::now_utc());
    let read_at = Timestamp(
        read_at
            .format(&Rfc3339)
            .context("write the probe's instant")?,
    );
    for reading in readings {
        record.usage(RecordUsage {
            snapshot_id: snapshot.clone(),
            read_at: read_at.clone(),
            account: reading.account,
            active: reading.active,
            health: reading.health,
            window: reading.window,
            used_percent: reading.used_percent,
            resets_at: reading.resets_at.map(Timestamp),
        })?;
    }
    Ok(())
}

/// One recorded window, as the forecast reads it.
#[derive(Debug, Clone)]
struct Row {
    account: String,
    health: String,
    window: String,
    used_percent: i64,
    resets_at: Option<OffsetDateTime>,
}

/// What the readings say of the pool: when it runs out and when a window next resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Forecast {
    /// When the pool's first window runs out; `None` without a burn rate.
    pub runs_out: Option<OffsetDateTime>,
    /// The earliest reset after the newest reading of a window any account has used.
    pub next_reset: Option<OffsetDateTime>,
}

impl Forecast {
    /// The forecast of the usage readings `store` holds of its complete snapshots; `None` when
    /// no complete snapshot holds one.
    #[must_use]
    pub fn of(store: &Store) -> Option<Self> {
        let complete: BTreeSet<String> = SnapshotStorage::list(store)
            .into_iter()
            .filter(|held| held.state == SnapshotState::Complete)
            .map(|held| held.data.snapshot_id.0.0)
            .collect();
        let rows = UsageObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| complete.contains(&data.snapshot_id.0.0));
        Self::from_rows(rows)
    }

    /// The forecast of `rows`, grouped into readings by their snapshot and ordered by the instant
    /// each was read.
    fn from_rows(rows: impl Iterator<Item = UsageObservationData>) -> Option<Self> {
        let mut readings: BTreeMap<String, (OffsetDateTime, Vec<Row>)> = BTreeMap::new();
        for data in rows {
            let Ok(read_at) = OffsetDateTime::parse(&data.read_at.0, &Rfc3339) else {
                continue;
            };
            let entry = readings
                .entry(data.snapshot_id.0.0.clone())
                .or_insert_with(|| (read_at, Vec::new()));
            entry.1.push(Row {
                account: data.account,
                health: data.health,
                window: data.window,
                used_percent: data.used_percent,
                resets_at: data
                    .resets_at
                    .and_then(|at| OffsetDateTime::parse(&at.0, &Rfc3339).ok()),
            });
        }
        let mut readings: Vec<(OffsetDateTime, Vec<Row>)> = readings.into_values().collect();
        readings.sort_by_key(|(read_at, _)| *read_at);
        let (newest_at, newest) = readings.pop()?;
        let previous = readings.pop();
        Some(Self::between(newest_at, &newest, previous.as_ref()))
    }

    /// The forecast of the reading `newest`, taken at `newest_at`, after `previous`.
    fn between(
        newest_at: OffsetDateTime,
        newest: &[Row],
        previous: Option<&(OffsetDateTime, Vec<Row>)>,
    ) -> Self {
        let windows: BTreeSet<&str> = newest.iter().map(|row| row.window.as_str()).collect();
        let mut runs_out: Option<OffsetDateTime> = None;
        for window in windows {
            let rows = || newest.iter().filter(move |row| row.window == window);
            let headroom: i64 = rows()
                .filter(|row| row.health != EXHAUSTED)
                .map(|row| 100 - row.used_percent)
                .sum();
            let at = if headroom <= 0 {
                Some(newest_at)
            } else {
                previous.and_then(|(previous_at, earlier)| {
                    let hours = (newest_at - *previous_at).as_seconds_f64() / 3600.0;
                    let used: i64 = rows()
                        .filter_map(|row| {
                            earlier
                                .iter()
                                .find(|before| {
                                    before.account == row.account && before.window == window
                                })
                                .map(|before| (row.used_percent - before.used_percent).max(0))
                        })
                        .sum();
                    if hours <= 0.0 || used == 0 {
                        return None;
                    }
                    #[allow(clippy::cast_precision_loss)]
                    let left = headroom as f64 / (used as f64 / hours);
                    Some(newest_at + Duration::seconds_f64(left * 3600.0))
                })
            };
            if let Some(at) = at {
                runs_out = Some(runs_out.map_or(at, |first| first.min(at)));
            }
        }
        let next_reset = newest
            .iter()
            .filter(|row| row.used_percent > 0)
            .filter_map(|row| row.resets_at)
            .filter(|at| *at > newest_at)
            .min();
        Self {
            runs_out,
            next_reset,
        }
    }

    /// The hour the pool runs out, `YYYY-MM-DD HH:00 UTC`, or `unknown`: what the watch prints
    /// the line once per change of.
    #[must_use]
    pub fn hour(&self) -> String {
        self.runs_out.map_or_else(
            || UNKNOWN.to_owned(),
            |at| {
                let at = at.to_offset(time::UtcOffset::UTC);
                format!(
                    "{:04}-{:02}-{:02} {:02}:00 UTC",
                    at.year(),
                    u8::from(at.month()),
                    at.day(),
                    at.hour()
                )
            },
        )
    }

    /// `usage: pool runs out ~<hour>, next reset <minute>`.
    #[must_use]
    pub fn line(&self) -> String {
        let reset = self.next_reset.map_or_else(
            || UNKNOWN.to_owned(),
            |at| {
                let at = at.to_offset(time::UtcOffset::UTC);
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02} UTC",
                    at.year(),
                    u8::from(at.month()),
                    at.day(),
                    at.hour(),
                    at.minute()
                )
            },
        );
        format!("usage: pool runs out ~{}, next reset {reset}", self.hour())
    }
}
