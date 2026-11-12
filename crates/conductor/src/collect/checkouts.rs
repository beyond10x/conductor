//! An instance pod's checkouts (`story:pod-source-checkouts`): a runtime alone in a pod
//! ([`in_pod`]) clones, by itself, each repository a `github` or `gitlab` source names under
//! `include` that has no checkout, and keeps each such checkout fetched. Nothing here runs on a
//! workstation or in a single container, and a source without `include` gets no checkout.
//!
//! - **When.** At the runtime's start, before it seeds its instance ([`at_start`]), so the
//!   workspace file, the trust and the activation the seeding derives see the checkouts; and at
//!   each pass of the `repositories` collector, for each included repository that still has
//!   none, each checkout of one fetched with `git fetch --quiet origin` under the collector's
//!   bound. The start clones [`repositories::WIDTH`] repositories at once and begins no clone
//!   once [`CLONE_BOUND`] ([`START_BOUND_VARIABLE`] seconds, where set) has passed since it
//!   began: it says how many it left, and the passes clone them. A clone at a pass brings the
//!   workspace file and the trust up to date ([`refresh`]): it trusts only the checkouts an
//!   `include` entry covers. The activation is the checkouts root's, which a new checkout under
//!   it needs nothing of.
//! - **Names no checkout.** A path step `.git`, in any case, or one that starts with [`PARTIAL`]
//!   ([`names_no_checkout`]): an `include` or `exclude` entry with one is refused, a listed
//!   GitLab project with one is left out of every collector as one member failure, [`bring`]
//!   refuses one, and a directory of that name under the root or a group is read as no checkout.
//!   A `.git` that holds a `.git` of its own is the work tree of a clone at a `.git` path, not a
//!   git directory: the directory that holds it is no checkout ([`is_checkout`]).
//! - **Where.** The path the collector reads the checkout at: `<checkouts root>/<name>` for a
//!   `github` repository, `<checkouts root>/<project path under the group>` for a `gitlab` one. A
//!   path more than two steps deep, or under a directory that is a checkout itself, is not one the
//!   collector reads: it is not cloned, and is reported.
//! - **How.** `git clone` over HTTPS from `https://github.com/<owner>/<name>.git` or
//!   `https://<host>/<group>/<path>.git`, as the collector runs git
//!   ([`collect::git_environment`], [`collect::run`]) under [`CLONE_BOUND`], into a directory
//!   under the checkouts root the collector takes for no checkout ([`PARTIAL`], the clone two
//!   steps below it), then renamed to its path. git gets the token only through the credential
//!   helper the pod's environment carries: no URL, argument or message holds one. A directory an
//!   earlier, killed clone left is removed before the next try. A failed clone is not tried again
//!   in the same pass.
//! - **Never touched.** A path that holds `.git` is left as it is: no reset, no clone, no change
//!   of `origin`. A path that holds no `.git` is left alone and reported.
//! - **Reported.** Each repository that could not be cloned or fetched is one member failure of
//!   the pass ([`Brought::Failed`]), its line with the reason and git's last error line, every
//!   credential-shaped text taken out ([`redacted`]). A pod that carries no token of a source's
//!   forge tries no clone: each of its repositories is a failure that says so.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow};
use conductor_model::config::{Instance, Source};

use crate::collect::{self, blockers, gitlab, repositories};
use crate::{config, trust};

/// The start of the name of the directory under the checkouts root a clone is made in,
/// `<root>/.conductor-clone-<path>/partial/<name>`: three steps under the root, deeper than the
/// collector reads a checkout, so a clone in progress is taken for none.
pub const PARTIAL: &str = ".conductor-clone-";

/// How long one clone may run before it is stopped: the collector's [`collect::BOUND`] is too
/// short for a large repository.
pub const CLONE_BOUND: Duration = Duration::from_secs(20 * 60);

/// The variable that, set to a whole number of seconds, is the start's bound on beginning clones
/// ([`at_start`]) in place of [`CLONE_BOUND`].
pub const START_BOUND_VARIABLE: &str = "CONDUCTOR_START_CLONE_BOUND";

/// Whether the path step `step` names no checkout: `.git` in any case, which a checkout's own git
/// directory is called, or a name that starts with [`PARTIAL`], in any case, where a clone is
/// made.
#[must_use]
pub fn names_no_checkout(step: &str) -> bool {
    step.eq_ignore_ascii_case(".git")
        || step
            .get(..PARTIAL.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(PARTIAL))
}

/// The first step of the `/`-separated path `path` that names no checkout
/// ([`names_no_checkout`]).
#[must_use]
pub fn step_naming_no_checkout(path: &str) -> Option<&str> {
    path.split('/').find(|step| names_no_checkout(step))
}

/// Whether `dir` is a checkout: it holds `.git`, and that `.git` holds no `.git` of its own, as
/// the work tree of a clone made at a `.git` path would.
#[must_use]
pub fn is_checkout(dir: &Path) -> bool {
    let git = dir.join(".git");
    git.exists() && git.join(".git").symlink_metadata().is_err()
}

/// What stands for a credential in a message.
const HIDDEN: &str = "[hidden]";

/// The beginnings of the tokens GitHub and GitLab issue: a word that starts with one is taken out
/// of a message whole.
const TOKEN_SHAPES: [&str; 10] = [
    "github_pat_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "glpat-",
    "gldt-",
    "glptt-",
    "glrt-",
];

/// Whether this process runs in an instance pod: [`crate::instance::LISTEN_VARIABLE`] is set, as
/// the pod's manifest sets it and every process the runtime starts inherits it.
#[must_use]
pub fn in_pod() -> bool {
    std::env::var(crate::instance::LISTEN_VARIABLE).is_ok_and(|listen| !listen.trim().is_empty())
}

/// One repository a source includes, as a pod brings its checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
    /// The repository's name, as its row and a member failure name it.
    pub name: String,
    /// The path under the checkouts root a clone goes to.
    pub path: String,
    /// Where it is cloned from.
    pub url: String,
    /// The kind of its source, `github` or `gitlab`.
    pub forge: &'static str,
    /// The forge's host.
    pub host: String,
    /// Its checkout, when it has one: fetched, never cloned.
    pub checkout: Option<PathBuf>,
    /// Whether the forge lists it archived: then it is not cloned.
    pub archived: bool,
}

impl Wanted {
    /// The repository `name` of the GitHub owner `owner`, its checkout `<root>/<name>`.
    #[must_use]
    pub fn github(owner: &str, name: &str, root: &Path, archived: bool) -> Self {
        let dir = root.join(name);
        Self {
            name: name.to_owned(),
            path: name.to_owned(),
            url: format!("https://{}/{owner}/{name}.git", crate::doctor::GITHUB_HOST),
            forge: "github",
            host: crate::doctor::GITHUB_HOST.to_owned(),
            checkout: is_checkout(&dir).then_some(dir),
            archived,
        }
    }

    /// The project `path` (under the group `group` on `host`) of a `gitlab` source, the
    /// repository `name`, and its checkout when one names it.
    #[must_use]
    pub fn gitlab(
        host: &str,
        group: &str,
        path: &str,
        name: &str,
        checkout: Option<PathBuf>,
        archived: bool,
    ) -> Self {
        let group = group.trim_matches('/');
        let full = if group.is_empty() {
            path.to_owned()
        } else {
            format!("{group}/{path}")
        };
        Self {
            name: name.to_owned(),
            path: path.to_owned(),
            url: format!("https://{host}/{full}.git"),
            forge: "gitlab",
            host: host.to_owned(),
            checkout,
            archived,
        }
    }
}

/// What [`bring`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Brought {
    /// Nothing: a checkout is there and no fetch was asked, the path holds `.git`, or the
    /// repository is archived.
    Left,
    /// The checkout was fetched.
    Fetched,
    /// It was cloned to this checkout.
    Cloned(PathBuf),
    /// It could not be cloned or fetched: the line a member failure records.
    Failed(String),
    /// The collector's budget was spent: nothing to record.
    Stopped,
}

/// Brings the checkout of `wanted` under the checkouts root `root`: its checkout fetched under
/// `fetch`, when that is given and it has one; else cloned, as the module documentation says.
#[must_use]
pub fn bring(root: &Path, wanted: &Wanted, fetch: Option<Duration>) -> Brought {
    let url = &wanted.url;
    if let Some(step) =
        step_naming_no_checkout(&wanted.path).or_else(|| step_naming_no_checkout(&wanted.name))
    {
        return Brought::Failed(format!(
            "{url} is not cloned or fetched: its path {} holds the step {step:?}, which names no \
             checkout",
            wanted.path
        ));
    }
    if let Some(dir) = &wanted.checkout {
        return fetch.map_or(Brought::Left, |bound| fetched(dir, bound));
    }
    let target = root.join(&wanted.path);
    if wanted.archived {
        return Brought::Left;
    }
    if target.join(".git").exists() {
        if is_checkout(&target) {
            return Brought::Left;
        }
        return Brought::Failed(format!(
            "{url} is not cloned: {} holds a .git of its own, so it is no git directory; it is \
             left as it is",
            target.join(".git").display()
        ));
    }
    let steps: Vec<&str> = wanted.path.split('/').collect();
    if steps.len() > 2 {
        return Brought::Failed(format!(
            "{url} is not cloned: its path {} is {} steps deep, and a checkout is read at most two \
             steps under the checkouts root",
            wanted.path,
            steps.len()
        ));
    }
    if steps.len() == 2 && root.join(steps[0]).join(".git").exists() {
        return Brought::Failed(format!(
            "{url} is not cloned: {} holds a checkout, so none at {} would be read",
            root.join(steps[0]).display(),
            target.display()
        ));
    }
    if target.symlink_metadata().is_ok() {
        return Brought::Failed(format!(
            "{url} is not cloned: {} is there and holds no git checkout; it is left as it is",
            target.display()
        ));
    }
    if !carries_token(wanted.forge) {
        let variable = config::FORGE_TOKEN_VARIABLES
            .iter()
            .find(|(forge, _)| *forge == wanted.forge)
            .map_or("", |(_, variable)| *variable);
        return Brought::Failed(format!(
            "{url} is not cloned: the pod carries no token of {} ({variable}), so no clone is \
             tried",
            wanted.host
        ));
    }
    cloned(root, wanted, &target)
}

/// Whether this process carries a token of the forge `forge` (`github`, `gitlab`): a variable of
/// [`config::FORGE_CLI_TOKEN_VARIABLES`] that holds one.
fn carries_token(forge: &str) -> bool {
    config::FORGE_CLI_TOKEN_VARIABLES
        .iter()
        .filter(|(kind, _)| *kind == forge)
        .flat_map(|(_, variables)| variables.iter())
        .any(|variable| std::env::var(variable).is_ok_and(|value| !value.trim().is_empty()))
}

/// `wanted` cloned into its directory under [`PARTIAL`], then renamed to `target`.
fn cloned(root: &Path, wanted: &Wanted, target: &Path) -> Brought {
    let url = &wanted.url;
    let slot = root.join(format!("{PARTIAL}{}", gitlab::encoded(&wanted.path)));
    let failed = |why: String| {
        let _ = fs::remove_dir_all(&slot);
        Brought::Failed(format!("{url} is not cloned: {why}"))
    };
    if slot.symlink_metadata().is_ok()
        && let Err(error) = fs::remove_dir_all(&slot)
    {
        return Brought::Failed(format!(
            "{url} is not cloned: {} of an earlier clone cannot be removed: {error}",
            slot.display()
        ));
    }
    let last = wanted.path.rsplit('/').next().unwrap_or(&wanted.path);
    let partial = slot.join("partial");
    let into = partial.join(last);
    if let Err(error) = fs::create_dir_all(&partial) {
        return failed(format!("create {}: {error}", partial.display()));
    }
    let mut command = crate::instance::command("git");
    command
        .args(["clone", "--quiet", "--", url])
        .arg(&into)
        .current_dir(root);
    collect::git_environment(&mut command, None);
    let output = match collect::run(&mut command, CLONE_BOUND) {
        Ok(output) => output,
        Err(error) if collect::is_over_budget(&error) => {
            let _ = fs::remove_dir_all(&slot);
            return Brought::Stopped;
        }
        Err(error) => return failed(format!("`git clone`: {}", one_line(&error))),
    };
    if !output.status.success() {
        return failed(format!("`git clone` {}", ended(&output)));
    }
    if let Some(parent) = target.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        return failed(format!("create {}: {error}", parent.display()));
    }
    if let Err(error) = fs::rename(&into, target) {
        return failed(format!("move the clone to {}: {error}", target.display()));
    }
    let _ = fs::remove_dir_all(&slot);
    Brought::Cloned(target.to_owned())
}

/// The checkout `dir` fetched, `git fetch --quiet origin` run as the collector runs it, once more
/// when it fails.
fn fetched(dir: &Path, bound: Duration) -> Brought {
    let attempt = || {
        let mut command = crate::instance::command("git");
        command
            .args(["fetch", "--quiet", "origin"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(dir);
        collect::git_environment(&mut command, Some(dir));
        collect::run(&mut command, bound)
    };
    let mut answered = attempt();
    if answered
        .as_ref()
        .is_ok_and(|output| !output.status.success())
    {
        answered = attempt();
    }
    let what = format!("`git fetch --quiet origin` in {}", dir.display());
    match answered {
        Ok(output) if output.status.success() => Brought::Fetched,
        Ok(output) => Brought::Failed(format!("{what} {}", ended(&output))),
        Err(error) if collect::is_over_budget(&error) => Brought::Stopped,
        Err(error) => Brought::Failed(format!("{what}: {}", one_line(&error))),
    }
}

/// How `output` ended, and git's last error line, every credential taken out.
fn ended(output: &Output) -> String {
    let status = output.status.code().map_or_else(
        || "was stopped by a signal".to_owned(),
        |code| format!("exited {code}"),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    match stderr.lines().map(str::trim).rfind(|line| !line.is_empty()) {
        Some(line) => format!("{status}: {}", redacted(line)),
        None => status,
    }
}

/// `error` on one line, every credential taken out.
fn one_line(error: &anyhow::Error) -> String {
    redacted(
        &format!("{error:#}")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// `text` with every credential-shaped part taken out: the value of each forge token variable
/// this process carries ([`config::FORGE_CLI_TOKEN_VARIABLES`]), the user and password of a URL
/// (`https://user:secret@host/` is `https://[hidden]@host/`), and each word that starts as a
/// GitHub or GitLab token does ([`TOKEN_SHAPES`]).
#[must_use]
pub fn redacted(text: &str) -> String {
    let mut out = text.to_owned();
    for variable in config::FORGE_CLI_TOKEN_VARIABLES
        .iter()
        .flat_map(|(_, variables)| variables.iter())
    {
        if let Ok(value) = std::env::var(variable) {
            let value = value.trim();
            if value.len() >= 4 {
                out = out.replace(value, HIDDEN);
            }
        }
    }
    shapes_hidden(&userinfo_hidden(&out))
}

/// `text` with the user part of each URL taken out.
fn userinfo_hidden(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        out.push_str(&rest[..at + 3]);
        rest = &rest[at + 3..];
        let end = rest
            .find(|c: char| c == '/' || c.is_whitespace() || c == '\'' || c == '"')
            .unwrap_or(rest.len());
        if let Some(sign) = rest[..end].rfind('@') {
            out.push_str(HIDDEN);
            rest = &rest[sign..];
        }
    }
    out.push_str(rest);
    out
}

/// `text` with each word that starts as a forge's token does taken out.
fn shapes_hidden(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let next = TOKEN_SHAPES
            .iter()
            .filter_map(|shape| rest.find(shape).map(|at| (at, shape.len())))
            .min();
        let Some((at, length)) = next else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..at]);
        out.push_str(HIDDEN);
        let after = &rest[at + length..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(after.len());
        rest = &after[end..];
    }
}

/// Whether `instance` has a `github` or `gitlab` source that names `include`.
#[must_use]
pub fn includes(instance: &Instance) -> bool {
    instance.sources.iter().any(|source| match source {
        Source::GitHub(github) => github.include.is_some(),
        Source::GitLab(gitlab) => gitlab.include.is_some(),
        Source::Local(_) => false,
    })
}

/// At the start of a runtime alone in a pod, before it seeds `instance`: each repository its
/// sources include that has no checkout is cloned, [`repositories::WIDTH`] at once, but none is
/// begun once the start's bound ([`start_bound`]) has passed since it began; what failed is
/// said, and how many were left to the passes, and the start goes on. `what` names the runtime
/// in what it says. Answers how many were cloned.
pub fn at_start(instance: &Instance, what: &str) -> usize {
    let began = Instant::now();
    if !includes(instance) {
        return 0;
    }
    let wanted = match repositories::included(instance) {
        Ok(wanted) => wanted,
        Err(error) => {
            crate::run::say(
                true,
                &format!(
                    "{what}: the repositories its sources include were not listed, and nothing \
                     is cloned before the first pass: {}",
                    one_line(&error)
                ),
            );
            return 0;
        }
    };
    let root = Path::new(&instance.checkouts.root);
    if let Err(error) = fs::create_dir_all(root) {
        crate::run::say(
            true,
            &format!(
                "{what}: nothing is cloned: create {}: {error}",
                root.display()
            ),
        );
        return 0;
    }
    let bound = start_bound(what);
    let wanted: Vec<Wanted> = wanted
        .into_iter()
        .filter(|wanted| wanted.checkout.is_none() && !wanted.archived)
        .collect();
    let brought = repositories::each(&wanted, repositories::WIDTH, |wanted| {
        Ok((began.elapsed() < bound).then(|| bring(root, wanted, None)))
    })
    .unwrap_or_default();
    let mut cloned = 0;
    let mut left = 0;
    for (wanted, brought) in wanted.iter().zip(brought) {
        match brought {
            Some(Brought::Cloned(dir)) => {
                cloned += 1;
                crate::run::say(
                    false,
                    &format!("{what}: cloned {} into {}", wanted.url, dir.display()),
                );
            }
            Some(Brought::Failed(line)) => crate::run::say(
                true,
                &format!("{what}: {}: {line}; the next pass tries again", wanted.name),
            ),
            Some(Brought::Left | Brought::Fetched | Brought::Stopped) => {}
            None => left += 1,
        }
    }
    if left > 0 {
        crate::run::say(
            true,
            &format!(
                "{what}: the start begins no clone after {} s; {left} included repositories \
                 without a checkout are left to the collect passes",
                bound.as_secs()
            ),
        );
    }
    cloned
}

/// The start's bound on beginning clones: [`START_BOUND_VARIABLE`] seconds where it is set to a
/// whole number, else [`CLONE_BOUND`]; a value that is none is said, under `what`.
fn start_bound(what: &str) -> Duration {
    let Some(value) = std::env::var_os(START_BOUND_VARIABLE) else {
        return CLONE_BOUND;
    };
    if let Some(seconds) = value
        .to_str()
        .and_then(|text| text.trim().parse::<u64>().ok())
    {
        return Duration::from_secs(seconds);
    }
    crate::run::say(
        true,
        &format!(
            "{what}: {START_BOUND_VARIABLE} {value:?} is not a whole number of seconds; the start \
             begins clones for {} s",
            CLONE_BOUND.as_secs()
        ),
    );
    CLONE_BOUND
}

/// What the start derives from the set of `instance`'s checkouts, brought up to date after a
/// clone: the workspace file under its records, where the records directory is there (the
/// seeding writes it otherwise), and the trust of each checkout an `include` entry covers
/// ([`included_checkouts`]), no other directory under the root. The activation is the checkouts
/// root's, which a new checkout under it needs nothing of. Writes nothing on standard output.
///
/// # Errors
///
/// The home directory is not absolute, the members cannot be read, or a file not written.
pub fn refresh(instance: &Instance) -> Result<()> {
    let environment = config::Environment::from_process()?;
    let index = config::load(config::explicit(), &environment)
        .ok()
        .and_then(|loaded| {
            loaded
                .config
                .instances
                .iter()
                .position(|other| other.name == instance.name)
        })
        .unwrap_or_default();
    let records = Path::new(&instance.records);
    if !records.is_dir() {
        return Ok(());
    }
    let file = records.join(blockers::WORKSPACE);
    let text = config::workspace_text(&config::workspace_members(instance, index)?);
    if fs::read_to_string(&file).ok().as_deref() != Some(text.as_str()) {
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&file, text)?;
    }
    let mut edit = trust::Edit::read(&environment.home()?.join(trust::CLAUDE_JSON))?;
    let missing: Vec<String> = included_checkouts(instance, index)?
        .into_iter()
        .filter(|path| !edit.trusts(path))
        .collect();
    if !missing.is_empty() {
        for path in &missing {
            edit.trust(path);
        }
        edit.write()?;
    }
    Ok(())
}

/// The physical path of each checkout of `instance` (the one at `index` of its config file) an
/// `include` entry of one of its `github` or `gitlab` sources covers, as
/// [`config::instance_checkouts`] lists a source's checkouts, each one [`is_checkout`]. A
/// directory under the root that no entry covers is none of them.
///
/// # Errors
///
/// A directory cannot be read, or a path cannot be resolved or is not UTF-8.
fn included_checkouts(instance: &Instance, index: usize) -> Result<Vec<String>> {
    let mut included = instance.clone();
    included.sources.retain(|source| match source {
        Source::GitHub(github) => github.include.is_some(),
        Source::GitLab(gitlab) => gitlab.include.is_some(),
        Source::Local(_) => false,
    });
    config::instance_checkouts(&included, index)?
        .into_iter()
        .filter(|dir| is_checkout(dir))
        .map(|dir| {
            let physical =
                fs::canonicalize(&dir).with_context(|| format!("resolve {}", dir.display()))?;
            physical
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("{} is not UTF-8", physical.display()))
        })
        .collect()
}

/// `conductor doctor`'s rows of the checkouts, in a pod only ([`in_pod`]): for each instance of
/// the config `flag` whose sources include repositories,
/// `checkouts <instance>: <n> of <m> included repositories have no checkout`, archived ones not
/// counted, or `checkouts <instance>: the included repositories were not listed: <why>`. Neither
/// is a failure of the verdict: the next pass clones what it can.
#[must_use]
pub fn doctor_lines(flag: Option<&Path>) -> Vec<(String, Option<String>)> {
    if !in_pod() {
        return Vec::new();
    }
    let Ok(loaded) = config::Environment::from_process()
        .and_then(|environment| config::load(flag, &environment))
    else {
        return Vec::new();
    };
    loaded
        .config
        .instances
        .iter()
        .filter(|instance| includes(instance))
        .map(|instance| {
            let name = &instance.name.0;
            let root = Path::new(&instance.checkouts.root);
            let line = match repositories::included(instance) {
                Ok(wanted) => {
                    let wanted: Vec<&Wanted> =
                        wanted.iter().filter(|wanted| !wanted.archived).collect();
                    let missing = wanted
                        .iter()
                        .filter(|wanted| {
                            wanted.checkout.is_none() && !is_checkout(&root.join(&wanted.path))
                        })
                        .count();
                    format!(
                        "checkouts {name}: {missing} of {} included repositories have no checkout",
                        wanted.len()
                    )
                }
                Err(error) => format!(
                    "checkouts {name}: the included repositories were not listed: {}",
                    one_line(&error)
                ),
            };
            (line, None)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{redacted, shapes_hidden, userinfo_hidden};

    #[test]
    fn a_urls_user_part_is_taken_out() {
        assert_eq!(
            userinfo_hidden("fatal: 'https://oauth2:abc@gitlab.example.com/x.git/' failed"),
            "fatal: 'https://[hidden]@gitlab.example.com/x.git/' failed"
        );
        assert_eq!(
            userinfo_hidden("https://gitlab.example.com/x.git"),
            "https://gitlab.example.com/x.git"
        );
    }

    #[test]
    fn a_word_shaped_as_a_forge_token_is_taken_out() {
        let shaped = ["the ", "glpat", "-", "abcDEF012", "_x-y is refused"].concat();
        assert_eq!(shapes_hidden(&shaped), "the [hidden] is refused");
        assert_eq!(redacted("nothing to hide"), "nothing to hide");
    }
}
