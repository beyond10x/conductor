//! `story:dashboard-history`: each completed snapshot updates one rollup per day and series from
//! the data the store holds, the rollups survive snapshot retention, and two days of fixture
//! snapshots read back as the series the fixture (`series_fixture/mod.rs`) describes.

mod active;
mod series_fixture;

use std::collections::BTreeMap;

use conductor_model::observation::RunConclusion;
use series_fixture::{A, B, Case, collector};

/// Every rollup of `case`'s store, `(series, by, key, day)` to its value.
fn rollups(case: &Case) -> BTreeMap<(String, String, String, String), String> {
    let store = case.store();
    let rows = store.daily_series();
    store.check().expect("the rollups read");
    rows.into_iter()
        .map(|row| {
            let data = row.data;
            ((data.series, data.by, data.key, data.day), data.value.0)
        })
        .collect()
}

/// The value of one rollup, or none.
fn value(
    rows: &BTreeMap<(String, String, String, String), String>,
    series: &str,
    by: &str,
    key: &str,
    day: &str,
) -> Option<String> {
    rows.get(&(
        series.to_owned(),
        by.to_owned(),
        key.to_owned(),
        day.to_owned(),
    ))
    .cloned()
}

fn n(text: &str) -> Option<String> {
    Some(text.to_owned())
}

#[test]
fn a_completed_snapshot_writes_the_rollups_of_its_days() {
    let case = Case::new("rollup", "one-snapshot");
    case.take(A, series_fixture::day_a());
    let rows = rollups(&case);
    assert_eq!(
        value(&rows, "prs_merged", "repository", "alpha", "2026-10-11"),
        n("1")
    );
    assert_eq!(
        value(&rows, "prs_merged", "repository", "alpha", "2026-10-12"),
        n("1")
    );
    assert_eq!(
        value(&rows, "prs_merged", "repository", "beta", "2026-10-12"),
        n("1")
    );
    assert_eq!(
        value(&rows, "reverts", "repository", "alpha", "2026-10-12"),
        n("1")
    );
    assert_eq!(
        value(&rows, "prs_opened", "repository", "alpha", "2026-10-11"),
        n("1")
    );
    assert_eq!(
        value(&rows, "ci_failed", "workflow", "release", "2026-10-12"),
        n("1")
    );
    assert_eq!(
        value(&rows, "ci_passed", "repository", "alpha", "2026-10-12"),
        n("1")
    );
    assert_eq!(
        value(&rows, "releases", "repository", "alpha", "2026-10-12"),
        n("1")
    );
    // Nothing closed: there is no snapshot before A to compare with.
    assert!(
        rows.keys()
            .all(|(series, ..)| series != "prs_closed_unmerged"),
        "{rows:#?}"
    );
}

#[test]
fn two_days_of_snapshots_read_back_as_the_fixture_says() {
    let case = Case::new("rollup", "two-days");
    case.build();
    let rows = rollups(&case);
    let expected = [
        ("prs_merged", "repository", "alpha", "2026-10-11", "1"),
        ("prs_merged", "repository", "alpha", "2026-10-12", "1"),
        ("prs_merged", "repository", "beta", "2026-10-12", "1"),
        ("prs_merged", "repository", "alpha", "2026-10-13", "1"),
        ("prs_merged", "repository", "beta", "2026-10-13", "1"),
        ("prs_opened", "repository", "alpha", "2026-10-11", "1"),
        ("prs_opened", "repository", "alpha", "2026-10-12", "1"),
        ("prs_opened", "repository", "beta", "2026-10-12", "1"),
        ("prs_opened", "repository", "beta", "2026-10-13", "1"),
        (
            "prs_closed_unmerged",
            "repository",
            "alpha",
            "2026-10-13",
            "1",
        ),
        ("reverts", "repository", "alpha", "2026-10-12", "1"),
        ("ci_passed", "repository", "alpha", "2026-10-12", "1"),
        ("ci_passed", "workflow", "check", "2026-10-12", "1"),
        ("ci_passed", "repository", "alpha", "2026-10-13", "2"),
        ("ci_passed", "workflow", "check", "2026-10-13", "1"),
        ("ci_passed", "workflow", "release", "2026-10-13", "1"),
        // The same failed run of beta, seen by both snapshots, counts once.
        ("ci_failed", "repository", "beta", "2026-10-12", "1"),
        ("ci_failed", "repository", "alpha", "2026-10-12", "1"),
        ("ci_failed", "workflow", "check", "2026-10-12", "1"),
        ("ci_failed", "workflow", "release", "2026-10-12", "1"),
        ("releases", "repository", "alpha", "2026-10-12", "1"),
        ("releases", "repository", "alpha", "2026-10-13", "1"),
        ("releases_feature", "repository", "alpha", "2026-10-12", "1"),
        ("releases_patch", "repository", "alpha", "2026-10-13", "1"),
        // alpha#3 (17 h) and alpha#4 (2 h) shipped in v1.0.0, alpha#6 (2 h) in v1.0.1.
        (
            "merged_to_released_hours",
            "total",
            "total",
            "2026-10-12",
            "2",
        ),
        (
            "merged_to_released_hours",
            "total",
            "total",
            "2026-10-13",
            "1",
        ),
        ("dispatch_lead_hours", "total", "total", "2026-10-12", "1"),
        ("dispatch_lead_hours", "total", "total", "2026-10-13", "1"),
        ("operator_touches", "total", "total", "2026-10-12", "2"),
        ("operator_touches", "total", "total", "2026-10-13", "2"),
        ("escalations", "total", "total", "2026-10-12", "1"),
        ("escalations", "total", "total", "2026-10-13", "1"),
        (
            "escalations_unnecessary",
            "total",
            "total",
            "2026-10-13",
            "1",
        ),
        ("guard_denials", "rule", "Bash", "2026-10-12", "1"),
        ("guard_denials", "rule", "Edit", "2026-10-12", "1"),
        ("guard_denials", "rule", "Bash", "2026-10-13", "1"),
        ("disk_granted", "repository", "alpha", "2026-10-12", "1"),
        ("disk_refused", "repository", "beta", "2026-10-12", "1"),
        ("disk_granted", "repository", "alpha", "2026-10-13", "1"),
        ("disk_waited", "repository", "alpha", "2026-10-13", "1"),
    ];
    for (series, by, key, day, count) in expected {
        assert_eq!(
            value(&rows, series, by, key, day),
            n(count),
            "{series} by {by} {key} on {day}; every rollup: {rows:#?}"
        );
    }
    assert_eq!(rows.len(), expected.len(), "no other rollup: {rows:#?}");
}

#[test]
fn the_rollups_survive_retention() {
    let case = Case::new("rollup", "retention");
    case.build();
    let before = rollups(&case);
    let dropped = case.store().retain(1).expect("retention runs");
    assert_eq!(dropped.len(), 1, "snapshot A's observations are dropped");
    // A third snapshot, a day later, sees none of the earlier PRs, runs or releases.
    case.take(
        "2026-10-14T06:00:00Z",
        collector(
            Vec::new(),
            Vec::new(),
            vec![(
                "alpha",
                "check",
                RunConclusion::Success,
                "2026-10-14T01:00:00Z",
            )],
            Vec::new(),
        ),
    );
    case.store().retain(1).expect("retention runs");
    let after = rollups(&case);
    for (key, value) in &before {
        assert_eq!(
            after.get(key),
            Some(value),
            "{key:?} survives retention and later snapshots"
        );
    }
    assert_eq!(
        value(&after, "ci_passed", "repository", "alpha", "2026-10-14"),
        n("1")
    );
    // alpha#2 and beta#8, open in B, are gone from the third snapshot without a merge.
    assert_eq!(
        value(
            &after,
            "prs_closed_unmerged",
            "repository",
            "alpha",
            "2026-10-14"
        ),
        n("1")
    );
    assert_eq!(
        value(
            &after,
            "prs_closed_unmerged",
            "repository",
            "beta",
            "2026-10-14"
        ),
        n("1")
    );
}

#[test]
fn the_rollup_is_kept_apart_from_the_other_records() {
    let case = Case::new("rollup", "apart");
    case.take(B, series_fixture::day_b());
    assert!(
        case.state().join("series").is_dir(),
        "the rollups are kept under state/series/"
    );
}
