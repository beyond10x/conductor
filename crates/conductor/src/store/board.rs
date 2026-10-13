//! `BoardStorage` on [`super::Store`] (`story:status-board`): each `conductor.observation.Board`
//! is one stream in `tree/`, keyed by its id, holding the board's snapshot after each write, kept
//! and read back as every other entity of the store is. A board is not an observation: it is kept
//! whatever `retention.snapshots` drops.

use conductor_model::behaviour::BoardStorage;
use conductor_model::observation::{BoardData, BoardId, BoardSnapshot, BoardState, SnapshotId};
use conductor_model::primitives::Uuid;
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.observation.Board`.
const BOARD: &str = "conductor.observation.Board";

/// The one state of a board, as the specification spells it.
const PUBLISHED: &str = "Published";

impl BoardStorage for Store {
    fn get(&self, identity: &BoardId) -> Option<BoardSnapshot> {
        let key = &identity.0.0;
        let body = self.read(BOARD, key)?;
        match decode(&body) {
            Ok(board) if board.data.board_id == *identity => Some(board),
            Ok(board) => {
                self.fail(undecodable(
                    BOARD,
                    key,
                    format!("holds {:?}", board.data.board_id.0.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(BOARD, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: BoardSnapshot) {
        let body = encode(&snapshot);
        self.write(BOARD, &snapshot.data.board_id.0.0, STORED, body);
    }

    fn delete(&mut self, identity: &BoardId) {
        self.write(BOARD, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<BoardSnapshot> {
        self.rows(BOARD)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(board) => Some(board),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: BOARD,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode(board: &BoardSnapshot) -> Value {
    let BoardState::Published = board.state;
    let data = &board.data;
    json!({
        "state": PUBLISHED,
        "board_id": data.board_id.0.0,
        "snapshot_id": data.snapshot_id.0.0,
        "previous_snapshot_id": data.previous_snapshot_id.as_ref().map(|id| &id.0.0),
        "deviation_rows": data.deviation_rows,
        "path": data.path,
    })
}

fn decode(body: &Value) -> Result<BoardSnapshot, String> {
    let state = text(body, "state")?;
    if state != PUBLISHED {
        return Err(format!("unknown Board state {state:?}"));
    }
    let previous_snapshot_id = match body.get("previous_snapshot_id") {
        None | Some(Value::Null) => None,
        Some(Value::String(id)) => Some(SnapshotId(Uuid(id.clone()))),
        Some(_) => return Err("field \"previous_snapshot_id\" is neither text nor null".to_owned()),
    };
    let deviation_rows = body
        .get("deviation_rows")
        .and_then(Value::as_i64)
        .ok_or_else(|| "no integer field \"deviation_rows\"".to_owned())?;
    Ok(BoardSnapshot {
        state: BoardState::Published,
        data: BoardData {
            board_id: BoardId(Uuid(text(body, "board_id")?)),
            snapshot_id: SnapshotId(Uuid(text(body, "snapshot_id")?)),
            previous_snapshot_id,
            deviation_rows,
            path: text(body, "path")?,
        },
    })
}
