//! `RepositoryMarkStorage` on [`super::Store`] (`story:repository-marks`): each
//! `conductor.direction.RepositoryMark` is one stream, keyed by its repository, holding the mark's
//! snapshot after each write, kept and read back as every other entity of the store is.

use conductor_model::behaviour::RepositoryMarkStorage;
use conductor_model::direction::{MarkedRepository, RepositoryMarkData, RepositoryMarkSnapshot};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};
use crate::repository::{marked_by_name, marked_by_named, state_name, state_named};

/// `conductor.direction.RepositoryMark`.
const REPOSITORY_MARK: &str = "conductor.direction.RepositoryMark";

impl RepositoryMarkStorage for Store {
    fn get(&self, identity: &MarkedRepository) -> Option<RepositoryMarkSnapshot> {
        let key = &identity.0;
        let body = self.read(REPOSITORY_MARK, key)?;
        match decode(&body) {
            Ok(mark) if mark.data.repository == *identity => Some(mark),
            Ok(mark) => {
                self.fail(undecodable(
                    REPOSITORY_MARK,
                    key,
                    format!("holds the mark of {:?}", mark.data.repository.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(REPOSITORY_MARK, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: RepositoryMarkSnapshot) {
        let body = encode(&snapshot);
        self.write(REPOSITORY_MARK, &snapshot.data.repository.0, STORED, body);
    }

    fn delete(&mut self, identity: &MarkedRepository) {
        self.write(REPOSITORY_MARK, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<RepositoryMarkSnapshot> {
        self.rows(REPOSITORY_MARK)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(mark) => Some(mark),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: REPOSITORY_MARK,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode(mark: &RepositoryMarkSnapshot) -> Value {
    let data = &mark.data;
    json!({
        "state": state_name(mark.state),
        "repository": data.repository.0,
        "marked_by": marked_by_name(data.marked_by),
        "reason": data.reason,
    })
}

fn decode(body: &Value) -> Result<RepositoryMarkSnapshot, String> {
    let state = text(body, "state")?;
    let marked_by = text(body, "marked_by")?;
    Ok(RepositoryMarkSnapshot {
        state: state_named(&state)
            .ok_or_else(|| format!("unknown RepositoryMark state {state:?}"))?,
        data: RepositoryMarkData {
            repository: MarkedRepository(text(body, "repository")?),
            marked_by: marked_by_named(&marked_by)
                .ok_or_else(|| format!("unknown MarkedBy {marked_by:?}"))?,
            reason: text(body, "reason")?,
        },
    })
}
