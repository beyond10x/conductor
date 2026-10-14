//! `DailySeriesStorage` on [`super::Store`] (`story:dashboard-history`): each
//! `conductor.observation.DailySeries` is one stream, keyed by its id, in the store's one tenant of
//! `state/series/`, a tree of its own: a rollup is not an observation, so retention never drops
//! it, and the tree every other command opens does not grow with it. It is opened at its first
//! use, by the rollup a completed snapshot runs and by the dashboard's `/api/series`.

use conductor_model::behaviour::DailySeriesStorage;
use conductor_model::observation::{
    DailySeriesData, DailySeriesId, DailySeriesSnapshot, DailySeriesState, SnapshotId,
};
use conductor_model::primitives::{Decimal, Uuid};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.observation.DailySeries`.
pub const DAILY_SERIES: &str = "conductor.observation.DailySeries";

/// The one state of a rollup, as the specification spells it.
const RECORDED: &str = "Recorded";

impl DailySeriesStorage for Store {
    fn get(&self, identity: &DailySeriesId) -> Option<DailySeriesSnapshot> {
        let key = &identity.0;
        let body = self.read(DAILY_SERIES, key)?;
        match decode(&body) {
            Ok(rollup) if rollup.data.daily_series_id == *identity => Some(rollup),
            Ok(rollup) => {
                self.fail(undecodable(
                    DAILY_SERIES,
                    key,
                    format!("holds {:?}", rollup.data.daily_series_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(DAILY_SERIES, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: DailySeriesSnapshot) {
        let body = encode(&snapshot);
        self.write(DAILY_SERIES, &snapshot.data.daily_series_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &DailySeriesId) {
        self.write(DAILY_SERIES, &identity.0, DELETED, json!({}));
    }
}

impl Store {
    /// Every rollup the store holds, in the order they were first stored. The specification
    /// declares no view of them, so the generated port lists none: `/api/series` and the rollup
    /// read them here. A rollup whose record does not decode is left out and kept for
    /// [`Store::check`].
    #[must_use]
    pub fn daily_series(&self) -> Vec<DailySeriesSnapshot> {
        self.rows(DAILY_SERIES)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(rollup) => Some(rollup),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: DAILY_SERIES,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode(rollup: &DailySeriesSnapshot) -> Value {
    let DailySeriesState::Recorded = rollup.state;
    let data = &rollup.data;
    json!({
        "state": RECORDED,
        "daily_series_id": data.daily_series_id.0,
        "series": data.series,
        "day": data.day,
        "by": data.by,
        "key": data.key,
        "value": data.value.0,
        "items": data.items,
        "snapshot_id": data.snapshot_id.0.0,
    })
}

fn decode(body: &Value) -> Result<DailySeriesSnapshot, String> {
    let state = text(body, "state")?;
    if state != RECORDED {
        return Err(format!("unknown DailySeries state {state:?}"));
    }
    let items = body
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| "no list field \"items\"".to_owned())?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "an item of \"items\" is not text".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DailySeriesSnapshot {
        state: DailySeriesState::Recorded,
        data: DailySeriesData {
            daily_series_id: DailySeriesId(text(body, "daily_series_id")?),
            series: text(body, "series")?,
            day: text(body, "day")?,
            by: text(body, "by")?,
            key: text(body, "key")?,
            value: Decimal(text(body, "value")?),
            items,
            snapshot_id: SnapshotId(Uuid(text(body, "snapshot_id")?)),
        },
    })
}
