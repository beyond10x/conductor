//! `MetricStorage` and `MetricReadingStorage` on [`super::Store`] (`story:kpi-records`): each
//! `conductor.improvement.Metric` and each `conductor.improvement.MetricReading` is one stream,
//! keyed by its identity, kept and read back as every other entity of the store is. A metric's
//! stream is its history: defined, then each target set on it. A decimal is kept as the text it
//! was given in, as the generated `Decimal` carries it.

use conductor_model::behaviour::{MetricReadingStorage, MetricStorage};
use conductor_model::improvement::{
    MetricData, MetricId, MetricReadingData, MetricReadingId, MetricReadingSnapshot, MetricSnapshot,
};
use conductor_model::primitives::{Decimal, Timestamp};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};
use crate::metric::{
    direction_name, direction_named, metric_state_name, metric_state_named, reading_state_name,
    reading_state_named,
};

/// `conductor.improvement.Metric`.
const METRIC: &str = "conductor.improvement.Metric";

/// `conductor.improvement.MetricReading`.
const METRIC_READING: &str = "conductor.improvement.MetricReading";

impl MetricStorage for Store {
    fn get(&self, identity: &MetricId) -> Option<MetricSnapshot> {
        let key = &identity.0;
        let body = self.read(METRIC, key)?;
        match decode_metric(&body) {
            Ok(metric) if metric.data.metric_id == *identity => Some(metric),
            Ok(metric) => {
                self.fail(undecodable(
                    METRIC,
                    key,
                    format!("holds metric {:?}", metric.data.metric_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(METRIC, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: MetricSnapshot) {
        let body = encode_metric(&snapshot);
        self.write(METRIC, &snapshot.data.metric_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &MetricId) {
        self.write(METRIC, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<MetricSnapshot> {
        self.rows(METRIC)
            .into_iter()
            .filter_map(|(stream, body)| match decode_metric(&body) {
                Ok(metric) => Some(metric),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: METRIC,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

impl MetricReadingStorage for Store {
    fn get(&self, identity: &MetricReadingId) -> Option<MetricReadingSnapshot> {
        let key = &identity.0;
        let body = self.read(METRIC_READING, key)?;
        match decode_reading(&body) {
            Ok(reading) if reading.data.reading_id == *identity => Some(reading),
            Ok(reading) => {
                self.fail(undecodable(
                    METRIC_READING,
                    key,
                    format!("holds reading {:?}", reading.data.reading_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(METRIC_READING, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: MetricReadingSnapshot) {
        let body = encode_reading(&snapshot);
        self.write(METRIC_READING, &snapshot.data.reading_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &MetricReadingId) {
        self.write(METRIC_READING, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<MetricReadingSnapshot> {
        self.rows(METRIC_READING)
            .into_iter()
            .filter_map(|(stream, body)| match decode_reading(&body) {
                Ok(reading) => Some(reading),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: METRIC_READING,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode_metric(metric: &MetricSnapshot) -> Value {
    let data = &metric.data;
    json!({
        "state": metric_state_name(metric.state),
        "metric_id": data.metric_id.0,
        "name": data.name,
        "unit": data.unit,
        "instrument": data.instrument,
        "definition": data.definition,
        "baseline": data.baseline.as_ref().map(|value| &value.0),
        "baseline_period": data.baseline_period,
        "target": data.target.as_ref().map(|value| &value.0),
        "direction": data.direction.map(direction_name),
        "target_owner": data.target_owner,
        "target_set_at": data.target_set_at.as_ref().map(|at| &at.0),
    })
}

fn decode_metric(body: &Value) -> Result<MetricSnapshot, String> {
    let state = text(body, "state")?;
    Ok(MetricSnapshot {
        state: metric_state_named(&state)
            .ok_or_else(|| format!("unknown Metric state {state:?}"))?,
        data: MetricData {
            metric_id: MetricId(text(body, "metric_id")?),
            name: text(body, "name")?,
            unit: text(body, "unit")?,
            instrument: text(body, "instrument")?,
            definition: text(body, "definition")?,
            baseline: optional_text(body, "baseline")?.map(Decimal),
            baseline_period: optional_text(body, "baseline_period")?,
            target: optional_text(body, "target")?.map(Decimal),
            direction: match optional_text(body, "direction")? {
                Some(direction) => Some(
                    direction_named(&direction)
                        .ok_or_else(|| format!("unknown TargetDirection {direction:?}"))?,
                ),
                None => None,
            },
            target_owner: optional_text(body, "target_owner")?,
            target_set_at: optional_text(body, "target_set_at")?.map(Timestamp),
        },
    })
}

fn encode_reading(reading: &MetricReadingSnapshot) -> Value {
    let data = &reading.data;
    json!({
        "state": reading_state_name(reading.state),
        "reading_id": data.reading_id.0,
        "metric_id": data.metric_id.0,
        "day": data.day,
        "scope": data.scope,
        "value": data.value.0,
        "sources": data.sources,
        "met": data.met,
        "recorded_at": data.recorded_at.0,
    })
}

fn decode_reading(body: &Value) -> Result<MetricReadingSnapshot, String> {
    let state = text(body, "state")?;
    let sources = match body.get("sources") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "a source that is not text".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("no list field \"sources\"".to_owned()),
    };
    let met = match body.get("met") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(met)) => Some(*met),
        Some(_) => return Err("no boolean or null field \"met\"".to_owned()),
    };
    Ok(MetricReadingSnapshot {
        state: reading_state_named(&state)
            .ok_or_else(|| format!("unknown MetricReading state {state:?}"))?,
        data: MetricReadingData {
            reading_id: MetricReadingId(text(body, "reading_id")?),
            metric_id: MetricId(text(body, "metric_id")?),
            day: text(body, "day")?,
            scope: text(body, "scope")?,
            value: Decimal(text(body, "value")?),
            sources,
            met,
            recorded_at: Timestamp(text(body, "recorded_at")?),
        },
    })
}

/// A field that is text or null; an absent field reads as null.
fn optional_text(body: &Value, field: &str) -> Result<Option<String>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("no text or null field {field:?}")),
    }
}
