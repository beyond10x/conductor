//! Adversarial cases on `story:instance-upgrade` (wave v1-3): the release tag of
//! `POST /v1/instances/{name}/upgrade` becomes a directory under the daemon's, which the daemon
//! removes before anything reads the tag. A tag with `..` removes the installed binaries, or any
//! directory the daemon's user may remove. The release here refuses a tag that is no x.y.z, as
//! `conductor upgrade`'s download does (`crate::upgrade::fetch_tag`).

mod active;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Instant, SystemTime};

use anyhow::{Result, bail};
use conductor_cli::confine::systemd::Service;
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Units;
use conductor_cli::daemon::upgrade::{self, Refusal, Releases, Upgraded};
use conductor_model::instance::ReconcileReason;

#[derive(Debug, Default, Clone)]
struct Fake {
    running: BTreeSet<String>,
}

impl Units for Fake {
    fn start(&mut self, service: &Service) -> Result<()> {
        self.running.insert(service.unit.clone());
        Ok(())
    }

    fn stop(&mut self, unit: &str) -> Result<()> {
        self.running.remove(unit);
        Ok(())
    }

    fn active(&mut self, unit: &str) -> Result<bool> {
        Ok(self.running.contains(unit))
    }

    fn answers(&mut self, unit: &str) -> Result<bool> {
        Ok(self.running.contains(unit))
    }

    fn enable(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn disable(&mut self, _: &str) -> Result<()> {
        Ok(())
    }
}

/// A release that, like the real download, refuses a tag that is no x.y.z.
#[derive(Debug, Default, Clone)]
struct Release;

impl Releases for Release {
    fn fetch(&mut self, tag: &str, work: &Path) -> Result<PathBuf> {
        let version = tag.trim().trim_start_matches('v');
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|part| part.parse::<u64>().is_err()) {
            bail!("ReleaseUnread: {tag:?} is no x.y.z");
        }
        let file = work.join("conductor");
        fs::write(&file, format!("conductor {tag}"))?;
        Ok(file)
    }

    fn doctor(&mut self, _: &Path, _: &Service) -> Result<()> {
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
            .join("adversary-v1-w3-upgrade")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn config(&self) {
        let store = format!("sqlite:{}", self.dir.join("shared.sqlite").display());
        let root = self.dir.join("alpha");
        let text = format!(
            "version: conductor.config/2\ndaemon: {{store: {store:?}, port: 7300}}\ninstances:\n  \
             - name: alpha\n    placement: local\n    session_prefix: alpha\n    \
             sources: [{{github: example-org}}]\n    \
             checkouts: {{root: {root:?}, trees: {trees:?}}}\n    \
             records: {records:?}\n    state: {state:?}\n    cache: {cache:?}\n",
            root = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    fn daemon(&self) -> Daemon<Fake> {
        let mut daemon = Daemon::open(
            self.file(),
            self.dir.join("home"),
            self.dir.join("daemon"),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Fake::default(),
        )
        .expect("open the daemon");
        daemon.releases = Box::new(Release);
        daemon
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("the first pass");
        daemon
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

/// alpha runs on an installed 1.1.0; an upgrade to the tag `../binaries` is refused (no x.y.z)
/// and must leave the binary alpha runs, and would roll back to, where it is.
#[test]
fn a_refused_tag_with_dot_dot_leaves_the_installed_binaries() {
    let case = Case::new("binaries");
    case.config();
    let mut daemon = case.daemon();
    let now = SystemTime::now();
    let upgraded = daemon
        .upgrade("alpha", "v1.1.0", &operator(), now)
        .expect("the upgrade");
    assert_eq!(
        upgraded,
        Upgraded::Upgrading {
            version: "1.1.0".to_owned()
        }
    );
    daemon.settle(now).expect("settle");
    let installed = case
        .dir
        .join("daemon")
        .join(upgrade::BINARIES_DIR)
        .join("1.1.0")
        .join("conductor");
    assert!(installed.exists());

    let refused = daemon.upgrade("alpha", "../binaries", &operator(), SystemTime::now());
    assert!(
        matches!(&refused, Err(Refusal::Refused { .. })),
        "{refused:?}"
    );
    assert!(
        installed.exists(),
        "the binary alpha runs, {}, was removed by a refused upgrade",
        installed.display()
    );
}

/// The tag `../../outside` names a directory beside the daemon's own; a refused upgrade must
/// not remove it.
#[test]
fn a_refused_tag_with_dot_dot_removes_nothing_outside_the_daemon_directory() {
    let case = Case::new("outside");
    case.config();
    let outside = case.dir.join("outside");
    fs::create_dir_all(&outside).expect("create the directory beside the daemon's");
    fs::write(outside.join("keep"), "an operator's file").expect("write a file there");
    let mut daemon = case.daemon();

    let refused = daemon.upgrade("alpha", "../../outside", &operator(), SystemTime::now());
    assert!(
        matches!(&refused, Err(Refusal::Refused { .. })),
        "{refused:?}"
    );
    assert!(
        outside.join("keep").exists(),
        "{} was removed by a refused upgrade",
        outside.display()
    );
}

/// The pinned binary of `name` in the daemon directory's `binaries.json`: `desired` or
/// `previous`.
fn pinned(case: &Case, name: &str, which: &str) -> Option<PathBuf> {
    let text = fs::read_to_string(case.dir.join("daemon").join("binaries.json")).ok()?;
    let pins: serde_json::Value = serde_json::from_str(&text).ok()?;
    pins[name][which]["path"].as_str().map(PathBuf::from)
}

/// Finding 8: an instance's first start pins the daemon's own binary. A self-install over the
/// daemon's path then changes, under the runtime, the binary it is pinned to and would roll back
/// to. The first start copies the daemon's binary into `binaries/<version>/` of the daemon's
/// directory and pins the copy; a later install over the daemon's path leaves both pins' bytes.
#[test]
fn a_self_install_over_the_daemon_path_changes_no_pinned_or_rollback_binary() {
    let case = Case::new("self-install");
    case.config();
    let own = case.dir.join("bin").join("conductor");
    fs::create_dir_all(own.parent().expect("a parent")).expect("create bin/");
    fs::write(&own, "conductor 1.0.0, the daemon's build").expect("write the daemon's binary");
    let mut daemon = Daemon::open(
        case.file(),
        case.dir.join("home"),
        case.dir.join("daemon"),
        Binary {
            path: own.clone(),
            version: "1.0.0".to_owned(),
        },
        Fake::default(),
    )
    .expect("open the daemon");
    daemon.releases = Box::new(Release);
    daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("the first pass");

    let copy = case
        .dir
        .join("daemon")
        .join(upgrade::BINARIES_DIR)
        .join("1.0.0")
        .join("conductor");
    assert_eq!(
        pinned(&case, "alpha", "desired").as_deref(),
        Some(copy.as_path()),
        "the first start pins the daemon's own path, not a copy in the daemon's directory"
    );

    let now = SystemTime::now();
    daemon
        .upgrade("alpha", "v1.1.0", &operator(), now)
        .expect("the upgrade");
    assert_eq!(
        pinned(&case, "alpha", "previous").as_deref(),
        Some(copy.as_path()),
        "the rollback binary is not the pinned copy"
    );

    // A self-install of another build over the daemon's path.
    fs::write(&own, "conductor 1.2.0, a self-install").expect("overwrite the daemon's binary");
    assert_eq!(
        fs::read_to_string(&copy).expect("read the pinned copy"),
        "conductor 1.0.0, the daemon's build",
        "the binary alpha rolls back to changed under it"
    );
}
