//! `conductor need new` (`story:need-id-unique`): the next id no earlier NEED used, over the `local`
//! callable `need-new` and its input `conductor.direction.NewNeed` (`spec/domains/direction.yaml`).
//!
//! An id is `N-<yyyymmdd>-<nn>`: the UTC day and a count from `01`, two digits at least. Each id
//! printed is kept as an empty file `need-ids/<id>` under the instance's state directory, created
//! with `create_new`, so two calls, in one process or two, never print the same id and a restart
//! changes nothing. An id of that day that the records already cite, in a hand-over
//! (`docs/handoff/*.md`) or a line of `decisions/` or `dispatches/` (`*.jsonl`), counts as used
//! too: the count goes on after the highest id of the day any of them holds.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use conductor_model::direction::NewNeed;
use time::format_description;
use time::{Date, OffsetDateTime};

use crate::cli::NeedNewArgs;
use crate::config;

/// The command's words, as its messages name it.
const COMMAND: &str = "need new";

/// The directory of the kept ids under the state directory.
pub const IDS: &str = "need-ids";

/// `conductor need new`: print the next unused id of the day, and keep it.
///
/// # Errors
///
/// `--day` is no `yyyy-mm-dd` date, a directory cannot be read or created, or standard output
/// cannot be written.
pub fn need_new(args: &NeedNewArgs) -> Result<ExitCode> {
    let input = NewNeed {
        day: args.day.clone(),
    };
    let day = match input.day.as_deref() {
        None => OffsetDateTime::now_utc().date(),
        Some(text) => parse_day(text)?,
    };
    let stamp = format!(
        "{:04}{:02}{:02}",
        day.year(),
        u8::from(day.month()),
        day.day()
    );
    let instance = &config::active().instance;
    let ids = PathBuf::from(&instance.state).join(IDS);
    fs::create_dir_all(&ids).with_context(|| format!("{COMMAND}: create {}", ids.display()))?;

    let prefix = format!("N-{stamp}-");
    let mut used = BTreeSet::new();
    cited_in(&ids, "", &prefix, &mut used)?;
    let records = PathBuf::from(&instance.records);
    cited_in(&records.join("docs/handoff"), ".md", &prefix, &mut used)?;
    cited_in(&records.join("decisions"), ".jsonl", &prefix, &mut used)?;
    cited_in(&records.join("dispatches"), ".jsonl", &prefix, &mut used)?;

    let mut count = used.last().copied().unwrap_or(0) + 1;
    let id = loop {
        let id = format!("{prefix}{count:02}");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(ids.join(&id))
        {
            Ok(_) => break id,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => count += 1,
            Err(error) => {
                return Err(error).with_context(|| format!("{COMMAND}: keep {id}"));
            }
        }
    };
    writeln!(io::stdout().lock(), "{id}").context("write standard output")?;
    Ok(ExitCode::SUCCESS)
}

/// The date `text` writes as `yyyy-mm-dd`.
fn parse_day(text: &str) -> Result<Date> {
    let format = format_description::parse_borrowed::<2>("[year]-[month]-[day]")
        .context("the day's format")?;
    match Date::parse(text, &format) {
        Ok(day) => Ok(day),
        Err(_) => bail!("{COMMAND}: --day {text:?} is not a date, yyyy-mm-dd"),
    }
}

/// Adds to `used` the count of each id that starts with `prefix` and that a file of `dir` whose
/// name ends in `suffix` holds. A directory that is not there holds none.
fn cited_in(dir: &Path, suffix: &str, prefix: &str, used: &mut BTreeSet<u64>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("{COMMAND}: read {}", dir.display()));
        }
    };
    for entry in entries {
        let entry = entry.with_context(|| format!("{COMMAND}: read {}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(suffix) {
            continue;
        }
        if suffix.is_empty() {
            // A kept id: the file's name is the id.
            counts(&name, prefix, used);
        } else if let Ok(bytes) = fs::read(entry.path()) {
            counts(&String::from_utf8_lossy(&bytes), prefix, used);
        }
    }
    Ok(())
}

/// Adds to `used` the digits after each occurrence of `prefix` in `text`.
fn counts(text: &str, prefix: &str, used: &mut BTreeSet<u64>) {
    for (at, _) in text.match_indices(prefix) {
        let digits: String = text[at + prefix.len()..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(count) = digits.parse() {
            used.insert(count);
        }
    }
}
