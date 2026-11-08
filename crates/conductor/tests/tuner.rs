//! `story:tuner`: after the daily tick the `tuner` role reads the KPI readings and the open
//! findings and writes one settings proposal; inside the tuning grant it applies it through the
//! function `PUT /v1/config` calls and, after the window, keeps or reverts it; outside the grant
//! it files a finding. The brief lists every proposal, and with an empty grant nothing is applied.

mod active;

use std::fs;
use std::path::PathBuf;

use conductor_cli::store::{Store, open, open_url};
use conductor_cli::tune::{self, Tuning};
use conductor_model::behaviour::{
    ConfigRevisionStorage, FrictionFindingStorage, MetricReadingStorage, MetricStorage,
    SettingsProposalStorage,
};
use conductor_model::improvement::{
    FrictionFindingState, MetricData, MetricId, MetricReadingData, MetricReadingId,
    MetricReadingSnapshot, MetricReadingState, MetricSnapshot, MetricState,
    SettingsProposalSnapshot, SettingsProposalState, TargetDirection,
};
use conductor_model::operator::{Actor, ActorKind};
use conductor_model::primitives::{Decimal, Timestamp};

const METRIC: &str = "K4.median";

struct Case {
    dir: PathBuf,
    file: PathBuf,
    store: Store,
    daemon: Store,
}

impl Case {
    /// A config file of one instance `one` whose `globals.tuning` is `grant`, and its stores,
    /// with the KPI `K4.median` (at most 1 hour) and its reading of `day`, 2.5 hours.
    fn new(name: &str, grant: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("tuner")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("state")).expect("create the case's directory");
        let file = dir.join("conductor.yaml");
        fs::write(
            &file,
            format!(
                "version: conductor.config/2\n\
                 globals:\n\
                 \x20 tuning: {grant}\n\
                 instances:\n\
                 \x20 - name: one\n\
                 \x20   placement: systemd\n\
                 \x20   session_prefix: one\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: ~/one, trees: ~/one-trees}}\n\
                 \x20   controllers: {{max_working: 2}}\n"
            ),
        )
        .expect("write the config file");
        let mut store = open(&dir.join("state")).expect("the instance's store opens");
        let daemon = open_url(
            &format!("sqlite:{}", dir.join("daemon.sqlite").display()),
            conductor_cli::daemon::TENANT,
        )
        .expect("the daemon's store opens");
        MetricStorage::put(
            &mut store,
            MetricSnapshot {
                state: MetricState::Targeted,
                data: MetricData {
                    metric_id: MetricId(METRIC.to_owned()),
                    name: "intent to merged, median".to_owned(),
                    unit: "hours".to_owned(),
                    instrument: "the records' dispatches/ log".to_owned(),
                    definition: "hours from sent to done".to_owned(),
                    baseline: None,
                    baseline_period: None,
                    target: Some(Decimal("1".to_owned())),
                    direction: Some(TargetDirection::AtMost),
                    target_owner: Some("operator".to_owned()),
                    target_set_at: Some(Timestamp("2026-11-01T00:00:00Z".to_owned())),
                    actor: daemon_actor(),
                },
            },
        );
        let mut case = Self {
            dir,
            file,
            store,
            daemon,
        };
        case.reading("2026-11-07", "2.5");
        case
    }

    /// Records the instance-wide reading of `K4.median` for `day`, superseding the one before.
    fn reading(&mut self, day: &str, value: &str) {
        for mut held in MetricReadingStorage::list(&self.store) {
            if held.state == MetricReadingState::Recorded {
                held.state = MetricReadingState::Superseded;
                MetricReadingStorage::put(&mut self.store, held);
            }
        }
        let met = value.parse::<f64>().expect("a number") <= 1.0;
        MetricReadingStorage::put(
            &mut self.store,
            MetricReadingSnapshot {
                state: MetricReadingState::Recorded,
                data: MetricReadingData {
                    reading_id: MetricReadingId(format!("{METRIC}|instance|{day}")),
                    metric_id: MetricId(METRIC.to_owned()),
                    day: day.to_owned(),
                    scope: "instance".to_owned(),
                    value: Decimal(value.to_owned()),
                    sources: Vec::new(),
                    target: Some(Decimal("1".to_owned())),
                    met: Some(met),
                    recorded_at: Timestamp(format!("{day}T23:59:00Z")),
                    actor: daemon_actor(),
                },
            },
        );
        self.store.check().expect("the reading is kept");
    }

    /// The tuner's step of the daily tick for `day`: the brief's lines.
    fn tune(&mut self, day: &str) -> Vec<String> {
        let tuning = Tuning {
            file: self.file.clone(),
            home: self.dir.clone(),
            instance: "one".to_owned(),
            day: day.to_owned(),
            now: Timestamp(format!("{day}T08:07:00Z")),
        };
        tune::tune(&tuning, &mut self.store, &mut self.daemon).expect("the tuner runs")
    }

    fn proposals(&self) -> Vec<SettingsProposalSnapshot> {
        SettingsProposalStorage::list(&self.store)
    }

    fn config(&self) -> String {
        fs::read_to_string(&self.file).expect("read the config file")
    }

    fn revisions(&self) -> usize {
        ConfigRevisionStorage::list(&self.daemon).len()
    }
}

fn daemon_actor() -> Actor {
    Actor {
        kind: ActorKind::Daemon,
        operator: None,
        agent: None,
    }
}

fn mentions(lines: &[String], words: &[&str]) -> bool {
    lines
        .iter()
        .any(|line| words.iter().all(|word| line.contains(word)))
}

#[test]
fn with_an_empty_grant_the_tuner_proposes_files_a_finding_and_applies_nothing() {
    let mut case = Case::new("empty-grant", "{keys: []}");
    let before = case.config();

    let brief = case.tune("2026-11-07");

    let proposals = case.proposals();
    assert_eq!(proposals.len(), 1, "one proposal: {proposals:?}");
    let proposal = &proposals[0].data;
    assert_eq!(proposals[0].state, SettingsProposalState::Refused);
    assert_eq!(proposal.key, "controllers.max_working");
    assert_eq!((proposal.old.as_str(), proposal.new.as_str()), ("2", "3"));
    assert_eq!(proposal.metric_id.0, METRIC);
    assert_eq!(proposal.reading.0, "K4.median|instance|2026-11-07");
    assert_eq!(proposal.baseline.0, "2.5");
    assert!(proposal.window_days >= 1);
    let actor = proposal
        .actor
        .agent
        .as_ref()
        .expect("the tuner is an agent");
    assert_eq!(actor.role.0, "tuner");
    let finding = proposal
        .finding
        .clone()
        .expect("a finding for a key outside the grant");
    let held = FrictionFindingStorage::get(&case.store, &finding).expect("the finding is kept");
    assert_eq!(held.state, FrictionFindingState::Open);
    assert_eq!(held.data.metric_id.0, METRIC);

    assert_eq!(case.config(), before, "nothing is applied");
    assert_eq!(case.revisions(), 0, "no config change is recorded");
    assert!(
        mentions(
            &brief,
            &["Refused", "controllers.max_working", "2", "3", "2.5"]
        ),
        "{brief:?}"
    );

    // The next tick files the same friction once, not again.
    case.tune("2026-11-08");
    assert_eq!(FrictionFindingStorage::list(&case.store).len(), 1);
}

#[test]
fn inside_the_grant_a_change_is_applied_and_kept_on_an_improved_reading() {
    let mut case = Case::new(
        "kept",
        "{keys: [{path: controllers.max_working, min: 1, max: 8}]}",
    );

    let brief = case.tune("2026-11-07");
    let proposals = case.proposals();
    assert_eq!(proposals.len(), 1, "{proposals:?}");
    assert_eq!(proposals[0].state, SettingsProposalState::Applied);
    assert!(proposals[0].data.revision.is_some());
    assert!(
        case.config().contains("max_working: 3"),
        "{}",
        case.config()
    );
    assert_eq!(case.revisions(), 1);
    assert!(
        mentions(&brief, &["Applied", "controllers.max_working"]),
        "{brief:?}"
    );

    // Inside the window: one change at a time, none settled.
    case.reading("2026-11-09", "0.5");
    case.tune("2026-11-09");
    assert_eq!(case.proposals().len(), 1);
    assert_eq!(case.proposals()[0].state, SettingsProposalState::Applied);

    case.reading("2026-11-14", "0.8");
    let brief = case.tune("2026-11-14");
    let settled = case
        .proposals()
        .into_iter()
        .find(|one| one.data.proposal_id == proposals[0].data.proposal_id)
        .expect("the proposal");
    assert_eq!(settled.state, SettingsProposalState::Kept);
    assert_eq!(
        settled.data.verdict.as_deref(),
        Some("K4.median|instance|2026-11-14")
    );
    assert!(
        case.config().contains("max_working: 3"),
        "{}",
        case.config()
    );
    assert_eq!(case.revisions(), 1);
    assert!(mentions(&brief, &["Kept", "2.5", "0.8"]), "{brief:?}");
}

#[test]
fn after_the_window_a_change_that_did_not_improve_the_reading_is_reverted() {
    let mut case = Case::new(
        "reverted",
        "{keys: [{path: controllers.max_working, min: 1, max: 8}]}",
    );
    case.tune("2026-11-07");
    assert!(case.config().contains("max_working: 3"));

    case.reading("2026-11-14", "3.0");
    let brief = case.tune("2026-11-14");
    let reverted = case
        .proposals()
        .into_iter()
        .find(|one| one.state == SettingsProposalState::Reverted)
        .expect("the proposal is reverted");
    assert!(reverted.data.revision.is_some());
    assert!(
        case.config().contains("max_working: 2"),
        "{}",
        case.config()
    );
    assert_eq!(case.revisions(), 2, "the apply and the revert");
    assert!(mentions(&brief, &["Reverted", "2.5", "3.0"]), "{brief:?}");
}

#[test]
fn a_duration_or_a_size_in_the_grant_is_refused_by_name() {
    let mut case = Case::new(
        "duration",
        "{keys: [{path: cadence.watch, min: 1, max: 600}]}",
    );
    let before = case.config();

    let brief = case.tune("2026-11-07");

    let proposals = case.proposals();
    assert_eq!(proposals.len(), 1, "{proposals:?}");
    assert_eq!(proposals[0].state, SettingsProposalState::Refused);
    assert_eq!(proposals[0].data.key, "cadence.watch");
    let reason = proposals[0].data.reason.clone().unwrap_or_default();
    assert!(
        reason.contains("cadence.watch") && reason.contains("not a number"),
        "{reason}"
    );
    assert_eq!(case.config(), before);
    assert_eq!(case.revisions(), 0);
    assert!(mentions(&brief, &["Refused", "cadence.watch"]), "{brief:?}");
}
