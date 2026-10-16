//! The fixture the series tests share (`story:dashboard-history`, `story:dashboard-series-api`,
//! `story:dashboard-kpi-panels`): two days of snapshots of `alpha` and `beta`, 2026-10-12 and
//! 2026-10-13 (UTC), and the dispatch, decision, guard and host records of the same two days.
//!
//! Snapshot A starts 2026-10-12T06:00Z, snapshot B 2026-10-13T06:00Z. What each holds, and what
//! the rollups count of it:
//!
//! | what | A | B |
//! |---|---|---|
//! | open PRs | alpha#1 (opened 10-11), alpha#2 (10-12), beta#7 (10-12) | alpha#2, beta#8 (10-13) |
//! | merged PRs | alpha#3 (10-11 12:00), alpha#4 `Revert …` (10-12 03:00), beta#5 (10-12 04:00) | the same, beta#7 (10-13 02:00), alpha#6 (10-13 03:00) |
//! | runs | alpha check ok (10-12), alpha release failed (10-12), beta check failed (10-12) | alpha check ok (10-13), alpha release ok (10-13), beta check failed (10-12, the same run) |
//! | releases | alpha v1.0.0 (10-12 05:00) | alpha v1.0.0, alpha v1.0.1 (10-13 05:00) |
//!
//! alpha#1 is open in A and gone from B without a merge: closed unmerged on 10-13. beta#7 is gone
//! from B's open list because it merged.
//!
//! Records: dispatch D-1 (alpha) sent 10-12 08:00, done 10-12 10:00 (2 h); D-2 (beta) sent 10-13
//! 00:00, done 10-13 06:00 (6 h); D-3 still sent. Decisions: DEC-1 by the operator 10-12, DEC-2 by
//! conductor 10-12, DEC-3 by the operator 10-13. Requests: R-1 escalated O, raised 10-12; R-2
//! escalated H, raised 10-13 and withdrawn; R-3 open, never escalated. Guard: Bash denied 10-12,
//! Edit denied 10-12, Bash denied 10-13, Bash allowed 10-13. Host: G-1 alpha disk granted at once
//! (10-12), G-2 beta disk refused (10-12), G-3 alpha disk granted ten minutes after its request
//! (10-13), G-4 a build slot.

#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store::{self, Store};
use conductor_model::behaviour::{
    DecisionRequestStorage, DecisionStorage, DispatchStorage, GuardDecisionStorage,
    HostGrantStorage,
};
use conductor_model::config::InstanceName;
use conductor_model::decision::{
    Decider, DecisionClass, DecisionData, DecisionId, DecisionRequestData, DecisionRequestSnapshot,
    DecisionRequestState, DecisionSnapshot, DecisionState, RequestId,
};
use conductor_model::direction::GoalId;
use conductor_model::dispatch::{
    DispatchData, DispatchId, DispatchSnapshot, DispatchState, GuardDecisionData, GuardDecisionId,
    GuardDecisionSnapshot, GuardDecisionState, Verdict,
};
use conductor_model::host::{GrantId, GrantKind, HostGrantData, HostGrantSnapshot, HostGrantState};
use conductor_model::observation::{
    CommitSha, Mergeability, RecordMergedPullRequest, RecordPullRequest, RecordRelease,
    RecordRepository, RecordWorkflowRun, RepositoryName, RunConclusion, SnapshotId, SnapshotState,
    StartSnapshot, Visibility,
};
use conductor_model::primitives::{Timestamp, Uuid};

/// The day the fixture's "today" is.
pub const TODAY: &str = "2026-10-13";

/// When snapshot A starts, and snapshot B.
pub const A: &str = "2026-10-12T06:00:00Z";
pub const B: &str = "2026-10-13T06:00:00Z";

/// One case's directory: `state/` (the store) and `records/` (the dispatch log).
pub struct Case {
    pub dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    pub fn new(suite: &str, name: &str) -> Self {
        crate::active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(suite)
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("records/dispatches")).expect("create dispatches/");
        fs::create_dir_all(dir.join("records/decisions")).expect("create decisions/");
        Self { dir }
    }

    pub fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    pub fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    /// The case's store, opened afresh.
    pub fn store(&self) -> Store {
        store::open(&self.state()).expect("open the store")
    }

    /// One snapshot started at `started_at` through `collector`; its id.
    pub fn take(&self, started_at: &str, collector: Collector) -> SnapshotId {
        let start = StartSnapshot {
            started_at: Timestamp(started_at.to_owned()),
            disk_free_bytes: 1_000_000,
            budget: None,
        };
        let taken = snapshot::take_from(self.store(), start, &[collector])
            .expect("the driver ends the snapshot");
        assert_eq!(taken.state, SnapshotState::Complete);
        taken.snapshot_id
    }

    /// The whole fixture: the records, then snapshots A and B, then the rollup of B over the
    /// case's own records (the driver's ran over the instance's, which hold no dispatch log).
    pub fn build(&self) -> SnapshotId {
        self.records_fixture();
        self.take(A, day_a());
        let b = self.take(B, day_b());
        let mut store = self.store();
        conductor_cli::rollup::update(&mut store, &b, &self.records())
            .expect("the rollup of snapshot B");
        store.check().expect("the store kept the rollups");
        b
    }

    /// The dispatch, decision, guard and host records, and the dispatch log's `done` lines.
    pub fn records_fixture(&self) {
        let mut store = self.store();
        let at = |text: &str| Timestamp(text.to_owned());
        let repo = |name: &str| RepositoryName(name.to_owned());
        for (id, repository, state, sent) in [
            ("D-1", "alpha", DispatchState::Done, "2026-10-12T08:00:00Z"),
            ("D-2", "beta", DispatchState::Done, "2026-10-13T00:00:00Z"),
            ("D-3", "alpha", DispatchState::Sent, "2026-10-13T01:00:00Z"),
        ] {
            DispatchStorage::put(
                &mut store,
                DispatchSnapshot {
                    state,
                    data: DispatchData {
                        dispatch_id: DispatchId(id.to_owned()),
                        repository: repo(repository),
                        goal_id: GoalId("G-1".to_owned()),
                        brief: format!("brief of {id}"),
                        sent_at: at(sent),
                        priority: 0,
                        waits_on: None,
                    },
                },
            );
        }
        fs::write(
            self.records().join("dispatches/2026-10.jsonl"),
            "{\"at\":\"2026-10-12T08:00:00Z\",\"event\":\"sent\",\"repo\":\"alpha\",\"id\":\"D-1\"}\n\
             {\"at\":\"2026-10-12T10:00:00Z\",\"event\":\"done\",\"repo\":\"alpha\",\"id\":\"D-1\"}\n\
             {\"at\":\"2026-10-13T06:00:00Z\",\"event\":\"done\",\"repo\":\"beta\",\"id\":\"D-2\"}\n",
        )
        .expect("write the dispatch log");
        for (id, decided_by, decided_at) in [
            ("DEC-1", Decider::Operator, "2026-10-12T09:00:00Z"),
            ("DEC-2", Decider::Conductor, "2026-10-12T09:30:00Z"),
            ("DEC-3", Decider::Operator, "2026-10-13T01:00:00Z"),
        ] {
            DecisionStorage::put(
                &mut store,
                DecisionSnapshot {
                    state: DecisionState::InForce,
                    data: DecisionData {
                        decision_id: DecisionId(id.to_owned()),
                        decision_class: DecisionClass::O,
                        decided_by,
                        question: "which".to_owned(),
                        options: "a, b".to_owned(),
                        choice: "a".to_owned(),
                        reason: "it fits".to_owned(),
                        evidence: "the run".to_owned(),
                        decided_at: at(decided_at),
                        recorded_by: None,
                        operator_words: None,
                        repository: Some(repo("alpha")),
                    },
                },
            );
        }
        for (id, state, class, raised) in [
            (
                "R-1",
                DecisionRequestState::Escalated,
                Some(DecisionClass::O),
                "2026-10-12T08:00:00Z",
            ),
            (
                "R-2",
                DecisionRequestState::Withdrawn,
                Some(DecisionClass::H),
                "2026-10-13T02:00:00Z",
            ),
            (
                "R-3",
                DecisionRequestState::Open,
                None,
                "2026-10-13T03:00:00Z",
            ),
        ] {
            DecisionRequestStorage::put(
                &mut store,
                DecisionRequestSnapshot {
                    state,
                    data: DecisionRequestData {
                        request_id: RequestId(id.to_owned()),
                        repository: repo("alpha"),
                        question: "which".to_owned(),
                        options: "a, b".to_owned(),
                        recommendation: "a".to_owned(),
                        raised_at: at(raised),
                        blocker: None,
                        escalation_class: class,
                        escalation_reason: class.map(|_| "needs a person".to_owned()),
                        dry_run: None,
                        no_dry_run: None,
                        kind: None,
                        lettered_options: None,
                        recommended_letters: None,
                        default_letters: None,
                        deadline: None,
                        decision_id: None,
                    },
                },
            );
        }
        for (n, tool, verdict, decided_at) in [
            (1, "Bash", Verdict::Deny, "2026-10-12T10:00:00Z"),
            (2, "Edit", Verdict::Deny, "2026-10-12T11:00:00Z"),
            (3, "Bash", Verdict::Deny, "2026-10-13T01:00:00Z"),
            (4, "Bash", Verdict::Allow, "2026-10-13T02:00:00Z"),
        ] {
            GuardDecisionStorage::put(
                &mut store,
                GuardDecisionSnapshot {
                    state: GuardDecisionState::Recorded,
                    data: GuardDecisionData {
                        guard_decision_id: GuardDecisionId(Uuid(format!(
                            "00000000-0000-4000-8000-00000000000{n}"
                        ))),
                        repository: repo("alpha"),
                        session_ref: "s-1".to_owned(),
                        tool: tool.to_owned(),
                        target: "x".to_owned(),
                        verdict,
                        reason: "the rule".to_owned(),
                        decided_at: at(decided_at),
                    },
                },
            );
        }
        for (id, repository, kind, state, requested, started) in [
            (
                "G-1",
                Some("alpha"),
                GrantKind::Disk,
                HostGrantState::Released,
                "2026-10-12T07:00:00Z",
                Some("2026-10-12T07:00:05Z"),
            ),
            (
                "G-2",
                Some("beta"),
                GrantKind::Disk,
                HostGrantState::Refused,
                "2026-10-12T08:00:00Z",
                None,
            ),
            (
                "G-3",
                Some("alpha"),
                GrantKind::Disk,
                HostGrantState::Running,
                "2026-10-13T01:00:00Z",
                Some("2026-10-13T01:10:00Z"),
            ),
            (
                "G-4",
                None,
                GrantKind::BuildSlot,
                HostGrantState::Granted,
                "2026-10-13T01:00:00Z",
                None,
            ),
        ] {
            HostGrantStorage::put(
                &mut store,
                HostGrantSnapshot {
                    state,
                    data: HostGrantData {
                        grant_id: GrantId(id.to_owned()),
                        instance: InstanceName("example-org".to_owned()),
                        repository: repository.map(repo),
                        kind,
                        bytes: Some(1 << 30),
                        peak_bytes: None,
                        pauses: None,
                        command: Some("cargo test".to_owned()),
                        process_group: None,
                        session_name: None,
                        reclaimed_bytes: None,
                        shortfall_bytes: None,
                        requested_at: at(requested),
                        started_at: started.map(at),
                        released_at: None,
                        release_reason: None,
                    },
                },
            );
        }
        store.check().expect("the store kept the records");
    }
}

/// An open PR, a merged PR, a run and a release, as the fixture lists them.
type Open = (&'static str, i64, &'static str);
type Merged = (&'static str, i64, &'static str, &'static str);
type Run = (&'static str, &'static str, RunConclusion, &'static str);
type Release = (&'static str, &'static str, &'static str);

/// A collector recording `alpha` and `beta` and the rows given.
pub fn collector(
    open: Vec<Open>,
    merged: Vec<Merged>,
    runs: Vec<Run>,
    releases: Vec<Release>,
) -> Collector {
    Collector::new("fixture", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let name = |name: &str| RepositoryName(name.to_owned());
        let at = |text: &str| Timestamp(text.to_owned());
        for repository in ["alpha", "beta"] {
            record.repository(RecordRepository {
                snapshot_id: snapshot.clone(),
                repository: name(repository),
                visibility: Visibility::Public,
                archived: false,
                local_checkout: true,
                main_head: CommitSha(format!("head-of-{repository}")),
                main_committed_at: at("2026-10-12T00:00:00Z"),
                main_commits_7d: 1,
                real_commits_7d: 1,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 0,
                open_issues: 0,
                oldest_open_issue_at: None,
                latest_release: None,
                latest_release_at: None,
                unreleased_commits: None,
                planning_store_version: None,
                in_catalog: None,
                gates_hooks_version: None,
                gates_version: None,
                wave_width: None,
                wave_ready: None,
                wave_reason: None,
            })?;
        }
        for (repository, number, opened_at) in &open {
            record.pull_request(RecordPullRequest {
                snapshot_id: snapshot.clone(),
                repository: name(repository),
                number: *number,
                title: format!("Change {number}"),
                draft: false,
                mergeability: Mergeability::Mergeable,
                failing_checks: 0,
                failing_check_names: Vec::new(),
                opened_at: at(opened_at),
                updated_at: at(opened_at),
                url: None,
            })?;
        }
        for (repository, number, title, merged_at) in &merged {
            record.merged_pull_request(RecordMergedPullRequest {
                snapshot_id: snapshot.clone(),
                repository: name(repository),
                number: *number,
                title: (*title).to_owned(),
                merged_at: at(merged_at),
                url: format!("https://example.invalid/{repository}/pull/{number}"),
            })?;
        }
        for (repository, workflow, conclusion, run_at) in &runs {
            record.workflow_run(RecordWorkflowRun {
                snapshot_id: snapshot.clone(),
                repository: name(repository),
                workflow: (*workflow).to_owned(),
                conclusion: *conclusion,
                run_at: at(run_at),
            })?;
        }
        for (repository, tag, published_at) in &releases {
            record.release(RecordRelease {
                snapshot_id: snapshot.clone(),
                repository: name(repository),
                tag: (*tag).to_owned(),
                published_at: at(published_at),
                url: format!("https://example.invalid/{repository}/releases/{tag}"),
            })?;
        }
        Ok(())
    })
}

/// The merged PRs snapshot A sees.
fn merged_a() -> Vec<Merged> {
    vec![
        ("alpha", 3, "Add the board", "2026-10-11T12:00:00Z"),
        (
            "alpha",
            4,
            "Revert \"Add the board\"",
            "2026-10-12T03:00:00Z",
        ),
        ("beta", 5, "Fix the build", "2026-10-12T04:00:00Z"),
    ]
}

/// Snapshot A's collector.
pub fn day_a() -> Collector {
    collector(
        vec![
            ("alpha", 1, "2026-10-11T10:00:00Z"),
            ("alpha", 2, "2026-10-12T01:00:00Z"),
            ("beta", 7, "2026-10-12T02:00:00Z"),
        ],
        merged_a(),
        vec![
            (
                "alpha",
                "check",
                RunConclusion::Success,
                "2026-10-12T03:30:00Z",
            ),
            (
                "alpha",
                "release",
                RunConclusion::Failure,
                "2026-10-12T04:30:00Z",
            ),
            (
                "beta",
                "check",
                RunConclusion::Failure,
                "2026-10-12T05:00:00Z",
            ),
        ],
        vec![("alpha", "v1.0.0", "2026-10-12T05:00:00Z")],
    )
}

/// Snapshot B's collector.
pub fn day_b() -> Collector {
    let mut merged = merged_a();
    merged.push(("beta", 7, "Speed up", "2026-10-13T02:00:00Z"));
    merged.push(("alpha", 6, "Add the chart", "2026-10-13T03:00:00Z"));
    collector(
        vec![
            ("alpha", 2, "2026-10-12T01:00:00Z"),
            ("beta", 8, "2026-10-13T01:00:00Z"),
        ],
        merged,
        vec![
            (
                "alpha",
                "check",
                RunConclusion::Success,
                "2026-10-13T04:00:00Z",
            ),
            (
                "alpha",
                "release",
                RunConclusion::Success,
                "2026-10-13T04:10:00Z",
            ),
            (
                "beta",
                "check",
                RunConclusion::Failure,
                "2026-10-12T05:00:00Z",
            ),
        ],
        vec![
            ("alpha", "v1.0.0", "2026-10-12T05:00:00Z"),
            ("alpha", "v1.0.1", "2026-10-13T05:00:00Z"),
        ],
    )
}

/// The fixture's "today", as a date.
pub fn today() -> time::Date {
    time::Date::parse(TODAY, &time::format_description::well_known::Iso8601::DATE).expect("a date")
}
