//! `story:getting-started-substrate-steps-portable`: the substrate steps of the guides run as
//! written on Debian, Fedora and Arch.
//!
//! - `install -d -m 700` sets the mode of the last component only, so the step that creates a
//!   directory beneath `~/.local/state/substrate` names that parent first, as a directory of its
//!   own. `substrate-daemon` refuses a state directory that is not mode 0700.
//! - `/etc/ssl/certs/ca-certificates.crt` is a symbolic link on Arch, and `substrate-daemon`
//!   refuses a `--ca-bundle` that is not a regular file, so the value is the resolved link
//!   (`readlink -f` or `realpath`), never a bare path under `/etc/ssl/certs`.
//!
//! Both are held on `website/docs/getting-started.md` and on step 9 of `SETUP.md`. Every file is
//! read relative to the workspace root, and a finding names the file and the line.

use std::fs;
use std::path::{Path, PathBuf};

/// The parent the substrate steps create new, and which `substrate-daemon` holds to mode 0700.
const PARENT: &str = "~/.local/state/substrate";

const PAGES: [&str; 2] = ["website/docs/getting-started.md", "SETUP.md"];

fn read(relative: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// The words of `line`, without the quotes, backticks and sentence punctuation around each.
fn words(line: &str) -> Vec<&str> {
    line.split_whitespace()
        .map(|word| word.trim_matches(|c| matches!(c, '"' | '`' | ',' | '.' | ';' | ':')))
        .collect()
}

/// What is wrong with the directory step of `page`: every `install -d -m 700` line that creates a
/// directory beneath [`PARENT`] without naming [`PARENT`] before it.
fn directory_problems(name: &str, page: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for (index, line) in page.lines().enumerate() {
        if !line.contains("install -d -m 700 ") {
            continue;
        }
        let words = words(line);
        let parent = words.iter().position(|word| *word == PARENT);
        for (at, word) in words.iter().enumerate() {
            let beneath = word
                .strip_prefix(PARENT)
                .is_some_and(|rest| rest.starts_with('/'));
            if beneath && parent.is_none_or(|parent| parent > at) {
                problems.push(format!(
                    "{name}:{}: `install -d -m 700` creates {word} and leaves {PARENT} at the \
                     default mode: name {PARENT} before it",
                    index + 1
                ));
            }
        }
    }
    problems
}

/// What is wrong with the CA bundle of `page`: every `--ca-bundle` whose value is a bare path
/// under `/etc/ssl/certs`, where the distributions keep symbolic links.
fn bundle_problems(name: &str, page: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for (index, line) in page.lines().enumerate() {
        for (at, _) in line.match_indices("--ca-bundle ") {
            let value = line[at + "--ca-bundle ".len()..].trim_start_matches('"');
            let resolved = value.starts_with("$(readlink -f ") || value.starts_with("$(realpath ");
            if value.starts_with("/etc/ssl/certs/") || !resolved {
                problems.push(format!(
                    "{name}:{}: `--ca-bundle` names {:?}, which is a symbolic link on Arch: \
                     resolve it with `readlink -f` or `realpath`",
                    index + 1,
                    value.split(['`', ' ']).next().unwrap_or_default()
                ));
            }
        }
    }
    problems
}

#[test]
fn the_directory_step_names_the_substrate_state_directory_itself() {
    let mut problems = Vec::new();
    for name in PAGES {
        problems.extend(directory_problems(name, &read(name)));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let guide = read(PAGES[0]);
    assert!(
        guide
            .lines()
            .any(|line| line.contains("install -d -m 700 ") && words(line).contains(&PARENT)),
        "{} has no `install -d -m 700` line naming {PARENT}",
        PAGES[0]
    );
}

#[test]
fn setup_md_names_the_substrate_state_directory_as_mode_700() {
    let setup = read("SETUP.md");
    let named = setup
        .lines()
        .enumerate()
        .filter(|(_, line)| words(line).contains(&PARENT))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let lines = setup.lines().collect::<Vec<_>>();
    assert!(
        named.iter().any(|index| {
            lines[*index..(*index + 3).min(lines.len())]
                .join(" ")
                .contains("mode 700")
        }),
        "SETUP.md does not say to create {PARENT} itself at mode 700"
    );
}

#[test]
fn the_ca_bundle_is_the_resolved_file_on_both_pages() {
    for name in PAGES {
        let page = read(name);
        assert!(
            page.contains("--ca-bundle "),
            "{name} names no `--ca-bundle`"
        );
        let problems = bundle_problems(name, &page);
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}

#[test]
fn the_checks_of_this_test_find_what_they_look_for() {
    let old = "install -d -m 700 \"$state/sessions\" ~/.local/state/substrate/workspaces\n";
    assert_eq!(directory_problems("page", old).len(), 1, "{old}");
    let late = "install -d -m 700 ~/.local/state/substrate/workspaces ~/.local/state/substrate\n";
    assert_eq!(directory_problems("page", late).len(), 1, "{late}");
    let new = "install -d -m 700 ~/.local/state/substrate ~/.local/state/substrate/workspaces\n";
    assert!(directory_problems("page", new).is_empty(), "{new}");

    let link = "  --ca-bundle /etc/ssl/certs/ca-certificates.crt\n";
    assert_eq!(bundle_problems("page", link).len(), 1, "{link}");
    let quoted = "  --ca-bundle \"$(readlink -f /etc/ssl/certs/ca-certificates.crt)\"\n";
    assert!(bundle_problems("page", quoted).is_empty(), "{quoted}");
    let bare = "- Fix: `x --ca-bundle $(realpath /etc/ssl/certs/ca-certificates.crt)`, kept\n";
    assert!(bundle_problems("page", bare).is_empty(), "{bare}");
}

/// Adversary cases of wave 50, unit U6, pass 1. Each is red on the tree it was written against.
mod adversary {
    use super::*;

    /// The predicate of `setup_md_names_the_substrate_state_directory_as_mode_700`, word for word,
    /// over a text instead of the file: that test holds it inline.
    fn setup_holds(setup: &str) -> bool {
        let lines = setup.lines().collect::<Vec<_>>();
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| words(line).contains(&PARENT))
            .any(|(index, _)| {
                lines[index..(index + 3).min(lines.len())]
                    .join(" ")
                    .contains("mode 700")
            })
    }

    /// The resolved path is one of the acceptance's three distributions only: the file exists on
    /// Fedora and neither on Debian nor on Arch, where `readlink -f` prints the name of a file
    /// that is not there and exits 0.
    #[test]
    fn adv_a_a_resolved_path_two_of_the_three_distributions_lack_is_found() {
        let fedora_only = "  --ca-bundle \"$(readlink -f /etc/pki/tls/certs/ca-bundle.crt)\"\n";
        assert_eq!(
            bundle_problems("page", fedora_only).len(),
            1,
            "{fedora_only}"
        );
    }

    /// `realpath -s` resolves no symbolic link: the value is the link the story reports.
    #[test]
    fn adv_b_a_realpath_that_resolves_no_link_is_found() {
        let unresolved = "  --ca-bundle \"$(realpath -s /etc/ssl/certs/ca-certificates.crt)\"\n";
        assert_eq!(bundle_problems("page", unresolved).len(), 1, "{unresolved}");
    }

    /// SETUP.md with its instruction turned to mode 755: the reason beside it, "a state directory
    /// that is not mode 700", must not stand in for the instruction.
    #[test]
    fn adv_c_setup_md_telling_mode_755_is_not_held_by_the_reason_beside_it() {
        let setup = read("SETUP.md");
        let mutant = setup.replace("each at mode 700", "each at mode 755");
        assert_ne!(setup, mutant, "SETUP.md no longer says `each at mode 700`");
        assert!(setup_holds(&setup), "the predicate holds the page as it is");
        assert!(
            !setup_holds(&mutant),
            "SETUP.md telling mode 755 for {PARENT} passes the check"
        );
    }
}
