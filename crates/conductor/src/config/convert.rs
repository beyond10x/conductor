//! `conductor config migrate` from `conductor.config/1` to `/2` (`story:config-globals`, design
//! architecture-next § 9), read once: [`second`] writes `version: conductor.config/2`, drops each
//! key `/2` removed with a note naming its YAML path, writes each instance's `placement` and
//! moves the host's roles to `globals.roles`.

use serde_yaml::{Mapping, Value as Yaml};

use super::{VERSION, VERSION_2};

/// Why each dropped key of an instance is gone.
const DROPPED: [(&str, &str); 2] = [
    (
        "dashboard_port",
        "the daemon serves the one dashboard, at daemon.dashboard",
    ),
    (
        "store",
        "the store is the deployment's one database, at daemon.store",
    ),
];

/// `map` with `key: value` written after `after`, or at its end when it has no `after`.
fn inserted_after(map: &Mapping, after: &str, key: &str, value: Yaml) -> Mapping {
    let mut out = Mapping::new();
    let mut pending = Some(value);
    for (name, item) in map {
        out.insert(name.clone(), item.clone());
        if name.as_str() == Some(after)
            && let Some(value) = pending.take()
        {
            out.insert(Yaml::from(key), value);
        }
    }
    if let Some(value) = pending {
        out.insert(Yaml::from(key), value);
    }
    out
}

/// The `conductor.config/1` file `text` as `conductor.config/2`, and one note per change, each
/// naming its YAML path. `placements` are the instances' placements as the `/1` file reads them
/// (named after its confinement's provider), in the file's order. `host` is the old host file's
/// path and its `roles`, when there is one: they become `globals.roles`, each without its
/// `session_name` (a global role's session is named after the role). `None` when `text` is not
/// a `/1` file whose `instances` is a list of mappings.
#[must_use]
pub fn second(
    text: &str,
    placements: &[String],
    host: Option<(&str, &Yaml)>,
) -> Option<(String, Vec<String>)> {
    let file: Yaml = serde_yaml::from_str(text).ok()?;
    let root = file.as_mapping()?;
    if root.get("version").and_then(Yaml::as_str) != Some(VERSION) {
        return None;
    }
    let mut notes = vec![format!("version: {VERSION_2} (was {VERSION})")];
    let mut instances = Vec::new();
    for (index, instance) in root.get("instances")?.as_sequence()?.iter().enumerate() {
        let mut map = instance.as_mapping()?.clone();
        let at = format!("instances[{index}]");
        for (key, why) in DROPPED {
            if map.remove(key).is_some() {
                notes.push(format!("{at}.{key}: dropped ({why})"));
            }
        }
        let placement = placements.get(index)?.clone();
        if let Some(confinement) = map.remove("confinement") {
            let confinement = confinement.as_mapping().cloned().unwrap_or_default();
            if confinement.contains_key("mode") {
                notes.push(format!(
                    "{at}.confinement.mode: dropped (an instance's box is its placement, which \
                     substrate confines)"
                ));
            }
            if confinement.contains_key("provider") {
                notes.push(format!(
                    "{at}.confinement.provider: dropped (written as {at}.placement: {placement})"
                ));
            }
            if let Some(limits) = confinement.get("limits") {
                map.insert(Yaml::from("limits"), limits.clone());
                notes.push(format!(
                    "{at}.confinement.limits: moved to {at}.limits (the instance's box as a whole)"
                ));
            }
        }
        map = inserted_after(&map, "name", "placement", Yaml::from(placement.clone()));
        notes.push(format!("{at}.placement: {placement}"));
        instances.push(Yaml::Mapping(map));
    }
    let mut out = Mapping::new();
    for (key, value) in root {
        match key.as_str() {
            Some("version") => {
                out.insert(key.clone(), Yaml::from(VERSION_2));
            }
            Some("instances") => {
                if let Some((file, roles)) = host
                    && let Some(roles) = roles.as_sequence()
                {
                    let roles: Vec<Yaml> = roles
                        .iter()
                        .map(|role| {
                            let mut role = role.clone();
                            if let Some(map) = role.as_mapping_mut() {
                                map.remove("session_name");
                            }
                            role
                        })
                        .collect();
                    let mut globals = Mapping::new();
                    globals.insert(Yaml::from("roles"), Yaml::Sequence(roles.clone()));
                    out.insert(Yaml::from("globals"), Yaml::Mapping(globals));
                    notes.push(format!(
                        "host.roles: moved to globals.roles (from {file}, which is no longer read)"
                    ));
                    // A global role reaches an instance's role of the same role (its name or
                    // its old name, `super::same_role`): a built-in role every instance has, or
                    // one an instance writes. Any other is kept, and said to reach none.
                    for (index, role) in roles.iter().enumerate() {
                        let Some(name) = role.get("role").and_then(Yaml::as_str) else {
                            continue;
                        };
                        let written = instances.iter().any(|instance| {
                            instance
                                .get("roles")
                                .and_then(Yaml::as_sequence)
                                .is_some_and(|own| {
                                    own.iter().any(|one| {
                                        one.get("role")
                                            .and_then(Yaml::as_str)
                                            .is_some_and(|own| super::same_role(own, name))
                                    })
                                })
                        });
                        if super::role_name(name).is_none() && !written {
                            notes.push(format!(
                                "globals.roles[{index}]: {name} is kept as the daemon's; it \
                                 applies to no instance, none of which has a role of its name"
                            ));
                        }
                    }
                }
                out.insert(key.clone(), Yaml::Sequence(instances.clone()));
            }
            _ => {
                out.insert(key.clone(), value.clone());
            }
        }
    }
    notes.push(
        "daemon.store: not written; the daemon starts only on a daemon.store the file writes"
            .to_owned(),
    );
    let written = serde_yaml::to_string(&Yaml::Mapping(out)).ok()?;
    Some((written, notes))
}
