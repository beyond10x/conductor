//! `story:supervisor-watch-command` (issues #254, #247): `conductor supervisor watch` blocks
//! until any supervisor trigger fires on the host, prints one line per fired trigger, and moves
//! its cursors only once the lines are printed.
//!
//! The trigger cases call the library ([`Watch::pass`], [`Watch::commit`]) over its one seam,
//! [`Sources`]: a fixed clock, a free-bytes reader that reads the number written in the file the
//! instance's `thresholds.disk_path` names, a read interval of a few milliseconds, and a host
//! directory, instances, stores and git repositories made under the case's own directory. The
//! process's instance is fixed with `active::isolate` first (AGENTS.md rule 5); the config the
//! watch reads is parsed from a text the case writes, with its home under the case's directory.
//! git runs with `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_NOSYSTEM` set, its dates from
//! `GIT_COMMITTER_DATE`, and a bare local remote; no network. The exit-code cases start the
//! binary from `common::conductor` with a config file under the case's directory named in
//! `CONDUCTOR_CONFIG`, and `--timeout 1s`.

mod active;
mod common;
mod scratch;

use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use conductor_cli::config;
use conductor_cli::store;
use conductor_cli::supervisor_watch::{Pass, Sources, Watch};
use conductor_model::behaviour::{ControllerStorage, DispatchStorage, ReconciliationStorage};
use conductor_model::config::{Config, InstanceName, SessionName};
use conductor_model::dispatch::{
    ControllerData, ControllerId, ControllerSnapshot, ControllerState, DispatchData, DispatchId,
    DispatchSnapshot, DispatchState,
};
use conductor_model::host::WatchSupervisor;
use conductor_model::instance::{
    ReconcileAction, ReconcileReason, ReconcileResult, ReconciliationData, ReconciliationId,
    ReconciliationSnapshot, ReconciliationState,
};
use conductor_model::observation::{Harness, RepositoryName};
use conductor_model::primitives::{Timestamp, Uuid};
use scratch::Scratch;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const GIB: u64 = 1 << 30;

/// The instant every library case's clock stands at, unless it names another.
const NOON: &str = "2026-10-07T12:00:00Z";

fn at(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).expect("a fixed instant")
}

fn noon() -> OffsetDateTime {
    at(NOON)
}

fn noon_plus_10() -> OffsetDateTime {
    at("2026-10-07T12:10:00Z")
}

fn noon_plus_31() -> OffsetDateTime {
    at("2026-10-07T12:31:00Z")
}

fn noon_plus_62() -> OffsetDateTime {
    at("2026-10-07T13:02:00Z")
}

fn noon_plus_93() -> OffsetDateTime {
    at("2026-10-07T13:33:00Z")
}

/// The free bytes the file at `path` holds as a number: the case's disk.
fn free_bytes(path: &Path) -> anyhow::Result<u64> {
    Ok(fs::read_to_string(path)?.trim().parse()?)
}

/// A fresh directory of the case, removed when the case passes.
fn case(name: &str) -> Scratch {
    active::isolate();
    Scratch::new(Path::new("supervisor-watch").join(name))
}

/// The config text of the instances `alpha` and `beta` under `dir`, each with its disk the file
/// `<instance>/disk`, and `globals` written as `globals`. `alpha` runs in a box (`placement:
/// systemd`), whose runtime writes `runtime.log`; `beta` at `placement: none`, whose runtime
/// logs to its unit's journal.
fn config_text(dir: &Path, globals: &str) -> String {
    let mut text = String::from("version: conductor.config/2\n");
    text.push_str(globals);
    text.push_str("instances:\n");
    for (name, placement) in [("alpha", "systemd"), ("beta", "none")] {
        let base = dir.join(name);
        text.push_str(&format!(
            "  - name: {name}\n\
             \x20   placement: {placement}\n\
             \x20   session_prefix: {name}\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n\
             \x20   thresholds: {{disk_path: {disk:?}, disk_low: 10G, disk_clear: 12G}}\n",
            root = base.join("root"),
            trees = base.join("trees"),
            records = base.join("records"),
            state = base.join("state"),
            cache = base.join("cache"),
            disk = base.join("disk"),
        ));
    }
    text
}

fn parsed(dir: &Path, globals: &str) -> Config {
    let home = dir.join("home");
    config::parse(&config_text(dir, globals), &home)
        .unwrap_or_else(|problems| panic!("the case's config parses: {problems:?}"))
}

fn sources(dir: &Path, clock: fn() -> OffsetDateTime) -> Sources {
    Sources {
        host: dir.join("host"),
        projects: dir.join("projects"),
        daemon_store: None,
        clock,
        free_bytes,
        interval: Duration::from_millis(5),
        git: PathBuf::from("git"),
        journal: fake_journal(dir),
        gh: quiet_forge(),
        glab: quiet_forge(),
        forge_budget: Duration::from_secs(60),
    }
}

/// A forge program that answers every read with an empty list: these cases read no forge
/// (story:supervisor-watch-forge-triggers has its own, `tests/supervisor_watch_forge.rs`).
fn quiet_forge() -> Vec<String> {
    ["sh", "-c", "echo '[]'", "forge"]
        .map(str::to_owned)
        .to_vec()
}

/// A journal of the case: the unit `<unit>`'s entries are the lines of `<dir>/journal/<unit>`,
/// its cursor the number of lines; `--after-cursor <n>` prints the lines after the n-th, `-n 1`
/// the last. Run with `sh`, as `journalctl` would answer `--show-cursor`.
fn fake_journal(dir: &Path) -> Vec<String> {
    let script = dir.join("fake-journal");
    fs::write(
        &script,
        "f=\"$1\"; shift\n\
         after=\"\"; last=\"\"\n\
         while [ $# -gt 0 ]; do\n\
         \x20 case \"$1\" in\n\
         \x20   --after-cursor) after=\"$2\"; shift;;\n\
         \x20   -n) last=\"$2\"; shift;;\n\
         \x20 esac\n\
         \x20 shift\n\
         done\n\
         [ -f \"$f\" ] || exit 0\n\
         total=$(wc -l < \"$f\" | tr -d ' ')\n\
         [ \"$total\" -gt 0 ] || exit 0\n\
         if [ -n \"$after\" ]; then tail -n +\"$((after + 1))\" \"$f\";\n\
         elif [ -n \"$last\" ]; then tail -n \"$last\" \"$f\";\n\
         else cat \"$f\"; fi\n\
         echo \"-- cursor: $total\"\n",
    )
    .expect("write the fake journal");
    vec![
        "sh".to_owned(),
        script.display().to_string(),
        dir.join("journal").join("{unit}").display().to_string(),
    ]
}

/// Appends `lines` to the journal of the unit of `instance`'s runtime.
fn journal(dir: &Path, instance: &str, lines: &str) {
    let dir = dir.join("journal");
    fs::create_dir_all(&dir).expect("create the journal");
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(conductor_cli::daemon::reconcile::unit_name(instance)))
        .and_then(|mut file| file.write_all(lines.as_bytes()))
        .expect("append to the journal");
}

/// A mail file name: the ULID of `ms` milliseconds since the epoch and the randomness `random`
/// (16 characters of Crockford's base 32), then `.md`.
fn ulid(ms: u64, random: &str) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut time = String::new();
    for n in (0..10).rev() {
        time.push(ALPHABET[((ms >> (5 * n)) & 31) as usize] as char);
    }
    format!("{time}{random}.md")
}

fn input() -> WatchSupervisor {
    WatchSupervisor {
        instance: Vec::new(),
        checkout: Vec::new(),
        timeout: None,
        hold: Vec::new(),
        format: None,
    }
}

fn watch(config: &Config, args: &WatchSupervisor, sources: Sources) -> Watch {
    Watch::new(config, args, Path::new("/"), sources).expect("the watch starts")
}

/// One pass, its lines as the command prints them, and its notes.
fn lines(watch: &mut Watch) -> (Pass, Vec<String>, String) {
    let mut notes = Vec::new();
    let pass = watch.pass(&mut notes).expect("a pass");
    let lines = pass
        .sorted()
        .iter()
        .map(|trigger| conductor_cli::supervisor_watch::render(trigger, false))
        .collect();
    (pass, lines, String::from_utf8_lossy(&notes).into_owned())
}

fn mailbox(dir: &Path, instance: &str) -> PathBuf {
    let mailbox = dir.join(instance).join("state/mailbox/conductor");
    fs::create_dir_all(&mailbox).expect("create the mailbox");
    mailbox
}

fn mail(mailbox: &Path, name: &str, text: &str) {
    fs::write(mailbox.join(name), text).expect("write a mail file");
}

// -------------------------------------------------------------------------------------------
// MAIL
// -------------------------------------------------------------------------------------------

/// A first run passes over the mail already there and says how many; every new file is listed,
/// not only the first, in arrival order; a new run resumes from the committed cursor.
#[test]
fn mail_lists_every_new_file_and_resumes_from_its_cursor_across_runs() {
    let scratch = case("mail-resumes");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let mailbox = mailbox(&dir, "alpha");
    mail(
        &mailbox,
        "01JA0000000000000000000001.md",
        "[NEED app R1] lib old\n",
    );

    let mut first = watch(&config, &input(), sources(&dir, noon));
    let (pass, found, notes) = lines(&mut first);
    assert!(found.is_empty(), "a first run lists no old mail: {found:?}");
    assert!(
        notes.contains("alpha") && notes.contains("passed over 1 mail file"),
        "a first run names how many mail files it passed over: {notes}"
    );
    first.commit(&pass).expect("commit");

    mail(
        &mailbox,
        "01JA0000000000000000000002.md",
        "[NEED app R2] lib two\nbody\n",
    );
    mail(
        &mailbox,
        "01JA0000000000000000000003.md",
        "[NEED app R3] lib three\n",
    );
    let (pass, found, _) = lines(&mut first);
    assert_eq!(
        found,
        vec![
            "MAIL alpha 01JA0000000000000000000002.md [NEED app R2] lib two",
            "MAIL alpha 01JA0000000000000000000003.md [NEED app R3] lib three",
        ]
    );
    first.commit(&pass).expect("commit");
    drop(first);

    let mut second = watch(&config, &input(), sources(&dir, noon));
    let (_, found, notes) = lines(&mut second);
    assert!(
        found.is_empty(),
        "a new run lists nothing handled: {found:?}"
    );
    assert!(
        !notes.contains("passed over"),
        "only a first run passes over mail: {notes}"
    );
    mail(
        &mailbox,
        "01JA0000000000000000000004.md",
        "[NEED app R4] lib four\n",
    );
    let (_, found, _) = lines(&mut second);
    assert_eq!(
        found,
        vec!["MAIL alpha 01JA0000000000000000000004.md [NEED app R4] lib four"]
    );
}

/// A report `progress`, `started`, `unblocked` or `blocked ci` is listed only when its text
/// waits on a person; a skipped file still moves the cursor.
#[test]
fn mail_skips_a_progress_report_unless_it_waits_on_a_person() {
    let scratch = case("mail-skips");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let mailbox = mailbox(&dir, "alpha");
    let mut watch = watch(&config, &input(), sources(&dir, noon));
    let (pass, _, _) = lines(&mut watch);
    watch.commit(&pass).expect("commit");

    let files = [
        (
            "01JB0000000000000000000001.md",
            "[REPORT app DSP-1] progress tests pass\n",
        ),
        (
            "01JB0000000000000000000002.md",
            "[REPORT app DSP-1] started on it\n",
        ),
        (
            "01JB0000000000000000000003.md",
            "[REPORT app DSP-1] unblocked again\n",
        ),
        (
            "01JB0000000000000000000004.md",
            "[REPORT app DSP-1] blocked ci runners queued\n",
        ),
        (
            "01JB0000000000000000000005.md",
            "[REPORT app DSP-1] progress half done\nThis WAITS ON the operator's answer.\n",
        ),
        (
            "01JB0000000000000000000006.md",
            "[REPORT app DSP-1] blocked ci waiting for a person to approve the run\n",
        ),
        (
            "01JB0000000000000000000007.md",
            "[REPORT app DSP-1] done merged\n",
        ),
        (
            "01JB0000000000000000000008.md",
            "[REPORT app DSP-1] blocked review\n",
        ),
    ];
    for (name, text) in files {
        mail(&mailbox, name, text);
    }
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec![
            "MAIL alpha 01JB0000000000000000000005.md [REPORT app DSP-1] progress half done",
            "MAIL alpha 01JB0000000000000000000006.md [REPORT app DSP-1] blocked ci waiting for a \
             person to approve the run",
            "MAIL alpha 01JB0000000000000000000007.md [REPORT app DSP-1] done merged",
            "MAIL alpha 01JB0000000000000000000008.md [REPORT app DSP-1] blocked review",
        ]
    );
    watch.commit(&pass).expect("commit");
    let (_, found, _) = lines(&mut watch);
    assert!(
        found.is_empty(),
        "the skipped files moved the cursor too: {found:?}"
    );
}

// -------------------------------------------------------------------------------------------
// LOG
// -------------------------------------------------------------------------------------------

/// Every trigger string of the runtime log is named past the cursor, a line not yet complete
/// waits for its end, and a log shorter than the cursor is read from its start.
#[test]
fn log_names_each_trigger_string_past_the_cursor_and_rereads_a_truncated_log() {
    let scratch = case("log");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let sessions = dir.join("alpha/state/sessions");
    fs::create_dir_all(&sessions).expect("create the sessions directory");
    let log = sessions.join("runtime.log");
    fs::write(&log, "old: usage limit before the first run\n").expect("write the log");
    let mut watch = watch(&config, &input(), sources(&dir, noon));
    let (pass, found, _) = lines(&mut watch);
    assert!(
        found.is_empty(),
        "a first run starts at the log's end: {found:?}"
    );
    watch.commit(&pass).expect("commit");

    let strings = [
        "context: alpha-conductor 81%",
        "spawn app: LaunchFailed exit 1",
        "cycle: no open dispatch for app",
        "decision: the request lists no lettered option",
        "report: DispatchNotFound DSP-9",
        "cycle: snapshot exited 1",
        "app: usage limit reached",
        "session: start .claude.json unreadable",
        "dispatch v1: app waits: ControllerLimit",
    ];
    let mut text = String::new();
    for (n, line) in strings.iter().enumerate() {
        text.push_str(&format!("[t{n}]: {line}\n"));
        text.push_str("[t]: a quiet line\n");
    }
    text.push_str("[t9]: app: usage limit, still being written");
    OpenOptions::new()
        .append(true)
        .open(&log)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .expect("append to the log");
    let (pass, found, _) = lines(&mut watch);
    let expected: Vec<String> = strings
        .iter()
        .enumerate()
        .map(|(n, line)| format!("LOG alpha - [t{n}]: {line}"))
        .collect();
    assert_eq!(found, expected);
    watch.commit(&pass).expect("commit");

    OpenOptions::new()
        .append(true)
        .open(&log)
        .and_then(|mut file| file.write_all(b"\n"))
        .expect("end the line");
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec!["LOG alpha - [t9]: app: usage limit, still being written"]
    );
    watch.commit(&pass).expect("commit");

    fs::write(&log, "[n]: LaunchFailed after a restart\n").expect("truncate the log");
    let (_, found, _) = lines(&mut watch);
    assert_eq!(found, vec!["LOG alpha - [n]: LaunchFailed after a restart"]);
}

// -------------------------------------------------------------------------------------------
// DAEMON
// -------------------------------------------------------------------------------------------

fn reconciliation(
    id: &str,
    instance: &str,
    action: ReconcileAction,
    reason: ReconcileReason,
    when: &str,
) -> ReconciliationSnapshot {
    ReconciliationSnapshot {
        state: ReconciliationState::Recorded,
        data: ReconciliationData {
            reconciliation_id: ReconciliationId(Uuid(id.to_owned())),
            instance: InstanceName(instance.to_owned()),
            action,
            reason,
            result: ReconcileResult::Succeeded,
            detail: None,
            actor: conductor_cli::actor::daemon(),
            at: Timestamp(when.to_owned()),
        },
    }
}

/// A `LostRuntime` reconciliation and an operator's upgrade of the instance past the daemon
/// cursor are named; another reason is not, and a handled record is not named again.
#[test]
fn daemon_names_a_lost_runtime_and_an_operator_upgrade() {
    let scratch = case("daemon");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let url = format!("sqlite:{}", dir.join("daemon.sqlite").display());
    {
        let mut daemon = store::open_url(&url, "daemon").expect("open the daemon's store");
        ReconciliationStorage::put(
            &mut daemon,
            reconciliation(
                "00000000-0000-4000-8000-000000000001",
                "alpha",
                ReconcileAction::Restart,
                ReconcileReason::LostRuntime,
                "2026-10-07T10:00:00Z",
            ),
        );
        daemon.check().expect("kept");
    }
    let mut sources = sources(&dir, noon);
    sources.daemon_store = Some(url.clone());
    let mut watch = watch(&config, &input(), sources);
    let (pass, found, _) = lines(&mut watch);
    assert!(
        found.is_empty(),
        "a first run starts after the records there: {found:?}"
    );
    watch.commit(&pass).expect("commit");
    {
        let mut daemon = store::open_url(&url, "daemon").expect("open the daemon's store");
        for record in [
            reconciliation(
                "00000000-0000-4000-8000-000000000002",
                "alpha",
                ReconcileAction::Restart,
                ReconcileReason::LostRuntime,
                "2026-10-07T11:00:00Z",
            ),
            reconciliation(
                "00000000-0000-4000-8000-000000000003",
                "alpha",
                ReconcileAction::Upgrade,
                ReconcileReason::OperatorCommand,
                "2026-10-07T11:10:00Z",
            ),
            reconciliation(
                "00000000-0000-4000-8000-000000000004",
                "alpha",
                ReconcileAction::Restart,
                ReconcileReason::ConfigChange,
                "2026-10-07T11:20:00Z",
            ),
            reconciliation(
                "00000000-0000-4000-8000-000000000005",
                "beta",
                ReconcileAction::Start,
                ReconcileReason::LostRuntime,
                "2026-10-07T11:30:00Z",
            ),
        ] {
            ReconciliationStorage::put(&mut daemon, record);
        }
        daemon.check().expect("kept");
    }
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec![
            "DAEMON alpha 2026-10-07T11:00:00Z Restart LostRuntime Succeeded",
            "DAEMON alpha 2026-10-07T11:10:00Z Upgrade OperatorCommand Succeeded",
            "DAEMON beta 2026-10-07T11:30:00Z Start LostRuntime Succeeded",
        ]
    );
    watch.commit(&pass).expect("commit");
    let (_, found, _) = lines(&mut watch);
    assert!(
        found.is_empty(),
        "a handled record is not named again: {found:?}"
    );
}

// -------------------------------------------------------------------------------------------
// INBOX
// -------------------------------------------------------------------------------------------

/// The inbox holding a request id the outbox does not fires, once per new id, naming the first
/// unanswered id and how many there are; with either key unset the trigger is off.
#[test]
fn inbox_fires_when_requests_outnumber_answers_and_is_off_without_both_keys() {
    let scratch = case("inbox");
    let dir = scratch.path().to_owned();
    let inbox = dir.join("inbox.txt");
    let outbox = dir.join("outbox.txt");
    fs::write(&inbox, "R1\nR2\n").expect("write the inbox");
    fs::write(&outbox, "R1\n").expect("write the outbox");
    let both = parsed(
        &dir,
        &format!("globals:\n  supervisor: {{inbox: {inbox:?}, outbox: {outbox:?}}}\n"),
    );
    let mut watch = watch(&both, &input(), sources(&dir, noon));
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(found, vec!["INBOX - R2 1 request unanswered"]);
    watch.commit(&pass).expect("commit");
    let (_, found, _) = lines(&mut watch);
    assert!(
        found.is_empty(),
        "an id named once is not named again: {found:?}"
    );
    fs::write(&inbox, "R1\nR2\nR3\n").expect("write the inbox");
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(found, vec!["INBOX - R2 2 requests unanswered"]);
    watch.commit(&pass).expect("commit");
    fs::write(&outbox, "R1\nR2\nR3\n").expect("write the outbox");
    let (_, found, _) = lines(&mut watch);
    assert!(found.is_empty(), "every request answered: {found:?}");

    fs::write(&inbox, "R1\nR4\n").expect("write the inbox");
    let only_inbox = parsed(
        &dir,
        &format!("globals:\n  supervisor: {{inbox: {inbox:?}}}\n"),
    );
    let mut watch = super_watch(&only_inbox, &dir);
    let (_, found, _) = lines(&mut watch);
    assert!(found.is_empty(), "off without an outbox: {found:?}");
    let none = parsed(&dir, "");
    let mut watch = super_watch(&none, &dir);
    let (_, found, _) = lines(&mut watch);
    assert!(found.is_empty(), "off without either key: {found:?}");
}

fn super_watch(config: &Config, dir: &Path) -> Watch {
    watch(config, &input(), sources(dir, noon))
}

// -------------------------------------------------------------------------------------------
// DISK
// -------------------------------------------------------------------------------------------

fn set_disk(dir: &Path, gib: u64) {
    for instance in ["alpha", "beta"] {
        let path = dir.join(instance);
        fs::create_dir_all(&path).expect("create the instance's directory");
        fs::write(path.join("disk"), (gib * GIB).to_string()).expect("write the disk");
    }
}

/// Free disk under `thresholds.disk_low` fires once per fall; a drop of 5 GiB or more under the
/// baseline fires, and a slow fill fires too: the baseline moves only when a DISK line fires or
/// once an hour, never on a quiet 30 s pass.
#[test]
fn disk_fires_under_disk_low_and_five_gigabytes_under_the_last_baseline() {
    let scratch = case("disk");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let args = WatchSupervisor {
        instance: vec!["alpha".to_owned()],
        ..input()
    };
    let step = |clock: fn() -> OffsetDateTime, gib: u64| {
        set_disk(&dir, gib);
        let mut watch = watch(&config, &args, sources(&dir, clock));
        let (pass, found, _) = lines(&mut watch);
        watch.commit(&pass).expect("commit");
        found
            .into_iter()
            .filter(|line| line.starts_with("DISK"))
            .collect::<Vec<_>>()
    };
    let disk = dir.join("alpha/disk").display().to_string();
    assert!(
        step(noon, 100).is_empty(),
        "the first reading is the baseline"
    );
    assert!(step(noon, 98).is_empty(), "2G under the baseline");
    assert!(step(noon, 96).is_empty(), "4G under the baseline");
    assert_eq!(
        step(noon, 94),
        vec![format!(
            "DISK alpha {disk} 94G free, 6G under the baseline 100G"
        )],
        "a slow fill fires: the quiet passes moved no baseline"
    );
    assert!(
        step(noon, 91).is_empty(),
        "the line moved the baseline to 94G"
    );
    assert!(
        step(noon_plus_62, 90).is_empty(),
        "4G under 94G; an hour on, the baseline moves to 90G"
    );
    assert_eq!(
        step(noon_plus_62, 85),
        vec![format!(
            "DISK alpha {disk} 85G free, 5G under the baseline 90G"
        )]
    );
    assert_eq!(
        step(noon_plus_62, 8),
        vec![format!(
            "DISK alpha {disk} 8G free, under disk_low 10G; 77G under the baseline 85G"
        )]
    );
    assert!(
        step(noon_plus_62, 8).is_empty(),
        "once per fall under disk_low"
    );
    assert!(step(noon_plus_62, 50).is_empty(), "back over disk_low");
    assert_eq!(
        step(noon_plus_62, 9),
        vec![format!("DISK alpha {disk} 9G free, under disk_low 10G")]
    );
}

// -------------------------------------------------------------------------------------------
// UNPUSHED
// -------------------------------------------------------------------------------------------

fn git(dir: &Path, args: &[&str], date: &str) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=example",
            "-c",
            "user.email=example@example-org.invalid",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn commit(dir: &Path, file: &str, date: &str) -> String {
    fs::write(dir.join(file), date).expect("write a file");
    git(dir, &["add", file], date);
    git(dir, &["commit", "-q", "-m", file], date);
    git(dir, &["rev-parse", "--short", "HEAD"], date)
}

/// A commit on a local branch that no remote branch holds, over 30 minutes old, is named, in a
/// checkout under `checkouts.root` and in a `--checkout` of no instance; a younger one is not;
/// a branch is named again only once its head moves.
#[test]
fn unpushed_names_a_local_only_commit_older_than_thirty_minutes_including_a_named_checkout() {
    let scratch = case("unpushed");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let remote = dir.join("remote.git");
    fs::create_dir_all(&remote).expect("create the remote");
    git(&remote, &["init", "-q", "--bare"], "2026-10-07T09:00:00Z");
    let root = dir.join("alpha/root");
    fs::create_dir_all(&root).expect("create the checkouts root");
    let app = root.join("app");
    fs::create_dir_all(&app).expect("create the checkout");
    git(&app, &["init", "-q"], "2026-10-07T09:00:00Z");
    git(
        &app,
        &["remote", "add", "origin", &remote.display().to_string()],
        "2026-10-07T09:00:00Z",
    );
    commit(&app, "a", "2026-10-07T10:00:00Z");
    git(
        &app,
        &["push", "-q", "origin", "main"],
        "2026-10-07T10:00:00Z",
    );
    git(
        &app,
        &["checkout", "-q", "-b", "feature"],
        "2026-10-07T11:50:00Z",
    );
    commit(&app, "d", "2026-10-07T11:50:00Z");
    git(&app, &["checkout", "-q", "main"], "2026-10-07T11:50:00Z");
    let old = commit(&app, "b", "2026-10-07T11:15:00Z");
    commit(&app, "c", "2026-10-07T11:20:00Z");

    let product = dir.join("product");
    fs::create_dir_all(&product).expect("create the product checkout");
    git(&product, &["init", "-q"], "2026-10-07T09:00:00Z");
    let product_old = commit(&product, "p", "2026-10-07T11:00:00Z");

    let args = WatchSupervisor {
        checkout: vec![product.display().to_string()],
        ..input()
    };
    let mut watch = watch(&config, &args, sources(&dir, noon));
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec![
            format!(
                "UNPUSHED - {}:main {product_old} 60m old, 1 commit",
                product.display()
            ),
            format!(
                "UNPUSHED alpha {}:main {old} 45m old, 2 commits",
                app.display()
            ),
        ],
        "the feature branch's one commit is 10 minutes old"
    );
    watch.commit(&pass).expect("commit");
    let (_, found, _) = lines(&mut watch);
    assert!(found.is_empty(), "a branch head named once: {found:?}");
    commit(&app, "e", "2026-10-07T11:25:00Z");
    let (_, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec![format!(
            "UNPUSHED alpha {}:main {old} 45m old, 3 commits",
            app.display()
        )]
    );
}

// -------------------------------------------------------------------------------------------
// IDLE
// -------------------------------------------------------------------------------------------

/// The project directory Claude Code keeps the transcripts of a session working in `cwd` in.
fn project(projects: &Path, cwd: &Path) -> PathBuf {
    let name: String = cwd
        .display()
        .to_string()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    projects.join(name)
}

/// A controller `Running` with an open dispatch whose transcript has not been written for 15
/// minutes is named, at most once per 30 minutes, and not while its newest report waits on a
/// person; a `blocked ci` report never holds it back.
#[test]
fn idle_names_a_silent_running_controller_unless_it_waits_on_a_person_once_per_half_hour() {
    let scratch = case("idle");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let state = dir.join("alpha/state");
    {
        let mut handle = store::open(&state).expect("open alpha's store");
        ControllerStorage::put(
            &mut handle,
            ControllerSnapshot {
                state: ControllerState::Running,
                data: ControllerData {
                    actor: conductor_cli::actor::daemon(),
                    controller_id: ControllerId(Uuid(
                        "11111111-1111-4111-8111-111111111111".to_owned(),
                    )),
                    repository: RepositoryName("app".to_owned()),
                    harness: Harness::Claude,
                    session_name: SessionName("alpha-app".to_owned()),
                    charter_revision: 1,
                },
            },
        );
        ControllerStorage::put(
            &mut handle,
            ControllerSnapshot {
                state: ControllerState::Running,
                data: ControllerData {
                    actor: conductor_cli::actor::daemon(),
                    controller_id: ControllerId(Uuid(
                        "22222222-2222-4222-8222-222222222222".to_owned(),
                    )),
                    repository: RepositoryName("lib".to_owned()),
                    harness: Harness::Claude,
                    session_name: SessionName("alpha-lib".to_owned()),
                    charter_revision: 1,
                },
            },
        );
        DispatchStorage::put(
            &mut handle,
            DispatchSnapshot {
                state: DispatchState::Started,
                data: DispatchData {
                    dispatch_id: DispatchId("DSP-20261007-01".to_owned()),
                    actor: conductor_cli::actor::daemon(),
                    repository: RepositoryName("app".to_owned()),
                    goal_id: None,
                    brief: "fix it".to_owned(),
                    sent_at: Timestamp("2026-10-07T09:00:00Z".to_owned()),
                    priority: 1,
                    waits_on: None,
                    reference: None,
                    start_ref: None,
                    issue: None,
                    links: Vec::new(),
                    waiting: None,
                },
            },
        );
        handle.check().expect("kept");
    }
    let projects = dir.join("projects");
    let silent = SystemTime::from(at("2026-10-07T11:40:00Z"));
    for repository in ["app", "lib"] {
        let transcripts = project(&projects, &dir.join("alpha/root").join(repository));
        fs::create_dir_all(&transcripts).expect("create the project directory");
        let transcript = transcripts.join("session.jsonl");
        fs::write(&transcript, "{}\n").expect("write a transcript");
        File::options()
            .write(true)
            .open(&transcript)
            .and_then(|file| file.set_modified(silent))
            .expect("date the transcript");
    }
    let line = |minutes: u64| {
        format!("IDLE alpha alpha-app silent {minutes}m with the open dispatch DSP-20261007-01")
    };
    let run = |clock: fn() -> OffsetDateTime| {
        let mut watch = watch(&config, &input(), sources(&dir, clock));
        let (pass, found, _) = lines(&mut watch);
        watch.commit(&pass).expect("commit");
        found
    };
    assert_eq!(run(noon), vec![line(20)], "lib has no open dispatch");
    assert!(run(noon_plus_10).is_empty(), "named once per 30 minutes");
    assert_eq!(run(noon_plus_31), vec![line(51)]);
    let mailbox = mailbox(&dir, "alpha");
    mail(
        &mailbox,
        "01JC0000000000000000000001.md",
        "[REPORT app DSP-20261007-01] blocked review waiting for the operator's review\n",
    );
    mail(
        &mailbox,
        "01JC0000000000000000000002.md",
        "[REPORT lib DSP-20261007-02] done\n",
    );
    let idle = |found: Vec<String>| -> Vec<String> {
        found
            .into_iter()
            .filter(|line| line.starts_with("IDLE"))
            .collect()
    };
    let mut again = watch(&config, &input(), sources(&dir, noon_plus_62));
    let (pass, found, _) = lines(&mut again);
    again.commit(&pass).expect("commit");
    assert!(
        idle(found).is_empty(),
        "its newest report waits on a person"
    );
    mail(
        &mailbox,
        "01JC0000000000000000000003.md",
        "[REPORT app DSP-20261007-01] blocked ci waiting for runners\n",
    );
    let mut last = watch(&config, &input(), sources(&dir, noon_plus_93));
    let (_, found, _) = lines(&mut last);
    assert_eq!(
        idle(found),
        vec![line(113)],
        "a blocked ci report never holds it back"
    );
}

// -------------------------------------------------------------------------------------------
// The binary: timeout, refusals, the lock, what it writes
// -------------------------------------------------------------------------------------------

/// A config file of the instances `alpha` and `beta` under `dir`, whose disks are `dir` with a
/// `disk_low` of 1G, so the host's own disk fires nothing.
fn config_file(dir: &Path) -> PathBuf {
    let text = config_text(dir, "")
        .replace(
            &format!("{:?}", dir.join("alpha/disk")),
            &format!("{:?}", dir),
        )
        .replace(
            &format!("{:?}", dir.join("beta/disk")),
            &format!("{:?}", dir),
        )
        .replace(
            "disk_low: 10G, disk_clear: 12G",
            "disk_low: 1G, disk_clear: 1G",
        );
    let file = dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    file
}

/// The binary over the config file `file`, with a fake `journalctl` first on its `PATH`, so the
/// unboxed instance `beta` reads no journal of the host (AGENTS.md rule 5). The fake, written by
/// the case into `<dir>/bin`, appends its arguments to `<dir>/bin/journal-read` and answers one
/// cursor. Fake `gh` and `glab` beside it answer every read with an empty list, so no forge is
/// read.
fn binary(dir: &Path, file: &Path) -> Command {
    use std::os::unix::fs::PermissionsExt as _;
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create the home");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("create the case's bin");
    let fake = bin.join("journalctl");
    fs::write(
        &fake,
        format!(
            "#!/bin/sh\necho \"$*\" >> {read:?}\necho \"-- cursor: fake-1\"\n",
            read = bin.join("journal-read")
        ),
    )
    .expect("write the fake journalctl");
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    for forge in ["gh", "glab"] {
        let fake = bin.join(forge);
        fs::write(&fake, "#!/bin/sh\necho '[]'\n").expect("write a fake forge program");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&path));
    let mut command = common::conductor(&home);
    command
        .env(config::CONFIG_VARIABLE, file)
        .env("PATH", std::env::join_paths(paths).expect("a PATH"))
        .current_dir(dir)
        .args(["supervisor", "watch"]);
    command
}

#[test]
fn timeout_prints_one_timeout_line_and_exits_zero() {
    let scratch = case("timeout");
    let dir = scratch.path().to_owned();
    let file = config_file(&dir);
    let output = binary(&dir, &file)
        .args(["--timeout", "1s"])
        .output()
        .expect("run the watch");
    assert!(
        output.status.success(),
        "exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "TIMEOUT - - -\n");
    let read = fs::read_to_string(dir.join("bin/journal-read")).expect("the fake journal was read");
    let unit = conductor_cli::daemon::reconcile::unit_name("beta");
    assert!(
        read.lines()
            .any(|line| line.starts_with("--user -u ") && line.contains(&unit)),
        "beta's journal is read through the fake journalctl: {read:?}"
    );
    let output = binary(&dir, &file)
        .args(["--timeout", "1s", "--format", "json"])
        .output()
        .expect("run the watch");
    assert!(output.status.success());
    let line: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one JSON object");
    assert_eq!(line["trigger"], "Timeout");
    assert_eq!(line["instance"], "-");
}

#[test]
fn a_bad_timeout_or_unknown_instance_exits_one_naming_the_error() {
    let scratch = case("refusals");
    let dir = scratch.path().to_owned();
    let file = config_file(&dir);
    for (args, error) in [
        (
            &["--timeout", "5x"][..],
            "conductor.host.InvalidWatchTimeout",
        ),
        (
            &["--timeout", "0s"][..],
            "conductor.host.InvalidWatchTimeout",
        ),
        (
            &["--timeout", "m"][..],
            "conductor.host.InvalidWatchTimeout",
        ),
        (
            &["--timeout", "1s", "--instance", "nobody"][..],
            "conductor.instance.UnknownInstance",
        ),
    ] {
        let output = binary(&dir, &file)
            .args(args)
            .output()
            .expect("run the watch");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{args:?}: exit 1: {stderr}");
        assert!(output.stdout.is_empty(), "{args:?}: prints no trigger");
        assert_eq!(stderr.lines().count(), 1, "{args:?}: one line: {stderr}");
        assert!(
            stderr.starts_with(error),
            "{args:?}: names {error}: {stderr}"
        );
    }
}

#[test]
fn a_second_run_exits_one_while_the_first_holds_the_lock() {
    let scratch = case("lock");
    let dir = scratch.path().to_owned();
    let file = config_file(&dir);
    let mut first = binary(&dir, &file)
        .args(["--timeout", "60s"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start the first watch");
    let lock = dir.join("host/supervisor-watch/lock");
    let pid = first.id().to_string();
    let deadline = Instant::now() + Duration::from_secs(30);
    while fs::read_to_string(&lock)
        .ok()
        .map(|text| text.trim().to_owned())
        != Some(pid.clone())
    {
        assert!(Instant::now() < deadline, "the first watch takes the lock");
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = binary(&dir, &file)
        .args(["--timeout", "1s"])
        .output()
        .expect("run the second watch");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "exit 1: {stderr}");
    assert!(
        stderr.starts_with("conductor.host.SupervisorWatchRunning") && stderr.contains(&pid),
        "names the error and the first run's pid: {stderr}"
    );
    first.kill().expect("stop the first watch");
    first.wait().expect("reap the first watch");
    let output = binary(&dir, &file)
        .args(["--timeout", "1s"])
        .output()
        .expect("run the third watch");
    assert!(
        output.status.success(),
        "a killed run blocks no next one: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The lines of a pass that is never committed are listed again by the next run.
#[test]
fn a_run_stopped_before_it_prints_advances_no_cursor() {
    let scratch = case("stopped");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let mailbox = mailbox(&dir, "alpha");
    let mut first = watch(&config, &input(), sources(&dir, noon));
    let (pass, _, _) = lines(&mut first);
    first.commit(&pass).expect("commit the baseline");
    let cursor = dir.join("host/supervisor-watch/alpha/mail");
    let before = fs::read_to_string(&cursor).expect("the mail cursor");
    mail(
        &mailbox,
        "01JD0000000000000000000001.md",
        "[NEED app R1] lib one\n",
    );
    let (_, found, _) = lines(&mut first);
    assert_eq!(
        found,
        vec!["MAIL alpha 01JD0000000000000000000001.md [NEED app R1] lib one"]
    );
    drop(first);
    assert_eq!(
        fs::read_to_string(&cursor).expect("the mail cursor"),
        before,
        "no cursor moved before the lines were printed"
    );
    let mut second = watch(&config, &input(), sources(&dir, noon));
    let (_, found, _) = lines(&mut second);
    assert_eq!(
        found,
        vec!["MAIL alpha 01JD0000000000000000000001.md [NEED app R1] lib one"],
        "the next run lists them again"
    );
}

/// Every file under `dir`, relative to it.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("read a directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(path.strip_prefix(dir).expect("under dir").to_owned());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn the_watch_creates_no_store_and_writes_only_under_the_host_directory() {
    let scratch = case("writes");
    let dir = scratch.path().to_owned();
    let file = config_file(&dir);
    let output = binary(&dir, &file)
        .args(["--timeout", "1s"])
        .output()
        .expect("run the watch");
    assert!(
        output.status.success(),
        "exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for instance in ["alpha", "beta"] {
        assert!(
            !dir.join(instance).exists(),
            "{instance}: no state, records or store is created"
        );
    }
    for written in files(&dir) {
        assert!(
            written == Path::new("conductor.yaml")
                || written.starts_with("host/supervisor-watch")
                || written.starts_with("bin"),
            "{} is written outside supervisor-watch/ of the host directory",
            written.display()
        );
    }
}

// -------------------------------------------------------------------------------------------
// Review findings (wave 69 adversary review)
// -------------------------------------------------------------------------------------------

/// An unboxed instance's runtime logs to its unit's journal, which LOG reads past a persisted
/// journal cursor; a boxed instance with no `runtime.log` is one note, not silence.
#[test]
fn log_reads_the_unit_journal_of_an_unboxed_instance_and_notes_a_missing_log() {
    let scratch = case("log-journal");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    journal(
        &dir,
        "beta",
        "2026-10-07T10:00 host conductor[1]: usage limit, old\n",
    );
    let mut watch = watch(&config, &input(), sources(&dir, noon));
    let (pass, found, notes) = lines(&mut watch);
    assert!(
        found.is_empty(),
        "a first run starts at the journal's end: {found:?}"
    );
    assert!(
        notes.contains("alpha") && notes.contains("runtime.log"),
        "the boxed instance's missing runtime.log is a note: {notes}"
    );
    watch.commit(&pass).expect("commit");
    journal(
        &dir,
        "beta",
        "2026-10-07T11:00 host conductor[1]: context: beta-app 81%\n\
         2026-10-07T11:01 host conductor[1]: a quiet line\n",
    );
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec!["LOG beta - 2026-10-07T11:00 host conductor[1]: context: beta-app 81%"]
    );
    watch.commit(&pass).expect("commit");
    let mut again = super_watch(&config, &dir);
    let (_, found, _) = lines(&mut again);
    assert!(found.is_empty(), "the journal cursor persists: {found:?}");
}

/// A mail file whose ULID sorts below the cursor, made a little earlier or in the same
/// millisecond but written later, is still listed once: the watch re-reads a window of a few
/// seconds below its cursor and keeps the names it handled there.
#[test]
fn mail_lists_a_file_that_lands_below_the_cursor_within_its_window() {
    let scratch = case("mail-window");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let mailbox = mailbox(&dir, "alpha");
    let mut watch = watch(&config, &input(), sources(&dir, noon));
    let (pass, _, _) = lines(&mut watch);
    watch.commit(&pass).expect("commit the baseline");
    const T: u64 = 1_791_370_800_000;
    let later = ulid(T + 1000, "0000000000000001");
    mail(&mailbox, &later, "[NEED app R1] lib later\n");
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec![format!("MAIL alpha {later} [NEED app R1] lib later")]
    );
    watch.commit(&pass).expect("commit");
    let earlier = ulid(T, "0000000000000002");
    let same = ulid(T + 1000, "0000000000000000");
    mail(&mailbox, &earlier, "[NEED app R2] lib earlier\n");
    mail(&mailbox, &same, "[NEED app R3] lib same millisecond\n");
    let (pass, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec![
            format!("MAIL alpha {earlier} [NEED app R2] lib earlier"),
            format!("MAIL alpha {same} [NEED app R3] lib same millisecond"),
        ]
    );
    watch.commit(&pass).expect("commit");
    let mut again = super_watch(&config, &dir);
    let (_, found, _) = lines(&mut again);
    assert!(found.is_empty(), "each file once: {found:?}");
}

/// A mail file that cannot be read is listed with the error, never passed over in silence.
#[test]
fn an_unreadable_mail_file_is_listed_with_its_error() {
    use std::os::unix::fs::PermissionsExt as _;
    let scratch = case("mail-unreadable");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let mailbox = mailbox(&dir, "alpha");
    let mut watch = watch(&config, &input(), sources(&dir, noon));
    let (pass, _, _) = lines(&mut watch);
    watch.commit(&pass).expect("commit the baseline");
    let name = "01JE0000000000000000000001.md";
    mail(&mailbox, name, "[NEED app R1] lib\n");
    fs::set_permissions(mailbox.join(name), fs::Permissions::from_mode(0o000))
        .expect("make the file unreadable");
    let (_, found, _) = lines(&mut watch);
    fs::set_permissions(mailbox.join(name), fs::Permissions::from_mode(0o644))
        .expect("make the file readable again");
    assert_eq!(found.len(), 1, "one line: {found:?}");
    assert!(
        found[0].starts_with(&format!("MAIL alpha {name} unreadable: ")),
        "names the file and the error: {found:?}"
    );
}

fn running(handle: &mut store::Store, id: &str, repository: &str) {
    ControllerStorage::put(
        handle,
        ControllerSnapshot {
            state: ControllerState::Running,
            data: ControllerData {
                actor: conductor_cli::actor::daemon(),
                controller_id: ControllerId(Uuid(id.to_owned())),
                repository: RepositoryName(repository.to_owned()),
                harness: Harness::Claude,
                session_name: SessionName(format!("alpha-{repository}")),
                charter_revision: 1,
            },
        },
    );
}

fn open_dispatch(handle: &mut store::Store, id: &str, repository: &str, sent_at: &str) {
    DispatchStorage::put(
        handle,
        DispatchSnapshot {
            state: DispatchState::Sent,
            data: DispatchData {
                dispatch_id: DispatchId(id.to_owned()),
                actor: conductor_cli::actor::daemon(),
                repository: RepositoryName(repository.to_owned()),
                goal_id: None,
                brief: "work".to_owned(),
                sent_at: Timestamp(sent_at.to_owned()),
                priority: 1,
                waits_on: None,
                reference: None,
                start_ref: None,
                issue: None,
                links: Vec::new(),
                waiting: None,
            },
        },
    );
}

fn transcript(directory: &Path, written: &str) {
    fs::create_dir_all(directory).expect("create the project directory");
    let file = directory.join("session.jsonl");
    fs::write(&file, "{}\n").expect("write a transcript");
    File::options()
        .write(true)
        .open(&file)
        .and_then(|file| file.set_modified(SystemTime::from(at(written))))
        .expect("date the transcript");
}

/// IDLE reads the transcripts of the controller's own checkout and trees, never another
/// repository's whose project name it is a prefix of; and a Running controller with no
/// transcript at all is silent since its open dispatch was sent.
#[test]
fn idle_reads_only_its_own_trees_and_counts_from_the_dispatch_without_a_transcript() {
    let scratch = case("idle-trees");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    {
        let mut handle = store::open(&dir.join("alpha/state")).expect("open alpha's store");
        running(&mut handle, "11111111-1111-4111-8111-111111111111", "app");
        running(&mut handle, "22222222-2222-4222-8222-222222222222", "lib");
        open_dispatch(
            &mut handle,
            "DSP-20261007-01",
            "app",
            "2026-10-07T09:00:00Z",
        );
        open_dispatch(
            &mut handle,
            "DSP-20261007-02",
            "lib",
            "2026-10-07T09:00:00Z",
        );
        handle.check().expect("kept");
    }
    let projects = dir.join("projects");
    let trees = dir.join("alpha/trees");
    let own = trees.join("app/t1");
    fs::create_dir_all(&own).expect("create app's tree");
    transcript(&project(&projects, &own), "2026-10-07T11:40:00Z");
    transcript(
        &project(&projects, &trees.join("app-bar/t1")),
        "2026-10-07T11:59:00Z",
    );
    let mut watch = watch(&config, &input(), sources(&dir, noon));
    let (_, found, _) = lines(&mut watch);
    let idle: Vec<&String> = found
        .iter()
        .filter(|line| line.starts_with("IDLE"))
        .collect();
    assert_eq!(
        idle,
        vec![
            "IDLE alpha alpha-app silent 20m with the open dispatch DSP-20261007-01",
            "IDLE alpha alpha-lib silent 180m with the open dispatch DSP-20261007-02",
        ]
    );
}

/// A run that prints nothing but `TIMEOUT` still moves the cursors of what it read: the log and
/// the mail it passed over are not read again.
#[test]
fn a_quiet_pass_and_a_timeout_move_the_cursors_past_what_they_read() {
    let scratch = case("quiet");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let sessions = dir.join("alpha/state/sessions");
    fs::create_dir_all(&sessions).expect("create the sessions directory");
    let log = sessions.join("runtime.log");
    fs::write(&log, "").expect("write the log");
    let mailbox = mailbox(&dir, "alpha");
    let args = WatchSupervisor {
        instance: vec!["alpha".to_owned()],
        timeout: Some("1s".to_owned()),
        ..input()
    };
    let mut first = watch(&config, &args, sources(&dir, noon));
    let (pass, _, _) = lines(&mut first);
    first.commit(&pass).expect("commit the baseline");
    fs::write(&log, "[t]: a quiet line\n".repeat(100)).expect("write quiet lines");
    let skipped = "01JF0000000000000000000001.md";
    mail(&mailbox, skipped, "[REPORT app DSP-1] progress fine\n");
    let mut out = Vec::new();
    let mut notes = Vec::new();
    first.run(false, &mut out, &mut notes).expect("run");
    assert_eq!(String::from_utf8_lossy(&out), "TIMEOUT - - -\n");
    let cursors = dir.join("host/supervisor-watch/alpha");
    let log_cursor = fs::read_to_string(cursors.join("log")).expect("the log cursor");
    assert_eq!(
        log_cursor.split_whitespace().next(),
        Some("1800"),
        "the log cursor is past the quiet lines: {log_cursor:?}"
    );
    let mail_cursor = fs::read_to_string(cursors.join("mail")).expect("the mail cursor");
    assert_eq!(
        mail_cursor.lines().next(),
        Some(skipped),
        "past the skipped report"
    );
}

/// A log rewritten as a new file longer than the cursor, its start not the start the cursor
/// read, is read from its start.
#[test]
fn log_reads_a_rewritten_log_from_its_start() {
    let scratch = case("log-rewritten");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let sessions = dir.join("alpha/state/sessions");
    fs::create_dir_all(&sessions).expect("create the sessions directory");
    let log = sessions.join("runtime.log");
    fs::write(&log, "[a]: the first log\n".repeat(5)).expect("write the log");
    let mut watch = watch(&config, &input(), sources(&dir, noon));
    let (pass, _, _) = lines(&mut watch);
    watch.commit(&pass).expect("commit the baseline");
    fs::write(
        &log,
        format!(
            "[b]: LaunchFailed in the new log\n{}",
            "[b]: a quiet line\n".repeat(10)
        ),
    )
    .expect("rewrite the log");
    let (_, found, _) = lines(&mut watch);
    assert_eq!(found, vec!["LOG alpha - [b]: LaunchFailed in the new log"]);
}

/// `--timeout` above a day is refused as `InvalidWatchTimeout`, never added to the clock.
#[test]
fn a_timeout_over_a_day_is_refused() {
    use conductor_cli::supervisor_watch::timeout;
    assert_eq!(timeout("24h").ok(), Some(Duration::from_secs(24 * 3600)));
    for text in ["25h", "86401s", "5000000000h"] {
        assert!(timeout(text).is_err(), "{text} is refused");
    }
}

/// UNPUSHED sees the commits of a detached HEAD in a worktree of a named checkout, which no
/// branch holds.
#[test]
fn unpushed_names_a_detached_head_in_a_worktree_of_a_named_checkout() {
    let scratch = case("unpushed-detached");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, "");
    let product = dir.join("product");
    fs::create_dir_all(&product).expect("create the product checkout");
    git(&product, &["init", "-q"], "2026-10-07T09:00:00Z");
    let first = commit(&product, "p", "2026-10-07T11:00:00Z");
    let tree = dir.join("product-tree");
    git(
        &product,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &tree.display().to_string(),
        ],
        "2026-10-07T11:05:00Z",
    );
    commit(&tree, "q", "2026-10-07T11:10:00Z");
    let args = WatchSupervisor {
        checkout: vec![product.display().to_string()],
        ..input()
    };
    let mut watch = watch(&config, &args, sources(&dir, noon));
    let (_, found, _) = lines(&mut watch);
    assert_eq!(
        found,
        vec![
            format!(
                "UNPUSHED - {}:main {first} 60m old, 1 commit",
                product.display()
            ),
            format!(
                "UNPUSHED - {}:HEAD {first} 60m old, 2 commits",
                tree.display()
            ),
        ]
    );
}
