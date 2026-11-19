//! `story:box-instance-upgrade`: `POST /v1/instances/{name}/upgrade` upgrades a runtime that runs
//! in a substrate box.
//!
//! The upgrade asks substrate whether the box's session runs, as the reconciler does, never the
//! service manager, which knows no `substrate:<session>` unit. Its restart closes the old box and
//! opens a new one on the new binary; it settles when substrate says the new session runs; it
//! rolls back, closing that box and opening one on the previous binary, when the new runtime stops
//! running. A box that does not run is still `InstanceStateConflict`.
//!
//! Every case runs against a fake substrate ([`Fake`]), a service manager that answers nothing
//! ([`NoUnits`]) and a fake release ([`Release`]); no real daemon, bubblewrap or unit runs.

mod active;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant, SystemTime};

use anyhow::{Result, bail};
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Units;
use conductor_cli::daemon::upgrade::{self, Refusal, Releases, Upgraded};
use conductor_cli::placement::{BoxRequest, Opened, Refused, Substrate};
use conductor_cli::systemd::Service;
use conductor_model::behaviour::ReconciliationStorage;
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use serde_json::Value;

/// What the fake substrate saw: each box asked for, each session stopped, each workspace closed,
/// and the sessions that run.
#[derive(Debug, Default)]
struct Log {
    opened: Vec<BoxRequest>,
    stopped: Vec<String>,
    closed: Vec<String>,
    running: BTreeSet<String>,
}

/// A substrate daemon that opens every box it is asked for: workspace `ws_<n>`, session `ses_<n>`.
#[derive(Debug, Default, Clone)]
struct Fake {
    log: Arc<Mutex<Log>>,
}

impl Fake {
    fn log(&self) -> std::sync::MutexGuard<'_, Log> {
        self.log.lock().expect("the log")
    }
}

impl Substrate for Fake {
    fn open(&mut self, request: &BoxRequest) -> Result<Opened, Refused> {
        let mut log = self.log();
        log.opened.push(request.clone());
        let n = log.opened.len();
        let session = format!("ses_{n}");
        log.running.insert(session.clone());
        Ok(Opened {
            workspace: format!("ws_{n}"),
            session,
        })
    }

    fn running(&mut self, session: &str) -> Result<bool> {
        Ok(self.log().running.contains(session))
    }

    fn stop(&mut self, session: &str) -> Result<()> {
        let mut log = self.log();
        log.running.remove(session);
        log.stopped.push(session.to_owned());
        Ok(())
    }

    fn close(&mut self, workspace: &str) -> Result<()> {
        self.log().closed.push(workspace.to_owned());
        Ok(())
    }
}

/// A service manager that records every call and runs nothing: a boxed runtime needs none.
#[derive(Debug, Default)]
struct NoUnits {
    calls: Vec<String>,
}

impl Units for NoUnits {
    fn start(&mut self, service: &Service) -> Result<()> {
        self.calls.push(format!("start {}", service.unit));
        Ok(())
    }
    fn stop(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("stop {unit}"));
        Ok(())
    }
    fn active(&mut self, unit: &str) -> Result<bool> {
        self.calls.push(format!("active {unit}"));
        Ok(false)
    }
    fn enable(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("enable {unit}"));
        Ok(())
    }
    fn disable(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("disable {unit}"));
        Ok(())
    }
}

/// A release that is always there and whose doctor always passes.
#[derive(Debug, Default, Clone, Copy)]
struct Release;

impl Releases for Release {
    fn fetch(&mut self, tag: &str, work: &Path) -> Result<PathBuf> {
        let file = work.join("conductor");
        fs::write(&file, format!("conductor {tag}"))?;
        Ok(file)
    }

    fn doctor(&mut self, binary: &Path, _: &Service) -> Result<()> {
        if !binary.is_file() {
            bail!("no binary at {}", binary.display());
        }
        Ok(())
    }
}

struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("box-upgrade")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        // The store in a directory of its own: a box writes the whole directory of a store.
        for sub in ["home", "store", "alpha/checkouts"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let root = dir.join("alpha");
        let text = format!(
            "version: conductor.config/2\ndaemon: {{port: 7300, store: {store:?}}}\ninstances:\n  \
             - name: alpha\n    placement: {{daemon: local, adapter: linux}}\n    \
             session_prefix: alpha\n    sources: [{{github: example-org}}]\n    \
             checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n    records: {records:?}\n    \
             state: {state:?}\n    cache: {cache:?}\n",
            store = format!("sqlite:{}", dir.join("store/shared.sqlite").display()),
            checkouts = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        );
        fs::write(dir.join("conductor.yaml"), text).expect("write the config file");
        Self { dir }
    }

    /// The daemon of this case on `fake`, alpha started in its box on `1.0.0`.
    fn daemon(&self, fake: &Fake) -> Daemon<NoUnits> {
        let mut daemon = Daemon::open(
            self.dir.join("conductor.yaml"),
            self.dir.join("home"),
            self.dir.join("daemon"),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            NoUnits::default(),
        )
        .expect("open the daemon");
        daemon.substrate = Box::new(fake.clone());
        daemon.releases = Box::new(Release);
        let done = daemon
            .pass(ReconcileReason::ConfigChange, Instant::now())
            .expect("the first pass");
        assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
        assert_eq!(daemon.started()["alpha"].unit, "substrate:ses_1");
        daemon
    }

    fn installed(&self, version: &str) -> PathBuf {
        self.dir
            .join("daemon")
            .join(upgrade::BINARIES_DIR)
            .join(version)
            .join("conductor")
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

fn operator() -> conductor_model::operator::Actor {
    conductor_cli::actor::operator("operator-one")
}

/// The steps of action `Upgrade`, in order: result and the detail's first word.
fn steps(daemon: &Daemon<NoUnits>) -> Vec<(ReconcileResult, String)> {
    let mut rows = ReconciliationStorage::list(daemon.store());
    rows.sort_by(|left, right| left.data.at.0.cmp(&right.data.at.0));
    rows.into_iter()
        .filter(|row| row.data.action == ReconcileAction::Upgrade)
        .map(|row| {
            let detail = row.data.detail.clone().unwrap_or_default();
            (
                row.data.result,
                detail.split(':').next().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn alpha(daemon: &Daemon<NoUnits>) -> Value {
    daemon
        .rows()
        .into_iter()
        .find(|row| row["name"] == "alpha")
        .expect("alpha's row")
}

/// The service manager was never asked about, nor told to start or stop, a boxed runtime; a
/// pass asks it about the host roles only.
fn untouched(daemon: &Daemon<NoUnits>) {
    let calls = &daemon.units.calls;
    assert!(
        calls
            .iter()
            .all(|call| call.starts_with("active ") && !call.contains("substrate:")),
        "{calls:?}"
    );
    assert!(
        calls.iter().all(|call| !call.contains("alpha")),
        "{calls:?}"
    );
}

#[test]
fn an_upgrade_restarts_a_boxed_runtime_in_a_new_box_on_the_new_binary_and_keeps_it_when_it_runs() {
    let case = Case::new("up");
    let fake = Fake::default();
    let mut daemon = case.daemon(&fake);
    let now = SystemTime::now();

    let upgraded = daemon.upgrade("alpha", "v1.1.0", &operator(), now);
    assert_eq!(
        upgraded,
        Ok(Upgraded::Upgrading {
            version: "1.1.0".to_owned()
        })
    );
    {
        let log = fake.log();
        assert_eq!(log.stopped, ["ses_1"]);
        assert_eq!(log.closed, ["ws_1"]);
        assert_eq!(log.opened.len(), 2);
        assert_eq!(
            log.opened[1].argv[0],
            case.installed("1.1.0").display().to_string(),
            "the new box runs the installed binary"
        );
    }
    assert_eq!(daemon.started()["alpha"].unit, "substrate:ses_2");
    assert_eq!(daemon.started()["alpha"].version, "1.1.0");
    assert_eq!(alpha(&daemon)["state"], "Upgrading");
    assert_eq!(alpha(&daemon)["binary_version"], "1.1.0");

    let done = daemon.settle(now).expect("settle");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(
        (done[0].action, done[0].result),
        (ReconcileAction::Upgrade, ReconcileResult::Succeeded)
    );
    assert_eq!(alpha(&daemon)["state"], "Running");
    assert_eq!(
        steps(&daemon),
        [
            (ReconcileResult::Succeeded, "fetch".to_owned()),
            (ReconcileResult::Succeeded, "install".to_owned()),
            (ReconcileResult::Succeeded, "doctor".to_owned()),
            (ReconcileResult::Succeeded, "restart".to_owned()),
            (ReconcileResult::Succeeded, "up".to_owned()),
        ]
    );

    // The upgraded binary is the instance's desired one: a pass leaves the new box alone.
    let done = daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("a pass");
    assert!(done.is_empty(), "{done:?}");
    assert_eq!(fake.log().opened.len(), 2);
    untouched(&daemon);
}

#[test]
fn a_boxed_runtime_lost_while_upgrading_is_rolled_back_in_a_box_on_the_previous_binary() {
    let case = Case::new("lost");
    let fake = Fake::default();
    let mut daemon = case.daemon(&fake);
    let previous = daemon.started()["alpha"].binary.clone();
    daemon
        .upgrade("alpha", "v1.1.0", &operator(), SystemTime::now())
        .expect("the upgrade");
    fake.log().running.remove("ses_2");

    let done = daemon.tick(Instant::now()).expect("tick");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(
        (done[0].action, done[0].reason, done[0].result),
        (
            ReconcileAction::Upgrade,
            ReconcileReason::LostRuntime,
            ReconcileResult::RolledBack
        )
    );
    {
        let log = fake.log();
        assert_eq!(log.closed, ["ws_1", "ws_2"]);
        assert_eq!(log.opened.len(), 3);
        assert_eq!(
            log.opened[2].argv[0], previous,
            "back on the previous binary"
        );
    }
    assert_eq!(daemon.started()["alpha"].unit, "substrate:ses_3");
    assert_eq!(daemon.started()["alpha"].version, "1.0.0");
    assert_eq!(alpha(&daemon)["state"], "Running");
    assert_eq!(alpha(&daemon)["binary_version"], "1.0.0");
    assert_eq!(steps(&daemon).last().expect("a step").1, "rollback");
    untouched(&daemon);
}

#[test]
fn a_boxed_runtime_that_does_not_run_is_an_instance_state_conflict_and_nothing_restarts() {
    let case = Case::new("stopped");
    let fake = Fake::default();
    let mut daemon = case.daemon(&fake);
    fake.log().running.remove("ses_1");
    let before = ReconciliationStorage::list(daemon.store()).len();

    let refused = daemon.upgrade("alpha", "v1.1.0", &operator(), SystemTime::now());
    assert!(
        matches!(&refused, Err(Refusal::InstanceStateConflict(name)) if name == "alpha"),
        "{refused:?}"
    );
    assert_eq!(ReconciliationStorage::list(daemon.store()).len(), before);
    let log = fake.log();
    assert_eq!(log.opened.len(), 1);
    assert!(log.closed.is_empty(), "{:?}", log.closed);
    drop(log);
    untouched(&daemon);
}

/// The probe a runtime in a box sends conductord's pool at its start, as conductord answers it
/// for the start the token of `alpha`'s runtime names.
fn probe(case: &Case, daemon: &Daemon<NoUnits>) -> Value {
    let token = daemon.started()["alpha"]
        .token
        .clone()
        .expect("the start's token");
    let store = format!("sqlite:{}", case.dir.join("store/shared.sqlite").display());
    let file = case.dir.join("conductor.yaml");
    conductor_cli::token::pool_call(
        &conductor_cli::token::Daemon {
            file: &file,
            store: &store,
            operators: &[],
        },
        "alpha",
        &conductor_cli::token::runtime_tag(&token),
        br#"{"call": "probe", "agent": "alpha-runtime"}"#,
    )
}

/// `story:upgrade-checks-daemon-protocols` (issue #223): a runtime in a box whose session runs
/// may still fail every spawn, when conductord does not answer its token pool calls. An upgrade
/// to a release that probes the pool is answered only once the new start's probe reached it.
#[test]
fn an_upgraded_boxed_runtime_answers_only_once_its_start_reached_the_token_pool() {
    let case = Case::new("probe");
    let fake = Fake::default();
    let mut daemon = case.daemon(&fake);
    let now = SystemTime::now();
    daemon
        .upgrade("alpha", "v1.17.0", &operator(), now)
        .expect("the upgrade");

    let done = daemon.settle(now).expect("settle");
    assert!(
        done.is_empty(),
        "its session runs, its pool not reached: {done:?}"
    );
    assert_eq!(alpha(&daemon)["state"], "Upgrading");

    let answer = probe(&case, &daemon);
    assert_eq!(answer["probed"], true, "{answer}");
    let done = daemon.settle(now).expect("settle");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    assert_eq!(alpha(&daemon)["state"], "Running");
}

/// A boxed runtime upgraded to a release that probes, whose start never reaches the pool, is
/// rolled back at `daemon.upgrade_timeout`, as one that does not answer.
#[test]
fn an_upgraded_boxed_runtime_that_never_reaches_the_token_pool_is_rolled_back() {
    let case = Case::new("no-probe");
    let fake = Fake::default();
    let mut daemon = case.daemon(&fake);
    let now = SystemTime::now();
    daemon
        .upgrade("alpha", "v1.17.0", &operator(), now)
        .expect("the upgrade");
    let later = now + std::time::Duration::from_secs(daemon.upgrade_timeout() + 1);
    let done = daemon.settle(later).expect("settle");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(
        (done[0].reason, done[0].result),
        (ReconcileReason::Cadence, ReconcileResult::RolledBack)
    );
    assert_eq!(daemon.started()["alpha"].version, "1.0.0");
}
