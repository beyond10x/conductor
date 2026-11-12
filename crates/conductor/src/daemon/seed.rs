//! `conductor daemon serve --daemon-config <file>` (design: instance pods,
//! `conductor.instance.ServeDaemon`): the daemon's config file lives on its volume, where every
//! instance arrives over HTTP, and its `daemon` section comes from a second file that holds only
//! `version` and `daemon` (`conductor.config.DaemonConfig`), such as a ConfigMap mounted
//! read-only. At every start, before the daemon reads its config file, [`merge`] writes that file
//! anew from the two.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_yaml::{Mapping, Value as Yaml};

use crate::config;

/// Writes the config file `file` from the daemon-config file `daemon_config` (`~` read as
/// `home`): its `version` and `daemon` from `daemon_config`, every other key of `file` (`globals`,
/// `default`, `instances`) kept as it is, in its order, and `instances: []` where `file` writes
/// none, such as where it does not exist yet; its directory is created. The file is written by a
/// rename beside it, as `PUT /v1/config` writes it, under the same lock, and only when its bytes
/// change; like that write it keeps no comment. Answers whether it wrote.
///
/// # Errors
///
/// `daemon_config` cannot be read or does not validate (named by its path and each problem's YAML
/// path, on one line); `file` exists and is no YAML mapping, which is not written over; or `file`
/// or its directory cannot be written.
pub fn merge(daemon_config: &Path, file: &Path, home: &Path) -> Result<bool> {
    let text = fs::read_to_string(daemon_config).with_context(|| {
        format!(
            "--daemon-config: read the daemon-config file {}",
            daemon_config.display()
        )
    })?;
    if let Err(problems) = config::parse_daemon_config(&text, home) {
        let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
        bail!(
            "--daemon-config {}: {}; the daemon does not start",
            daemon_config.display(),
            said.join("; ")
        );
    }
    let mapped: Yaml = serde_yaml::from_str(&text)
        .with_context(|| format!("--daemon-config: read {}", daemon_config.display()))?;
    if let Some(dir) = file.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(dir)
            .with_context(|| format!("create the config file's directory {}", dir.display()))?;
    }
    let _held = config::put::lock(file)?;
    let old = match fs::read_to_string(file) {
        Ok(old) => Some(old),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| format!("read the config file {}", file.display()));
        }
    };
    let kept = match old.as_deref().map(serde_yaml::from_str::<Option<Yaml>>) {
        None | Some(Ok(None)) => Mapping::new(),
        Some(Ok(Some(Yaml::Mapping(kept)))) => kept,
        Some(_) => bail!(
            "the config file {} is no YAML mapping; the daemon does not write its \
             --daemon-config over it",
            file.display()
        ),
    };
    let mut next = Mapping::new();
    for key in ["version", "daemon"] {
        if let Some(value) = mapped.get(key) {
            next.insert(Yaml::from(key), value.clone());
        }
    }
    for (key, value) in kept {
        if !matches!(key.as_str(), Some("version" | "daemon")) {
            next.insert(key, value);
        }
    }
    if !next.contains_key("instances") {
        next.insert(Yaml::from("instances"), Yaml::Sequence(Vec::new()));
    }
    let new = serde_yaml::to_string(&Yaml::Mapping(next)).context("write the config as YAML")?;
    if old.as_deref() == Some(new.as_str()) {
        return Ok(false);
    }
    let partial = file.with_extension("yaml.next");
    fs::write(&partial, &new).with_context(|| format!("write {}", partial.display()))?;
    fs::rename(&partial, file).with_context(|| format!("write {}", file.display()))?;
    Ok(true)
}
