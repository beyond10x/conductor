//! The index of the instance's streams (`story:status-current-snapshot`): one row per stream of
//! the instance's tenant, keyed `<type>/<stream id>`, holding when the stream was first stored and
//! the version of its newest record, so that a list of one entity reads that entity's rows and
//! each one's newest event, not every record of the instance. An instance of long standing holds
//! thousands of grants of the host ledger an older binary kept in its store and every guard
//! decision; a list of its snapshots or open dispatches read them all, on every call, before the
//! index.
//!
//! The index holds only what a redaction cannot change: no body, no redacted flag. A redaction
//! erases an event's body in place and adds nothing to the feed, so a copy of the body in the
//! index would outlive it. The newest event is read from the log's own rows on each list, where a
//! record redacted since the index caught up reads as redacted: left out, and reported as a
//! record this build cannot read, as without the index (`story:store-hardening`).
//!
//! Eventlog keeps it as a catch-up projection ([`Streams`]) beside the log, with the position of
//! the feed it is as of in one more row ([`POSITION`]). The handle that completes a snapshot
//! brings it up to date ([`Store::index`]), once a cycle in the loop of the instance's runtime. A
//! list reads the index's rows of its entity and then the feed after the index's position, which
//! holds only what was committed since it was last brought up to date; so a reader writes
//! nothing, and a list is as current as the feed.
//! Where the database holds no index yet (one written only by an older binary), a list reads the
//! whole feed, as before it.

use std::sync::Arc;

use eventlog_core::{
    AtomicEventStore, BoxFuture, EventLogError, MAX_READ_LIMIT, ProjectionSpec, ProjectionStore,
    Projector, RecordedEvent, StreamId, TenantId,
};
use serde_json::{Value, json};
use time::OffsetDateTime;

use super::{Store, StoreError, log_error};

/// The index's one table.
const SPEC: ProjectionSpec = ProjectionSpec {
    name: "conductor_streams",
    indexed: &[],
};

/// The key of the row that holds the position of the feed the index is as of. No stream's key
/// starts with `~`: a stream's type is a qualified name.
const POSITION: &str = "~position";

/// The projector of the index.
pub(super) struct Streams;

impl Projector for Streams {
    fn name(&self) -> &'static str {
        SPEC.name
    }

    fn projections(&self) -> &'static [ProjectionSpec] {
        &[SPEC]
    }

    fn apply<'a>(
        &'a self,
        event: &'a RecordedEvent,
        store: &'a mut dyn ProjectionStore,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        Box::pin(async move {
            let key = format!("{}/{}", event.stream_type, event.stream_id);
            let held = store.get(&SPEC, &event.tenant, &key).await?;
            if held
                .as_ref()
                .and_then(|row| row["version"].as_u64())
                .is_some_and(|version| version >= event.version)
            {
                return Ok(());
            }
            let first = held.map_or_else(
                || {
                    json!({
                        "at": event.recorded_at.unix_timestamp_nanos().to_string(),
                        "seq": event.global_seq,
                    })
                },
                |row| row["first"].clone(),
            );
            let row = json!({
                "first": first,
                "version": event.version,
            });
            store.upsert(&SPEC, &event.tenant, &key, &row).await?;
            store
                .upsert(
                    &SPEC,
                    &event.tenant,
                    POSITION,
                    &json!({"position": event.global_seq}),
                )
                .await
        })
    }
}

/// One stream of the index: its id, when it was first stored, the version of its newest record
/// and that record as the log holds it now.
pub(super) struct Indexed {
    pub stream: String,
    pub first: (OffsetDateTime, u64),
    pub version: u64,
    pub newest: Result<Option<Value>, StoreError>,
}

impl Store {
    /// Creates the index's table where it is not there and brings the index up to date with the
    /// instance's feed. A failure is answered, not kept: the lists read the feed all the same.
    pub(super) fn index(&self) -> Result<(), EventLogError> {
        let projector: Arc<dyn Projector> = Arc::new(Streams);
        self.runtime
            .block_on(self.log.create_projections(Arc::clone(&projector)))?;
        loop {
            let progress = self.runtime.block_on(self.log.run_catch_up(
                Arc::clone(&projector),
                &self.tenant,
                MAX_READ_LIMIT,
            ))?;
            if !progress.more_waiting || progress.applied == 0 {
                return Ok(());
            }
        }
    }

    /// The position of the feed the index is as of, and the streams of `entity` it holds, each
    /// with its newest record read from the log; `None` where the database holds no index of the
    /// instance.
    pub(super) fn indexed(
        &self,
        entity: &'static str,
    ) -> Result<Option<(u64, Vec<Indexed>)>, StoreError> {
        let failed = |source| log_error(entity, None, source);
        let position =
            match self
                .runtime
                .block_on(self.log.projection_get(&SPEC, &self.tenant, POSITION))
            {
                Ok(Some(row)) => row["position"].as_u64().unwrap_or(0),
                Ok(None) | Err(_) => return Ok(None),
            };
        let prefix = format!("{entity}/");
        let mut cursor: Option<String> = None;
        let mut rows = Vec::new();
        loop {
            let page = self
                .runtime
                .block_on(self.log.projection_page(
                    &SPEC,
                    &self.tenant,
                    Some(&prefix),
                    cursor.as_deref(),
                    MAX_READ_LIMIT,
                ))
                .map_err(failed)?;
            rows.extend(
                page.rows
                    .into_iter()
                    .map(|(key, row)| (key[prefix.len()..].to_owned(), row)),
            );
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        let (log, tenant) = (&*self.log, &self.tenant);
        let streams = self.runtime.block_on(async move {
            let mut streams = Vec::with_capacity(rows.len());
            for (stream, row) in rows {
                let (first, version) = placed(&row);
                let newest = newest_of(log, tenant, entity, &stream, version).await;
                streams.push(Indexed {
                    stream,
                    first,
                    version,
                    newest,
                });
            }
            streams
        });
        Ok(Some((position, streams)))
    }
}

/// The event of version `version` of the stream `stream` of `entity` in `tenant` of `log`, the
/// stream's newest the index names, as [`super::body`] reads it from the event rows: redacted
/// since the index was brought up to date, it does not read.
async fn newest_of(
    log: &dyn AtomicEventStore,
    tenant: &TenantId,
    entity: &'static str,
    stream: &str,
    version: u64,
) -> Result<Option<Value>, StoreError> {
    let failed = |source| log_error(entity, Some(stream), source);
    let id = StreamId::new(tenant.clone(), entity, stream).map_err(failed)?;
    let slice = log
        .read_stream(&id, version.saturating_sub(1), 1)
        .await
        .map_err(failed)?;
    match slice.events.into_iter().next() {
        Some(event) if event.version == version => super::body(entity, event),
        _ => Err(failed(EventLogError::NotFound)),
    }
}

/// Where the stream of index row `row` stands: when it was first stored, and its newest version.
fn placed(row: &Value) -> ((OffsetDateTime, u64), u64) {
    let first = (
        row["first"]["at"]
            .as_str()
            .and_then(|at| at.parse::<i128>().ok())
            .and_then(|at| OffsetDateTime::from_unix_timestamp_nanos(at).ok())
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        row["first"]["seq"].as_u64().unwrap_or(0),
    );
    (first, row["version"].as_u64().unwrap_or(0))
}
