//! `SecretStorage` (the model's port) on [`super::Store`] (`story:secrets-source`): each
//! `conductor.secret.Secret` a `conductor secret put` records is one stream, keyed by its name,
//! in the daemon's tenant of the deployment's database. A record holds the name, the source and
//! who put it, never a value: the value is in the secrets source only.

use conductor_model::behaviour::SecretStorage;
use conductor_model::config::InstanceName;
use conductor_model::primitives::Timestamp;
use conductor_model::secret::{
    Namespace, SecretData, SecretName, SecretSnapshot, SecretState, Source,
};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.secret.Secret`.
const SECRET: &str = "conductor.secret.Secret";

impl SecretStorage for Store {
    fn get(&self, identity: &SecretName) -> Option<SecretSnapshot> {
        let body = self.read(SECRET, &identity.0)?;
        match decode(&body) {
            Ok(found) if found.data.name == *identity => Some(found),
            Ok(found) => {
                self.fail(undecodable(
                    SECRET,
                    &identity.0,
                    format!("holds the secret {:?}", found.data.name.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(SECRET, &identity.0, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: SecretSnapshot) {
        let body = encode(&snapshot);
        self.write(SECRET, &snapshot.data.name.0, STORED, body);
    }

    fn delete(&mut self, identity: &SecretName) {
        self.write(SECRET, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<SecretSnapshot> {
        self.rows(SECRET)
            .into_iter()
            .filter_map(|(stream, body)| match decode(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: SECRET,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

/// The source's name as the specification spells it.
#[must_use]
pub const fn source_name(source: Source) -> &'static str {
    match source {
        Source::Keychain => "Keychain",
        Source::Remote => "Remote",
        Source::Files => "Files",
    }
}

fn encode(found: &SecretSnapshot) -> Value {
    let SecretState::Stored = found.state;
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.created_by),
        "state": "Stored",
        "name": data.name.0,
        "source": source_name(data.source),
        "namespace": data.namespace.as_ref().map(|namespace| &namespace.0),
        "instances": data.instances.iter().map(|name| &name.0).collect::<Vec<_>>(),
        "created_at": data.created_at.0,
        "last_resolved_at": data.last_resolved_at.as_ref().map(|at| &at.0),
    })
}

fn decode(body: &Value) -> Result<SecretSnapshot, String> {
    let state = text(body, "state")?;
    if state != "Stored" {
        return Err(format!("unknown Secret state {state:?}"));
    }
    let source = match text(body, "source")?.as_str() {
        "Keychain" => Source::Keychain,
        "Remote" => Source::Remote,
        "Files" => Source::Files,
        other => return Err(format!("unknown Source {other:?}")),
    };
    let optional = |field: &str| body.get(field).and_then(Value::as_str).map(str::to_owned);
    Ok(SecretSnapshot {
        state: SecretState::Stored,
        data: SecretData {
            name: SecretName(text(body, "name")?),
            source,
            namespace: optional("namespace").map(Namespace),
            instances: body
                .get("instances")
                .and_then(Value::as_array)
                .map(|names| {
                    names
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|name| InstanceName(name.to_owned()))
                        .collect()
                })
                .unwrap_or_default(),
            created_by: crate::actor::stored(body)?,
            created_at: Timestamp(text(body, "created_at")?),
            last_resolved_at: optional("last_resolved_at").map(Timestamp),
        },
    })
}
