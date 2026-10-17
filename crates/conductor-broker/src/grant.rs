//! A session's grant (`conductor.tool.Grant`) and the check of one call against it.

use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::{Verb, verb_named, verb_word};

/// What one session may call: its verbs, the repositories and the paths they read (each relative
/// to the session's working directory; none, the whole directory), the most bytes one answer
/// carries, and when the grant ends, an RFC 3339 instant (none: with the session).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionGrant {
    pub tools: Vec<Verb>,
    pub repositories: Vec<String>,
    pub paths: Vec<String>,
    pub max_output_bytes: i64,
    pub expires_at: Option<String>,
}

/// Why a call is refused before it runs: the error's outcome and the grant rule it broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refusal {
    pub outcome: crate::CallOutcome,
    pub rule: String,
}

impl SessionGrant {
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "tools": self.tools.iter().map(|verb| verb_word(*verb)).collect::<Vec<_>>(),
            "repositories": self.repositories,
            "paths": self.paths,
            "max_output_bytes": self.max_output_bytes,
            "expires_at": self.expires_at,
        })
    }

    pub(crate) fn from_json(value: &Value) -> Result<Self, String> {
        let texts = |key: &str| -> Result<Vec<String>, String> {
            value[key]
                .as_array()
                .ok_or_else(|| format!("grant has no list {key}"))?
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("grant {key} holds a non-text item"))
                })
                .collect()
        };
        let tools = texts("tools")?
            .iter()
            .map(|word| verb_named(word).ok_or_else(|| format!("grant names no verb {word:?}")))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            tools,
            repositories: texts("repositories")?,
            paths: texts("paths")?,
            max_output_bytes: value["max_output_bytes"]
                .as_i64()
                .filter(|bytes| *bytes >= 1)
                .ok_or("grant has no max_output_bytes of 1 or more")?,
            expires_at: value["expires_at"].as_str().map(str::to_owned),
        })
    }

    /// The refusal of a call of `verb` at `now`, before any path is looked at: an expired grant,
    /// then a verb the grant does not name.
    pub(crate) fn check_verb(&self, verb: Verb, now: OffsetDateTime) -> Result<(), Refusal> {
        if let Some(expires_at) = &self.expires_at {
            let ended = OffsetDateTime::parse(expires_at, &Rfc3339).map_or(true, |end| end <= now);
            if ended {
                return Err(Refusal {
                    outcome: crate::CallOutcome::GrantExpired,
                    rule: format!("ttl: expired at {expires_at}"),
                });
            }
        }
        if !self.tools.contains(&verb) {
            return Err(Refusal {
                outcome: crate::CallOutcome::NotGranted,
                rule: format!("tools: {}", verb_word(verb)),
            });
        }
        Ok(())
    }

    /// The rule a path refusal names: the grant's paths, or the working directory when it names
    /// none.
    fn paths_rule(&self) -> String {
        if self.paths.is_empty() {
            "paths: the working directory".to_owned()
        } else {
            format!("paths: {}", self.paths.join(", "))
        }
    }

    /// `path`, relative to `workdir`, as the absolute path a verb reads, when it lies under the
    /// grant's paths: resolved lexically, then through the file system's links where it exists,
    /// so neither `..` nor a link leads out.
    pub(crate) fn resolve(&self, workdir: &Path, path: &str) -> Result<PathBuf, Refusal> {
        let refused = || Refusal {
            outcome: crate::CallOutcome::NotGranted,
            rule: self.paths_rule(),
        };
        let relative = normal(Path::new(path)).ok_or_else(refused)?;
        let root = workdir
            .canonicalize()
            .unwrap_or_else(|_| workdir.to_owned());
        let allowed: Vec<PathBuf> = if self.paths.is_empty() {
            vec![PathBuf::new()]
        } else {
            self.paths
                .iter()
                .filter_map(|granted| normal(Path::new(granted)))
                .collect()
        };
        if !allowed.iter().any(|granted| relative.starts_with(granted)) {
            return Err(refused());
        }
        let full = root.join(&relative);
        if let Ok(real) = full.canonicalize() {
            let inside = allowed
                .iter()
                .any(|granted| real.starts_with(root.join(granted)));
            if !inside {
                return Err(refused());
            }
            return Ok(real);
        }
        Ok(full)
    }
}

/// `path` with `.` dropped and each `..` taking off the part before it; `None` when it is
/// absolute or climbs above its start.
fn normal(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Normal(name) => out.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}
