//! `conductor config show --effective` (`story:config-globals`, design architecture-next § 9):
//! every key of one instance's effective config with its value and the level it came from
//! (`conductor.config.EffectiveSettings`). A key the instance's role writes is `Role`; one the
//! instance writes is `Instance`; one `globals` writes (its `cadence`, `thresholds`, `limits`, and
//! the keys of its role of the same name) is `Global`; any other is conductor's built-in
//! `Default`. The value is the one every command reads ([`super::document`]); the level is read
//! from the file's own text, which the effective config cannot tell.

use std::fmt::Write as _;

use conductor_model::config::{Instance, Level};
use serde_json::{Value, json};
use serde_yaml::Value as Yaml;

use crate::store::level_name;

/// The keys of an instance that `globals` holds a value for (`conductor.config.Globals`), apart
/// from its roles.
pub const GLOBAL_KEYS: [&str; 3] = ["cadence", "thresholds", "limits"];

/// One step of a YAML path.
#[derive(Debug, Clone)]
enum Step {
    Key(String),
    Index(usize),
}

/// One row of `conductor.config.EffectiveSettings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    /// The role the key holds for; none for an instance-wide key.
    pub role: Option<String>,
    /// The key's YAML path under the instance, or under the role.
    pub path: String,
    /// The effective value as `config show` writes it.
    pub value: String,
    /// The level it came from.
    pub level: Level,
}

/// The rows of `instance`, the `index`-th instance of the file whose YAML is `file` (null for
/// the built-in defaults), each key once: the instance's own keys in the order `config show`
/// writes them, then each role's.
#[must_use]
pub fn settings(file: &Value, index: usize, instance: &Instance) -> Vec<Setting> {
    let shown = super::document(instance);
    let raw = &file["instances"][index];
    let globals = &file["globals"];
    let mut rows = Vec::new();
    let Some(Yaml::Mapping(map)) = shown.get("instances").and_then(|items| items.get(0)) else {
        return rows;
    };
    for (key, value) in map {
        let Some(key) = key.as_str() else { continue };
        if key == "roles" {
            continue;
        }
        let mut leaves = Vec::new();
        flatten(value, vec![Step::Key(key.to_owned())], &mut leaves);
        for (steps, value) in leaves {
            let level = if find(raw, &steps).is_some() {
                Level::Instance
            } else if GLOBAL_KEYS.contains(&key) && find(globals, &steps).is_some() {
                Level::Global
            } else {
                Level::Default
            };
            rows.push(Setting {
                role: None,
                path: written(&steps),
                value,
                level,
            });
        }
    }
    for role in &instance.roles {
        let name = role.role.as_str();
        let own = named_role(&raw["roles"], name);
        let global = named_role(&globals["roles"], name);
        let mut leaves = Vec::new();
        flatten(&super::shown::role(role), Vec::new(), &mut leaves);
        for (steps, value) in leaves {
            if matches!(steps.first(), Some(Step::Key(key)) if key == "role") {
                continue;
            }
            let level = if own.and_then(|own| find(own, &steps)).is_some() {
                Level::Role
            } else if global.and_then(|global| find(global, &steps)).is_some() {
                Level::Global
            } else {
                Level::Default
            };
            rows.push(Setting {
                role: Some(name.to_owned()),
                path: written(&steps),
                value,
                level,
            });
        }
    }
    rows
}

/// The role named `name` in the list `roles` of the file, if it writes one.
fn named_role<'v>(roles: &'v Value, name: &str) -> Option<&'v Value> {
    roles.as_array()?.iter().find(|role| {
        role["role"]
            .as_str()
            .is_some_and(|written| super::same_role(written, name))
    })
}

/// The value at `steps` under `value`, if the file writes one there.
fn find<'v>(value: &'v Value, steps: &[Step]) -> Option<&'v Value> {
    let mut at = value;
    for step in steps {
        at = match step {
            Step::Key(key) => at.as_object()?.get(key)?,
            Step::Index(index) => at.as_array()?.get(*index)?,
        };
    }
    (!at.is_null()).then_some(at)
}

/// The path of `steps`, `a.b[0].c`.
fn written(steps: &[Step]) -> String {
    let mut out = String::new();
    for step in steps {
        match step {
            Step::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            Step::Index(index) => {
                let _ = write!(out, "[{index}]");
            }
        }
    }
    out
}

/// Every scalar of `value`, and every empty list or mapping, with its steps under `at`, as
/// [`super::lines`] writes them.
fn flatten(value: &Yaml, at: Vec<Step>, out: &mut Vec<(Vec<Step>, String)>) {
    match value {
        Yaml::Mapping(map) if !map.is_empty() => {
            for (key, item) in map {
                let mut next = at.clone();
                next.push(Step::Key(key.as_str().unwrap_or_default().to_owned()));
                flatten(item, next, out);
            }
        }
        Yaml::Sequence(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                let mut next = at.clone();
                next.push(Step::Index(index));
                flatten(item, next, out);
            }
        }
        Yaml::Mapping(_) => out.push((at, "{}".to_owned())),
        Yaml::Sequence(_) => out.push((at, "[]".to_owned())),
        Yaml::String(text) => out.push((at, text.replace(['\r', '\n'], " "))),
        other => out.push((
            at,
            serde_yaml::to_string(other)
                .unwrap_or_default()
                .trim_end()
                .to_owned(),
        )),
    }
}

/// The rows as JSON (`conductor.config.EffectiveSettings`).
#[must_use]
pub fn rows_json(instance: &str, rows: &[Setting]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json!({
                    "setting_key": format!(
                        "{instance}/{}/{}",
                        row.role.as_deref().unwrap_or("-"),
                        row.path
                    ),
                    "state": "Effective",
                    "instance": instance,
                    "role": row.role,
                    "path": row.path,
                    "value": row.value,
                    "level": level_name(row.level),
                })
            })
            .collect(),
    )
}

/// The rows as text: `<path>: <value> (<level>)`, one line each, a role's key under
/// `roles.<role>.`.
#[must_use]
pub fn rows_text(rows: &[Setting]) -> String {
    let mut out = String::new();
    for row in rows {
        let path = match &row.role {
            Some(role) => format!("roles.{role}.{}", row.path),
            None => row.path.clone(),
        };
        let _ = writeln!(
            out,
            "{path}: {} ({})",
            row.value,
            level_name(row.level).to_ascii_lowercase()
        );
    }
    out
}
