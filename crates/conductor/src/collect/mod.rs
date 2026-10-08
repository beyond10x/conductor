//! The collectors a snapshot runs, and the seam each one fills.
//!
//! A [`Collector`] is one source a sense run reads: a name, and the function that records what the
//! source shows through a [`Recorder`]. The driver, [`crate::snapshot::take`], takes the set it
//! runs: `conductor snapshot start-snapshot` hands it [`registered`], and a test or
//! `story:watch-command` a set of its own.
//!
//! `story:snapshot-driver` owns the registration and the order. Each collector story fills only its
//! own module's `collect`, which answers [`NotImplemented`](crate::NotImplemented) until then, so
//! a snapshot taken through the registered set fails naming the first collector not yet written.
//!
//! Each live `collect` reads its sources from the instance the process runs
//! ([`config::active`]): where the repositories come from ([`Origins`]), where their checkouts and
//! managed trees are, and, when a config file names the instance, its records and cache. Without
//! a file that instance is the built-in one, whose values are today's.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use conductor_model::config::{Checkouts, GitHubSource, Instance, LocalSource, Source};

use crate::config;
use crate::snapshot::Recorder;

pub mod blockers;
pub mod github;
pub mod repositories;
pub mod sessions;
pub mod specifications;

/// What a collector runs: it records into the snapshot `Recorder` holds, which replaces whatever
/// snapshot an input names, and answers an error when its source did not answer or an observation
/// it recorded was refused. A collector that panics has not answered: the driver fails the
/// snapshot naming it.
pub type Collect = dyn Fn(&mut Recorder<'_>) -> Result<()>;

/// One source a snapshot reads.
pub struct Collector {
    name: &'static str,
    collect: Box<Collect>,
}

impl Collector {
    /// The collector `name`, which records through `collect`.
    pub fn new(
        name: &'static str,
        collect: impl Fn(&mut Recorder<'_>) -> Result<()> + 'static,
    ) -> Self {
        Self {
            name,
            collect: Box::new(collect),
        }
    }

    /// The name a failed snapshot's reason gives this collector.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Records what the source shows through `recorder`.
    ///
    /// # Errors
    ///
    /// The source did not answer, or an observation it recorded was refused.
    pub fn collect(&self, recorder: &mut Recorder<'_>) -> Result<()> {
        (self.collect)(recorder)
    }
}

impl fmt::Debug for Collector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Collector")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// The collectors `conductor snapshot start-snapshot` runs, in this order: the repositories (git
/// and each owner's list), GitHub's pull requests and `main` runs, the planning stores'
/// blockers, the specifications (whose conformance status reads the blockers' workspace listing),
/// and the live sessions.
#[must_use]
pub fn registered() -> Vec<Collector> {
    vec![
        Collector::new("repositories", repositories::collect),
        Collector::new("github", github::collect),
        Collector::new("blockers", blockers::collect),
        Collector::new("specifications", specifications::collect),
        Collector::new("sessions", sessions::collect),
    ]
}

/// The programs the [`registered`] collectors start on every run for `instance`
/// (`story:install-prerequisites`): `gh` when it has a `github` source, `git` when its checkouts
/// root or a `local` source holds a repository (a directory holding `.git`, at depth 1 or in a
/// group at depth 2), and `aep` and `claude` always. `ess` is not among them: the
/// `specifications` collector starts it only for a repository whose export holds a
/// specification, which is known only once the export is made.
#[must_use]
pub fn programs(instance: &Instance) -> Vec<&'static str> {
    let origins = Origins::of(instance);
    let mut programs = Vec::new();
    if !origins.owners.is_empty() {
        programs.push("gh");
    }
    let roots = std::iter::once(PathBuf::from(&instance.checkouts.root)).chain(origins.local);
    let holds_repository = |root: &Path| {
        std::fs::read_dir(root).is_ok_and(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .any(|dir| holds_git(&dir) || is_group(&dir))
        })
    };
    if roots.into_iter().any(|root| holds_repository(&root)) {
        programs.push("git");
    }
    programs.extend(["aep", "claude"]);
    programs
}

/// Where an instance's repositories come from, its `sources` (`conductor.config.Source`): the
/// GitHub owners whose repositories are listed, and the local directories whose checkouts are
/// read as they are. The live collectors read them from [`config::active`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Origins {
    /// Each `github:` source's owner, in the instance's order, each once.
    pub owners: Vec<String>,
    /// Each `local:` source's directory, in the instance's order, each once.
    pub local: Vec<PathBuf>,
}

impl Origins {
    /// The origins of `instance`.
    #[must_use]
    pub fn of(instance: &Instance) -> Self {
        let mut origins = Self::default();
        for source in &instance.sources {
            match source {
                Source::GitHub(GitHubSource { owner }) => {
                    if !origins.owners.contains(owner) {
                        origins.owners.push(owner.clone());
                    }
                }
                // No collector reads a GitLab forge: conductor reads it through its session's
                // integration and records the rows with the snapshot record commands.
                Source::GitLab(_) => {}
                Source::Local(LocalSource { path, .. }) => {
                    let path = PathBuf::from(path);
                    if !origins.local.contains(&path) {
                        origins.local.push(path);
                    }
                }
            }
        }
        origins
    }

    /// The one owner `owner`.
    #[must_use]
    pub fn owner(owner: &str) -> Self {
        Self {
            owners: vec![owner.to_owned()],
            local: Vec::new(),
        }
    }

    /// The built-in instance's origins ([`config::built_in`]), which do not depend on the home or
    /// working directory it is given.
    #[must_use]
    pub fn built_in() -> Self {
        Self::of(&config::built_in(Path::new("/"), Path::new("/")))
    }
}

/// Which of an instance's two roots a path lies under: the checkouts root, `<root>/<repo>/…`, or
/// the managed trees, `<trees>/<repo>/<tree>/…`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Under {
    /// `checkouts.root`.
    Checkouts,
    /// `checkouts.trees`.
    Trees,
}

/// The repository the steps `steps` under `root` (the checkouts root) or under `trees` (the
/// managed trees) lie in (`story:grouped-instance`), named by its path under the root.
///
/// A repository is the directory that holds `.git` at depth 1 or 2 under the root, `<root>/<repo>`
/// or `<root>/<group>/<repo>`; a group is a directory under the root that holds no `.git` but
/// holds a repository. The checkouts root decides, in this order, under either root:
/// 1. `<root>/<first>` holds `.git`: the repository is `<first>`, and a directory nested in it is
///    its own;
/// 2. `<root>/<first>` is a group: the repository is `<first>/<second>`, whether or not it holds
///    `.git` yet, and the group directory itself is none.
///
/// Under the managed trees of a group that has no directory under the root, the trees decide:
/// `<trees>/<first>/<second>/<tree>` holding `.git` is the repository `<first>/<second>`. No
/// reading comes from a directory a controller of the group can write: the group directory is
/// outside each of its repositories' checkouts and worktrees, and the deeper reading of a tree
/// only narrows. When nothing decides, the first step is the repository, as it was before groups:
/// the layout with every repository at depth 1 reads as it did, whether or not its checkouts
/// exist.
///
/// `None` for no step, or for a group directory itself.
#[must_use]
pub fn repository_of(root: &Path, trees: &Path, under: Under, steps: &[&str]) -> Option<String> {
    let first = *steps.first()?;
    let second = steps.get(1).copied();
    let checkout = root.join(first);
    if holds_git(&checkout) {
        return Some(first.to_owned());
    }
    if is_group(&checkout) {
        return second.map(|second| format!("{first}/{second}"));
    }
    if let (Under::Trees, Some(second), Some(tree)) = (under, second, steps.get(2))
        && holds_git(&trees.join(first).join(second).join(tree))
    {
        return Some(format!("{first}/{second}"));
    }
    Some(first.to_owned())
}

/// Whether `dir` holds `.git`, a directory or the file a managed tree has.
fn holds_git(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Whether `dir` is a group: it holds no `.git`, and a directory directly under it does.
fn is_group(dir: &Path) -> bool {
    !holds_git(dir)
        && std::fs::read_dir(dir).is_ok_and(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|entry| holds_git(&entry.path()))
        })
}

/// Whether the steps `steps` under a root are excluded by `exclude`, a source's `exclude`
/// entries under that root: an entry's steps begin them.
#[must_use]
pub fn is_excluded<S: AsRef<str>>(steps: &[S], exclude: &[String]) -> bool {
    exclude.iter().any(|entry| {
        let entry: Vec<&str> = entry.split('/').collect();
        !entry.is_empty()
            && steps.len() >= entry.len()
            && steps
                .iter()
                .zip(&entry)
                .all(|(step, part)| step.as_ref() == *part)
    })
}

/// The `exclude` entries of `instance`'s sources whose root is `root`: a `local` source's path,
/// or, for a `gitlab` source, the checkouts root. A `github` source has none.
#[must_use]
pub fn excluded_under(instance: &Instance, root: &Path) -> Vec<String> {
    let checkouts = Path::new(&instance.checkouts.root);
    let mut entries: Vec<String> = Vec::new();
    for source in &instance.sources {
        let (source_root, exclude) = match source {
            Source::GitHub(_) => continue,
            Source::Local(local) => (Path::new(&local.path), &local.exclude),
            Source::GitLab(gitlab) => (checkouts, &gitlab.exclude),
        };
        if source_root == root {
            for entry in exclude {
                if !entries.contains(entry) {
                    entries.push(entry.clone());
                }
            }
        }
    }
    entries
}

/// The built-in instance's checkouts and managed trees under the home directory `home`
/// ([`config::built_in`]): where a collector given only a home directory places repositories and
/// sessions, as it did before there was a config file.
#[must_use]
pub fn built_in_checkouts(home: &Path) -> Checkouts {
    config::built_in(home, home).checkouts
}

/// How long one command a collector runs may take before [`run`] stops it (`story:collector-bounds`).
/// A source that does not answer in this time has not answered.
pub const BOUND: Duration = Duration::from_secs(120);

/// Runs `command` with no standard input and answers its exit status and output. A command still
/// running after `bound` is killed, and the answer is an error naming the program and the bound,
/// so no collector waits on a source for ever (`gh` on a network that does not answer). Every
/// external command a collector runs goes through here.
///
/// # Errors
///
/// The program does not start, or it is still running after `bound`.
pub fn run(command: &mut Command, bound: Duration) -> Result<Output> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("start `{program}`"))?;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("wait for `{program}`"))?
        {
            break status;
        }
        if started.elapsed() >= bound {
            let _ = child.kill();
            let _ = child.wait();
            bail!("`{program}` gave no answer within {} s", bound.as_secs());
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ok(Output {
        status,
        stdout: collected(stdout),
        stderr: collected(stderr),
    })
}

/// Reads `pipe` to its end on a thread of its own, so a full pipe never stalls the child.
fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// What a [`drain`] thread read; nothing when there was no pipe or the thread panicked.
fn collected(reader: Option<JoinHandle<Vec<u8>>>) -> Vec<u8> {
    reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::run;

    #[test]
    fn a_command_that_answers_in_time_gives_its_status_and_output() {
        let output = run(
            Command::new("sh").args(["-c", "echo out; echo err >&2; exit 3"]),
            Duration::from_secs(10),
        )
        .expect("sh answers");
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(output.stdout, b"out\n");
        assert_eq!(output.stderr, b"err\n");
    }

    #[test]
    fn a_command_past_its_bound_is_stopped_and_named() {
        let started = Instant::now();
        let refused = run(Command::new("sleep").arg("30"), Duration::from_secs(1))
            .expect_err("sleep 30 runs past a bound of 1 s");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "run waited {:?}",
            started.elapsed()
        );
        assert_eq!(refused.to_string(), "`sleep` gave no answer within 1 s");
    }

    #[test]
    fn a_program_that_does_not_exist_is_named() {
        let refused = run(
            &mut Command::new("conductor-no-such-program"),
            Duration::from_secs(1),
        )
        .expect_err("no such program");
        assert_eq!(refused.to_string(), "start `conductor-no-such-program`");
    }
}
