//! A usage limit's reset and the loop's back-off (`story:limit-resume-at-reset`).
//!
//! - [`reset`] reads the reset a limit text names, in its two forms: the five-hour window's
//!   `resets 9:40pm (UTC)` (a clock time, the next one at or after the text was seen) and the
//!   weekly one's `resets Nov 20, 11am (UTC)` (a date and a clock time). The dashboard shows it
//!   and the loop resumes a limited session at it.
//! - [`backoff`] is how long the loop waits before it resumes a session a usage limit stopped
//!   again after the loop resumed it: [`BACKOFF_FIRST`] for the second limit, doubling with
//!   each further one, at most [`BACKOFF_MOST`].
//! - [`RESUMED`] is the loop's file, in `<state>/run/`, of each session it resumed after a usage
//!   limit: when, in nanoseconds since the Unix epoch (`at`), and how many limits in a row it was
//!   resumed after so far (`attempts`); an entry of an older loop is the instant alone, as a
//!   string. The watch reports a limit text again for a session resumed since its last report.

use std::path::Path;

use serde_json::Value;
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

/// Where the host keeps its time zones, which a usage-limit line names its reset in.
const ZONEINFO: &str = "/usr/share/zoneinfo";

/// The loop's file of the sessions it resumed after a usage limit, in `<state>/run/`.
pub const RESUMED: &str = "resumed.json";

/// The wait after a second usage limit that follows a resume.
pub const BACKOFF_FIRST: Duration = Duration::minutes(5);

/// The longest wait [`backoff`] answers.
pub const BACKOFF_MOST: Duration = Duration::hours(1);

/// How long the loop waits after the usage limit that follows the `attempts`-th resume in a row
/// (1 for the first resume): [`BACKOFF_FIRST`], doubled per further attempt, at most
/// [`BACKOFF_MOST`]; nothing for 0.
#[must_use]
pub fn backoff(attempts: u32) -> Duration {
    if attempts == 0 {
        return Duration::ZERO;
    }
    let factor = 1_i32 << attempts.saturating_sub(1).min(16);
    (BACKOFF_FIRST * factor).min(BACKOFF_MOST)
}

/// When the [`RESUMED`] entry `entry` says the session was resumed, in nanoseconds since the
/// Unix epoch.
#[must_use]
pub fn resumed_at(entry: &Value) -> Option<u128> {
    entry
        .as_str()
        .or_else(|| entry["at"].as_str())
        .and_then(|at| at.parse().ok())
}

/// How many usage limits in a row the [`RESUMED`] entry `entry` says the session was resumed
/// after; 0 for an older loop's entry.
#[must_use]
pub fn attempts_of(entry: &Value) -> u32 {
    entry["attempts"]
        .as_u64()
        .and_then(|attempts| u32::try_from(attempts).ok())
        .unwrap_or(0)
}

/// The reset a limit text names, `resets 7:50pm (Europe/Berlin)`, `resets 11am (UTC)`,
/// `resets 19:50` or `resets Nov 20, 11am (UTC)`, in that zone (UTC when it names none). A clock
/// time alone is its first instant at or after `seen`; a date with it is that day's, in the year
/// of `seen`, or the next year's when that day is more than a month before `seen`. None when the
/// text names no reset this reads.
#[must_use]
pub fn reset(text: &str, seen: OffsetDateTime) -> Option<OffsetDateTime> {
    let (_, rest) = text.split_once("resets ")?;
    let rest = rest.trim_start();
    let (first, after) = rest.split_once(' ').unwrap_or((rest, ""));
    let (day, rest) = match month(first) {
        Some(month) => {
            let after = after.trim_start();
            let (day, after) = after.split_once(' ').unwrap_or((after, ""));
            let day = day.trim_end_matches([',', '.']).parse::<u8>().ok()?;
            let after = after.trim_start();
            let after = after.strip_prefix("at ").unwrap_or(after);
            (Some((month, day)), after)
        }
        None => (None, rest),
    };
    let (clock, rest) = rest.split_once(' ').unwrap_or((rest, ""));
    let clock = clock.trim_end_matches([',', '.', ';']).to_ascii_lowercase();
    let (digits, pm) = match (clock.strip_suffix("am"), clock.strip_suffix("pm")) {
        (Some(digits), _) => (digits, Some(false)),
        (_, Some(digits)) => (digits, Some(true)),
        _ => (clock.as_str(), None),
    };
    let (hour, minute) = match digits.split_once(':') {
        Some((hour, minute)) => (hour.parse::<u8>().ok()?, minute.parse::<u8>().ok()?),
        None => (digits.parse::<u8>().ok()?, 0),
    };
    let hour = match pm {
        Some(_) if hour == 0 || hour > 12 => return None,
        Some(pm) => hour % 12 + if pm { 12 } else { 0 },
        None => hour,
    };
    let clock = Time::from_hms(hour, minute, 0).ok()?;
    let zone = rest
        .trim()
        .strip_prefix('(')
        .and_then(|zone| zone.split_once(')'))
        .map(|(zone, _)| zone.trim());
    let offset = |at: OffsetDateTime| zone.map_or(Some(UtcOffset::UTC), |zone| offset(zone, at));
    let local = seen.to_offset(offset(seen)?);
    let at = |day: Date| -> Option<OffsetDateTime> {
        let wall = PrimitiveDateTime::new(day, clock);
        Some(wall.assume_offset(offset(wall.assume_utc())?))
    };
    if let Some((month, day)) = day {
        let this = at(Date::from_calendar_date(local.year(), month, day).ok()?)?;
        if this >= seen - Duration::days(31) {
            return Some(this);
        }
        return at(Date::from_calendar_date(local.year() + 1, month, day).ok()?);
    }
    let today = at(local.date())?;
    if today >= seen {
        return Some(today);
    }
    at(local.date().next_day()?)
}

/// The month `word` names, by its English name or its first three letters, in any case.
fn month(word: &str) -> Option<Month> {
    const MONTHS: [Month; 12] = [
        Month::January,
        Month::February,
        Month::March,
        Month::April,
        Month::May,
        Month::June,
        Month::July,
        Month::August,
        Month::September,
        Month::October,
        Month::November,
        Month::December,
    ];
    let word = word.trim_end_matches([',', '.']).to_ascii_lowercase();
    if word.len() < 3 || !word.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    MONTHS.into_iter().find(|month| {
        let name = month.to_string().to_ascii_lowercase();
        name == word || (word.len() == 3 && name.starts_with(&word))
    })
}

/// The offset of the time zone `zone` at `at`: UTC for `UTC` and its names, else from the host's
/// zone file; none for a name that is no zone file's.
fn offset(zone: &str, at: OffsetDateTime) -> Option<UtcOffset> {
    if matches!(zone, "UTC" | "Etc/UTC" | "GMT" | "Etc/GMT" | "Z") {
        return Some(UtcOffset::UTC);
    }
    let plain = !zone.is_empty()
        && zone
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'))
        && zone
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('-'));
    if !plain {
        return None;
    }
    let bytes = std::fs::read(Path::new(ZONEINFO).join(zone)).ok()?;
    tzif_offset(&bytes, at.unix_timestamp())
}

/// The offset a TZif file `bytes` gives at the Unix time `at`: that of the last transition at or
/// before it, from the 64-bit block of a version 2 file and the 32-bit block of a version 1 one;
/// the first local time type before every transition.
fn tzif_offset(bytes: &[u8], at: i64) -> Option<UtcOffset> {
    fn counts(block: &[u8]) -> Option<[usize; 6]> {
        if block.get(..4)? != b"TZif" {
            return None;
        }
        let mut counts = [0; 6];
        for (index, count) in counts.iter_mut().enumerate() {
            let at = 20 + 4 * index;
            let raw: [u8; 4] = block.get(at..at + 4)?.try_into().ok()?;
            *count = usize::try_from(u32::from_be_bytes(raw)).ok()?;
        }
        Some(counts)
    }
    let [ut, std, leap, times, types, chars] = counts(bytes)?;
    let (block, width) = if *bytes.get(4)? >= b'2' {
        let skip = 44 + times * 5 + types * 6 + chars + leap * 8 + std + ut;
        (bytes.get(skip..)?, 8)
    } else {
        (bytes, 4)
    };
    let [_, _, _, times, types, _] = counts(block)?;
    let data = block.get(44..)?;
    let transition = |index: usize| -> Option<i64> {
        let raw = data.get(index * width..(index + 1) * width)?;
        Some(if width == 8 {
            i64::from_be_bytes(raw.try_into().ok()?)
        } else {
            i64::from(i32::from_be_bytes(raw.try_into().ok()?))
        })
    };
    let indices = data.get(times * width..times * width + times)?;
    let infos = data.get(times * width + times..times * width + times + types * 6)?;
    let mut kind = 0;
    for index in 0..times {
        if transition(index)? > at {
            break;
        }
        kind = usize::from(*indices.get(index)?);
    }
    let info: [u8; 4] = infos.get(kind * 6..kind * 6 + 4)?.try_into().ok()?;
    UtcOffset::from_whole_seconds(i32::from_be_bytes(info)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::format_description::well_known::Rfc3339;

    fn instant(text: &str) -> OffsetDateTime {
        OffsetDateTime::parse(text, &Rfc3339).expect("an instant")
    }

    fn at(text: &str, seen: &str) -> Option<String> {
        reset(text, instant(seen)).map(|at| at.format(&Rfc3339).expect("RFC 3339"))
    }

    /// A version 1 TZif file of two types, +01:00 then +02:00 from the Unix time 1000.
    fn tzif() -> Vec<u8> {
        let mut bytes = b"TZif".to_vec();
        bytes.push(0);
        bytes.extend([0; 15]);
        for count in [0_u32, 0, 0, 1, 2, 4] {
            bytes.extend(count.to_be_bytes());
        }
        bytes.extend(1000_i32.to_be_bytes());
        bytes.push(1);
        bytes.extend(3600_i32.to_be_bytes());
        bytes.extend([0, 0]);
        bytes.extend(7200_i32.to_be_bytes());
        bytes.extend([1, 0]);
        bytes.extend(b"AB\0\0");
        bytes
    }

    #[test]
    fn a_tzif_file_gives_the_offset_of_the_last_transition_before_the_instant() {
        let hour = |hours: i8| UtcOffset::from_hms(hours, 0, 0).ok();
        assert_eq!(tzif_offset(&tzif(), 999), hour(1));
        assert_eq!(tzif_offset(&tzif(), 1000), hour(2));
        assert_eq!(tzif_offset(&tzif(), 5000), hour(2));
        assert_eq!(tzif_offset(b"nope", 0), None);
    }

    #[test]
    fn a_daily_reset_parses() {
        let seen = "2026-10-17T14:07:00Z";
        assert_eq!(
            at("You've hit your limit · resets 9:40pm (UTC)", seen).as_deref(),
            Some("2026-10-17T21:40:00Z")
        );
        assert_eq!(
            at(
                "usage limit: You've hit your limit · resets 9:40pm (UTC); 1 sessions: ex-a",
                seen
            )
            .as_deref(),
            Some("2026-10-17T21:40:00Z"),
            "as the watch's line carries it"
        );
        assert_eq!(
            at("limit · resets 11am (UTC)", seen).as_deref(),
            Some("2026-10-18T11:00:00Z"),
            "a clock time already past that day is the next day's"
        );
        assert_eq!(
            at("limit · resets 12:30am", seen).as_deref(),
            Some("2026-10-18T00:30:00Z")
        );
        assert_eq!(
            at("limit · resets 16:00", seen).as_deref(),
            Some("2026-10-17T16:00:00Z")
        );
        assert_eq!(at("Usage limit reached", seen), None);
        assert_eq!(at("limit · resets 13pm (UTC)", seen), None);
        assert_eq!(at("limit · resets 7pm (../../etc/passwd)", seen), None);
    }

    #[test]
    fn a_weekly_reset_with_a_date_parses() {
        let seen = "2026-11-18T09:00:00Z";
        assert_eq!(
            at("You've hit your limit · resets Nov 20, 11am (UTC)", seen).as_deref(),
            Some("2026-11-20T11:00:00Z")
        );
        assert_eq!(
            at(
                "usage limit: You've hit your limit · resets Nov 20, 11am (UTC); 2 sessions: a b",
                seen
            )
            .as_deref(),
            Some("2026-11-20T11:00:00Z")
        );
        assert_eq!(
            at("limit · resets December 2, 9:30pm (UTC)", seen).as_deref(),
            Some("2026-12-02T21:30:00Z")
        );
        assert_eq!(
            at("limit · resets Jan 3, 11am (UTC)", "2026-12-30T09:00:00Z").as_deref(),
            Some("2027-01-03T11:00:00Z"),
            "a date months before it was seen is next year's"
        );
        assert_eq!(
            at("limit · resets Nov 18, 8am (UTC)", seen).as_deref(),
            Some("2026-11-18T08:00:00Z"),
            "a date just passed stays passed"
        );
        assert_eq!(at("limit · resets Nov 31, 11am (UTC)", seen), None);
    }

    #[test]
    fn a_second_limit_backs_off_five_minutes_doubling_to_an_hour() {
        assert_eq!(backoff(0), Duration::ZERO);
        assert_eq!(backoff(1), Duration::minutes(5));
        assert_eq!(backoff(2), Duration::minutes(10));
        assert_eq!(backoff(4), Duration::minutes(40));
        assert_eq!(backoff(5), Duration::hours(1));
        assert_eq!(backoff(40), Duration::hours(1));
    }
}
