//! `conductor trust` (`story:portable-trust`): marks every git checkout directly under the
//! instance's `checkouts.root`, and its records directory by its physical path, as trusted Claude
//! Code workspaces, `projects[<path>].hasTrustDialogAccepted = true` in `~/.claude.json`.
//!
//! Claude Code keys `projects` by a session's working directory, which is a physical path with no
//! trailing `/`; so each path is canonicalised. Every other key and project, and every other
//! field of a trusted project, is kept in its order ([`Json`]). The file is written through a
//! temporary file in its own directory, with its mode, and renamed over it; when the file's
//! content changed between the read and the write (Claude Code writes it while it runs), nothing
//! is written. No GNU tool runs: this replaces the shell of `task trust`, which needed
//! `stat -c %Y` and `chmod --reference`.

use std::fmt;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::cli::TrustArgs;
use crate::config::{self, Environment};

/// The file under the home directory that holds Claude Code's projects.
pub const CLAUDE_JSON: &str = ".claude.json";

/// The key of `projects` that marks a workspace trusted.
const TRUSTED: &str = "hasTrustDialogAccepted";

/// A JSON value that keeps the order of every object's keys, so that a rewrite changes only what
/// it sets.
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The value of `key`, an object made when it is absent or not an object.
    fn object_at(&mut self, key: &str) -> &mut Self {
        let Self::Object(entries) = self else {
            unreachable!("object_at is called on an object");
        };
        let index = match entries.iter().position(|(name, _)| name == key) {
            Some(index) => index,
            None => {
                entries.push((key.to_owned(), Self::Object(Vec::new())));
                entries.len() - 1
            }
        };
        let value = &mut entries[index].1;
        if !matches!(value, Self::Object(_)) {
            *value = Self::Object(Vec::new());
        }
        value
    }

    /// Sets `key` to `value` in place, or adds it at the end.
    fn set(&mut self, key: &str, value: Self) {
        let Self::Object(entries) = self else {
            unreachable!("set is called on an object");
        };
        match entries.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key.to_owned(), value)),
        }
    }
}

impl Serialize for Json {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => value.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Self::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Json, E> {
        Ok(Json::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Json, E> {
        serde_json::Number::from_f64(value)
            .map(Json::Number)
            .ok_or_else(|| E::custom("a number JSON cannot hold"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Json, E> {
        Ok(Json::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Json, E> {
        Ok(Json::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element()? {
            values.push(value);
        }
        Ok(Json::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut entries: Vec<(String, Json)> = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Json>()? {
            // A repeated key: the last one wins, as it does for every JSON reader Claude Code
            // uses, in the place of the first.
            match entries.iter_mut().find(|(name, _)| *name == key) {
                Some(entry) => entry.1 = value,
                None => entries.push((key, value)),
            }
        }
        Ok(Json::Object(entries))
    }
}

/// One edit of `~/.claude.json`: what was read, and the document with the trust marks set.
#[derive(Debug)]
pub struct Edit {
    file: PathBuf,
    read: Vec<u8>,
    document: Json,
}

impl Edit {
    /// Reads `file`, which must hold a JSON object.
    ///
    /// # Errors
    ///
    /// The file cannot be read, or holds no JSON object.
    pub fn read(file: &Path) -> Result<Self> {
        let read = fs::read(file).with_context(|| format!("read {}", file.display()))?;
        let document: Json = serde_json::from_slice(&read)
            .with_context(|| format!("{} is not JSON", file.display()))?;
        if !matches!(document, Json::Object(_)) {
            bail!("{} holds no JSON object", file.display());
        }
        Ok(Self {
            file: file.to_owned(),
            read,
            document,
        })
    }

    /// Marks `path` trusted: `projects[path].hasTrustDialogAccepted = true`, every other field of
    /// that project kept.
    pub fn trust(&mut self, path: &str) {
        self.document
            .object_at("projects")
            .object_at(path)
            .set(TRUSTED, Json::Bool(true));
    }

    /// Writes the document over the file: to a temporary file in its directory, with its mode,
    /// then renamed over it. Refuses, writing nothing and leaving no temporary file, when the
    /// file's content is no longer what [`Edit::read`] read.
    ///
    /// # Errors
    ///
    /// The file changed since it was read, or a write, mode change or rename failed.
    pub fn write(self) -> Result<()> {
        let file = &self.file;
        let mut text = serde_json::to_vec_pretty(&self.document)
            .with_context(|| format!("render {}", file.display()))?;
        if self.read.ends_with(b"\n") {
            text.push(b'\n');
        }
        let permissions = fs::metadata(file)
            .with_context(|| format!("read the mode of {}", file.display()))?
            .permissions();
        let dir = file
            .parent()
            .ok_or_else(|| anyhow!("{} has no directory", file.display()))?;
        let name = file
            .file_name()
            .ok_or_else(|| anyhow!("{} names no file", file.display()))?
            .to_string_lossy();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let temporary = dir.join(format!(".{name}.trust.{}.{stamp}", std::process::id()));
        let written = (|| -> Result<()> {
            let mut out = File::options()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .with_context(|| format!("create {}", temporary.display()))?;
            fs::set_permissions(&temporary, permissions)
                .with_context(|| format!("set the mode of {}", temporary.display()))?;
            out.write_all(&text)
                .and_then(|()| out.sync_all())
                .with_context(|| format!("write {}", temporary.display()))?;
            let now = fs::read(file).with_context(|| format!("read {}", file.display()))?;
            if now != self.read {
                bail!(
                    "{} changed while conductor trust ran (its content is not what was read); \
                     nothing is written: run conductor trust again",
                    file.display()
                );
            }
            fs::rename(&temporary, file)
                .with_context(|| format!("rename {} to {}", temporary.display(), file.display()))
        })();
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        written
    }
}

/// The paths `conductor trust` marks, as `projects` keys: each git checkout directly under
/// `root` (a directory holding `.git`), in name order, then `records`, each by its physical
/// path.
///
/// # Errors
///
/// `records` holds a line break or is not a directory, `root` is not a directory, or a path
/// cannot be resolved or is not UTF-8.
pub fn paths(root: &Path, records: &str) -> Result<Vec<String>> {
    if records.contains(['\n', '\r']) {
        bail!(
            "the records directory of the instance holds a line break; trust refuses it and \
             trusts nothing"
        );
    }
    let records_dir = Path::new(records);
    if !records_dir.is_dir() {
        bail!(
            "the records directory {records} of the instance is not a directory; trust trusts \
             nothing"
        );
    }
    if !root.is_dir() {
        bail!(
            "the checkouts root {} of the instance is not a directory; trust trusts nothing",
            root.display()
        );
    }
    let mut checkouts: Vec<PathBuf> = fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|dir| dir.is_dir() && dir.join(".git").exists())
        .collect();
    checkouts.sort();
    checkouts.push(records_dir.to_owned());
    checkouts
        .iter()
        .map(|dir| {
            let physical =
                fs::canonicalize(dir).with_context(|| format!("resolve {}", dir.display()))?;
            physical
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("{} is not UTF-8", physical.display()))
        })
        .collect()
}

/// `conductor trust`: marks the instance's checkouts and records directory trusted in
/// `~/.claude.json` and prints one line per path it trusted. The instance is selected as for
/// `config show`, and a config file must name it.
///
/// # Errors
///
/// The config does not load or names no such instance, there is no config file, a refusal of
/// [`paths`], `~/.claude.json` cannot be read or holds no JSON object, or [`Edit::write`] fails.
pub fn trust(config_flag: Option<&Path>, args: &TrustArgs) -> Result<ExitCode> {
    const WHAT: &str = "trust";
    let environment = Environment::from_process().context(WHAT)?;
    let loaded = config::load(config_flag, &environment).context(WHAT)?;
    if !loaded.read {
        bail!(
            "{WHAT}: no config file at {}; trust marks the directories an instance of one names",
            loaded.path.display()
        );
    }
    let instance = config::select(&loaded, args.instance.as_deref(), &environment).context(WHAT)?;
    let paths = paths(Path::new(&instance.checkouts.root), &instance.records).context(WHAT)?;
    let file = environment.home().context(WHAT)?.join(CLAUDE_JSON);
    let mut edit = Edit::read(&file).context(WHAT)?;
    for path in &paths {
        edit.trust(path);
    }
    edit.write().context(WHAT)?;
    for path in &paths {
        println!("trusted: {path}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rewrite_keeps_key_order_and_numbers() {
        let text = r#"{"b": 1, "a": {"y": [1, 2.5, -3, null, true], "x": "s"}, "n": 12345678901234567890}"#;
        let document: Json = serde_json::from_str(text).expect("JSON");
        let back = serde_json::to_string(&document).expect("render");
        assert_eq!(
            back,
            r#"{"b":1,"a":{"y":[1,2.5,-3,null,true],"x":"s"},"n":12345678901234567890}"#
        );
    }

    #[test]
    fn trust_sets_the_mark_in_place_and_keeps_the_projects_other_fields() {
        let text = r#"{"projects": {"/p": {"hasTrustDialogAccepted": false, "k": 1}}, "z": 0}"#;
        let mut document: Json = serde_json::from_str(text).expect("JSON");
        document
            .object_at("projects")
            .object_at("/p")
            .set(TRUSTED, Json::Bool(true));
        document
            .object_at("projects")
            .object_at("/q")
            .set(TRUSTED, Json::Bool(true));
        assert_eq!(
            serde_json::to_string(&document).expect("render"),
            r#"{"projects":{"/p":{"hasTrustDialogAccepted":true,"k":1},"/q":{"hasTrustDialogAccepted":true}},"z":0}"#
        );
    }
}
