//! The config file of one instance pod (design: instance pods, `conductor.config.Kubernetes`,
//! decision D8): where `daemon.kubernetes` is set, the daemon runs each instance's runtime as a
//! pod of its own and hands it a config file it renders from its own ([`render`]). The pod
//! supervisor puts that file into the instance's Secret under the key [`CONFIG_FILE`], mounts the
//! Secret read-only on [`SECRET_MOUNT`] and the instance's volume claim on [`DATA_MOUNT`], and
//! runs `conductor --config <DATA_MOUNT>/<CONFIG_FILE> instance serve` with
//! `CONDUCTOR_CONFIG_SOURCE` naming `<SECRET_MOUNT>/<CONFIG_FILE>`, so the runtime copies the file
//! onto its volume at every start.
//!
//! The rendered file is a complete `conductor.config/2` file that holds the one instance and what
//! its runtime reads:
//! - `daemon.store`: the daemon's own when it is a `postgres://` URL, the database every instance
//!   shares; else the instance's own sqlite file [`STORE_FILE`] under the data mount, since no
//!   volume is shared between the daemon and a pod;
//! - `daemon.secrets`: a `files` source over [`SECRET_MOUNT`], where the Secret holds one file per
//!   secret the instance names (its `model_token` among them, whose `model_token_kind` the file
//!   writes where the instance names one);
//! - `daemon.unattended`: true, since a pod has no terminal and no person to answer a prompt;
//! - `daemon.operators`: the daemon's, by name and grant;
//! - the instance as `config show` writes it, every global key it takes written into it, and each
//!   of its paths (`checkouts.root`, `checkouts.trees`, `records`, `state`, `cache`) under the data
//!   mount: one it writes lies there already (`config validate` refuses another, [`within`]), and
//!   one it leaves out is filled in there ([`PLACES`]); its `standing_rules`
//!   (story:instance-rules-text) where it holds them, which the runtime reads from the mounted
//!   file at each session's start, so the daemon writes a change of them alone into the Secret
//!   without replacing the pod; its `forge_access`
//!   (story:forge-access-anonymous) where it names one, which the runtime reads as the daemon's
//!   file does: it is the runtime that clones with no credential and refuses the forge writes,
//!   and its file validates the member by the pod it runs in rather than by a
//!   `daemon.kubernetes`.
//!
//! It holds no `daemon.kubernetes` and no `daemon.control_token`: an instance pod creates no pods
//! and admits no control plane. Nor does it hold the instance's `forge_token`, which only the pod
//! supervisor reads, into a variable of the pod (story:instance-forge-token), or its
//! `git_identity`, which it reads into the pod's git identity variables
//! (story:instance-git-identity): the runtime reads neither key, and its file, which writes no
//! `daemon.kubernetes`, would not validate with them.

use std::path::{Component, Path};

use conductor_model::config::{Config, Instance};
use serde_yaml::{Mapping, Value as Yaml};

use super::{
    Confinement, Legacy, VERSION_2, daemon_document, dashboard_port, default_confinement,
    shown_instance, text_of,
};

/// Where the pod mounts the instance's volume claim (`conductor-instance-<name>-data`): a
/// directory under the release image's home `/var/lib/conductor`, so what the image installs in
/// the home (the harness in `~/.local/bin`) stays visible.
pub const DATA_MOUNT: &str = "/var/lib/conductor/data";

/// Where the pod mounts the instance's Secret (`conductor-instance-<name>`), read-only: the
/// directory of the rendered file's `files` secrets source, one file per key.
pub const SECRET_MOUNT: &str = "/run/secrets/conductor";

/// The name of the rendered file: its key in the instance's Secret, so the file
/// `<SECRET_MOUNT>/conductor.yaml` the runtime copies, and the file `<DATA_MOUNT>/conductor.yaml`
/// its `--config` names.
pub const CONFIG_FILE: &str = "conductor.yaml";

/// The instance's own sqlite store under the data mount, where the daemon's store is no
/// `postgres://` URL.
pub const STORE_FILE: &str = "conductor.sqlite";

/// Each path of a pod-run instance, by its YAML path in the instance, with the directory under the
/// data mount it is filled in as where the instance leaves it out.
pub const PLACES: [(&str, &str); 5] = [
    ("checkouts.root", "checkouts"),
    ("checkouts.trees", "trees"),
    ("records", "records"),
    ("state", "state"),
    ("cache", "cache"),
];

/// Whether `path` lies under the data mount: absolute, without a `..` step, and [`DATA_MOUNT`]
/// or a path under it.
#[must_use]
pub fn within(path: &str) -> bool {
    let path = Path::new(path);
    path.is_absolute()
        && !path
            .components()
            .any(|component| component == Component::ParentDir)
        && path.starts_with(DATA_MOUNT)
}

/// The config file of `instance`'s pod, as the module documentation says, its paths under
/// [`DATA_MOUNT`] and its secrets source on [`SECRET_MOUNT`]. `config` is the daemon's file, which
/// holds `instance`.
#[must_use]
pub fn render(config: &Config, instance: &Instance) -> String {
    render_at(
        config,
        instance,
        Path::new(DATA_MOUNT),
        Path::new(SECRET_MOUNT),
    )
}

/// [`render`] for a pod whose volume claim is mounted on `data` and whose Secret on `secrets`: a
/// path under [`DATA_MOUNT`] is written under `data` instead, at the same place.
#[must_use]
pub fn render_at(config: &Config, instance: &Instance, data: &Path, secrets: &Path) -> String {
    let place = |path: &str, name: &str| {
        let under = Path::new(path)
            .strip_prefix(DATA_MOUNT)
            .ok()
            .filter(|_| within(path));
        text_of(&under.map_or_else(|| data.join(name), |rest| data.join(rest)))
    };
    let mut own = instance.clone();
    own.checkouts.root = place(&instance.checkouts.root, PLACES[0].1);
    own.checkouts.trees = place(&instance.checkouts.trees, PLACES[1].1);
    own.records = place(&instance.records, PLACES[2].1);
    own.state = place(&instance.state, PLACES[3].1);
    own.cache = place(&instance.cache, PLACES[4].1);

    let store = if config.daemon.store.0.starts_with("postgres://") {
        config.daemon.store.0.clone()
    } else {
        format!("sqlite:{}", text_of(&data.join(STORE_FILE)))
    };
    let mut daemon = Mapping::new();
    daemon.insert(Yaml::from("store"), Yaml::from(store));
    let mut source = Mapping::new();
    source.insert(Yaml::from("source"), Yaml::from("files"));
    source.insert(Yaml::from("dir"), Yaml::from(text_of(secrets)));
    daemon.insert(Yaml::from("secrets"), Yaml::Mapping(source));
    daemon.insert(Yaml::from("unattended"), Yaml::Bool(true));
    if let Some(operators) = daemon_document(&config.daemon).get("operators") {
        daemon.insert(Yaml::from("operators"), operators.clone());
    }

    // Written as a `conductor.config/2` file writes it; the conductor that serves it and the
    // remote daemon of its placement are derived again from the one file.
    let legacy = Legacy {
        dashboard_port: dashboard_port(0),
        confinement: Confinement {
            limits: own.limits.clone(),
            ..default_confinement()
        },
        shown: false,
    };
    let mut shown = shown_instance(&own, &legacy);
    if let Yaml::Mapping(map) = &mut shown {
        // First and in place, so the keys stand where they stood before the key existed.
        map.shift_remove("forge_token");
        map.shift_remove("git_identity");
        // The runtime reads the model token's kind, the standing rules and the forge access
        // (story:forge-access-anonymous); a mapping without any of them renders as before, and
        // rules that are empty or only white space are none.
        for key in ["model_token_kind", "standing_rules", "forge_access"] {
            let absent = map.get(key).is_some_and(|value| {
                value.is_null()
                    || (key == "standing_rules"
                        && value
                            .as_str()
                            .is_some_and(|text| super::standing_rules_given(text).is_none()))
            });
            if absent {
                map.shift_remove(key);
            }
        }
        map.remove("conductor");
        map.remove("remote");
    }

    let mut file = Mapping::new();
    file.insert(Yaml::from("version"), Yaml::from(VERSION_2));
    file.insert(Yaml::from("daemon"), Yaml::Mapping(daemon));
    file.insert(Yaml::from("instances"), Yaml::Sequence(vec![shown]));
    serde_yaml::to_string(&Yaml::Mapping(file)).unwrap_or_default()
}
