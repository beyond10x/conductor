//! `conductor build run` (`story:build-run`) and the operator item a shortfall becomes
//! (`story:disk-escalation`).
//!
//! Each case runs [`build::run`] over a case directory: the store under `state/`, free disk
//! written as a byte count in `free.txt` and read through the same injectable reader the watch's
//! cases use (`Sources::free_bytes`, a `fn(&Path) -> Result<u64>` over `Sources::disk`), and a
//! fake reclaim (`Sources::reclaim`) that rewrites `free.txt` as freeing space would, and counts
//! its calls in `reclaims.txt`. The command is a real `sh -c` in its own process group, run in
//! `work/` of the case, so pausing it with `SIGSTOP` is seen as its `ticks` file stopping.

mod active;
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::build::{self, Outcome, Request, Settings, Sources};
use conductor_cli::gc::reclaim::ReclaimReport;
use conductor_cli::host;
use conductor_model::behaviour::DecisionRequestStorage as _;
use conductor_model::decision::{DecisionClass, DecisionRequestState};
use conductor_model::host::{GrantKind, HostGrantState, ReleaseReason};
use time::OffsetDateTime;

const GIB: u64 = 1 << 30;

/// One case: its directory, the state directory under it, and the command's working directory.
struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("build_run")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("work")).expect("create the case's work directory");
        // A Cargo project: the run's peak is measured on its `target/`.
        fs::write(
            dir.join("work/Cargo.toml"),
            "[package]\nname = \"example\"\n",
        )
        .expect("write the case's Cargo.toml");
        let case = Self { dir };
        case.free(100 * GIB);
        case
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn work(&self) -> PathBuf {
        self.dir.join("work")
    }

    /// Writes the free bytes the next reading sees.
    fn free(&self, bytes: u64) {
        replace(&self.dir.join("free.txt"), &bytes.to_string());
    }

    fn sources(&self, reclaim: fn(&Path, u64) -> anyhow::Result<ReclaimReport>) -> Sources {
        Sources {
            disk: self.dir.join("free.txt"),
            free_bytes: written_bytes,
            reclaim,
            clock: OffsetDateTime::now_utc,
            every: Duration::from_millis(50),
        }
    }

    fn request(&self, reserve: u64, script: &str) -> Request {
        Request {
            reserve,
            repository: Some("example-repo".to_owned()),
            session_name: None,
            command: vec!["sh".to_owned(), "-c".to_owned(), script.to_owned()],
            cwd: self.work(),
        }
    }

    /// How many times the fake reclaim ran.
    fn reclaims(&self) -> usize {
        fs::read_to_string(self.dir.join("reclaims.txt")).map_or(0, |text| text.lines().count())
    }

    fn grants(&self) -> Vec<conductor_model::host::Grants> {
        host::rows(&self.state()).expect("the ledger reads")
    }

    /// The class-H requests waiting for the operator's hands (`decision hands-todo`).
    fn hands_todo(&self) -> Vec<conductor_model::decision::DecisionRequestSnapshot> {
        let store = conductor_cli::store::open(&self.state()).expect("the store opens");
        store
            .list()
            .into_iter()
            .filter(|request| {
                request.state == DecisionRequestState::Escalated
                    && request.data.escalation_class == Some(DecisionClass::H)
            })
            .collect()
    }
}

/// The thresholds of every case: the floor (`thresholds.disk_admit`) 10G, `gate_stop` 5G and
/// `gate_resume` 8G.
fn settings() -> Settings {
    Settings {
        instance: "example-org".to_owned(),
        floor: 10 * GIB,
        gate_stop: 5 * GIB,
        gate_resume: 8 * GIB,
    }
}

fn written_bytes(file: &Path) -> anyhow::Result<u64> {
    Ok(fs::read_to_string(file)?.trim().parse()?)
}

fn replace(file: &Path, text: &str) {
    let next = file.with_extension("next");
    fs::write(&next, text).expect("write a case file");
    fs::rename(&next, file).expect("replace a case file");
}

/// The case directory of a state directory, as the fake reclaims are given it.
fn case_of(state: &Path) -> PathBuf {
    state
        .parent()
        .expect("the state lies in its case")
        .to_owned()
}

fn count_reclaim(state: &Path) {
    let file = case_of(state).join("reclaims.txt");
    let mut text = fs::read_to_string(&file).unwrap_or_default();
    text.push_str("reclaim\n");
    fs::write(file, text).expect("count the reclaim");
}

/// A reclaim that frees 20G, which reaches any size these cases ask for.
fn reclaim_frees_20g(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    count_reclaim(state);
    let free = case_of(state).join("free.txt");
    let now = written_bytes(&free)?;
    replace(&free, &(now + 20 * GIB).to_string());
    Ok(ReclaimReport {
        reclaimed_bytes: 20 * GIB,
        steps: vec![(
            "/example/trees/idle/target".to_owned(),
            conductor_model::host::ReclaimKind::IdleTreeBuildOutput,
            20 * GIB,
        )],
        left: Vec::new(),
        reached: true,
    })
}

/// A reclaim that frees 1G and leaves 2G only a person may remove.
fn reclaim_frees_1g(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    count_reclaim(state);
    let free = case_of(state).join("free.txt");
    let now = written_bytes(&free)?;
    replace(&free, &(now + GIB).to_string());
    Ok(ReclaimReport {
        reclaimed_bytes: GIB,
        steps: vec![(
            "/example/cache/old".to_owned(),
            conductor_model::host::ReclaimKind::Cache,
            GIB,
        )],
        left: vec![("/example/trees/live/target".to_owned(), 2 * GIB)],
        reached: false,
    })
}

/// A reclaim that finds nothing to free.
fn reclaim_nothing(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    count_reclaim(state);
    Ok(ReclaimReport {
        reclaimed_bytes: 0,
        steps: Vec::new(),
        left: Vec::new(),
        reached: false,
    })
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn lines(file: &Path) -> usize {
    fs::read_to_string(file).map_or(0, |text| text.lines().count())
}

#[test]
fn a_reservation_that_fits_runs_the_command_and_is_released_with_its_peak() {
    let case = Case::new("fits");
    let outcome = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(
            GIB,
            "mkdir -p target && head -c 1048576 /dev/zero > target/blob && exit 7",
        ),
    )
    .expect("the run ends");
    assert_eq!(
        outcome.exit_code(),
        7,
        "build run exits with the command's code"
    );
    let Outcome::Ran {
        ref grant,
        code,
        killed,
        ..
    } = outcome
    else {
        panic!("the reservation fits and the command runs: {outcome:?}");
    };
    assert_eq!((code, killed), (7, false), "the command's exit code");
    assert_eq!(
        case.reclaims(),
        0,
        "nothing was reclaimed for a reservation that fits"
    );
    let grants = case.grants();
    let row = grants
        .iter()
        .find(|row| row.grant_id == *grant)
        .expect("the ledger holds the grant");
    assert_eq!(row.kind, GrantKind::Disk);
    assert_eq!(row.state, HostGrantState::Released, "{row:?}");
    assert_eq!(row.release_reason, Some(ReleaseReason::Exited), "{row:?}");
    assert_eq!(row.bytes, Some(i64::try_from(GIB).unwrap()), "{row:?}");
    assert_eq!(row.reclaimed_bytes, Some(0), "{row:?}");
    assert!(
        row.peak_bytes.is_some_and(|peak| peak >= 1_048_576),
        "the peak of target/ is recorded: {row:?}"
    );
    assert!(row.process_group.is_some_and(|group| group > 0), "{row:?}");
    assert!(
        row.started_at.is_some() && row.released_at.is_some(),
        "{row:?}"
    );
    assert!(
        row.command
            .as_deref()
            .is_some_and(|command| command.starts_with("sh -c ")),
        "{row:?}"
    );
}

#[test]
fn a_reservation_that_does_not_fit_is_granted_once_reclaim_reaches_it() {
    let case = Case::new("reclaim-reaches");
    // 10.5G free over a 10G floor leaves 0.5G; 1G is asked.
    case.free(10 * GIB + GIB / 2);
    let outcome = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_frees_20g),
        &case.request(GIB, "touch ran"),
    )
    .expect("the run ends");
    let Outcome::Ran {
        ref grant,
        code,
        reclaimed,
        ..
    } = outcome
    else {
        panic!("reclaim reaches the size, so the command runs: {outcome:?}");
    };
    assert_eq!(code, 0);
    assert_eq!(reclaimed, 20 * GIB);
    assert_eq!(case.reclaims(), 1, "one reclaim run, before the answer");
    assert!(case.work().join("ran").exists(), "the command ran");
    let grants = case.grants();
    let row = grants.iter().find(|row| row.grant_id == *grant).unwrap();
    assert_eq!(row.reclaimed_bytes, Some(i64::try_from(20 * GIB).unwrap()));
    assert_eq!(row.state, HostGrantState::Released);
    assert!(
        case.hands_todo().is_empty(),
        "a reached size needs no hands"
    );
}

#[test]
fn a_shortfall_is_refused_with_exit_2_and_the_command_never_runs() {
    let case = Case::new("refused");
    // 10G free over a 10G floor leaves nothing; 4G is asked, reclaim frees 1G: 3G short.
    case.free(10 * GIB);
    let outcome = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_frees_1g),
        &case.request(4 * GIB, "touch ran"),
    )
    .expect("the run answers");
    let Outcome::Refused {
        ref grant,
        shortfall,
        ref left,
        ..
    } = outcome
    else {
        panic!("reclaim falls short, so the reservation is refused: {outcome:?}");
    };
    assert_eq!(shortfall, 3 * GIB, "the shortfall after reclaim");
    assert_eq!(
        left,
        &vec![("/example/trees/live/target".to_owned(), 2 * GIB)]
    );
    assert_eq!(outcome.exit_code(), 2, "a refusal exits 2");
    assert_eq!(case.reclaims(), 1);
    assert!(!case.work().join("ran").exists(), "the command never ran");
    let grants = case.grants();
    let row = grants.iter().find(|row| row.grant_id == *grant).unwrap();
    assert_eq!(row.state, HostGrantState::Refused, "{row:?}");
    assert_eq!(row.shortfall_bytes, Some(i64::try_from(3 * GIB).unwrap()));
    assert_eq!(row.reclaimed_bytes, Some(i64::try_from(GIB).unwrap()));
    assert!(
        row.started_at.is_none() && row.process_group.is_none(),
        "{row:?}"
    );
}

#[test]
fn a_shortfall_records_one_operator_item_with_its_numbers() {
    let case = Case::new("escalated");
    case.free(10 * GIB);
    let outcome = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_frees_1g),
        &case.request(4 * GIB, "touch ran"),
    )
    .expect("the run answers");
    let Outcome::Refused { item, .. } = &outcome else {
        panic!("refused: {outcome:?}");
    };
    let todo = case.hands_todo();
    assert_eq!(todo.len(), 1, "one operator item: {todo:?}");
    let request = &todo[0].data;
    assert_eq!(item.as_ref(), Some(&request.request_id.0), "{outcome:?}");
    assert_eq!(request.repository.0, "example-repo");
    assert!(
        request.question.contains("3G short") && request.question.contains("sh -c touch ran"),
        "the item names the shortfall and the command: {:?}",
        request.question
    );
    assert!(
        request.options.contains("/example/trees/live/target") && request.options.contains("2G"),
        "the item names what is left by path and size: {:?}",
        request.options
    );
    assert!(
        request
            .recommendation
            .contains("rm -rf -- /example/trees/live/target"),
        "the item names the command only a person runs: {:?}",
        request.recommendation
    );

    // A second refusal in the same fall records none.
    let again = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(4 * GIB, "touch ran"),
    )
    .expect("the second run answers");
    assert!(
        matches!(again, Outcome::Refused { item: None, .. }),
        "{again:?}"
    );
    assert_eq!(case.hands_todo().len(), 1, "still one item in this fall");

    // Once a reservation fits again the fall is over, and the next shortfall is a new item.
    case.free(100 * GIB);
    let fits = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the fitting run ends");
    assert_eq!(fits.exit_code(), 0, "{fits:?}");
    case.free(10 * GIB);
    let next = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(4 * GIB, "touch ran"),
    )
    .expect("the next run answers");
    assert!(
        matches!(next, Outcome::Refused { item: Some(_), .. }),
        "{next:?}"
    );
    assert_eq!(case.hands_todo().len(), 2, "a new fall, a new item");
}

#[test]
fn a_reading_under_gate_stop_pauses_the_group_and_one_over_gate_resume_resumes_it() {
    let case = Case::new("watchdog");
    let state = case.state();
    let sources = case.sources(reclaim_nothing);
    let request = case.request(
        GIB,
        "while [ ! -e stop ]; do echo tick >> ticks; sleep 0.05; done",
    );
    let runner = thread::spawn(move || build::run(&state, &settings(), &sources, &request));
    let ticks = case.work().join("ticks");
    wait_until("the command ticks", || lines(&ticks) >= 3);

    // Under gate_stop (5G): the group is stopped and reclaim runs.
    case.free(4 * GIB);
    wait_until("reclaim runs for the paused run", || case.reclaims() >= 1);
    thread::sleep(Duration::from_millis(300));
    let paused = lines(&ticks);
    thread::sleep(Duration::from_millis(500));
    assert_eq!(lines(&ticks), paused, "a stopped group writes nothing");

    // Between gate_stop and gate_resume (8G): still paused.
    case.free(6 * GIB);
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        lines(&ticks),
        paused,
        "under gate_resume the group stays stopped"
    );

    // Over gate_resume: continued.
    case.free(9 * GIB);
    wait_until("the command ticks again", || lines(&ticks) > paused + 2);
    fs::write(case.work().join("stop"), "").expect("stop the command");
    let outcome = runner
        .join()
        .expect("the runner ends")
        .expect("the run ends");
    let Outcome::Ran {
        code,
        paused,
        resumed,
        ..
    } = outcome
    else {
        panic!("the command ran: {outcome:?}");
    };
    assert_eq!(code, 0);
    assert_eq!((paused, resumed), (1, 1), "one pause, one resume");
}

#[test]
fn the_command_killed_releases_the_grant() {
    let case = Case::new("killed");
    let outcome = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "kill -9 $$"),
    )
    .expect("the run ends");
    let Outcome::Ran {
        ref grant,
        code,
        killed,
        ..
    } = outcome
    else {
        panic!("the command ran: {outcome:?}");
    };
    assert!(killed, "the command was killed: {outcome:?}");
    assert_eq!(code, 128 + 9, "a killed command exits 128 + its signal");
    let grants = case.grants();
    let row = grants.iter().find(|row| row.grant_id == *grant).unwrap();
    assert_eq!(row.state, HostGrantState::Released, "{row:?}");
    assert_eq!(row.release_reason, Some(ReleaseReason::Killed), "{row:?}");
}

#[test]
fn two_runs_at_once_each_reserve_and_the_second_sees_the_first() {
    let case = Case::new("two-at-once");
    // 12G free over a 10G floor: 2G to reserve.
    case.free(12 * GIB);
    let state = case.state();
    let sources = case.sources(reclaim_nothing);
    let first = case.request(GIB, "while [ ! -e stop ]; do sleep 0.05; done");
    let runner = thread::spawn(move || build::run(&state, &settings(), &sources, &first));
    wait_until("the first run is running", || {
        host::rows(&case.state())
            .is_ok_and(|rows| rows.iter().any(|row| row.state == HostGrantState::Running))
    });

    // The second fits beside the first: 2G − 1G leaves 1G.
    let second = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the second run ends");
    assert!(matches!(second, Outcome::Ran { code: 0, .. }), "{second:?}");

    // A third asks 1.5G while the first holds 1G: 0.5G short, which it would not be alone.
    let third = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB + GIB / 2, "touch third"),
    )
    .expect("the third run answers");
    assert!(
        matches!(third, Outcome::Refused { shortfall, .. } if shortfall == GIB / 2),
        "the third sees the first's reservation: {third:?}"
    );
    assert!(!case.work().join("third").exists());

    fs::write(case.work().join("stop"), "").expect("stop the first command");
    let first = runner
        .join()
        .expect("the runner ends")
        .expect("the run ends");
    assert!(matches!(first, Outcome::Ran { code: 0, .. }), "{first:?}");
    let live: Vec<_> = case
        .grants()
        .into_iter()
        .filter(|row| matches!(row.state, HostGrantState::Granted | HostGrantState::Running))
        .collect();
    assert!(live.is_empty(), "every grant is released: {live:?}");
}

#[test]
fn a_grant_whose_process_group_is_gone_is_released_by_the_next_run() {
    let case = Case::new("group-gone");
    // A process group that has ended: a child that led its own group and was waited for.
    let mut child = {
        use std::os::unix::process::CommandExt as _;
        Command::new("true")
            .process_group(0)
            .spawn()
            .expect("start true")
    };
    let group = i64::from(child.id());
    child.wait().expect("wait for true");
    let orphan = {
        let mut ledger = host::Ledger::lock(&case.state()).expect("the ledger opens");
        let orphan = ledger
            .request(
                "example-org",
                None,
                None,
                GIB,
                "sh -c 'sleep 600'",
                &host::now(OffsetDateTime::now_utc()),
            )
            .expect("a disk request is recorded");
        ledger
            .reserve(&orphan, 0, 0)
            .expect("the request is granted");
        orphan
    };
    host::start_run(
        &case.state(),
        &orphan,
        group,
        &host::now(OffsetDateTime::now_utc()),
    )
    .expect("the grant runs");

    let outcome = build::run(
        &case.state(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the run ends");
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}");
    let grants = case.grants();
    let row = grants.iter().find(|row| row.grant_id == orphan).unwrap();
    assert_eq!(row.state, HostGrantState::Released, "{row:?}");
    assert_eq!(
        row.release_reason,
        Some(ReleaseReason::ProcessGroupGone),
        "{row:?}"
    );
}

/// The binary: `host request-disk`, `reserve-disk`, `start-run`, `record-peak` and
/// `release-grant` move one grant, `host grants` lists it, and `build run` exits with its
/// command's code, free disk faked through `CONDUCTOR_DISK_READINGS`.
#[test]
fn the_binary_moves_a_grant_and_runs_a_build() {
    let case = Case::new("binary");
    let home = case.dir.join("home");
    fs::create_dir_all(&home).expect("create the case's home");
    let readings = case.dir.join("readings.txt");
    fs::write(&readings, format!("{}\n", 1000 * GIB)).expect("write the readings");
    let state = case.state();
    let run = |args: &[&str]| {
        common::conductor(&home)
            .arg("--state-dir")
            .arg(&state)
            .args(args)
            .env(conductor_cli::gc::reclaim::READINGS_VARIABLE, &readings)
            .current_dir(case.work())
            .output()
            .expect("conductor runs")
    };
    let requested = run(&[
        "host",
        "request-disk",
        "--instance",
        "example-org",
        "--bytes",
        "1024",
        "--command",
        "true",
        "--requested-at",
        "2026-10-13T12:00:00Z",
    ]);
    assert!(requested.status.success(), "{requested:?}");
    let grant = String::from_utf8(requested.stdout)
        .unwrap()
        .trim()
        .to_owned();
    let grant = grant.as_str();
    for args in [
        &[
            "host",
            "reserve-disk",
            "--grant-id",
            grant,
            "--shortfall-bytes",
            "0",
            "--reclaimed-bytes",
            "0",
        ][..],
        &[
            "host",
            "start-run",
            "--grant-id",
            grant,
            "--process-group",
            "4242",
            "--started-at",
            "2026-10-13T12:00:01Z",
        ][..],
        &[
            "host",
            "record-peak",
            "--grant-id",
            grant,
            "--peak-bytes",
            "512",
        ][..],
        &[
            "host",
            "release-grant",
            "--grant-id",
            grant,
            "--reason",
            "Exited",
            "--released-at",
            "2026-10-13T12:00:02Z",
        ][..],
    ] {
        let output = run(args);
        assert!(output.status.success(), "{args:?}: {output:?}");
    }
    let again = run(&[
        "host",
        "release-grant",
        "--grant-id",
        grant,
        "--released-at",
        "2026-10-13T12:00:03Z",
    ]);
    assert_eq!(again.status.code(), Some(1), "released twice: {again:?}");
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("HostGrantStateConflict"),
        "{again:?}"
    );

    let listed = run(&["host", "grants", "--format", "json"]);
    assert!(listed.status.success(), "{listed:?}");
    let rows: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json rows");
    assert_eq!(rows[0]["grant_id"], grant, "{rows}");
    assert_eq!(rows[0]["state"], "Released", "{rows}");
    assert_eq!(rows[0]["peak_bytes"], 512, "{rows}");
    assert_eq!(rows[0]["release_reason"], "Exited", "{rows}");

    let built = run(&[
        "build",
        "run",
        "--reserve",
        "1G",
        "--",
        "sh",
        "-c",
        "exit 3",
    ]);
    assert_eq!(built.status.code(), Some(3), "{built:?}");
    let listed = run(&["host", "grants", "--format", "json"]);
    let rows: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json rows");
    assert_eq!(rows[0]["command"], "sh -c exit 3", "newest first: {rows}");
    assert_eq!(rows[0]["state"], "Released", "{rows}");
    assert_eq!(rows[0]["bytes"], GIB, "{rows}");
}

#[test]
fn reserve_reads_a_size_in_gibibytes() {
    assert_eq!(build::gibibytes("8G"), Ok(8));
    assert_eq!(build::gibibytes("8"), Ok(8));
    assert!(build::gibibytes("4.2").is_err());
    assert!(build::gibibytes("x").is_err());
    assert!(build::gibibytes("8M").is_err());
}
