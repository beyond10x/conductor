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
            home: Some(self.home()),
            var_tmp: self.var_tmp(),
        }
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

    // Once free disk is back at `disk_clear` the fall is over, and the next shortfall is a new item.
    let first = item.clone().expect("the first refusal recorded the item");
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
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request_in("example-small", GIB, "true"),
    )
    .expect("the small run ends");
    assert_eq!(small.exit_code(), 0, "{small:?}");

    let second = build::run(
        &case.state(),
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
        &settings(),
        &case.sources(reclaim_nothing),
        &case.request(GIB, "true"),
    )
    .expect("the run ends");
    assert_eq!(fits.exit_code(), 0, "{fits:?}");
    assert_eq!(state_of(&item), Some(DecisionRequestState::Withdrawn));
    assert!(case.hands_todo().is_empty(), "{:?}", case.hands_todo());
    assert!(!case.state().join("host.fall").exists());

    // A new fall, cleared by the watch's pass alone: no build runs in between.
    case.free(10 * GIB);
    let refused = build::run(
        &case.state(),
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
        host::clear_fall(&case.state(), 19 * GIB, 20 * GIB).expect("the pass reads"),
        None,
        "under disk_clear nothing is withdrawn"
    );
    assert_eq!(state_of(&next), Some(DecisionRequestState::Escalated));
    assert_eq!(
        host::clear_fall(&case.state(), 30 * GIB, 20 * GIB).expect("the pass withdraws"),
        Some(next.clone())
    );
    assert_eq!(state_of(&next), Some(DecisionRequestState::Withdrawn));
    assert_eq!(
        host::clear_fall(&case.state(), 30 * GIB, 20 * GIB).expect("a second pass reads"),
        None,
        "withdrawn once"
    );
}

/// A reclaim that frees 1G and leaves, only a person may remove, an 8G cache under the case's
/// home and three `/var/tmp` entries of the case: one named after the user (3G), one after a
/// third party's project (2G), and the smaller ones in one row.
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
        ],
        reached: false,
    })
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

/// A running grant holds only what its run has not written yet: free disk already counts the
/// growth it recorded, so a reservation beside it is not refused for space that is in use.
#[test]
fn a_running_grant_holds_its_reservation_less_what_it_wrote() {
    let case = Case::new("running-less-written");
    // 12G free over a 10G floor: 2G to reserve.
    case.free(12 * GIB);
    let state = case.state();
    let sources = case.sources(reclaim_nothing);
    let first = case.request(2 * GIB, "while [ ! -e stop ]; do sleep 0.05; done");
    let runner = thread::spawn(move || build::run(&state, &settings(), &sources, &first));
    wait_until("the first run is running", || {
        host::rows(&case.state())
            .is_ok_and(|rows| rows.iter().any(|row| row.state == HostGrantState::Running))
    });
    let running = case
        .grants()
        .into_iter()
        .find(|row| row.state == HostGrantState::Running)
        .expect("a running grant");
    // The first run has written 1.5G of its 2G: it holds 0.5G more.
    host::record_peak_of(&case.state(), &running.grant_id, GIB + GIB / 2)
        .expect("the growth is recorded");

    let second = build::run(
        &case.state(),
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
    let sources = case.sources(reclaim_nothing);
    let first = case.request(2 * GIB, "sleep 0.5");
    let runner = thread::spawn(move || build::run(&state, &settings(), &sources, &first));
    wait_until("the first run is running", || {
        host::rows(&case.state())
            .is_ok_and(|rows| rows.iter().any(|row| row.state == HostGrantState::Running))
    });
    let waiting = Settings {
        wait: Duration::from_secs(30),
        ..settings()
    };
    let second = build::run(
        &case.state(),
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
        !case.state().join("host.fall").exists(),
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
            let sources = case.sources(reclaim_nothing);
            let request = case.request(GIB, "sleep 0.2");
            thread::spawn(move || build::run(&state, &settings(), &sources, &request))
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
        let mut ledger = host::Ledger::lock(&case.state()).expect("the ledger opens");
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
    // 12G free over a 10G floor: the dead 6G grant would leave no room for 1G.
    case.free(12 * GIB);
    let outcome = build::run(
        &case.state(),
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
        let mut ledger = host::Ledger::lock(&case.state()).expect("the ledger opens");
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
        &case.state(),
        &grant,
        group,
        &host::now(OffsetDateTime::now_utc()),
    )
    .expect("the grant runs");
    let lock = host::hold_run(&case.state(), &grant).expect("the run lock is held");

    let swept = host::Ledger::lock(&case.state())
        .expect("the ledger opens")
        .sweep(&host::now(OffsetDateTime::now_utc()))
        .expect("the sweep reads");
    assert!(swept.is_empty(), "a held run is not swept: {swept:?}");
    host::record_peak_of(&case.state(), &grant, GIB / 2).expect("its peak is recorded");
    host::release(
        &case.state(),
        &grant,
        Some(ReleaseReason::Exited),
        &host::now(OffsetDateTime::now_utc()),
    )
    .expect("its own release is kept");
    host::end_run(&case.state(), &grant, lock);
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
