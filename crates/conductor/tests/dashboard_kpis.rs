//! `story:dashboard-kpi-panels`, the served half: `GET /api/kpis` answers one row per KPI, K1–K12
//! then T1–T4. A measured KPI carries its value over the last seven days of the fixture
//! (`series_fixture/mod.rs`), computed as its definition says, its target, direction and the
//! series its trend reads; a KPI without a source answers `measured: false` with the story that
//! adds it, and no value.

mod active;
mod series_fixture;

use serde_json::{Value, json};
use series_fixture::{Case, today};

use conductor_cli::dashboard::series;

/// The rows `/api/kpis` answers over the fixture's store at its today.
fn kpis(case: &Case) -> Vec<Value> {
    let store = case.store();
    series::kpis(Some(&store), today())
        .as_array()
        .expect("an array")
        .clone()
}

fn kpi<'a>(rows: &'a [Value], id: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no {id} in {rows:#?}"))
}

/// Case `name`'s directory, the process's instance fixed first: the dashboard reads it.
fn case(name: &str) -> Case {
    active::isolate();
    Case::new("dashboard-kpis", name)
}

#[test]
fn the_rows_come_in_the_contract_order() {
    let case = case("order");
    case.build();
    let ids: Vec<String> = kpis(&case)
        .iter()
        .map(|row| row["id"].as_str().expect("an id").to_owned())
        .collect();
    let expected: Vec<String> = (1..=12)
        .map(|n| format!("K{n}"))
        .chain((1..=4).map(|n| format!("T{n}")))
        .collect();
    assert_eq!(ids, expected);
}

#[test]
fn the_measured_kpis_match_their_definitions() {
    let case = case("measured");
    case.build();
    let rows = kpis(&case);
    // Over 2026-10-07..13: four operator touches (DEC-1, DEC-3, R-1 class O, R-2 class H), five
    // merged PRs (alpha#3, alpha#4, beta#5, beta#7, alpha#6), two escalations of which one was
    // withdrawn, dispatch leads of 2 h and 6 h, merged-to-released ages of 17 h, 2 h and 2 h, two
    // failed runs, three guard denials, one patch and one feature release.
    for (id, value, series) in [
        ("K1", json!(0.8), "operator_touches_per_pr"),
        ("K2", json!(50), "unnecessary_escalations_pct"),
        ("K4", json!(6), "dispatch_lead_hours"),
        ("K5", json!(17), "merged_to_released_hours"),
        ("K6", json!(0.4), "ci_failed_per_pr"),
        ("K10", json!(0.429), "guard_denials"),
        ("K12", json!(1), "patch_per_feature_release"),
    ] {
        let row = kpi(&rows, id);
        assert_eq!(row["measured"], true, "{id}: {row}");
        assert_eq!(row["value"], value, "{id}: {row}");
        assert_eq!(row["series"], series, "{id}: {row}");
        assert_eq!(row["window_days"], 7, "{id}: {row}");
        assert_eq!(row["direction"], "lower", "{id}: {row}");
        for field in ["title", "definition", "unit", "source"] {
            assert!(row[field].is_string(), "{id}: {field} in {row}");
        }
        assert!(row.get("story").is_none(), "{id}: {row}");
    }
    let k1 = kpi(&rows, "K1");
    assert_eq!(k1["title"], "Operator touches per merged PR");
    assert_eq!(k1["unit"], "ratio");
    assert_eq!(k1["target"], json!(0.1));
    assert_eq!(kpi(&rows, "K2")["target"], json!(10));
    assert_eq!(kpi(&rows, "K4")["target"], json!(3));
    assert_eq!(kpi(&rows, "K5")["target"], json!(4));
    assert_eq!(kpi(&rows, "K6")["target"], json!(0.1));
    assert_eq!(kpi(&rows, "K10")["target"], json!(8));
    assert_eq!(kpi(&rows, "K12")["target"], json!(0.2));
}

#[test]
fn the_kpis_without_a_source_carry_their_story_and_no_value() {
    let case = case("not-measured");
    case.build();
    let rows = kpis(&case);
    for (id, story, unit, target) in [
        ("K3", "story:kpi-transcripts", "count", json!(2)),
        ("K7", "story:kpi-transcripts", "hours", json!(34)),
        ("K8", "story:kpi-transcripts", "hours", json!(67)),
        ("K9", "story:kpi-transcripts", "count", json!(0)),
        ("K11", "story:kpi-transcripts", "tokens", Value::Null),
        ("T1", "story:kpi-per-task", "tokens", Value::Null),
        ("T2", "story:kpi-per-task", "hours", Value::Null),
        ("T3", "story:kpi-per-task", "USD", Value::Null),
        ("T4", "story:kpi-per-task", "count", Value::Null),
    ] {
        let row = kpi(&rows, id);
        assert_eq!(row["measured"], false, "{id}: {row}");
        assert_eq!(row["story"], story, "{id}: {row}");
        assert_eq!(row["unit"], unit, "{id}: {row}");
        assert_eq!(row["target"], target, "{id}: {row}");
        for absent in ["value", "series", "window_days"] {
            assert!(row.get(absent).is_none(), "{id}: no {absent} in {row}");
        }
        assert!(
            row["title"].is_string() && row["definition"].is_string(),
            "{row}"
        );
    }
    assert_eq!(kpi(&rows, "T4")["title"], "LLM calls per task");
    assert_eq!(kpi(&rows, "K11")["title"], "Tokens per merged PR");
}

#[test]
fn a_measured_kpi_with_nothing_in_its_window_has_a_null_value() {
    let case = case("empty");
    let rows = series::kpis(None, today());
    let rows = rows.as_array().expect("an array");
    let k1 = kpi(rows, "K1");
    assert_eq!(k1["measured"], true);
    assert_eq!(
        k1["value"],
        Value::Null,
        "no merged PR: no ratio, never a zero"
    );
    drop(case);
}
