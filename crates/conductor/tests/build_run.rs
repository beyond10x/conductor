//! `conductor build run` (`story:build-run`) and the operator item a shortfall becomes
//! (`story:disk-escalation`).
//!
//! Each case runs [`build::run`] over a case directory: the store under `state/`, free disk
//! written as a byte count in `free.txt` and read through the same injectable reader the watch's
//! cases use (`Sources::free_bytes`, a `fn(&Path) -> Result<u64>` over `Sources::disk`), and a
//! fake reclaim (`Sources::reclaim`) that rewrites `free.txt` as freeing space would, and counts
//! its calls in `reclaims.txt`. The command is a real `sh -c` in its own process group, run in
//! `work/` of the case, so pausing it with `SIGSTOP` is seen as its `ticks` file stopping. The
//! home directory and `/var/tmp` a disk item is written against are `home/example-user/` and
//! `var-tmp/` of the case (`Sources::home`, `Sources::var_tmp`); nothing reads the real ones.
//! What a run writes to standard error goes to `stderr.txt` of the case (`Sources::say`).

mod active;
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::build::{self, Outcome, Request, Settings, Sources};
use conductor_cli::gc::reclaim::{READINGS_VARIABLE, ReclaimReport, VAR_TMP_VARIABLE};
use conductor_cli::host;
use conductor_model::behaviour::DecisionRequestStorage as _;
use conductor_model::decision::{DecisionClass, DecisionRequestState};
use conductor_model::host::{GrantKind, HostGrantState, ReleaseReason};
use nix::sys::signal::{Signal, kill, killpg};
use nix::unistd::Pid;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

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

    /// The host directory, which holds the ledger (`story:instance-isolation`).
    fn host(&self) -> PathBuf {
        self.dir.join("host")
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
            home: Some(self.home()),
            var_tmp: self.var_tmp(),
            say: said_to_file,
            env: |_| None,
            // No session list: a library case names no harness session.
            sessions: Vec::new(),
        }
    }

    /// What the runs of the case wrote to standard error (`Sources::say`).
    fn said(&self) -> String {
        fs::read_to_string(self.dir.join("stderr.txt")).unwrap_or_default()
    }

    /// The case's home directory, named after its user.
    fn home(&self) -> PathBuf {
        self.dir.join("home").join("example-user")
    }

    /// The case's `/var/tmp`.
    fn var_tmp(&self) -> PathBuf {
        self.dir.join("var-tmp")
    }

    fn request_in(&self, repository: &str, reserve: u64, script: &str) -> Request {
        Request {
            repository: Some(repository.to_owned()),
            ..self.request(reserve, script)
        }
    }

    /// Every disk request of the store, in any state, with its state.
    fn disk_items(&self) -> Vec<conductor_model::decision::DecisionRequestSnapshot> {
        let store = conductor_cli::store::open(&self.state()).expect("the store opens");
        store
            .list()
            .into_iter()
            .filter(|request| request.data.request_id.0.starts_with("disk-"))
            .collect()
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
        host::rows(&self.host()).expect("the ledger reads")
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

/// The thresholds of every case: the floor (`thresholds.disk_admit`) 10G, `disk_clear` 20G,
/// `gate_stop` 5G and `gate_resume` 8G.
fn settings() -> Settings {
    Settings {
        instance: "example-org".to_owned(),
        floor: 10 * GIB,
        clear: 20 * GIB,
        gate_stop: 5 * GIB,
        gate_resume: 8 * GIB,
        wait: Duration::ZERO,
        cores: 20,
        cpu_reserve: 2,
    }
}

fn written_bytes(file: &Path) -> anyhow::Result<u64> {
    Ok(fs::read_to_string(file)?.trim().parse()?)
}

/// Appends `line` to `stderr.txt` of the case.
fn said_to_file(state: &Path, line: &str) {
    use std::io::Write as _;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(case_of(state).join("stderr.txt"))
        .expect("open the case's stderr.txt");
    writeln!(file, "{line}").expect("write the case's stderr.txt");
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
        &case.host(),
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

/// story:build-run-spare-cores: the command runs at nice 19 (field 19 of its /proc stat), in the
/// idle I/O class, with the job counts of the cores less the reserve.
#[test]
fn the_command_runs_at_nice_19_with_the_spare_cores_as_its_job_count() {
    let case = Case::new("spare-cores");
    let script = "awk '{print $19}' /proc/self/stat > nice.txt; \
                  echo \"$CARGO_BUILD_JOBS $MAKEFLAGS\" > jobs.txt; \
                  if command -v ionice >/dev/null; then ionice -p $$ > io.txt; fi";
    let outcome = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, script),
    )
    .expect("the run ends");
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}");
    let read = |name: &str| fs::read_to_string(case.work().join(name)).unwrap_or_default();
    assert_eq!(read("nice.txt").trim(), "19", "the command's own nice");
    assert_eq!(read("jobs.txt").trim(), "18 -j18", "20 cores less 2");
    if case.work().join("io.txt").exists() {
        assert!(read("io.txt").starts_with("idle"), "{}", read("io.txt"));
    }
}

/// A job count the run's environment already holds is kept, the other is set; a reserve over the
/// cores leaves one job.
#[test]
fn a_job_count_the_environment_holds_is_kept_and_a_large_reserve_leaves_one() {
    let case = Case::new("spare-cores-kept");
    let script = "echo \"$CARGO_BUILD_JOBS $MAKEFLAGS\" > jobs.txt";
    let jobs = || fs::read_to_string(case.work().join("jobs.txt")).unwrap_or_default();
    let run = |settings: &Settings, sources: Sources| {
        build::run(
            &case.state(),
            &case.host(),
            settings,
            &sources,
            &case.request(GIB, script),
        )
        .expect("the run ends")
    };
    // The environment holds CARGO_BUILD_JOBS (as the child inherits it): it is left as it is.
    let inherited = std::env::var("CARGO_BUILD_JOBS").unwrap_or_default();
    let holding = Sources {
        env: |name| (name == "CARGO_BUILD_JOBS").then(|| "7".to_owned()),
        ..case.sources(reclaim_nothing)
    };
    run(&settings(), holding);
    // Trimmed on both sides: with no CARGO_BUILD_JOBS in this test's own environment the line
    // starts with the empty value and a space.
    assert_eq!(jobs().trim(), format!("{inherited} -j18").trim());
    let tight = Settings {
        cores: 2,
        cpu_reserve: 5,
        ..settings()
    };
    run(&tight, case.sources(reclaim_nothing));
    assert_eq!(jobs().trim(), "1 -j1", "never fewer than one job");
}

#[test]
fn a_reservation_that_does_not_fit_is_granted_once_reclaim_reaches_it() {
    let case = Case::new("reclaim-reaches");
    // 10.5G free over a 10G floor leaves 0.5G; 1G is asked.
    case.free(10 * GIB + GIB / 2);
    let outcome = build::run(
        &case.state(),
        &case.host(),
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
        &case.host(),
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
        &case.host(),
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
    assert_eq!(
        request.repository.0, "example-org",
        "the item is the fall's, of the instance, not one repository's"
    );
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
        request.recommendation.contains(
            "review and remove what is no longer needed, largest first: /example/trees/live/target"
        ) && !request.recommendation.contains("rm -rf"),
        "the item names what a person reviews, and no removal command to paste: {:?}",
        request.recommendation
    );
    // `story:hand-item-dry-run`: the class-H item carries its dry run, the numbers conductor
    // measured with no credential.
    let dry_run = request.dry_run.as_deref().unwrap_or_default();
    assert!(
        dry_run.contains("3G short")
            && dry_run.contains("asked 4G")
            && dry_run.contains("/example/trees/live/target (2G)"),
        "the item's dry run is its numbers: {request:?}"
    );
    assert_eq!(request.no_dry_run, None, "{request:?}");

    // A second refusal in the same fall records none.
    let again = build::run(
        &case.state(),
        &case.host(),
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

    // Once free disk is back at `disk_clear` the fall is over, and the next shortfall is a new item.
    let first = item.clone().expect("the first refusal recorded the item");
    case.free(100 * GIB);
    let fits = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the fitting run ends");
    assert_eq!(fits.exit_code(), 0, "{fits:?}");
    case.free(10 * GIB);
    let next = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(4 * GIB, "touch ran"),
    )
    .expect("the next run answers");
    let Outcome::Refused {
        item: Some(second), ..
    } = &next
    else {
        panic!("a new fall, a new item: {next:?}");
    };
    assert_ne!(second, &first, "a new fall, a new item");
    let todo: Vec<String> = case
        .hands_todo()
        .into_iter()
        .map(|request| request.data.request_id.0)
        .collect();
    assert_eq!(
        todo,
        vec![second.clone()],
        "the first fall's item was withdrawn"
    );
}

/// `story:disk-item-per-fall`: refusals for different commands and repositories in one fall of
/// free disk share one item, even when a smaller reservation fits between them while free disk
/// stays under `disk_clear`.
#[test]
fn refusals_in_one_fall_for_different_commands_and_repositories_record_one_item() {
    let case = Case::new("one-item-per-fall");
    case.free(10 * GIB);
    let first = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_frees_1g),
        &case.request_in("example-repo", 4 * GIB, "touch first"),
    )
    .expect("the first run answers");
    assert!(
        matches!(first, Outcome::Refused { item: Some(_), .. }),
        "{first:?}"
    );

    // 11G free over a 10G floor: a 1G build fits, and free disk is still under `disk_clear`.
    let small = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request_in("example-small", GIB, "true"),
    )
    .expect("the small run ends");
    assert_eq!(small.exit_code(), 0, "{small:?}");

    let second = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request_in("example-other", 6 * GIB, "touch second"),
    )
    .expect("the second run answers");
    assert!(
        matches!(second, Outcome::Refused { item: None, .. }),
        "the fall has its item already: {second:?}"
    );
    let todo = case.hands_todo();
    assert_eq!(todo.len(), 1, "one item for the fall: {todo:?}");
    assert_eq!(case.disk_items().len(), 1, "{:?}", case.disk_items());
}

/// `story:disk-item-per-fall`: the fall's item is withdrawn on its own once free disk is back at
/// `disk_clear`: by the next `build run`, or by the watch's pass ([`host::clear_fall`]), with no
/// refusal or person in between; under `disk_clear` it stays.
#[test]
fn the_falls_item_is_withdrawn_once_free_disk_is_back_at_disk_clear() {
    let case = Case::new("withdrawn-at-disk-clear");
    case.free(10 * GIB);
    let refused = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(4 * GIB, "touch ran"),
    )
    .expect("the run answers");
    let Outcome::Refused {
        item: Some(item), ..
    } = refused
    else {
        panic!("refused with an item: {refused:?}");
    };
    let state_of = |id: &str| {
        case.disk_items()
            .into_iter()
            .find(|request| request.data.request_id.0 == id)
            .map(|request| request.state)
    };

    // 15G free: over the floor, under `disk_clear` (20G): a build fits, the item stays.
    case.free(15 * GIB);
    let fits = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the run ends");
    assert_eq!(fits.exit_code(), 0, "{fits:?}");
    assert_eq!(
        state_of(&item),
        Some(DecisionRequestState::Escalated),
        "under disk_clear the fall goes on"
    );

    // 20G free: back at `disk_clear`; the next run withdraws the item before it reserves.
    case.free(20 * GIB);
    let fits = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the run ends");
    assert_eq!(fits.exit_code(), 0, "{fits:?}");
    assert_eq!(state_of(&item), Some(DecisionRequestState::Withdrawn));
    assert!(case.hands_todo().is_empty(), "{:?}", case.hands_todo());
    assert!(!case.host().join("host.fall").exists());

    // A new fall, cleared by the watch's pass alone: no build runs in between.
    case.free(10 * GIB);
    let refused = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(4 * GIB, "touch ran"),
    )
    .expect("the run answers");
    let Outcome::Refused {
        item: Some(next), ..
    } = refused
    else {
        panic!("a new fall, a new item: {refused:?}");
    };
    assert_eq!(
        host::clear_fall(
            &case.host(),
            &case.state(),
            "example-org",
            19 * GIB,
            20 * GIB
        )
        .expect("the pass reads"),
        None,
        "under disk_clear nothing is withdrawn"
    );
    assert_eq!(state_of(&next), Some(DecisionRequestState::Escalated));
    assert_eq!(
        host::clear_fall(
            &case.host(),
            &case.state(),
            "example-org",
            30 * GIB,
            20 * GIB
        )
        .expect("the pass withdraws"),
        Some(next.clone())
    );
    assert_eq!(state_of(&next), Some(DecisionRequestState::Withdrawn));
    assert_eq!(
        host::clear_fall(
            &case.host(),
            &case.state(),
            "example-org",
            30 * GIB,
            20 * GIB
        )
        .expect("a second pass reads"),
        None,
        "withdrawn once"
    );
}

/// A reclaim that frees 1G and leaves, only a person may remove, an 8G cache under the case's
/// home, three `/var/tmp` entries of the case: one named after the user (3G), one after a
/// third party's project (2G), and the smaller ones in one row; and the rustup toolchains under
/// the home (1G), which are no scratch or cache, so the shortfall still needs a person
/// (`story:reclaim-scratch-class-c`).
fn reclaim_leaves_personal_paths(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    count_reclaim(state);
    let case = case_of(state);
    let free = case.join("free.txt");
    let now = written_bytes(&free)?;
    replace(&free, &(now + GIB).to_string());
    let var_tmp = case.join("var-tmp");
    let home = case.join("home").join("example-user");
    Ok(ReclaimReport {
        reclaimed_bytes: GIB,
        steps: Vec::new(),
        left: vec![
            (home.join(".cache/sccache").display().to_string(), 8 * GIB),
            (var_tmp.join("example-user").display().to_string(), 3 * GIB),
            (
                var_tmp.join("example-project").display().to_string(),
                2 * GIB,
            ),
            (var_tmp.display().to_string(), GIB / 4),
            (home.join(".rustup/toolchains").display().to_string(), GIB),
        ],
        reached: false,
    })
}

/// What a reclaim that frees 1G leaves when only scratch and caches are left, none of them stale
/// or each held: the agents' scratch (3G) and a cache (2G) under the case's home, a `/var/tmp`
/// entry (1G) and the smaller `/var/tmp` entries in one row.
fn scratch_left(state: &Path) -> anyhow::Result<Vec<(String, u64)>> {
    count_reclaim(state);
    let case = case_of(state);
    let free = case.join("free.txt");
    let now = written_bytes(&free)?;
    replace(&free, &(now + GIB).to_string());
    let var_tmp = case.join("var-tmp");
    let home = case.join("home").join("example-user");
    Ok(vec![
        (
            home.join(".cache/claude-tmp").display().to_string(),
            3 * GIB,
        ),
        (home.join(".cache/go-build").display().to_string(), 2 * GIB),
        (var_tmp.join("held").display().to_string(), GIB),
        (var_tmp.display().to_string(), GIB / 4),
    ])
}

/// A reclaim that frees 1G and leaves only scratch and caches ([`scratch_left`]).
fn reclaim_leaves_only_scratch(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    Ok(ReclaimReport {
        reclaimed_bytes: GIB,
        steps: Vec::new(),
        left: scratch_left(state)?,
        reached: false,
    })
}

/// A reclaim that frees 1G and leaves scratch and caches ([`scratch_left`]) and the build output
/// of a tree a live session works in (2G).
fn reclaim_leaves_scratch_and_a_tree(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    let mut left = scratch_left(state)?;
    left.push(("/example/trees/live/target".to_owned(), 2 * GIB));
    Ok(ReclaimReport {
        reclaimed_bytes: GIB,
        steps: Vec::new(),
        left,
        reached: false,
    })
}

/// `story:reclaim-scratch-class-c`: freeing scratch and caches is class C, so a shortfall left
/// only by them records no operator item; the run is still refused and says so. One left by
/// anything else as well still records the item.
#[test]
fn a_shortfall_left_only_by_scratch_and_caches_records_no_operator_item() {
    let case = Case::new("scratch-left");
    fs::create_dir_all(case.var_tmp().join("held")).expect("create a /var/tmp entry");
    case.free(10 * GIB);
    let outcome = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_leaves_only_scratch),
        &case.request(4 * GIB, "true"),
    )
    .expect("the run answers");
    let Outcome::Refused {
        shortfall, item, ..
    } = &outcome
    else {
        panic!("refused: {outcome:?}");
    };
    assert_eq!(*shortfall, 3 * GIB);
    assert_eq!(*item, None, "no operator item: {outcome:?}");
    assert!(case.hands_todo().is_empty(), "{:?}", case.hands_todo());
    assert!(case.disk_items().is_empty(), "{:?}", case.disk_items());
    assert!(case.said().contains("refused: 4G asked"), "{}", case.said());

    let case = Case::new("scratch-and-tree-left");
    fs::create_dir_all(case.var_tmp().join("held")).expect("create a /var/tmp entry");
    case.free(10 * GIB);
    let outcome = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_leaves_scratch_and_a_tree),
        &case.request(4 * GIB, "true"),
    )
    .expect("the run answers");
    assert!(
        matches!(outcome, Outcome::Refused { item: Some(_), .. }),
        "refused with an operator item: {outcome:?}"
    );
    let todo = case.hands_todo();
    assert_eq!(todo.len(), 1, "{todo:?}");
    assert_eq!(
        todo[0].data.escalation_class,
        Some(DecisionClass::H),
        "{todo:?}"
    );
}

/// `story:disk-item-redacted`: a disk item names the `/var/tmp` entries outside the instance only
/// as a count and a total size, and every path under the home directory with `~/`, in the store
/// and in the records' decisions log.
#[test]
fn a_disk_item_names_no_var_tmp_entry_and_writes_the_home_as_tilde() {
    let case = Case::new("redacted");
    for entry in ["example-user", "example-project", "small"] {
        fs::create_dir_all(case.var_tmp().join(entry)).expect("create a /var/tmp entry");
    }
    fs::create_dir_all(case.home().join(".cache/sccache")).expect("create the home's cache");
    let home = case.home().display().to_string();
    case.free(10 * GIB);
    let outcome = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_leaves_personal_paths),
        &case.request(4 * GIB, &format!("touch {home}/ran")),
    )
    .expect("the run answers");
    let Outcome::Refused {
        item: Some(item), ..
    } = &outcome
    else {
        panic!("refused with an item: {outcome:?}");
    };
    let todo = case.hands_todo();
    assert_eq!(todo.len(), 1, "{todo:?}");
    let request = &todo[0].data;
    let texts = [&request.question, &request.options, &request.recommendation];
    for text in texts {
        for named in ["example-user", "example-project", home.as_str()] {
            assert!(!text.contains(named), "the item names {named:?}: {text:?}");
        }
    }
    assert!(
        request
            .options
            .contains("3 entries outside the instance, 6G"),
        "the /var/tmp entries as a count and a total size: {:?}",
        request.options
    );
    assert!(
        request.options.contains("~/.cache/sccache (8G)"),
        "a path under the home directory is written with ~/: {:?}",
        request.options
    );
    assert!(
        request.question.contains("touch ~/ran"),
        "the command's home path is written with ~/: {:?}",
        request.question
    );
    assert!(
        request.recommendation.contains("~/.cache/sccache"),
        "{:?}",
        request.recommendation
    );

    // The decisions log of the records holds the same text.
    let records =
        PathBuf::from(&conductor_cli::config::active().instance.records).join("decisions");
    let mut logged = 0;
    for file in fs::read_dir(&records).expect("the decisions log exists") {
        let text = fs::read_to_string(file.expect("a log file").path()).expect("a log reads");
        for line in text.lines().filter(|line| line.contains(item.as_str())) {
            logged += 1;
            for named in ["example-user", "example-project", home.as_str()] {
                assert!(!line.contains(named), "the log names {named:?}: {line}");
            }
        }
    }
    assert!(
        logged >= 2,
        "the item's requested and escalated lines are logged"
    );
}

/// `story:build-run-stderr-redacted`: a refused `build run` names the `/var/tmp` entries outside the
/// instance on standard error only as a count and a total size, and the home directory as `~`, as
/// the disk item does.
#[test]
fn a_refused_build_run_names_no_personal_path_on_standard_error() {
    let case = Case::new("stderr-redacted");
    for entry in ["example-user", "example-project", "small"] {
        fs::create_dir_all(case.var_tmp().join(entry)).expect("create a /var/tmp entry");
    }
    fs::create_dir_all(case.home().join(".cache/sccache")).expect("create the home's cache");
    let home = case.home().display().to_string();
    case.free(10 * GIB);
    let outcome = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_leaves_personal_paths),
        &case.request(4 * GIB, &format!("touch {home}/ran")),
    )
    .expect("the run answers");
    assert!(
        matches!(outcome, Outcome::Refused { .. }),
        "refused: {outcome:?}"
    );
    let said = case.said();
    assert!(said.contains("refused: 4G asked"), "{said}");
    for named in ["example-user", "example-project", home.as_str()] {
        assert!(
            !said.contains(named),
            "standard error names {named:?}: {said}"
        );
    }
    assert!(
        said.contains("3 entries outside the instance (6G)"),
        "the /var/tmp entries as a count and a total size: {said}"
    );
    assert!(
        said.contains("left: ~/.cache/sccache (8G)"),
        "a path under the home directory is written with ~/: {said}"
    );
}

/// Writes each of its stop files when dropped, so a failing case leaves no command running.
struct Stops(Vec<PathBuf>);

impl Drop for Stops {
    fn drop(&mut self) {
        for file in &self.0 {
            let _ = fs::write(file, "");
        }
    }
}

/// The states of the case's grants whose command is `sh -c <script>`, by script.
fn state_of(case: &Case, script: &str) -> Option<HostGrantState> {
    let command = format!("sh -c {script}");
    case.grants()
        .into_iter()
        .find(|row| row.command.as_deref() == Some(command.as_str()))
        .map(|row| row.state)
}

/// Starts `build run` of `script` reserving `reserve` in a thread, and waits until its request is
/// on the ledger, so the next one arrives after it.
fn arrive(
    case: &Case,
    settings: &Settings,
    reserve: u64,
    script: &str,
) -> thread::JoinHandle<anyhow::Result<Outcome>> {
    let state = case.state();
    let host = case.host();
    let sources = case.sources(reclaim_nothing);
    let request = case.request(reserve, script);
    let settings = settings.clone();
    let runner = thread::spawn(move || build::run(&state, &host, &settings, &sources, &request));
    wait_until("the request is on the ledger", || {
        host::rows(&case.host()).is_ok_and(|rows| {
            rows.iter()
                .any(|row| row.command.as_deref() == Some(format!("sh -c {script}").as_str()))
        })
    });
    runner
}

/// `story:build-run-queue`: requests are granted in arrival order. A 6G request made before two
/// 2G requests is granted first when 6G frees; the 2G requests, which alone would fit, wait behind
/// it while it does not, and each names its position in the queue on standard error once it has
/// waited longer than `thresholds.watchdog_every`.
#[test]
fn requests_are_granted_in_arrival_order_and_a_later_small_one_does_not_overtake() {
    let case = Case::new("queue");
    let _stops = Stops(vec![
        case.work().join("stop-first"),
        case.work().join("stop-head"),
    ]);
    // 20G free over a 10G floor: 10G to reserve.
    case.free(20 * GIB);
    let waiting = Settings {
        wait: Duration::from_secs(30),
        ..settings()
    };
    // A 6G run holds 6G: 4G is left to reserve.
    let first_script = "while [ ! -e stop-first ]; do sleep 0.05; done";
    let first = arrive(&case, &waiting, 6 * GIB, first_script);
    wait_until("the first run is running", || {
        state_of(&case, first_script) == Some(HostGrantState::Running)
    });
    // The head asks 6G: 2G short while the first holds 6G, so it waits for the first.
    let head_script = "while [ ! -e stop-head ]; do sleep 0.05; done";
    let head = arrive(&case, &waiting, 6 * GIB, head_script);
    // Two 2G requests after it: each alone fits in the 4G left.
    let small_a = arrive(&case, &waiting, 2 * GIB, "touch small-a");
    let small_b = arrive(&case, &waiting, 2 * GIB, "touch small-b");

    thread::sleep(Duration::from_millis(600));
    for script in [head_script, "touch small-a", "touch small-b"] {
        assert_eq!(
            state_of(&case, script),
            Some(HostGrantState::Requested),
            "{script:?} waits: nothing overtakes the head"
        );
    }
    assert!(!case.work().join("small-a").exists());
    assert!(!case.work().join("small-b").exists());
    wait_until("each waiter names its position in the queue", || {
        let said = case.said();
        said.contains("position 2 in the queue") && said.contains("position 3 in the queue")
    });

    // The first ends: 10G to reserve. The head is granted first; the 2G requests then fit beside
    // it.
    fs::write(case.work().join("stop-first"), "").expect("stop the first command");
    wait_until("both 2G runs ran", || {
        let small = [
            state_of(&case, "touch small-a"),
            state_of(&case, "touch small-b"),
        ];
        let head = state_of(&case, head_script);
        assert!(
            small
                .iter()
                .all(|state| *state == Some(HostGrantState::Requested))
                || head != Some(HostGrantState::Requested),
            "a 2G request was answered before the head: {small:?}, head {head:?}"
        );
        small
            .iter()
            .all(|state| *state == Some(HostGrantState::Released))
    });
    assert_eq!(state_of(&case, head_script), Some(HostGrantState::Running));
    fs::write(case.work().join("stop-head"), "").expect("stop the head's command");
    for runner in [first, head, small_a, small_b] {
        let outcome = runner
            .join()
            .expect("the runner ends")
            .expect("the run ends");
        assert!(
            matches!(outcome, Outcome::Ran { code: 0, .. }),
            "{outcome:?}"
        );
    }
}

/// `story:build-run-queue`: the head of the queue waits for live reservations at most
/// `Settings::wait`; then it is refused and leaves the queue, and the request behind it is
/// answered.
#[test]
fn a_head_that_waits_past_its_bound_is_refused_and_the_next_request_runs() {
    let case = Case::new("queue-head-bound");
    let _stops = Stops(vec![case.work().join("stop-first")]);
    case.free(20 * GIB);
    let waiting = Settings {
        wait: Duration::from_secs(30),
        ..settings()
    };
    let first_script = "while [ ! -e stop-first ]; do sleep 0.05; done";
    let first = arrive(&case, &waiting, 6 * GIB, first_script);
    wait_until("the first run is running", || {
        state_of(&case, first_script) == Some(HostGrantState::Running)
    });
    let bounded = Settings {
        wait: Duration::from_millis(500),
        ..settings()
    };
    let head = arrive(&case, &bounded, 6 * GIB, "touch head");
    let small = arrive(&case, &waiting, 2 * GIB, "touch small");
    let head = head
        .join()
        .expect("the head's runner ends")
        .expect("the head answers");
    assert!(
        matches!(head, Outcome::Refused { item: None, .. }),
        "the head waited its bound and was refused, with no operator item: {head:?}"
    );
    let small = small
        .join()
        .expect("the small runner ends")
        .expect("the small run ends");
    assert!(matches!(small, Outcome::Ran { code: 0, .. }), "{small:?}");
    fs::write(case.work().join("stop-first"), "").expect("stop the first command");
    first
        .join()
        .expect("the runner ends")
        .expect("the run ends");
}

#[test]
fn a_reading_under_gate_stop_pauses_the_group_and_one_over_gate_resume_resumes_it() {
    let case = Case::new("watchdog");
    let state = case.state();
    let host = case.host();
    let sources = case.sources(reclaim_nothing);
    let request = case.request(
        GIB,
        "while [ ! -e stop ]; do echo tick >> ticks; sleep 0.05; done",
    );
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &request));
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

/// story:build-run-pause-visible: a run the watchdog paused says so on exit, and its grant
/// carries the count; a run that was never paused says nothing.
#[test]
fn a_paused_run_names_the_pause_on_exit_and_records_the_count_on_its_grant() {
    let case = Case::new("pause-visible");
    let (state, host) = (case.state(), case.host());
    let sources = case.sources(reclaim_nothing);
    let request = case.request(
        GIB,
        "while [ ! -e stop ]; do echo tick >> ticks; sleep 0.05; done",
    );
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &request));
    let ticks = case.work().join("ticks");
    wait_until("the command ticks", || lines(&ticks) >= 3);
    case.free(4 * GIB);
    wait_until("reclaim runs for the paused run", || case.reclaims() >= 1);
    thread::sleep(Duration::from_millis(300));
    case.free(9 * GIB);
    let before = lines(&ticks);
    wait_until("the command ticks again", || lines(&ticks) > before + 2);
    fs::write(case.work().join("stop"), "").expect("stop the command");
    let outcome = runner
        .join()
        .expect("the runner ends")
        .expect("the run ends");
    let Outcome::Ran { ref grant, .. } = outcome else {
        panic!("the command ran: {outcome:?}");
    };
    let said = case.said();
    let line = said
        .lines()
        .find(|line| line.contains("paused 1 times for"))
        .unwrap_or_else(|| panic!("the exit names the pause: {said}"));
    assert!(
        line.starts_with("conductor: build run: paused 1 times for ")
            && line.ends_with("s during this run; a failure may come from the pause"),
        "{line}"
    );
    let row = case
        .grants()
        .into_iter()
        .find(|row| row.grant_id == *grant)
        .expect("the ledger holds the grant");
    assert_eq!(row.pauses, Some(1), "{row:?}");

    // A run that was never paused prints no such line and records none.
    let calm = Case::new("pause-visible-calm");
    build::run(
        &calm.state(),
        &calm.host(),
        &settings(),
        &calm.sources(reclaim_nothing),
        &calm.request(GIB, "exit 0"),
    )
    .expect("the run ends");
    assert!(!calm.said().contains("times for"), "{}", calm.said());
    assert!(calm.grants().iter().all(|row| row.pauses == Some(0)));
}

#[test]
fn the_command_killed_releases_the_grant() {
    let case = Case::new("killed");
    let outcome = build::run(
        &case.state(),
        &case.host(),
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
    let host = case.host();
    let sources = case.sources(reclaim_nothing);
    let first = case.request(GIB, "while [ ! -e stop ]; do sleep 0.05; done");
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &first));
    wait_until("the first run is running", || {
        host::rows(&case.host())
            .is_ok_and(|rows| rows.iter().any(|row| row.state == HostGrantState::Running))
    });

    // The second fits beside the first: 2G − 1G leaves 1G.
    let second = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the second run ends");
    assert!(matches!(second, Outcome::Ran { code: 0, .. }), "{second:?}");

    // A third asks 1.5G while the first holds 1G: 0.5G short, which it would not be alone.
    let third = build::run(
        &case.state(),
        &case.host(),
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

/// Opens the lock file `path`, creating it; not locked.
fn lock_file(path: &Path) -> fs::File {
    fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .unwrap_or_else(|error| panic!("open {}: {error}", path.display()))
}

/// Holds the writer lock of the store under `dir` (`dir/tree/.lock`), as another writer, a
/// snapshot say, does, until the answer is dropped.
fn hold_tree(dir: &Path) -> fs::File {
    drop(conductor_cli::store::open(dir).expect("the store opens"));
    let tree = lock_file(&dir.join("tree").join(".lock"));
    tree.lock().expect("hold the tree's writer lock");
    tree
}

/// `story:store-readers-hold-no-writer-lock`: a run whose ledger store's tree lock another writer
/// holds waits for it without holding `host.lock`, so the host's other runs are not queued behind
/// it; once the tree lock is let go, the run reserves and runs its command.
#[test]
fn a_run_waiting_on_the_stores_tree_lock_does_not_hold_host_lock() {
    let case = Case::new("tree-lock-without-host-lock");
    let tree = hold_tree(&case.host());

    let (state, host) = (case.state(), case.host());
    let sources = case.sources(reclaim_nothing);
    let request = case.request(GIB, "touch ran");
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &request));
    // Long enough for the run to reach the tree lock; then `host.lock` is read for half a second.
    thread::sleep(Duration::from_millis(500));
    let mut held = 0;
    for _ in 0..10 {
        let host = lock_file(&case.host().join("host.lock"));
        if host.try_lock().is_err() {
            held += 1;
        }
        drop(host);
        thread::sleep(Duration::from_millis(50));
    }
    let waiting = !runner.is_finished();
    drop(tree);
    let outcome = runner
        .join()
        .expect("the runner ends")
        .expect("the run ends");
    assert_eq!(
        held, 0,
        "build run held host.lock in {held} of 10 reads while it waited on the tree lock"
    );
    assert!(waiting, "the run did not wait on the tree lock");
    assert!(
        matches!(outcome, Outcome::Ran { code: 0, .. }),
        "{outcome:?}"
    );
    assert!(case.work().join("ran").exists(), "the command ran");
}

/// `story:store-readers-hold-no-writer-lock`: free disk back at `disk_clear` ends the open fall,
/// whose item is in the instance's store; while another writer holds that store's tree lock the
/// run, under `host.lock`, does not wait on it: it runs its command and leaves the fall to the
/// next run, which withdraws the item.
#[test]
fn a_run_under_host_lock_never_waits_on_the_instance_stores_tree_lock() {
    let case = Case::new("instance-tree-lock-under-host-lock");
    case.free(10 * GIB);
    let refused = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(4 * GIB, "true"),
    )
    .expect("the run answers");
    let Outcome::Refused {
        item: Some(item), ..
    } = refused
    else {
        panic!("refused with an item: {refused:?}");
    };
    case.free(20 * GIB);
    let tree = hold_tree(&case.state());

    let (state, host) = (case.state(), case.host());
    let sources = case.sources(reclaim_nothing);
    let request = case.request(GIB, "touch ran");
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &request));
    let started = Instant::now();
    while !runner.is_finished() && started.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(50));
    }
    let finished = runner.is_finished();
    drop(tree);
    let outcome = runner
        .join()
        .expect("the runner ends")
        .expect("the run ends");
    assert!(
        finished,
        "the run waited over 10 s, under host.lock, on the instance store's tree lock"
    );
    assert!(
        matches!(outcome, Outcome::Ran { code: 0, .. }),
        "{outcome:?}"
    );
    let state_of = |id: &str| {
        case.disk_items()
            .into_iter()
            .find(|request| request.data.request_id.0 == id)
            .map(|request| request.state)
    };
    assert_eq!(
        state_of(&item),
        Some(DecisionRequestState::Escalated),
        "the fall is left to the next run"
    );

    let next = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the run ends");
    assert_eq!(next.exit_code(), 0, "{next:?}");
    assert_eq!(state_of(&item), Some(DecisionRequestState::Withdrawn));
}

/// A running grant holds only what its run has not written yet: free disk already counts the
/// growth it recorded, so a reservation beside it is not refused for space that is in use.
#[test]
fn a_running_grant_holds_its_reservation_less_what_it_wrote() {
    let case = Case::new("running-less-written");
    // 12G free over a 10G floor: 2G to reserve.
    case.free(12 * GIB);
    let state = case.state();
    let host = case.host();
    let sources = case.sources(reclaim_nothing);
    let first = case.request(2 * GIB, "while [ ! -e stop ]; do sleep 0.05; done");
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &first));
    wait_until("the first run is running", || {
        host::rows(&case.host())
            .is_ok_and(|rows| rows.iter().any(|row| row.state == HostGrantState::Running))
    });
    let running = case
        .grants()
        .into_iter()
        .find(|row| row.state == HostGrantState::Running)
        .expect("a running grant");
    // The first run has written 1.5G of its 2G: it holds 0.5G more.
    host::record_peak_of(&case.host(), &running.grant_id, GIB + GIB / 2)
        .expect("the growth is recorded");

    let second = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB + GIB / 2, "true"),
    )
    .expect("the second run ends");
    assert!(
        matches!(second, Outcome::Ran { code: 0, .. }),
        "1.5G fits beside the 0.5G the first still holds: {second:?}"
    );

    fs::write(case.work().join("stop"), "").expect("stop the first command");
    runner
        .join()
        .expect("the runner ends")
        .expect("the run ends");
}

/// A shortfall live reservations cover is a wait: the run waits for them to release and then
/// runs, and no operator item is raised.
#[test]
fn a_shortfall_live_reservations_cover_waits_for_them_and_raises_no_item() {
    let case = Case::new("waits-for-reservations");
    // 12G free over a 10G floor: 2G to reserve.
    case.free(12 * GIB);
    let state = case.state();
    let host = case.host();
    let sources = case.sources(reclaim_nothing);
    let first = case.request(2 * GIB, "sleep 0.5");
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &first));
    wait_until("the first run is running", || {
        host::rows(&case.host())
            .is_ok_and(|rows| rows.iter().any(|row| row.state == HostGrantState::Running))
    });
    let waiting = Settings {
        wait: Duration::from_secs(30),
        ..settings()
    };
    let second = build::run(
        &case.state(),
        &case.host(),
        &waiting,
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the second run ends");
    assert!(
        matches!(second, Outcome::Ran { code: 0, .. }),
        "it waited for the first to release and ran: {second:?}"
    );
    runner
        .join()
        .expect("the runner ends")
        .expect("the first run ends");
    assert!(
        !case.host().join("host.fall").exists(),
        "no operator item for a wait"
    );
}

/// Runs reserving, starting and releasing at once keep every write: none fails on another's.
#[test]
fn runs_at_once_keep_every_ledger_write() {
    let case = Case::new("many-at-once");
    case.free(100 * GIB);
    let runners: Vec<_> = (0..6)
        .map(|_| {
            let state = case.state();
            let host = case.host();
            let sources = case.sources(reclaim_nothing);
            let request = case.request(GIB, "sleep 0.2");
            thread::spawn(move || build::run(&state, &host, &settings(), &sources, &request))
        })
        .collect();
    for runner in runners {
        let outcome = runner
            .join()
            .expect("the runner ends")
            .expect("the run keeps its writes");
        assert!(
            matches!(outcome, Outcome::Ran { code: 0, .. }),
            "{outcome:?}"
        );
    }
    let live: Vec<_> = case
        .grants()
        .into_iter()
        .filter(|row| matches!(row.state, HostGrantState::Granted | HostGrantState::Running))
        .collect();
    assert!(live.is_empty(), "every grant is released: {live:?}");
}

#[test]
fn a_request_or_grant_whose_run_is_gone_is_closed_by_the_next_run() {
    let case = Case::new("requester-gone");
    let at = host::now(OffsetDateTime::now_utc());
    let (request, granted) = {
        let mut ledger = host::Ledger::lock(&case.host()).expect("the ledger opens");
        let request = ledger
            .request("example-org", None, None, GIB, "a waiter that died", &at)
            .expect("a request is recorded");
        let granted = ledger
            .request(
                "example-org",
                None,
                None,
                6 * GIB,
                "a run that died before it started",
                &at,
            )
            .expect("a request is recorded");
        ledger.reserve(&granted, 0, 0).expect("it is granted");
        (request, granted)
    };
    // Both runs held their lock and died: the files stay, nobody holds them.
    drop(host::hold_run(&case.host(), &request).expect("a run lock"));
    drop(host::hold_run(&case.host(), &granted).expect("a run lock"));
    // 12G free over a 10G floor: the dead 6G grant would leave no room for 1G.
    case.free(12 * GIB);
    let outcome = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the run ends");
    assert_eq!(
        outcome.exit_code(),
        0,
        "the dead grant no longer holds space: {outcome:?}"
    );
    let grants = case.grants();
    let state_of = |id| grants.iter().find(|row| row.grant_id == id).unwrap().state;
    assert_eq!(state_of(request), HostGrantState::Refused);
    assert_eq!(state_of(granted), HostGrantState::Released);
}

/// A request with no run lock file is a waiter of a binary before 0.10.6, which locks only from its
/// start: a sweep leaves it while it is younger than the legacy age.
#[test]
fn a_young_request_with_no_run_lock_file_is_left_to_its_waiter() {
    let case = Case::new("legacy-waiter");
    let at = host::now(OffsetDateTime::now_utc());
    let request = host::Ledger::lock(&case.host())
        .expect("the ledger opens")
        .request(
            "example-org",
            None,
            None,
            GIB,
            "a waiter of an older binary",
            &at,
        )
        .expect("a request is recorded");
    let swept = host::Ledger::lock(&case.host())
        .expect("the ledger opens")
        .sweep(&host::now(OffsetDateTime::now_utc()))
        .expect("the sweep reads");
    assert!(swept.is_empty(), "{swept:?}");
    let grants = case.grants();
    let row = grants.iter().find(|row| row.grant_id == request).unwrap();
    assert_eq!(row.state, HostGrantState::Requested);
}

#[test]
fn a_grant_whose_run_still_holds_its_lock_is_not_swept_when_its_group_is_gone() {
    let case = Case::new("group-gone-run-held");
    // A command that has ended while its `build run` still records its peak and releases.
    let mut child = {
        use std::os::unix::process::CommandExt as _;
        Command::new("true")
            .process_group(0)
            .spawn()
            .expect("start true")
    };
    let group = i64::from(child.id());
    child.wait().expect("wait for true");
    let grant = {
        let mut ledger = host::Ledger::lock(&case.host()).expect("the ledger opens");
        let grant = ledger
            .request(
                "example-org",
                None,
                None,
                GIB,
                "true",
                &host::now(OffsetDateTime::now_utc()),
            )
            .expect("a disk request is recorded");
        ledger
            .reserve(&grant, 0, 0)
            .expect("the request is granted");
        grant
    };
    host::start_run(
        &case.host(),
        &grant,
        group,
        &host::now(OffsetDateTime::now_utc()),
    )
    .expect("the grant runs");
    let lock = host::hold_run(&case.host(), &grant).expect("the run lock is held");

    let swept = host::Ledger::lock(&case.host())
        .expect("the ledger opens")
        .sweep(&host::now(OffsetDateTime::now_utc()))
        .expect("the sweep reads");
    assert!(swept.is_empty(), "a held run is not swept: {swept:?}");
    host::record_peak_of(&case.host(), &grant, GIB / 2).expect("its peak is recorded");
    host::release(
        &case.host(),
        &grant,
        Some(ReleaseReason::Exited),
        &host::now(OffsetDateTime::now_utc()),
    )
    .expect("its own release is kept");
    host::end_run(&case.host(), &grant, lock);
    let grants = case.grants();
    let row = grants.iter().find(|row| row.grant_id == grant).unwrap();
    assert_eq!(row.release_reason, Some(ReleaseReason::Exited), "{row:?}");
    assert_eq!(
        row.peak_bytes,
        Some(i64::try_from(GIB / 2).unwrap()),
        "{row:?}"
    );
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
        let mut ledger = host::Ledger::lock(&case.host()).expect("the ledger opens");
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
        &case.host(),
        &orphan,
        group,
        &host::now(OffsetDateTime::now_utc()),
    )
    .expect("the grant runs");

    let outcome = build::run(
        &case.state(),
        &case.host(),
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

/// Two short runs in a row walk the disk once: the second reuses the reclaim the first ran a
/// moment ago (`build::RECLAIM_REUSE`), what it left included, rather than walk again.
#[test]
fn two_short_runs_in_a_row_run_one_reclaim() {
    let case = Case::new("one-reclaim");
    case.free(10 * GIB);
    for _ in 0..2 {
        let outcome = build::run(
            &case.state(),
            &case.host(),
            &settings(),
            &case.sources(reclaim_frees_1g),
            &case.request(4 * GIB, "true"),
        )
        .expect("the run answers");
        let Outcome::Refused { left, .. } = &outcome else {
            panic!("refused: {outcome:?}");
        };
        assert_eq!(
            left,
            &[("/example/trees/live/target".to_owned(), 2 * GIB)],
            "each refusal names what the one reclaim left: {outcome:?}"
        );
    }
    assert_eq!(case.reclaims(), 1, "one reclaim walk for two short runs");
}

/// A reclaim that writes `reclaim.started` in its case and waits, at most 20 seconds, until the
/// case holds `reclaim.go`; it frees nothing.
fn slow_reclaim(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    count_reclaim(state);
    let case = case_of(state);
    fs::write(case.join("reclaim.started"), "").expect("mark the reclaim started");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !case.join("reclaim.go").exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    Ok(ReclaimReport {
        reclaimed_bytes: 0,
        steps: Vec::new(),
        left: Vec::new(),
        reached: false,
    })
}

/// Writes `reclaim.go` in the case when dropped, so a failing case leaves no reclaim waiting.
struct Go(PathBuf);

impl Drop for Go {
    fn drop(&mut self) {
        let _ = fs::write(&self.0, "");
    }
}

/// A run that reclaims holds no ledger lock meanwhile: another run takes the lock while the
/// reclaim walks, so the host queue never waits on one run's walk.
#[test]
fn the_ledger_lock_is_free_while_a_reclaim_runs() {
    let case = Case::new("lock-free-during-reclaim");
    case.free(10 * GIB);
    let go = Go(case.dir.join("reclaim.go"));
    let state = case.state();
    let host = case.host();
    let sources = case.sources(slow_reclaim);
    let request = case.request(4 * GIB, "true");
    let runner = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &request));
    wait_until("the reclaim runs", || {
        case.dir.join("reclaim.started").exists()
    });
    let taken = host::Ledger::try_lock(&case.host()).expect("the ledger opens");
    let free = taken.is_some();
    drop(taken);
    drop(go);
    let outcome = runner
        .join()
        .expect("the run does not panic")
        .expect("the run answers");
    assert!(free, "the ledger lock is free while the reclaim runs");
    assert!(
        matches!(outcome, Outcome::Refused { .. }),
        "refused: {outcome:?}"
    );
}

/// `story:build-run-cancel-group`: what the binary's cases of a signal to `build run` share. The
/// binary runs with the case's home, its state directory, free disk read from `readings.txt`
/// (1000G: every reservation fits and no reclaim runs) and the case's `var-tmp/`.
impl Case {
    fn binary(&self, args: &[&str]) -> Command {
        let home = self.dir.join("home");
        fs::create_dir_all(&home).expect("create the case's home");
        let readings = self.dir.join("readings.txt");
        fs::write(&readings, format!("{}\n", 1000 * GIB)).expect("write the readings");
        let mut command = common::conductor(&home);
        command
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .env(READINGS_VARIABLE, &readings)
            .env(VAR_TMP_VARIABLE, self.var_tmp())
            .current_dir(self.work())
            .stdin(Stdio::null());
        command
    }

    /// `build run --reserve 1G -- sh -c <script>`, started.
    fn build_run(&self, script: &str) -> Running {
        let child = self
            .binary(&["build", "run", "--reserve", "1G", "--", "sh", "-c", script])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("conductor runs");
        Running { child, group: None }
    }

    /// The row of `host grants --format json` whose command is `sh -c <script>`.
    fn grant_row(&self, script: &str) -> Option<Value> {
        let output = self
            .binary(&["host", "grants", "--format", "json"])
            .output()
            .expect("conductor runs");
        if !output.status.success() {
            return None;
        }
        let rows: Value = serde_json::from_slice(&output.stdout).ok()?;
        let command = format!("sh -c {script}");
        rows.as_array()?
            .iter()
            .find(|row| row["command"] == command.as_str())
            .cloned()
    }
}

/// A `build run` a case started, and the process group of its command once known: both are
/// killed when the case ends, passed or not.
struct Running {
    child: Child,
    group: Option<i32>,
}

impl Running {
    /// Sends `signal` to `build run` itself, not to its command's group.
    fn signal(&self, signal: Signal) {
        let pid = i32::try_from(self.child.id()).expect("a pid");
        kill(Pid::from_raw(pid), signal).expect("signal build run");
    }

    /// The exit of `build run`, within 30 seconds.
    fn exit(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.child.try_wait().expect("wait for build run") {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "build run did not exit within 30 seconds of the signal"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(group) = self.group {
            let _ = killpg(Pid::from_raw(group), Signal::SIGKILL);
        }
    }
}

/// `story:build-run-cancel-group`: `signal` to a `build run` whose command runs stops the whole
/// process group of the command, the background member `script` starts included; the grant is
/// released as killed, and `build run` exits with 128 + the signal.
fn a_signal_stops_the_running_group(name: &str, signal: Signal, script: &str) {
    let case = Case::new(name);
    let mut run = case.build_run(script);
    let mut group = None;
    wait_until("the command runs in its process group", || {
        group = case
            .grant_row(script)
            .filter(|row| row["state"] == "Running")
            .and_then(|row| row["process_group"].as_i64());
        group.is_some()
    });
    let group = group.expect("a process group");
    run.group = Some(i32::try_from(group).expect("a process group id"));
    // The script's background member starts.
    wait_until("the background member runs", || {
        case.work().join("started").exists()
    });
    run.signal(signal);
    let status = run.exit();
    assert_eq!(
        status.code(),
        Some(128 + signal as i32),
        "build run exits with 128 + {signal}: {status}"
    );
    wait_until("no process of the group is left", || !host::alive(group));
    let row = case.grant_row(script).expect("the grant is listed");
    assert_eq!(row["state"], "Released", "{row}");
    assert_eq!(row["release_reason"], "Killed", "{row}");
}

/// The payload of the running cases: a background `sleep` and a foreground one, in one group.
const PAYLOAD: &str = "sleep 600 & touch started; sleep 600; wait";

/// The review's probe, `cancelling_build_wrapper_must_stop_its_payload`: SIGTERM.
#[test]
fn cancelling_build_wrapper_must_stop_its_payload() {
    a_signal_stops_the_running_group("cancel-term", Signal::SIGTERM, PAYLOAD);
}

#[test]
fn sigint_to_build_run_stops_its_whole_group() {
    a_signal_stops_the_running_group("cancel-int", Signal::SIGINT, PAYLOAD);
}

#[test]
fn sighup_to_build_run_stops_its_whole_group() {
    a_signal_stops_the_running_group("cancel-hup", Signal::SIGHUP, PAYLOAD);
}

/// A group that ignores the forwarded signal is killed once the grace period has passed.
#[test]
fn a_group_that_ignores_the_signal_is_killed_after_the_grace_period() {
    a_signal_stops_the_running_group(
        "cancel-ignored",
        Signal::SIGTERM,
        "trap '' TERM; sleep 600 & touch started; sleep 600; wait",
    );
}

/// `story:build-run-cancel-group`: `signal` to a `build run` whose request still waits in the
/// queue withdraws the request: the grant reads `Withdrawn`, its command never ran, and `build
/// run` exits with 128 + the signal. The request ahead of it is one `host request-disk` recorded
/// a minute before, whose run lock the case holds as its waiter would.
fn a_signal_withdraws_the_queued_request(name: &str, signal: Signal) {
    let case = Case::new(name);
    let before = (OffsetDateTime::now_utc() - time::Duration::minutes(1))
        .format(&Rfc3339)
        .expect("a time");
    let head = case
        .binary(&[
            "host",
            "request-disk",
            "--instance",
            "example-org",
            "--bytes",
            "1073741824",
            "--command",
            "sh -c head",
            "--requested-at",
            &before,
        ])
        .output()
        .expect("conductor runs");
    assert!(head.status.success(), "{head:?}");
    let head = conductor_model::host::GrantId(
        String::from_utf8(head.stdout)
            .expect("a grant id")
            .trim()
            .to_owned(),
    );
    // The binary's host directory: `~/.b10x/conductor/host` of the case's home, as no config
    // file names one (`config::host_dir`).
    let host_dir = case
        .dir
        .join("home")
        .join(".b10x")
        .join("conductor")
        .join("host");
    let _waiter = host::hold_run(&host_dir, &head).expect("hold the head's run lock");
    let script = "touch ran";
    let mut run = case.build_run(script);
    wait_until("the request waits in the queue", || {
        case.grant_row(script)
            .is_some_and(|row| row["state"] == "Requested")
    });
    // Behind the head, it stays queued.
    thread::sleep(Duration::from_millis(300));
    assert_eq!(
        case.grant_row(script).map(|row| row["state"].clone()),
        Some(Value::from("Requested"))
    );
    run.signal(signal);
    let status = run.exit();
    assert_eq!(
        status.code(),
        Some(128 + signal as i32),
        "build run exits with 128 + {signal}: {status}"
    );
    let row = case.grant_row(script).expect("the request is listed");
    assert_eq!(row["state"], "Withdrawn", "{row}");
    assert_eq!(row["process_group"], Value::Null, "{row}");
    assert!(!case.work().join("ran").exists(), "the command never ran");
    assert_eq!(
        case.grant_row("head").map(|row| row["state"].clone()),
        Some(Value::from("Requested")),
        "the request ahead is left"
    );
    // `host withdraw-disk-request` withdraws a waiting request once, and refuses it then.
    let withdraw = || {
        case.binary(&[
            "host",
            "withdraw-disk-request",
            "--grant-id",
            &head.0,
            "--withdrawn-at",
            "2026-10-15T12:00:00Z",
        ])
        .output()
        .expect("conductor runs")
    };
    let withdrawn = withdraw();
    assert!(withdrawn.status.success(), "{withdrawn:?}");
    assert_eq!(
        case.grant_row("head").map(|row| row["state"].clone()),
        Some(Value::from("Withdrawn"))
    );
    let again = withdraw();
    assert_eq!(again.status.code(), Some(1), "withdrawn twice: {again:?}");
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("HostGrantStateConflict"),
        "{again:?}"
    );
}

#[test]
fn sigterm_withdraws_a_queued_request() {
    a_signal_withdraws_the_queued_request("withdraw-term", Signal::SIGTERM);
}

#[test]
fn sigint_withdraws_a_queued_request() {
    a_signal_withdraws_the_queued_request("withdraw-int", Signal::SIGINT);
}

#[test]
fn sighup_withdraws_a_queued_request() {
    a_signal_withdraws_the_queued_request("withdraw-hup", Signal::SIGHUP);
}

/// The variable the harness sets for the commands of a session, naming it
/// (`story:build-run-session-bound`): Claude Code's.
const SESSION_VARIABLE: &str = "CLAUDE_CODE_SESSION_ID";

impl Case {
    /// Puts a fake `claude` first on the `PATH` of [`Case::in_session`]: `claude agents --json`
    /// prints `list` and counts its call in `agents-calls.txt`; any other arguments exit 64. A
    /// `list` of `None` makes the session list unreadable: the fake exits 1.
    fn session_list(&self, list: Option<&str>) {
        use std::os::unix::fs::PermissionsExt as _;
        let bin = self.dir.join("bin");
        fs::create_dir_all(&bin).expect("create the case's bin");
        let calls = self.dir.join("agents-calls.txt");
        let answer = match list {
            Some(list) => {
                fs::write(self.dir.join("agents.json"), list).expect("write the session list");
                format!("cat '{}'", self.dir.join("agents.json").display())
            }
            None => "echo 'claude: not logged in' >&2; exit 1".to_owned(),
        };
        let claude = bin.join("claude");
        fs::write(
            &claude,
            format!(
                "#!/bin/sh\n[ \"$*\" = 'agents --json' ] || exit 64\necho call >> '{}'\n{answer}\n",
                calls.display()
            ),
        )
        .expect("write the fake claude");
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755))
            .expect("make the fake claude executable");
    }

    /// How often `build run` read the session list.
    fn session_list_reads(&self) -> usize {
        lines(&self.dir.join("agents-calls.txt"))
    }

    /// [`Case::binary`] run as a command of the harness session `session`, with the fake
    /// `claude` of [`Case::session_list`] first on its `PATH`.
    fn in_session(&self, args: &[&str], session: &str) -> Command {
        let path = std::env::var("PATH").unwrap_or_default();
        let mut command = self.binary(args);
        command
            .env("PATH", format!("{}:{path}", self.dir.join("bin").display()))
            .env(SESSION_VARIABLE, session);
        command
    }

    /// A request recorded a minute ago by `host request-disk`, whose run lock the case holds as
    /// its waiter would, so a `build run` started now waits behind it in the queue.
    fn queue_head(&self) -> fs::File {
        let before = (OffsetDateTime::now_utc() - time::Duration::minutes(1))
            .format(&Rfc3339)
            .expect("a time");
        let head = self
            .binary(&[
                "host",
                "request-disk",
                "--instance",
                "example-org",
                "--bytes",
                "1073741824",
                "--command",
                "sh -c head",
                "--requested-at",
                &before,
            ])
            .output()
            .expect("conductor runs");
        assert!(head.status.success(), "{head:?}");
        let head = conductor_model::host::GrantId(
            String::from_utf8(head.stdout)
                .expect("a grant id")
                .trim()
                .to_owned(),
        );
        let host_dir = self
            .dir
            .join("home")
            .join(".b10x")
            .join("conductor")
            .join("host");
        host::hold_run(&host_dir, &head).expect("hold the head's run lock")
    }
}

/// `story:build-run-session-bound`: `build run` records the session the harness names in its
/// environment on its request, and `host grants` shows it, as JSON and as text.
#[test]
fn a_build_run_records_the_session_of_its_caller() {
    let case = Case::new("session-recorded");
    case.session_list(Some(
        r#"[{"id":"example-session-live","sessionId":"example-session-live","pid":4242,"cwd":"/example","status":"busy"}]"#,
    ));
    let script = "touch ran";
    let run = case
        .in_session(
            &["build", "run", "--reserve", "1G", "--", "sh", "-c", script],
            "example-session-live",
        )
        .output()
        .expect("conductor runs");
    assert!(run.status.success(), "{run:?}");
    let row = case.grant_row(script).expect("the grant is listed");
    assert_eq!(row["session_id"], "example-session-live", "{row}");
    assert_eq!(row["state"], "Released", "{row}");
    let text = case
        .binary(&["host", "grants"])
        .output()
        .expect("conductor runs");
    assert!(text.status.success(), "{text:?}");
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(
        text.contains("session_id") && text.contains("example-session-live"),
        "`host grants` shows each request's session:\n{text}"
    );
}

/// `story:build-run-session-bound`: a request still waiting in the queue whose session the
/// session list shows stopped (listed without a `pid`, as `claude agents` keeps an exited
/// session) is withdrawn at the next ledger pass: the grant reads `Withdrawn` with
/// `SessionStopped`, its command never ran, and `build run` exits 2 naming the session. The
/// request ahead of it, which names no session, is left.
#[test]
fn a_queued_request_whose_session_stopped_is_withdrawn() {
    use std::io::Read as _;
    let case = Case::new("session-stopped");
    case.session_list(Some(
        r#"[{"id":"example-other","sessionId":"example-session-other","pid":4242,"cwd":"/example","status":"busy"},
            {"sessionId":"example-session-stopped","cwd":"/example","status":"idle"}]"#,
    ));
    let _waiter = case.queue_head();
    let script = "touch ran";
    let child = case
        .in_session(
            &["build", "run", "--reserve", "1G", "--", "sh", "-c", script],
            "example-session-stopped",
        )
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("conductor runs");
    let mut run = Running { child, group: None };
    let status = run.exit();
    let mut stderr = String::new();
    run.child
        .stderr
        .take()
        .expect("piped stderr")
        .read_to_string(&mut stderr)
        .expect("read build run's stderr");
    assert_eq!(
        status.code(),
        Some(2),
        "build run exits 2: {status}\n{stderr}"
    );
    assert!(
        stderr.contains("withdrawn: session example-session-stopped stopped"),
        "{stderr}"
    );
    let row = case.grant_row(script).expect("the request is listed");
    assert_eq!(row["state"], "Withdrawn", "{row}");
    assert_eq!(row["release_reason"], "SessionStopped", "{row}");
    assert_eq!(row["session_id"], "example-session-stopped", "{row}");
    assert!(!case.work().join("ran").exists(), "the command never ran");
    assert_eq!(
        case.grant_row("head").map(|row| row["state"].clone()),
        Some(Value::from("Requested")),
        "the request ahead, of no session, is left"
    );
}

/// A request whose session the list shows running, or whose session list does not read, stays
/// in the queue after a ledger pass read the list.
fn a_queued_request_stays(name: &str, list: Option<&str>) {
    let case = Case::new(name);
    case.session_list(list);
    let _waiter = case.queue_head();
    let script = "touch ran";
    let child = case
        .in_session(
            &["build", "run", "--reserve", "1G", "--", "sh", "-c", script],
            "example-session-live",
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("conductor runs");
    let mut run = Running { child, group: None };
    wait_until("a ledger pass reads the session list", || {
        case.session_list_reads() > 0
    });
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        run.child.try_wait().expect("wait for build run"),
        None,
        "build run still waits"
    );
    let row = case.grant_row(script).expect("the request is listed");
    assert_eq!(row["state"], "Requested", "{row}");
    assert_eq!(row["session_id"], "example-session-live", "{row}");
}

#[test]
fn a_queued_request_whose_session_runs_stays_queued() {
    a_queued_request_stays(
        "session-live",
        Some(
            r#"[{"id":"example-session-live","sessionId":"example-session-live","pid":4242,"cwd":"/example","status":"waiting"}]"#,
        ),
    );
}

#[test]
fn a_queued_request_stays_when_the_session_list_does_not_read() {
    a_queued_request_stays("session-list-unread", None);
}

/// `story:build-run-backfill`: an 8G head at 5G available lacks 3G, more than the live
/// reservations hold (none), so it can only be refused. While it walks the disk (a reclaim that
/// waits for `reclaim.go`), a 2G request behind it, which fits beside the floor, is granted at
/// once and runs. A 4G request behind that one, which would take the floor once the 2G holds its
/// reservation, is never granted. The head is refused once its reclaim frees nothing.
#[test]
fn a_request_behind_a_head_that_can_only_be_refused_is_granted_when_it_fits() {
    let case = Case::new("backfill");
    let go = Go(case.dir.join("reclaim.go"));
    let _stops = Stops(vec![case.work().join("stop-small")]);
    // 15G free over a 10G floor: 5G to reserve.
    case.free(15 * GIB);
    let state = case.state();
    let host = case.host();
    let sources = case.sources(slow_reclaim);
    let request = case.request(8 * GIB, "touch head");
    let head = thread::spawn(move || build::run(&state, &host, &settings(), &sources, &request));
    wait_until("the head's reclaim runs", || {
        case.dir.join("reclaim.started").exists()
    });

    let waiting = Settings {
        wait: Duration::from_secs(30),
        ..settings()
    };
    let small_script = "while [ ! -e stop-small ]; do sleep 0.05; done";
    let small = arrive(&case, &waiting, 2 * GIB, small_script);
    wait_until("the 2G request behind the head runs", || {
        state_of(&case, small_script) == Some(HostGrantState::Running)
    });
    assert_eq!(
        state_of(&case, "touch head"),
        Some(HostGrantState::Requested),
        "the 2G request was granted while the head still waited"
    );

    // 3G is left beside the floor and the 2G run: a 4G request is never granted past the floor.
    let unbounded = Settings {
        wait: Duration::ZERO,
        ..settings()
    };
    let over = arrive(&case, &unbounded, 4 * GIB, "touch over");
    thread::sleep(Duration::from_millis(600));
    assert_eq!(
        state_of(&case, "touch over"),
        Some(HostGrantState::Requested),
        "4G does not fit beside the floor and the 2G run"
    );

    drop(go);
    let head = head
        .join()
        .expect("the head's runner ends")
        .expect("the head answers");
    assert!(
        matches!(head, Outcome::Refused { shortfall, .. } if shortfall == 5 * GIB),
        "the head lacks 5G once the 2G run holds its reservation: {head:?}"
    );
    let over = over
        .join()
        .expect("the 4G runner ends")
        .expect("the 4G request answers");
    assert!(
        matches!(over, Outcome::Refused { .. }),
        "the 4G request never ran past the floor: {over:?}"
    );
    assert!(!case.work().join("over").exists());
    assert!(!case.work().join("head").exists());

    fs::write(case.work().join("stop-small"), "").expect("stop the 2G command");
    let small = small
        .join()
        .expect("the 2G runner ends")
        .expect("the 2G run ends");
    assert!(matches!(small, Outcome::Ran { code: 0, .. }), "{small:?}");
}

/// A reclaim that reached what it was asked, since that was free already, and freed nothing.
fn reclaim_reached_nothing(state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    count_reclaim(state);
    Ok(ReclaimReport {
        reclaimed_bytes: 0,
        steps: Vec::new(),
        left: Vec::new(),
        reached: true,
    })
}

/// `story:build-run-backfill`: a reclaim run that finished a moment ago freed nothing, so an 8G
/// head at 5G available, which would ask reclaim for more than that run was asked, is refused at
/// once, naming that run, and walks the disk no second time.
#[test]
fn a_head_that_does_not_fit_after_a_recent_reclaim_freed_nothing_is_refused_at_once() {
    let case = Case::new("refused-after-barren-reclaim");
    // 15G free over a 10G floor: 5G to reserve.
    case.free(15 * GIB);
    let first = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_reached_nothing),
        &case.request(6 * GIB, "touch first"),
    )
    .expect("the first run answers");
    assert!(matches!(first, Outcome::Refused { .. }), "{first:?}");
    assert_eq!(case.reclaims(), 1);

    let head = build::run(
        &case.state(),
        &case.host(),
        &settings(),
        &case.sources(reclaim_reached_nothing),
        &case.request(8 * GIB, "touch head"),
    )
    .expect("the head answers");
    assert!(
        matches!(head, Outcome::Refused { shortfall, reclaimed: 0, .. } if shortfall == 3 * GIB),
        "{head:?}"
    );
    assert_eq!(case.reclaims(), 1, "the head walks the disk no second time");
    let said = case.said();
    assert!(
        said.contains("the reclaim run started") && said.contains("freed nothing"),
        "the refusal names the run that freed nothing: {said}"
    );
    assert!(!case.work().join("head").exists());
}
