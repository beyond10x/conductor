//! Adversary cases for `story:box-runtime-starts`, pass 2: the four writable roots and the
//! refusal `placement.writable-root-shared`, `conductor setup --apply` beside a store that is
//! there, and the layouts the pages show.
//!
//! The library cases ask `placement::request` or `placement::unmountable` as conductord does;
//! the command cases run the built binary with the environment cleared, `HOME` the case's
//! `home/` and `PATH` the case's `bin/` of fakes. No daemon, box or unit runs.

mod active;
mod common;
mod scratch;

use std::env;
use std::fs;
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::placement::{self, BoxRequest};
use conductor_model::config::Instance;

const OLD_STORE: &str = "sqlite:~/.b10x/conductor/conductor.sqlite";
const NEW_STORE: &str = "sqlite:~/.b10x/conductor/store/conductor.sqlite";

/// The case's directory, removed when the case passes, and its path with every link resolved.
fn case(name: &str) -> (scratch::Scratch, PathBuf) {
    active::isolate();
    let kept = scratch::Scratch::new(Path::new("adv_box_mounts_pass2").join(name));
    fs::create_dir_all(kept.path().join("home/src")).expect("create the case's home");
    fs::create_dir_all(kept.path().join("bin")).expect("create the case's bin");
    let dir = kept.path().canonicalize().expect("the case's directory");
    (kept, dir)
}

fn config(store: &str, more: &str) -> String {
    format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 store: {store}\n\
         instances:\n\
         \x20 - name: home\n\
         \x20   placement: systemd\n\
         \x20   session_prefix: home\n\
         \x20   sources:\n\
         \x20     - local: ~/src\n\
         \x20   checkouts:\n\
         \x20     root: ~/src\n\
         \x20     trees: ~/.local/state/worktree/trees/home\n\
         {more}"
    )
}

fn instance(text: &str, home: &Path) -> Instance {
    conductor_cli::config::parse(text, home)
        .unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"))
        .instances
        .remove(0)
}

/// The box conductord asks for: `HOME`, a `PATH` that finds no program, the store as written.
fn request(instance: &Instance, file: &Path, home: &Path, store: &str) -> BoxRequest {
    let env = vec![
        ("HOME".to_owned(), home.display().to_string()),
        ("PATH".to_owned(), home.join("no-bin").display().to_string()),
        (
            conductor_cli::store::STORE_VARIABLE.to_owned(),
            store.to_owned(),
        ),
    ];
    placement::request(instance, file, Path::new("/usr/bin/conductor"), env)
}

fn shown(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `conductor <args>` in `cwd`, the environment cleared but `HOME` and the case's `bin/`.
fn conductor(dir: &Path, cwd: &Path, args: &[&str]) -> Output {
    let home = dir.join("home");
    common::conductor(&home)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", &home)
        .env("USER", "operator")
        .env("PATH", dir.join("bin"))
        .env(
            conductor_cli::secret::TEST_KEYCHAIN_VARIABLE,
            home.join(".test-keychain.ron"),
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Example Operator")
        .env("GIT_AUTHOR_EMAIL", "operator@example.invalid")
        .env("GIT_COMMITTER_NAME", "Example Operator")
        .env("GIT_COMMITTER_EMAIL", "operator@example.invalid")
        .stdin(Stdio::null())
        .output()
        .expect("conductor runs")
}

/// The case's `bin/` (a link to the real `git`, a fake of each program conductor starts) and a
/// git repository `home/src/<name>` with a planning store of today's version.
fn onboarding(dir: &Path, name: &str) -> PathBuf {
    let bin = dir.join("bin");
    let git = env::split_paths(&env::var_os("PATH").expect("PATH is set"))
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git is on PATH");
    symlink(&git, bin.join("git")).expect("link git");
    for (program, line) in [
        ("claude", "2.1.295 (Claude Code)"),
        ("jq", "jq-1.8.2"),
        ("aep", "aep 99.1.0"),
        ("worktree", "worktree-cli 99.1.0"),
        ("ess", "ess 99.1.0"),
        ("substrate-daemon", "substrate-daemon 99.1.0"),
        ("b10x-gates", "b10x-gates 0.1.27"),
    ] {
        let file = bin.join(program);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\nif [ \"$1\" = --version ]; then printf '%s\\n' '{line}'; fi\nexit 0\n"
            ),
        )
        .expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }
    let home = dir.join("home");
    fs::write(home.join(".claude.json"), "{}\n").expect("write ~/.claude.json");
    let repository = home.join("src").join(name);
    fs::create_dir_all(repository.join(".engineering")).expect("create the repository");
    let made = Command::new(&git)
        .arg("-C")
        .arg(&repository)
        .args(["init", "--quiet"])
        .env("HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(made.status.success(), "git init: {}", shown(&made));
    fs::write(
        repository.join(".engineering/project.yaml"),
        "version: aep.project/5\n",
    )
    .expect("write the project file");
    repository
}

/// Focus 4. A person who has a database at the path a config without `daemon.store` means and
/// no config file (every command before the file is written uses that path) runs
/// `conductor setup --apply`: the file it writes is one `config validate` passes, or `--apply`
/// says what conductord will refuse.
#[test]
fn setup_apply_beside_a_store_at_the_default_path_writes_a_file_validate_passes_or_says_why() {
    let (_kept, dir) = case("setup-old-store");
    let repository = onboarding(&dir, "theta");
    let top = dir.join("home/.b10x/conductor");
    fs::create_dir_all(&top).expect("create the directory");
    fs::write(top.join("conductor.sqlite"), "").expect("an empty database file");
    let applied = conductor(&dir, &repository, &["setup", "--apply"]);
    let file = top.join("conductor.yaml");
    assert!(file.is_file(), "--apply wrote no file: {}", shown(&applied));
    let validated = conductor(&dir, &repository, &["config", "validate"]);
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&applied.stdout),
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(
        validated.status.code() == Some(0) || said.contains(placement::ROOT_SHARED),
        "--apply wrote a file whose box conductord refuses and did not say so.\nthe file:\n{}\n\
         setup --apply: {}\nconfig validate: {}",
        fs::read_to_string(&file).unwrap_or_default(),
        shown(&applied),
        shown(&validated)
    );
}

/// Focus 4. The commands SETUP.md runs before the config file is there (`doctor`, `setup`) leave
/// no database at the default path, `--apply` then writes a file `config validate` passes, and a
/// second `--apply` changes no byte of it.
#[test]
fn the_commands_before_the_config_file_leave_apply_a_file_validate_passes_twice() {
    let (_kept, dir) = case("setup-fresh");
    let repository = onboarding(&dir, "iota");
    let old = dir.join("home/.b10x/conductor/conductor.sqlite");
    for args in [&["doctor"][..], &["setup"][..]] {
        let output = conductor(&dir, &repository, args);
        assert!(
            fs::symlink_metadata(&old).is_err(),
            "conductor {args:?} left a database at the default path, which --apply then names: {}",
            shown(&output)
        );
    }
    let applied = conductor(&dir, &repository, &["setup", "--apply"]);
    let file = dir.join("home/.b10x/conductor/conductor.yaml");
    let first = fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("--apply wrote no file ({error}): {}", shown(&applied)));
    assert!(first.contains(NEW_STORE), "{first}");
    let validated = conductor(&dir, &repository, &["config", "validate"]);
    assert_eq!(validated.status.code(), Some(0), "{}", shown(&validated));
    let again = conductor(&dir, &repository, &["setup", "--apply"]);
    assert_eq!(
        fs::read_to_string(&file).expect("the file"),
        first,
        "a second --apply changed the file: {}",
        shown(&again)
    );
    let validated = conductor(&dir, &repository, &["config", "validate"]);
    assert_eq!(validated.status.code(), Some(0), "{}", shown(&validated));
}

/// Focus 1 and 5. A sqlite store written without a directory (`sqlite:conductor.sqlite`):
/// conductord opens it in its own working directory, and the runtime in the box is handed the
/// same URL and starts in its state directory. Where `config validate` passes the file, the box
/// either writes the daemon's store or is refused; it is not sent with no store root and no word.
#[test]
fn a_box_handed_a_store_without_a_directory_is_refused_or_writes_that_store() {
    let (_kept, dir) = case("relative-store");
    let home = dir.join("home");
    let top = home.join(".b10x/conductor");
    fs::create_dir_all(&top).expect("create the directory");
    let store = "sqlite:conductor.sqlite";
    let text = config(store, "");
    let file = top.join("conductor.yaml");
    fs::write(&file, &text).expect("write the config file");
    let validated = conductor(&dir, &home, &["config", "validate"]);
    assert_eq!(
        validated.status.code(),
        Some(0),
        "config validate refuses the file, so nobody reaches the box: {}",
        shown(&validated)
    );
    let request = request(&instance(&text, &home), &file, &home, store);
    assert!(
        request.refused.is_some()
            || request
                .writable_roots
                .iter()
                .any(|root| root.name == "store"),
        "validate passes {store}, and the box is sent with no store root and no refusal: the \
         runtime opens another database in its state directory. roots {:?}",
        request.writable_roots
    );
}

/// Focus 8, the mutant `shares` without `at_or_under(dir, mount)`: the config file elsewhere (as
/// `--config` names it), the store at the default path, whose directory holds conductord's own.
/// Every refusal the unit's suite holds for that store is earned by the config file first.
#[test]
fn a_store_directory_that_holds_only_the_daemons_directory_is_refused() {
    let (_kept, dir) = case("daemon-only");
    let home = dir.join("home");
    let file = home.join("example-org/conductor.yaml");
    fs::create_dir_all(file.parent().expect("a parent")).expect("create the directory");
    let text = config(OLD_STORE, "");
    fs::write(&file, &text).expect("write the config file");
    let request = request(&instance(&text, &home), &file, &home, OLD_STORE);
    let refused = request
        .refused
        .clone()
        .expect("the store's directory holds daemon/");
    assert_eq!(refused.name, placement::ROOT_SHARED);
    assert!(
        refused
            .detail
            .contains(&home.join(".b10x/conductor/daemon").display().to_string()),
        "{refused:?}"
    );
    assert!(
        request
            .writable_roots
            .iter()
            .all(|root| root.name != "store"),
        "{:?}",
        request.writable_roots
    );
}

/// Focus 8, the mutant `layout` without `real(config_file)`: the config file a link outside every
/// writable directory, the file it names inside the records.
#[test]
fn a_records_directory_that_holds_the_file_the_config_link_names_is_refused() {
    let (_kept, dir) = case("link-target");
    let home = dir.join("home");
    let records = home.join("records/home");
    fs::create_dir_all(&records).expect("create the records");
    let text = config(NEW_STORE, "    records: ~/records/home\n");
    let held = records.join("conductor.yaml");
    fs::write(&held, &text).expect("write the config file");
    let file = home.join("etc/conductor.yaml");
    fs::create_dir_all(file.parent().expect("a parent")).expect("create the directory");
    symlink(&held, &file).expect("link");
    let request = request(&instance(&text, &home), &file, &home, NEW_STORE);
    assert_eq!(
        request
            .refused
            .as_ref()
            .map(|refused| refused.name.as_str()),
        Some(placement::ROOT_SHARED),
        "the box writes {} and reads its config from it: roots {:?}",
        records.display(),
        request.writable_roots
    );
}

/// Each fenced block of `text` that is a whole `conductor.config/2` file, its indentation taken
/// off.
fn config_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, Vec<String>)> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        if line.trim_start().starts_with("```") {
            match open.take() {
                Some((_, lines)) => blocks.push(lines.join("\n") + "\n"),
                None => open = Some((indent, Vec::new())),
            }
        } else if let Some((indent, lines)) = &mut open {
            lines.push(line.get(*indent..).unwrap_or("").to_owned());
        }
    }
    blocks
        .into_iter()
        .filter(|block| {
            block.contains("version: conductor.config/2") && block.contains("placement:")
        })
        .collect()
}

/// Focus 7. Every whole config file a page shows (README, SETUP.md, the example file, the
/// hand-written pages) with an instance in a box is a layout conductord admits, read where the
/// pages put the file: `~/.b10x/conductor/conductor.yaml`.
#[test]
fn every_config_file_a_page_shows_with_a_box_is_one_conductord_admits() {
    let (_kept, dir) = case("pages");
    let home = dir.join("home");
    let file = home.join(".b10x/conductor/conductor.yaml");
    let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut pages = vec![top.join("README.md"), top.join("SETUP.md")];
    let mut walk = vec![top.join("website/docs")];
    while let Some(folder) = walk.pop() {
        for entry in fs::read_dir(&folder).expect("read the pages") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "reference") {
                    walk.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "md") {
                pages.push(path);
            }
        }
    }
    pages.sort();
    let mut texts: Vec<(String, String)> = vec![(
        "conductor.example.yaml".to_owned(),
        fs::read_to_string(top.join("conductor.example.yaml")).expect("the example file"),
    )];
    for page in pages {
        let text = fs::read_to_string(&page).expect("the page");
        let name = page
            .strip_prefix(&top)
            .unwrap_or(&page)
            .display()
            .to_string();
        texts.extend(
            config_blocks(&text)
                .into_iter()
                .map(|block| (name.clone(), block)),
        );
    }
    let mut boxes = 0;
    let mut refused: Vec<String> = Vec::new();
    for (page, text) in &texts {
        let Ok(parsed) = conductor_cli::config::parse(text, &home) else {
            continue;
        };
        let host = placement::Host {
            home: Some(home.clone()),
            path: None,
            store: Some(parsed.daemon.store.0.clone()),
            daemon: Some(home.join(".b10x/conductor/daemon")),
        };
        for instance in parsed.instances.iter().filter(|one| placement::boxed(one)) {
            boxes += 1;
            let binary = Path::new("/usr/bin/conductor");
            if let Some(why) = placement::unmountable(instance, &file, binary, &host) {
                refused.push(format!(
                    "{page}: instance {}: {}: {}",
                    instance.name.0, why.name, why.detail
                ));
            }
        }
    }
    assert!(
        boxes >= 6,
        "the pages show a box in at least six whole config files, read {boxes}"
    );
    assert!(
        refused.is_empty(),
        "of {boxes} boxes the pages show, conductord refuses:\n{}",
        refused.join("\n")
    );
}
