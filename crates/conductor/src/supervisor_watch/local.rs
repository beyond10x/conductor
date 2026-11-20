//! The eight host-local triggers of `conductor supervisor watch` (story:supervisor-watch-command):
//! each reads its source for every instance, adds what fired to a [`Pass`], and names the cursor
//! file and content [`Watch::commit`] writes once the lines are printed. A source that cannot be
//! read is a note, and the others are read.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write};
use std::os::fd::AsFd as _;
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

/// `MAIL`: every file of the instance's mailbox past its mail cursor. A first run with no cursor
/// starts at the newest file, and names how many it passed over.
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
        put(&cursor_file, &newest)?;
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
    let cursor = cursor.trim().to_owned();
    let mut newest = None;
    for file in names.iter().filter(|file| **file > cursor) {
        newest = Some(file.clone());
        let text = match fs::read(dir.join(file)) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => {
                watch.note(notes, format!("{name}: MAIL: read {file}: {error}"));
                continue;
            }
        };
        let line = first_line(&text);
        if report_words(line).is_some_and(|words| quiet(&words)) && !waits_on_a_person(&text) {
            continue;
        }
        pass.fire(Kind::Mail, name, file, line, at);
    }
    if let Some(newest) = newest {
        pass.write(cursor_file, newest);
    }
    Ok(())
}

/// `LOG`: each complete line of the instance's runtime log past its log cursor that carries one
/// of [`LOG_STRINGS`]. A log shorter than the cursor is read from its start; a first run with no
/// cursor starts at the log's end.
pub(super) fn log(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
    at: &Timestamp,
    notes: &mut dyn Write,
) -> Result<()> {
    let name = &instance.name.0;
    let path = runtime_log(instance);
    let cursor_file = watch.cursors(name).join(LOG_CURSOR);
    let mut file = match File::open(&path) {
        Ok(file) => Some(file),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            watch.note(
                notes,
                format!("{name}: LOG: read {}: {error}", path.display()),
            );
            return Ok(());
        }
    };
    let length = file
        .as_ref()
        .and_then(|file| file.metadata().ok())
        .map_or(0, |meta| meta.len());
    let Some(cursor) = read(&cursor_file)? else {
        put(&cursor_file, &length.to_string())?;
        return Ok(());
    };
    let mut cursor: u64 = cursor.trim().parse().unwrap_or(0);
    if length < cursor {
        cursor = 0;
        pass.write(cursor_file.clone(), "0");
    }
    let Some(file) = file.as_mut().filter(|_| length > cursor) else {
        return Ok(());
    };
    let mut bytes = Vec::new();
    let read_all = file
        .seek(SeekFrom::Start(cursor))
        .and_then(|_| file.read_to_end(&mut bytes));
    if let Err(error) = read_all {
        watch.note(
            notes,
            format!("{name}: LOG: read {}: {error}", path.display()),
        );
        return Ok(());
    }
    let Some(end) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        return Ok(());
    };
    let complete = String::from_utf8_lossy(&bytes[..=end]);
    for line in complete.lines() {
        if LOG_STRINGS.iter().any(|string| line.contains(string)) {
            pass.fire(Kind::Log, name, "", line, at);
        }
    }
    pass.write(cursor_file, (cursor + end as u64 + 1).to_string());
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

/// `DISK`: free bytes on the instance's `thresholds.disk_path` under its `disk_low` (once per
/// fall: `disk.low` is kept until free disk is back over it), or [`DISK_DROP`] or more under the
/// reading of the previous pass, which every pass rewrites.
pub(super) fn disk(
    watch: &mut Watch,
    instance: &Instance,
    pass: &mut Pass,
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
    let dir = watch.cursors(name);
    let baseline_file = dir.join(DISK_BASELINE);
    let previous: Option<u64> = read(&baseline_file)?.and_then(|text| text.trim().parse().ok());
    put(&baseline_file, &free.to_string())?;
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
        fs::remove_file(&marker).with_context(|| format!("remove {}", marker.display()))?;
    }
    if let Some(previous) = previous
        && previous.saturating_sub(free) >= DISK_DROP
    {
        parts.push(format!(
            "{}G under {}G at the last read",
            (previous - free) / GIB,
            previous / GIB
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
    Ok(found)
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

/// When the transcripts of a session working in `<root>/<repository>`, or in a managed tree of
/// it, were last written: the newest write of each `*.jsonl` of those project directories and of
/// its sub-agents ([`crate::watch::newest_write`]); none without a transcript.
fn last_written(projects: &Path, instance: &Instance, repository: &str) -> Option<SystemTime> {
    let checkout = project_name(&Path::new(&instance.checkouts.root).join(repository));
    let trees = format!(
        "{}-",
        project_name(&Path::new(&instance.checkouts.trees).join(repository))
    );
    let mut newest = None;
    for entry in fs::read_dir(projects).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name != checkout && !name.starts_with(&trees) {
            continue;
        }
        for transcript in fs::read_dir(entry.path()).into_iter().flatten().flatten() {
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

/// The text of the newest report of `repository` in the instance's mailbox, if any.
fn newest_report(instance: &Instance, repository: &str) -> Option<String> {
    let dir = mailbox(instance);
    let names = mail_files(&dir).ok()?;
    names.iter().rev().find_map(|name| {
        let text = fs::read_to_string(dir.join(name)).ok()?;
        let line = first_line(&text);
        (line.trim_start().starts_with("[REPORT ")
            && crate::message::named_repository(line).as_deref() == Some(repository))
        .then_some(text)
    })
}

/// `IDLE`: a controller the instance's store holds `Running`, with an open dispatch of its
/// repository, whose transcripts have not been written for [`IDLE_SILENCE`], unless its newest
/// report waits on a person (a `blocked ci` report never holds it back); at most once per
/// [`IDLE_AGAIN`] per controller. Opens only a store that exists, read only.
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
    let open: BTreeMap<String, String> = DispatchStorage::list(&store)
        .into_iter()
        .filter(|dispatch| {
            matches!(
                dispatch.state,
                DispatchState::Sent | DispatchState::Started | DispatchState::Blocked
            )
        })
        .map(|dispatch| (dispatch.data.repository.0, dispatch.data.dispatch_id.0))
        .collect();
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
    let mut fired = false;
    for controller in running {
        let repository = &controller.data.repository.0;
        let session = &controller.data.session_name.0;
        let Some(dispatch) = open.get(repository) else {
            continue;
        };
        let Some(written) = last_written(&projects, instance, repository) else {
            continue;
        };
        let silence = SystemTime::from(now)
            .duration_since(written)
            .unwrap_or_default();
        if silence <= IDLE_SILENCE {
            continue;
        }
        if let Some(report) = newest_report(instance, repository) {
            let blocked_ci = report_words(first_line(&report))
                .is_some_and(|words| words.starts_with(&["blocked", "ci"]));
            if !blocked_ci && waits_on_a_person(&report) {
                continue;
            }
        }
        if named
            .get(session)
            .is_some_and(|when| now.unix_timestamp() - when < again)
        {
            continue;
        }
        fired = true;
        named.insert(session.clone(), now.unix_timestamp());
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
