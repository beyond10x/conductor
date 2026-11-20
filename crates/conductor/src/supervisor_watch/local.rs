//! The eight host-local triggers of `conductor supervisor watch` (story:supervisor-watch-command):
//! each reads its source for every instance, adds what fired to a [`Pass`], and names the cursor
//! file and content [`Watch::commit`] writes once the lines are printed. A source that cannot be
//! read is a note, and the others are read.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write};
use std::os::fd::AsFd as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result};
use conductor_model::behaviour::{ControllerStorage, DispatchStorage, ReconciliationStorage};
use conductor_model::config::Instance;
use conductor_model::dispatch::{ControllerState, DispatchState};
use conductor_model::host::SupervisorTriggerKind as Kind;
use conductor_model::instance::{ReconcileAction, ReconcileReason};
use conductor_model::primitives::Timestamp;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{NO_INSTANCE, Pass, Watch, put, read};
use crate::message::MAILBOX;

/// The strings of the runtime log a `LOG` line carries one of. `]: context: `, not `context: `:
/// a worktree name can end in `-context`.
pub const LOG_STRINGS: [&str; 9] = [
    "]: context: ",
    "LaunchFailed",
    "no open dispatch",
    "lists no lettered option",
    "DispatchNotFound",
    "snapshot exited",
    "usage limit",
    "start .claude.",
    "waits: ControllerLimit",
];

/// The report words a `MAIL` file is skipped for, unless its text waits on a person.
const QUIET_REPORTS: [&[&str]; 4] = [
    &["progress"],
    &["started"],
    &["unblocked"],
    &["blocked", "ci"],
];

/// What a text holds when it waits on a person (case-insensitive).
const WAITS_ON_A_PERSON: [&str; 2] = ["waits on", "waiting for"];

/// A drop of free disk under the previous pass's reading that fires `DISK`: 5 GiB.
pub const DISK_DROP: u64 = 5 << 30;

/// How old the oldest unpushed commit of a branch is before `UNPUSHED` fires: 30 minutes.
pub const UNPUSHED_AGE: Duration = Duration::from_secs(30 * 60);

/// How long a controller's transcripts are silent before `IDLE` fires: 15 minutes.
pub const IDLE_SILENCE: Duration = Duration::from_secs(15 * 60);

/// How long after an `IDLE` line the same controller is named again at the earliest: 30 minutes.
pub const IDLE_AGAIN: Duration = Duration::from_secs(30 * 60);

const GIB: u64 = 1 << 30;

// Cursor and baseline files in an instance's cursor directory.
const MAIL_CURSOR: &str = "mail";
const LOG_CURSOR: &str = "log";
const DAEMON_CURSOR: &str = "daemon";
const INBOX_CURSOR: &str = "inbox";
const DISK_BASELINE: &str = "disk";
const DISK_LOW: &str = "disk.low";
const UNPUSHED_CURSOR: &str = "unpushed";
const IDLE_CURSOR: &str = "idle";
const JOURNAL_CURSOR: &str = "journal";

/// The mailbox of `instance`'s conductor.
fn mailbox(instance: &Instance) -> PathBuf {
    Path::new(&instance.state).join(MAILBOX).join("conductor")
}

/// The runtime log of `instance`.
fn runtime_log(instance: &Instance) -> PathBuf {
    crate::placement::sessions_dir(Path::new(&instance.state)).join(crate::placement::RUNTIME_LOG)
}

/// The mail files of `dir`, by name (ULIDs, so in arrival order); none when it is not there.
fn mail_files(dir: &Path) -> io::Result<Vec<String>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && let Some(name) = entry.file_name().to_str()
            && !name.starts_with('.')
        {
            names.push(name.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// The first line of `text`.
fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default().trim_end()
}

/// Whether `text` waits on a person.
fn waits_on_a_person(text: &str) -> bool {
    let lower = text.to_lowercase();
    WAITS_ON_A_PERSON.iter().any(|words| lower.contains(words))
}

/// The words after the brackets of a `[REPORT …]` first line, if it is one.
fn report_words(line: &str) -> Option<Vec<&str>> {
    let rest = line.trim().strip_prefix("[REPORT ")?;
    let (_, after) = rest.split_once(']')?;
    Some(after.split_whitespace().collect())
}

/// Whether a report whose words are `words` is one of [`QUIET_REPORTS`].
fn quiet(words: &[&str]) -> bool {
    QUIET_REPORTS
        .iter()
        .any(|quiet| words.len() >= quiet.len() && words[..quiet.len()] == **quiet)
}

/// How far below the mail cursor, in ULID time, the watch reads again for a file that landed
/// late: a ULID is made before its file is written, in another process, so a file can sort below
/// one already handled, the more so within one millisecond.
pub const MAIL_WINDOW_MS: u64 = 5_000;

/// The milliseconds since the epoch of a name that starts with a ULID.
fn ulid_ms(name: &str) -> Option<u64> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut ms: u64 = 0;
    for byte in name.get(..10)?.bytes() {
        let digit = ALPHABET
            .iter()
            .position(|known| *known == byte.to_ascii_uppercase())?;
        ms = ms.checked_mul(32)?.checked_add(digit as u64)?;
    }
    Some(ms)
}

/// The mail cursor's file: the newest name handled on its first line, then each name of
/// `handled` at or under it within [`MAIL_WINDOW_MS`] of it, one per line.
fn mail_cursor(newest: &str, handled: &BTreeSet<String>) -> String {
    let floor = ulid_ms(newest).map(|ms| ms.saturating_sub(MAIL_WINDOW_MS));
    let mut text = format!("{newest}\n");
    for name in handled {
        let within = floor.is_some_and(|floor| ulid_ms(name).is_some_and(|ms| ms >= floor));
        if within && name.as_str() <= newest {
            text.push_str(name);
            text.push('\n');
        }
    }
    text
}

/// `MAIL`: every file of the instance's mailbox past its mail cursor, and every file within
/// [`MAIL_WINDOW_MS`] below it that the cursor does not name as handled. A file that cannot be
/// read is listed with its error. A first run with no cursor starts at the newest file, and names
/// how many it passed over.
pub(super) fn mail(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
    at: &Timestamp,
    notes: &mut dyn Write,
) -> Result<()> {
    let name = &instance.name.0;
    let dir = mailbox(instance);
    let cursor_file = watch.cursors(name).join(MAIL_CURSOR);
    let names = match mail_files(&dir) {
        Ok(names) => names,
        Err(error) => {
            watch.note(
                notes,
                format!("{name}: MAIL: read {}: {error}", dir.display()),
            );
            return Ok(());
        }
    };
    let Some(cursor) = read(&cursor_file)? else {
        let newest = names.last().cloned().unwrap_or_default();
        let all: BTreeSet<String> = names.iter().cloned().collect();
        put(&cursor_file, &mail_cursor(&newest, &all))?;
        watch.note(
            notes,
            format!(
                "{name}: first run: passed over {} mail file(s) in {}; the mail cursor starts at \
                 the newest",
                names.len(),
                dir.display()
            ),
        );
        return Ok(());
    };
    let mut lines = cursor.lines();
    let newest_handled = lines.next().unwrap_or_default().trim().to_owned();
    let mut handled: BTreeSet<String> = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let floor = ulid_ms(&newest_handled).map(|ms| ms.saturating_sub(MAIL_WINDOW_MS));
    let fresh = |file: &String| {
        *file > newest_handled
            || (!handled.contains(file)
                && floor.is_some_and(|floor| ulid_ms(file).is_some_and(|ms| ms >= floor)))
    };
    let fresh: Vec<String> = names.iter().filter(|file| fresh(file)).cloned().collect();
    if fresh.is_empty() {
        return Ok(());
    }
    for file in &fresh {
        handled.insert(file.clone());
        let text = match fs::read(dir.join(file)) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => {
                pass.fire(Kind::Mail, name, file, &format!("unreadable: {error}"), at);
                continue;
            }
        };
        let line = first_line(&text);
        if report_words(line).is_some_and(|words| quiet(&words)) && !waits_on_a_person(&text) {
            continue;
        }
        pass.fire(Kind::Mail, name, file, line, at);
    }
    let newest = fresh
        .iter()
        .max()
        .filter(|newest| **newest > newest_handled)
        .cloned()
        .unwrap_or(newest_handled);
    pass.write(cursor_file, mail_cursor(&newest, &handled));
    Ok(())
}

/// How many bytes from its start the log cursor keeps of the log, to know it again.
const LOG_PREFIX: u64 = 64;

/// Where the log cursor stands: the offset read to, the log's inode, and the hex of its first
/// bytes (at most [`LOG_PREFIX`], at most the offset), so a log replaced or rewritten under the
/// cursor is read from its start.
#[derive(Debug, Default, PartialEq, Eq)]
struct LogCursor {
    offset: u64,
    inode: Option<u64>,
    prefix: String,
}

impl LogCursor {
    fn parse(text: &str) -> Self {
        let mut words = text.split_whitespace();
        Self {
            offset: words.next().and_then(|word| word.parse().ok()).unwrap_or(0),
            inode: words.next().and_then(|word| word.parse().ok()),
            prefix: words.next().unwrap_or_default().to_owned(),
        }
    }

    fn text(&self) -> String {
        format!(
            "{} {} {}\n",
            self.offset,
            self.inode.unwrap_or(0),
            if self.prefix.is_empty() {
                "-"
            } else {
                &self.prefix
            }
        )
    }
}

/// The hex of the first `upto` bytes of `file` (at most [`LOG_PREFIX`]).
fn prefix(file: &mut File, upto: u64) -> io::Result<String> {
    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    io::Read::by_ref(file)
        .take(upto.min(LOG_PREFIX))
        .read_to_end(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// `LOG`: each complete line of the instance's runtime log past its log cursor that carries one
/// of [`LOG_STRINGS`]. A boxed runtime writes `<state>/sessions/runtime.log`; a runtime in a unit
/// logs to the unit's journal ([`journal`]). A log replaced, shortened or rewritten under the
/// cursor is read from its start; a first run with no cursor starts at the log's end; a missing
/// log is a note.
pub(super) fn log(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
    at: &Timestamp,
    notes: &mut dyn Write,
) -> Result<()> {
    if !crate::placement::boxed(instance) {
        return journal(watch, instance, pass, at, notes);
    }
    let name = &instance.name.0;
    let path = runtime_log(instance);
    let cursor_file = watch.cursors(name).join(LOG_CURSOR);
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            let what = if error.kind() == io::ErrorKind::NotFound {
                format!("{name}: LOG: no runtime log at {}", path.display())
            } else {
                format!("{name}: LOG: read {}: {error}", path.display())
            };
            watch.note(notes, what);
            if read(&cursor_file)?.is_none() {
                put(&cursor_file, &LogCursor::default().text())?;
            }
            return Ok(());
        }
    };
    let (length, inode) = match file.metadata() {
        Ok(meta) => (meta.len(), meta.ino()),
        Err(error) => {
            watch.note(
                notes,
                format!("{name}: LOG: read {}: {error}", path.display()),
            );
            return Ok(());
        }
    };
    let Some(text) = read(&cursor_file)? else {
        let start = LogCursor {
            offset: length,
            inode: Some(inode),
            prefix: prefix(&mut file, length).unwrap_or_default(),
        };
        put(&cursor_file, &start.text())?;
        return Ok(());
    };
    let mut cursor = LogCursor::parse(&text);
    let known = cursor.prefix.is_empty() || cursor.prefix == "-";
    let same = cursor
        .inode
        .is_none_or(|known| known == 0 || known == inode)
        && length >= cursor.offset
        && (known || prefix(&mut file, cursor.offset).is_ok_and(|now| now == cursor.prefix));
    if !same {
        cursor = LogCursor::default();
    }
    let mut bytes = Vec::new();
    let read_all = file
        .seek(SeekFrom::Start(cursor.offset))
        .and_then(|_| file.read_to_end(&mut bytes));
    if let Err(error) = read_all {
        watch.note(
            notes,
            format!("{name}: LOG: read {}: {error}", path.display()),
        );
        return Ok(());
    }
    let end = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |end| end + 1);
    for line in String::from_utf8_lossy(&bytes[..end]).lines() {
        if LOG_STRINGS.iter().any(|string| line.contains(string)) {
            pass.fire(Kind::Log, name, "", line, at);
        }
    }
    let offset = cursor.offset + end as u64;
    let next = LogCursor {
        offset,
        inode: Some(inode),
        prefix: prefix(&mut file, offset).unwrap_or_default(),
    };
    if next != LogCursor::parse(&text) {
        pass.write(cursor_file, next.text());
    }
    Ok(())
}

/// The `LOG` of an instance whose runtime runs in a unit: the lines of the unit's journal
/// ([`crate::daemon::reconcile::unit_name`]) past the journal cursor, read through
/// [`super::Sources::journal`]. A first run reads the last entry only, for its cursor. A journal
/// that cannot be read, or holds no entry of the unit, is a note.
fn journal(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
    at: &Timestamp,
    notes: &mut dyn Write,
) -> Result<()> {
    const CURSOR: &str = "-- cursor: ";
    let name = &instance.name.0;
    let unit = crate::daemon::reconcile::unit_name(name);
    let cursor_file = watch.cursors(name).join(JOURNAL_CURSOR);
    let cursor = read(&cursor_file)?.map(|text| text.trim().to_owned());
    let mut words: Vec<String> = watch
        .sources
        .journal
        .iter()
        .map(|word| word.replace("{unit}", &unit))
        .collect();
    match &cursor {
        None => words.extend(["-n".to_owned(), "1".to_owned()]),
        Some(cursor) if !cursor.is_empty() => {
            words.extend(["--after-cursor".to_owned(), cursor.clone()]);
        }
        Some(_) => {}
    }
    let Some((program, args)) = words.split_first() else {
        return Ok(());
    };
    let output = match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            watch.note(
                notes,
                format!(
                    "{name}: LOG: the journal of {unit}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            );
            return Ok(());
        }
        Err(error) => {
            watch.note(
                notes,
                format!("{name}: LOG: the journal of {unit}: {error}"),
            );
            return Ok(());
        }
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut next = None;
    let mut entries = Vec::new();
    for line in text.lines() {
        if let Some(found) = line.strip_prefix(CURSOR) {
            next = Some(found.trim().to_owned());
        } else if !line.starts_with("-- ") {
            entries.push(line);
        }
    }
    let Some(cursor) = cursor else {
        put(&cursor_file, next.as_deref().unwrap_or_default())?;
        if next.is_none() {
            watch.note(
                notes,
                format!("{name}: LOG: the journal holds no entry of {unit}"),
            );
        }
        return Ok(());
    };
    for line in entries {
        if LOG_STRINGS.iter().any(|string| line.contains(string)) {
            pass.fire(Kind::Log, name, "", line, at);
        }
    }
    match next {
        Some(next) if next != cursor => pass.write(cursor_file, next),
        Some(_) => {}
        None if cursor.is_empty() => {
            watch.note(
                notes,
                format!("{name}: LOG: the journal holds no entry of {unit}"),
            );
        }
        None => {}
    }
    Ok(())
}

/// `text` as an instant, if it reads as one.
fn instant(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

/// `DAEMON`: each `Reconciliation` of an instance past its daemon cursor (the newest record time
/// handled) with reason `LostRuntime`, or action `Upgrade` with reason `OperatorCommand`. A
/// first run with no cursor starts at the newest record there. Off, with a note, while the config
/// file writes no `daemon.store` or the daemon's store is not there.
pub(super) fn daemon(
    watch: &mut Watch,
    instances: &[Instance],
    pass: &mut Pass,
    at: &Timestamp,
    notes: &mut dyn Write,
) -> Result<()> {
    let Some(url) = watch.sources.daemon_store.clone() else {
        watch.note(
            notes,
            "DAEMON is off: the config file writes no daemon.store".to_owned(),
        );
        return Ok(());
    };
    let records = match crate::store::open_reader_url(&url, crate::daemon::TENANT) {
        Ok(store) => ReconciliationStorage::list(&store),
        Err(error) => {
            watch.note(notes, format!("DAEMON: the daemon's store: {error:#}"));
            return Ok(());
        }
    };
    for instance in instances {
        let name = &instance.name.0;
        let cursor_file = watch.cursors(name).join(DAEMON_CURSOR);
        let mut own: Vec<_> = records
            .iter()
            .filter(|record| &record.data.instance.0 == name)
            .filter_map(|record| Some((instant(&record.data.at.0)?, record)))
            .collect();
        own.sort_by_key(|(when, _)| *when);
        let Some(cursor) = read(&cursor_file)? else {
            let newest = own
                .last()
                .map(|(_, record)| record.data.at.0.clone())
                .unwrap_or_default();
            put(&cursor_file, &newest)?;
            continue;
        };
        let cursor = instant(cursor.trim());
        let mut newest = None;
        for (when, record) in own {
            if cursor.is_some_and(|cursor| when <= cursor) {
                continue;
            }
            newest = Some(record.data.at.0.clone());
            let data = &record.data;
            let fires = data.reason == ReconcileReason::LostRuntime
                || (data.action == ReconcileAction::Upgrade
                    && data.reason == ReconcileReason::OperatorCommand);
            if fires {
                pass.fire(
                    Kind::Daemon,
                    name,
                    &data.at.0,
                    // Each variant's name is the specification's.
                    &format!("{:?} {:?} {:?}", data.action, data.reason, data.result),
                    at,
                );
            }
        }
        if let Some(newest) = newest {
            pass.write(cursor_file, newest);
        }
    }
    Ok(())
}

/// The request ids of the file at `path`, one per line, in order, each once; none when it is not
/// there.
fn ids(path: &Path) -> io::Result<Vec<String>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut seen = BTreeSet::new();
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|id| !id.is_empty() && seen.insert(id.to_owned()))
        .map(str::to_owned)
        .collect())
}

/// `INBOX`: the inbox holds a request id the outbox does not, one the watch has not named yet;
/// off unless `globals.supervisor.inbox` and `outbox` are both set.
pub(super) fn inbox(watch: &mut Watch, pass: &mut Pass, at: &Timestamp, notes: &mut dyn Write) {
    let (Some(inbox), Some(outbox)) = (watch.inbox.clone(), watch.outbox.clone()) else {
        return;
    };
    let read_ids =
        |path: &Path| ids(path).map_err(|error| format!("read {}: {error}", path.display()));
    let (requests, answers) = match (read_ids(&inbox), read_ids(&outbox)) {
        (Ok(requests), Ok(answers)) => (requests, answers),
        (Err(error), _) | (_, Err(error)) => {
            watch.note(notes, format!("INBOX: {error}"));
            return;
        }
    };
    let answers: BTreeSet<String> = answers.into_iter().collect();
    let open: Vec<String> = requests
        .into_iter()
        .filter(|id| !answers.contains(id))
        .collect();
    let cursor_file = watch.cursors(NO_INSTANCE).join(INBOX_CURSOR);
    let named: BTreeSet<String> = match read(&cursor_file) {
        Ok(text) => text
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect(),
        Err(error) => {
            watch.note(notes, format!("INBOX: {error:#}"));
            return;
        }
    };
    if open.iter().all(|id| named.contains(id)) {
        return;
    }
    let count = open.len();
    pass.fire(
        Kind::Inbox,
        NO_INSTANCE,
        &open[0],
        &format!(
            "{count} request{} unanswered",
            if count == 1 { "" } else { "s" }
        ),
        at,
    );
    let mut text = open.join("\n");
    text.push('\n');
    pass.write(cursor_file, text);
}

/// How long the disk baseline stands while no DISK line fires: an hour.
pub const DISK_BASELINE_AGE: i64 = 3600;

/// `DISK`: free bytes on the instance's `thresholds.disk_path` under its `disk_low` (once per
/// fall: `disk.low` is kept until free disk is back over it), or [`DISK_DROP`] or more under the
/// baseline. The baseline is the first reading; it moves to the reading of a pass that prints a
/// DISK line, and to the reading of the first pass an hour ([`DISK_BASELINE_AGE`]) after it was
/// set, each written with the other cursors, so a slow fill fires too.
pub(super) fn disk(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
    now: OffsetDateTime,
    at: &Timestamp,
    notes: &mut dyn Write,
) -> Result<()> {
    let name = &instance.name.0;
    let path = PathBuf::from(&instance.thresholds.disk_path);
    let free = match (watch.sources.free_bytes)(&path) {
        Ok(free) => free,
        Err(error) => {
            watch.note(notes, format!("{name}: DISK: {error:#}"));
            return Ok(());
        }
    };
    let now = now.unix_timestamp();
    let dir = watch.cursors(name);
    let baseline_file = dir.join(DISK_BASELINE);
    let reading = format!("{free} {now}\n");
    let baseline: Option<(u64, i64)> = read(&baseline_file)?.and_then(|text| {
        let mut words = text.split_whitespace();
        let bytes = words.next()?.parse().ok()?;
        Some((
            bytes,
            words
                .next()
                .and_then(|word| word.parse().ok())
                .unwrap_or(now),
        ))
    });
    let low_gib = instance.thresholds.disk_low.0;
    let low = u64::try_from(low_gib).unwrap_or(0) * GIB;
    let marker = dir.join(DISK_LOW);
    let mut parts = Vec::new();
    if free < low {
        if read(&marker)?.is_none() {
            parts.push(format!("under disk_low {low_gib}G"));
            pass.write(marker, "1");
        }
    } else if marker.exists() {
        pass.remove(marker);
    }
    let Some((base, set)) = baseline else {
        put(&baseline_file, &reading)?;
        if !parts.is_empty() {
            pass.fire(
                Kind::Disk,
                name,
                &instance.thresholds.disk_path,
                &format!("{}G free, {}", free / GIB, parts.join("; ")),
                at,
            );
        }
        return Ok(());
    };
    if base.saturating_sub(free) >= DISK_DROP {
        parts.push(format!(
            "{}G under the baseline {}G",
            (base - free) / GIB,
            base / GIB
        ));
    }
    if !parts.is_empty() {
        pass.fire(
            Kind::Disk,
            name,
            &instance.thresholds.disk_path,
            &format!("{}G free, {}", free / GIB, parts.join("; ")),
            at,
        );
        pass.write(baseline_file, reading);
    } else if now - set >= DISK_BASELINE_AGE {
        pass.write(baseline_file, reading);
    }
    Ok(())
}

/// The directories in `dir`, in path order; none when it cannot be read.
fn children(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// The checkouts of `instance`: each directory under `checkouts.root`, and each managed tree
/// `<checkouts.trees>/<repository>/<tree>`, that holds `.git`, in path order.
fn checkouts(instance: &Instance) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = children(Path::new(&instance.checkouts.root))
        .into_iter()
        .filter(|path| path.join(".git").exists())
        .collect();
    for repository in children(Path::new(&instance.checkouts.trees)) {
        found.extend(
            children(&repository)
                .into_iter()
                .filter(|path| path.join(".git").exists()),
        );
    }
    found
}

/// `git -C <dir> <args>`'s standard output.
fn git(program: &Path, dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .with_context(|| format!("run git in {}", dir.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// One branch whose commits no remote branch holds.
struct Unpushed {
    /// `<path>:<branch>`.
    subject: String,
    /// The branch's head.
    head: String,
    /// The oldest such commit, short.
    oldest: String,
    /// Its committer time, in seconds since the epoch.
    committed: i64,
    /// How many such commits the branch holds.
    count: usize,
}

/// The branches of the checkout `dir` whose commits no remote branch holds
/// (`git rev-list --branches --not --remotes`).
fn unpushed_of(program: &Path, dir: &Path) -> Result<Vec<Unpushed>> {
    let local: BTreeSet<String> = git(
        program,
        dir,
        &["rev-list", "--branches", "--not", "--remotes"],
    )?
    .lines()
    .map(str::to_owned)
    .collect();
    if local.is_empty() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    let heads = git(
        program,
        dir,
        &[
            "for-each-ref",
            "--format=%(objectname) %(refname:short)",
            "refs/heads",
        ],
    )?;
    for line in heads.lines() {
        let Some((head, branch)) = line.split_once(' ') else {
            continue;
        };
        if !local.contains(head) {
            continue;
        }
        let log = git(
            program,
            dir,
            &["log", "--format=%h %ct", head, "--not", "--remotes"],
        )?;
        let commits: Vec<(&str, i64)> = log
            .lines()
            .filter_map(|line| {
                let (sha, time) = line.split_once(' ')?;
                Some((sha, time.trim().parse().ok()?))
            })
            .collect();
        let Some((oldest, committed)) = commits.iter().min_by_key(|(_, time)| *time) else {
            continue;
        };
        found.push(Unpushed {
            subject: format!("{}:{branch}", dir.display()),
            head: head.to_owned(),
            oldest: (*oldest).to_owned(),
            committed: *committed,
            count: commits.len(),
        });
    }
    // A detached HEAD of a worktree of the repository, which no branch holds.
    let worktrees = git(program, dir, &["worktree", "list", "--porcelain"])?;
    for block in worktrees.split("\n\n") {
        let mut path = None;
        let mut head = None;
        let mut detached = false;
        for line in block.lines() {
            if let Some(found) = line.strip_prefix("worktree ") {
                path = Some(found);
            } else if let Some(found) = line.strip_prefix("HEAD ") {
                head = Some(found);
            } else if line == "detached" {
                detached = true;
            }
        }
        let (Some(path), Some(head), true) = (path, head, detached) else {
            continue;
        };
        if let Some(branch) = unpushed_head(program, dir, head, &format!("{path}:HEAD"))? {
            found.push(branch);
        }
    }
    Ok(found)
}

/// The commits of `head` no remote branch holds, as [`Unpushed`] under `subject`; none when it
/// holds none.
fn unpushed_head(
    program: &Path,
    dir: &Path,
    head: &str,
    subject: &str,
) -> Result<Option<Unpushed>> {
    let log = git(
        program,
        dir,
        &["log", "--format=%h %ct", head, "--not", "--remotes"],
    )?;
    let commits: Vec<(&str, i64)> = log
        .lines()
        .filter_map(|line| {
            let (sha, time) = line.split_once(' ')?;
            Some((sha, time.trim().parse().ok()?))
        })
        .collect();
    Ok(commits
        .iter()
        .min_by_key(|(_, time)| *time)
        .map(|(oldest, committed)| Unpushed {
            subject: subject.to_owned(),
            head: head.to_owned(),
            oldest: (*oldest).to_owned(),
            committed: *committed,
            count: commits.len(),
        }))
}

/// `UNPUSHED`: a local branch of a checkout of an instance (or of a `--checkout`, instance `-`)
/// whose oldest commit no remote branch holds is over [`UNPUSHED_AGE`] old, once per branch head.
/// Two checkouts of one repository (a managed tree and its checkout) are read once, by the first.
pub(super) fn unpushed(
    watch: &mut Watch,
    instances: &[Instance],
    pass: &mut Pass,
    now: OffsetDateTime,
    at: &Timestamp,
    notes: &mut dyn Write,
) {
    let mut groups: Vec<(String, Vec<PathBuf>)> = instances
        .iter()
        .map(|instance| (instance.name.0.clone(), checkouts(instance)))
        .collect();
    groups.push((NO_INSTANCE.to_owned(), watch.checkouts.clone()));
    let program = watch.sources.git.clone();
    let oldest = i64::try_from(UNPUSHED_AGE.as_secs()).unwrap_or(i64::MAX);
    let mut repositories = BTreeSet::new();
    for (name, paths) in groups {
        let cursor_file = watch.cursors(&name).join(UNPUSHED_CURSOR);
        let named: BTreeSet<String> = read(&cursor_file)
            .ok()
            .flatten()
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect();
        let mut kept = Vec::new();
        let mut fired = false;
        for path in paths {
            let branches = git(
                &program,
                &path,
                &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            )
            .and_then(|common| {
                if repositories.insert(common.trim().to_owned()) {
                    unpushed_of(&program, &path)
                } else {
                    Ok(Vec::new())
                }
            });
            let branches = match branches {
                Ok(branches) => branches,
                Err(error) => {
                    watch.note(notes, format!("{name}: UNPUSHED: {error:#}"));
                    continue;
                }
            };
            for branch in branches {
                let age = now.unix_timestamp() - branch.committed;
                if age <= oldest {
                    continue;
                }
                let key = format!("{} {}", branch.subject, branch.head);
                if !named.contains(&key) {
                    fired = true;
                    pass.fire(
                        Kind::Unpushed,
                        &name,
                        &branch.subject,
                        &format!(
                            "{} {}m old, {} commit{}",
                            branch.oldest,
                            age / 60,
                            branch.count,
                            if branch.count == 1 { "" } else { "s" }
                        ),
                        at,
                    );
                }
                kept.push(key);
            }
        }
        if fired {
            let mut text = kept.join("\n");
            text.push('\n');
            pass.write(cursor_file, text);
        }
    }
}

/// `path` as Claude Code names the project directory of a session working in it: each character
/// but an ASCII letter or digit written `-`.
fn project_name(path: &Path) -> String {
    path.display()
        .to_string()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// When the transcripts of a session working in `<root>/<repository>`, or in one of the
/// repository's managed trees `<trees>/<repository>/<tree>`, were last written: the newest write
/// of each `*.jsonl` of exactly those project directories and of its sub-agents
/// ([`crate::watch::newest_write`]); none without a transcript.
fn last_written(projects: &Path, instance: &Instance, repository: &str) -> Option<SystemTime> {
    let mut directories = vec![Path::new(&instance.checkouts.root).join(repository)];
    directories.extend(children(
        &Path::new(&instance.checkouts.trees).join(repository),
    ));
    let mut newest = None;
    for directory in directories {
        let project = projects.join(project_name(&directory));
        for transcript in fs::read_dir(project).into_iter().flatten().flatten() {
            let path = transcript.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "jsonl")
                && let Some(written) = crate::watch::newest_write(&path)
            {
                newest = newest.max(Some(written));
            }
        }
    }
    newest
}

/// The text of the newest report of each of `repositories` in the instance's mailbox: read
/// newest first, stopping once each has one.
fn newest_reports(instance: &Instance, repositories: &BTreeSet<&str>) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    if repositories.is_empty() {
        return found;
    }
    let dir = mailbox(instance);
    let Ok(names) = mail_files(&dir) else {
        return found;
    };
    for name in names.iter().rev() {
        let Ok(text) = fs::read_to_string(dir.join(name)) else {
            continue;
        };
        let line = first_line(&text);
        if !line.trim_start().starts_with("[REPORT ") {
            continue;
        }
        if let Some(repository) = crate::message::named_repository(line)
            && repositories.contains(repository.as_str())
            && !found.contains_key(&repository)
        {
            found.insert(repository, text);
            if found.len() == repositories.len() {
                break;
            }
        }
    }
    found
}

/// `IDLE`: a controller the instance's store holds `Running`, with an open dispatch of its
/// repository, whose transcripts have not been written for [`IDLE_SILENCE`] (with no transcript,
/// since its newest open dispatch was sent: the store keeps no start time of a controller),
/// unless its newest report waits on a person (a `blocked ci` report never holds it back); at
/// most once per [`IDLE_AGAIN`] per controller. Opens only a store that exists, read only.
pub(super) fn idle(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
    now: OffsetDateTime,
    at: &Timestamp,
    notes: &mut dyn Write,
) {
    let name = &instance.name.0;
    let state = Path::new(&instance.state);
    let store =
        match crate::store::open_reader_of(state, name, watch.sources.daemon_store.as_deref()) {
            Ok(store) => store,
            Err(error) => {
                watch.note(notes, format!("{name}: IDLE: {error:#}"));
                return;
            }
        };
    // Each repository's newest open dispatch: its id and when it was sent.
    let mut open: BTreeMap<String, (String, Option<OffsetDateTime>)> = BTreeMap::new();
    for dispatch in DispatchStorage::list(&store) {
        if !matches!(
            dispatch.state,
            DispatchState::Sent | DispatchState::Started | DispatchState::Blocked
        ) {
            continue;
        }
        let sent = instant(&dispatch.data.sent_at.0);
        let entry = open
            .entry(dispatch.data.repository.0.clone())
            .or_insert_with(|| (dispatch.data.dispatch_id.0.clone(), sent));
        if sent > entry.1 {
            *entry = (dispatch.data.dispatch_id.0.clone(), sent);
        }
    }
    let mut running: Vec<_> = ControllerStorage::list(&store)
        .into_iter()
        .filter(|controller| controller.state == ControllerState::Running)
        .collect();
    drop(store);
    running.sort_by(|one, other| one.data.session_name.0.cmp(&other.data.session_name.0));
    let box_projects = state
        .join(crate::placement::BOX_CLAUDE_DIR)
        .join("projects");
    let projects = if box_projects.is_dir() {
        box_projects
    } else {
        watch.sources.projects.clone()
    };
    let cursor_file = watch.cursors(name).join(IDLE_CURSOR);
    let mut named: BTreeMap<String, i64> = read(&cursor_file)
        .ok()
        .flatten()
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (session, when) = line.split_once(' ')?;
            Some((session.to_owned(), when.parse().ok()?))
        })
        .collect();
    let again = i64::try_from(IDLE_AGAIN.as_secs()).unwrap_or(i64::MAX);
    let mut silent = Vec::new();
    for controller in &running {
        let repository = &controller.data.repository.0;
        let session = &controller.data.session_name.0;
        let Some((dispatch, sent)) = open.get(repository) else {
            continue;
        };
        let last = match last_written(&projects, instance, repository) {
            Some(written) => written,
            None => match sent {
                Some(sent) => SystemTime::from(*sent),
                None => continue,
            },
        };
        let silence = SystemTime::from(now)
            .duration_since(last)
            .unwrap_or_default();
        if silence <= IDLE_SILENCE
            || named
                .get(session)
                .is_some_and(|when| now.unix_timestamp() - when < again)
        {
            continue;
        }
        silent.push((
            repository.as_str(),
            session.as_str(),
            dispatch.as_str(),
            silence,
        ));
    }
    let repositories: BTreeSet<&str> = silent.iter().map(|(repository, ..)| *repository).collect();
    let reports = newest_reports(instance, &repositories);
    let mut fired = false;
    for (repository, session, dispatch, silence) in silent {
        if let Some(report) = reports.get(repository) {
            let blocked_ci = report_words(first_line(report))
                .is_some_and(|words| words.starts_with(&["blocked", "ci"]));
            if !blocked_ci && waits_on_a_person(report) {
                continue;
            }
        }
        fired = true;
        named.insert(session.to_owned(), now.unix_timestamp());
        pass.fire(
            Kind::Idle,
            name,
            session,
            &format!(
                "silent {}m with the open dispatch {dispatch}",
                silence.as_secs() / 60
            ),
            at,
        );
    }
    if fired {
        let text: String = named
            .iter()
            .map(|(session, when)| format!("{session} {when}\n"))
            .collect();
        pass.write(cursor_file, text);
    }
}

/// The changes the mail and log triggers wake on: inotify on each instance's mailbox and
/// sessions directory that exists. Without inotify, a wait only sleeps.
pub(super) struct Changes {
    inotify: Option<Inotify>,
}

impl Changes {
    /// Watches the mailbox and sessions directories of `instances`.
    pub(super) fn new(instances: &[Instance]) -> Self {
        let inotify = Inotify::init(InitFlags::IN_NONBLOCK | InitFlags::IN_CLOEXEC).ok();
        if let Some(inotify) = &inotify {
            let mask = AddWatchFlags::IN_CREATE
                | AddWatchFlags::IN_MOVED_TO
                | AddWatchFlags::IN_MODIFY
                | AddWatchFlags::IN_CLOSE_WRITE;
            for instance in instances {
                for dir in [
                    mailbox(instance),
                    crate::placement::sessions_dir(Path::new(&instance.state)),
                ] {
                    if dir.is_dir() {
                        let _ = inotify.add_watch(&dir, mask);
                    }
                }
            }
        }
        Self { inotify }
    }

    /// Waits at most `wait` for a change; answers whether one came.
    pub(super) fn wait(&mut self, wait: Duration) -> bool {
        let Some(inotify) = &self.inotify else {
            std::thread::sleep(wait);
            return false;
        };
        let millis = i32::try_from(wait.as_millis()).unwrap_or(i32::MAX);
        let timeout = PollTimeout::try_from(millis).unwrap_or(PollTimeout::MAX);
        let mut fds = [PollFd::new(inotify.as_fd(), PollFlags::POLLIN)];
        match poll(&mut fds, timeout) {
            Ok(n) if n > 0 => {
                // Drain what is queued: the reads that follow see every change.
                while inotify.read_events().is_ok_and(|events| !events.is_empty()) {}
                true
            }
            Ok(_) => false,
            Err(_) => {
                std::thread::sleep(wait);
                false
            }
        }
    }
}
