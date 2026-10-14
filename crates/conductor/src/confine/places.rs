//! The place resolver (`story:session-envelope-seam`): each of a role's `writable` entries made
//! the absolute paths it stands for, before the request reaches a provider
//! (`conductor.session.PlaceResolved`).
//!
//! | place | paths |
//! |---|---|
//! | `checkout` | the directory the session starts in |
//! | `trees` | `<checkouts.trees>/<repository>`: the managed worktrees of that checkout, the repository being its path under the checkouts root, else the start directory's own name |
//! | `build-cache` | `~/.cargo` and `~/.npm`, the build tools' caches under the home directory |
//! | `scratch` | `$TMPDIR` (`/tmp` when it is not set), and for a controller `~/.cache/<repo>-*`, every entry of `~/.cache` whose name starts `<repo>-`, written as that pattern |
//! | `harness-state` | `~/.claude` and `~/.claude.json` for `claude`, `~/.codex` for `codex` |
//! | `records` | the instance's records directory |
//!
//! Any other entry is a path the config already made absolute, kept as written. A path is
//! resolved whether or not it exists: the provider checks existence. Nothing here reads the
//! disk.

use std::path::{Path, PathBuf};

use conductor_model::config::{Harness, Writable};
use conductor_model::session::PlaceResolved;

/// What a session's places are resolved against.
#[derive(Debug, Clone)]
pub struct Placement {
    /// The home directory.
    pub home: PathBuf,
    /// `$TMPDIR`, or `/tmp`.
    pub tmpdir: PathBuf,
    /// The directory the session starts in.
    pub start: PathBuf,
    /// The repository of that directory under the checkouts root, `<repo>` or `<group>/<repo>`,
    /// when it is one.
    pub repository: Option<String>,
    /// `checkouts.trees`.
    pub trees: PathBuf,
    /// The instance's records directory.
    pub records: PathBuf,
    /// The harness the role runs on.
    pub harness: Harness,
    /// Whether the session is a repository's controller.
    pub controller: bool,
}

impl Placement {
    /// The repository the start directory stands for: its path under the checkouts root, else
    /// the directory's own name.
    fn repository(&self) -> String {
        self.repository.clone().unwrap_or_else(|| {
            self.start
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }
}

/// Each of `writable` resolved against `at`, in order, as the module documentation says.
#[must_use]
pub fn resolve(writable: &[Writable], at: &Placement) -> Vec<PlaceResolved> {
    writable
        .iter()
        .map(|place| PlaceResolved {
            place: place.clone(),
            paths: paths(&place.0, at)
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        })
        .collect()
}

/// The paths of one place word, or the path itself.
fn paths(place: &str, at: &Placement) -> Vec<PathBuf> {
    let home = |rest: &str| at.home.join(rest);
    match place {
        "checkout" => vec![at.start.clone()],
        "trees" => vec![at.trees.join(at.repository())],
        "build-cache" => vec![home(".cargo"), home(".npm")],
        "scratch" => {
            let mut paths = vec![at.tmpdir.clone()];
            if at.controller {
                let repository = at.repository();
                let name = repository.rsplit('/').next().unwrap_or(&repository);
                paths.push(home(".cache").join(format!("{name}-*")));
            }
            paths
        }
        "harness-state" => match at.harness {
            Harness::Claude => vec![home(".claude"), home(".claude.json")],
            Harness::Codex => vec![home(".codex")],
        },
        "records" => vec![at.records.clone()],
        path => vec![Path::new(path).to_owned()],
    }
}
