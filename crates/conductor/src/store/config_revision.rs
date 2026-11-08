//! `ConfigRevisionStorage` on [`super::Store`] (`story:config-globals`): each
//! `conductor.config.ConfigRevision` an accepted config change records is one stream, keyed by its
//! id, written once when the change is committed, in the daemon's tenant of the deployment's
//! database.

use conductor_model::behaviour::ConfigRevisionStorage;
use conductor_model::config::{
    ConfigRevisionData, ConfigRevisionSnapshot, ConfigRevisionState, InstanceName, Level,
    RevisionId, RoleName,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.config.ConfigRevision`.
const CONFIG_REVISION: &str = "conductor.config.ConfigRevision";

impl ConfigRevisionStorage for Store {
    fn get(&self, identity: &RevisionId) -> Option<ConfigRevisionSnapshot> {
        let key = &identity.0.0;
        let body = self.read(CONFIG_REVISION, key)?;
        match decode(&body) {
            Ok(found) if found.data.revision_id == *identity => Some(found),
            Ok(found) => {
                self.fail(undecodable(
                    CONFIG_REVISION,
                    key,
                    format!("holds the revision {:?}", found.data.revision_id.0.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(CONFIG_REVISION, key, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: ConfigRevisionSnapshot) {
        let body = encode(&snapshot);
        self.write(
            CONFIG_REVISION,
            &snapshot.data.revision_id.0.0,
            STORED,
            body,
        );
    }

    fn delete(&mut self, identity: &RevisionId) {
        self.write(CONFIG_REVISION, &identity.0.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<ConfigRevisionSnapshot> {
        self.rows(CONFIG_REVISION)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: CONFIG_REVISION,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

/// The level's name as the specification spells it.
#[must_use]
pub const fn level_name(level: Level) -> &'static str {
    match level {
        Level::Default => "Default",
        Level::Global => "Global",
        Level::Instance => "Instance",
        Level::Role => "Role",
    }
}

/// The level the specification spells `name`.
#[must_use]
pub fn level_named(name: &str) -> Option<Level> {
    [Level::Default, Level::Global, Level::Instance, Level::Role]
        .into_iter()
        .find(|level| level_name(*level) == name)
}

fn encode(found: &ConfigRevisionSnapshot) -> Value {
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": "Committed",
        "revision_id": data.revision_id.0.0,
        "level": level_name(data.level),
        "instance": data.instance.as_ref().map(|name| &name.0),
        "role": data.role.as_ref().map(|name| &name.0),
        "path": data.path,
        "value": data.value,
        "reason": data.reason,
        "commit": data.commit,
        "changed_at": data.changed_at.0,
    })
}

fn optional(body: &Value, field: &str) -> Option<String> {
    body.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn decode(body: &Value) -> Result<ConfigRevisionSnapshot, String> {
    let state = text(body, "state")?;
    if state != "Committed" {
        return Err(format!("unknown ConfigRevision state {state:?}"));
    }
    let level = text(body, "level")?;
    Ok(ConfigRevisionSnapshot {
        state: ConfigRevisionState::Committed,
        data: ConfigRevisionData {
            revision_id: RevisionId(Uuid(text(body, "revision_id")?)),
            actor: crate::actor::stored(body)?,
            level: level_named(&level).ok_or_else(|| format!("unknown Level {level:?}"))?,
            instance: optional(body, "instance").map(InstanceName),
            role: optional(body, "role").map(RoleName),
            path: text(body, "path")?,
            value: optional(body, "value"),
            reason: text(body, "reason")?,
            commit: text(body, "commit")?,
            changed_at: Timestamp(text(body, "changed_at")?),
        },
    })
}
