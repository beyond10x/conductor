//! `story:gitlab-guide`: the hand-written documentation describes a GitLab instance as the binary
//! runs it.
//!
//! - `website/docs/guides/gitlab.md` exists, `website/sidebars.ts` lists it as `guides/gitlab`, and
//!   the page holds `host: gitlab.example.com`, `glab auth login --hostname`, and the words
//!   repositories, merge requests, pipelines and releases in its list of what conductor reads.
//! - No hand-written page (`README.md`, `docs/design/conductor.md`, and `website/docs/` but its
//!   generated `reference/`) says the binary does not read a forge.
//! - The README's requirements table has a `gh` row for a GitHub source and a `glab` row for a
//!   GitLab source.
//! - The sample `conductor doctor` output of `website/docs/getting-started.md` ends with its verdict
//!   line (`story:doctor-verdict`).
//!
//! Every file is read relative to the workspace root, and a finding names the file and the line.

use std::fs;
use std::path::{Path, PathBuf};

/// The phrases that say the binary does not read a forge; none is left on a hand-written page.
const NOT_READ: [&str; 3] = [
    "not read by the binary",
    "the binary does not read it",
    "does not read that forge",
];

/// The words the guide's list of what conductor reads from GitLab holds.
const READS: [&str; 4] = ["repositories", "merge requests", "pipelines", "releases"];

/// The heading of the guide's section on what conductor reads from GitLab.
const READS_HEADING: &str = "## What conductor reads from GitLab";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Every Markdown file under `dir`, recursively, but those under `skip`.
fn pages(dir: &Path, skip: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
    {
        let path = entry.expect("a directory entry").path();
        if path.starts_with(skip) {
            continue;
        }
        if path.is_dir() {
            pages(&path, skip, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "md" || extension == "mdx")
        {
            out.push(path);
        }
    }
}

/// `text` in lower case with every run of whitespace one space, so a phrase broken across lines
/// is still found.
fn flattened(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[test]
fn the_sidebar_lists_the_gitlab_guide() {
    let guide = workspace().join("website/docs/guides/gitlab.md");
    assert!(guide.is_file(), "{} is not a file", guide.display());
    let sidebar = read("website/sidebars.ts");
    assert!(
        sidebar.contains("'guides/gitlab'"),
        "website/sidebars.ts does not list 'guides/gitlab'"
    );
}

#[test]
fn the_gitlab_guide_names_the_host_the_login_and_what_conductor_reads() {
    let guide = read("website/docs/guides/gitlab.md");
    for needed in ["host: gitlab.example.com", "glab auth login --hostname"] {
        assert!(
            guide.contains(needed),
            "website/docs/guides/gitlab.md does not contain {needed:?}"
        );
    }
    let section = guide
        .split_once(READS_HEADING)
        .map(|(_, after)| after.split("\n## ").next().unwrap_or_default())
        .unwrap_or_else(|| {
            panic!("website/docs/guides/gitlab.md has no section {READS_HEADING:?}")
        });
    let list: Vec<String> = section
        .lines()
        .filter(|line| line.trim_start().starts_with("- "))
        .map(str::to_lowercase)
        .collect();
    assert!(
        !list.is_empty(),
        "the section {READS_HEADING:?} of website/docs/guides/gitlab.md has no list"
    );
    for word in READS {
        assert!(
            list.iter().any(|item| item.contains(word)),
            "the list under {READS_HEADING:?} in website/docs/guides/gitlab.md does not name \
             {word}: {list:#?}"
        );
    }
}

#[test]
fn no_hand_written_page_says_the_binary_does_not_read_a_forge() {
    let root = workspace();
    let mut files = vec![
        root.join("README.md"),
        root.join("docs/design/conductor.md"),
    ];
    let website = root.join("website/docs");
    pages(&website, &website.join("reference"), &mut files);
    assert!(files.len() > 2, "no page found under {}", website.display());
    let mut findings = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
        let flat = flattened(&text);
        for phrase in NOT_READ {
            if flat.contains(phrase) {
                let line = text
                    .lines()
                    .position(|line| flattened(line).contains(phrase))
                    .map_or_else(|| "a line break".to_owned(), |at| (at + 1).to_string());
                findings.push(format!(
                    "{}:{line}: {phrase:?}",
                    file.strip_prefix(&root).unwrap_or(file).display()
                ));
            }
        }
    }
    assert!(
        findings.is_empty(),
        "hand-written pages say the binary does not read a forge:\n{}",
        findings.join("\n")
    );
}

#[test]
fn the_readme_requires_gh_for_a_github_source_and_glab_for_a_gitlab_source() {
    let readme = read("README.md");
    for (start, text) in [("| `gh`", "GitHub source"), ("| `glab`", "GitLab source")] {
        assert!(
            readme
                .lines()
                .any(|line| line.starts_with(start) && line.contains(text)),
            "README.md has no row starting {start:?} that contains {text:?}"
        );
    }
}

#[test]
fn the_getting_started_doctor_sample_ends_with_its_verdict() {
    let page = read("website/docs/getting-started.md");
    let mut samples = Vec::new();
    let mut block: Option<Vec<&str>> = None;
    for line in page.lines() {
        match block.as_mut() {
            None if line.trim_start().starts_with("```") => block = Some(Vec::new()),
            None => {}
            Some(lines) if line.trim_start().starts_with("```") => {
                if lines.first().map(|first| first.trim()) == Some("$ conductor doctor") {
                    samples.push(lines.clone());
                }
                block = None;
            }
            Some(lines) => lines.push(line),
        }
    }
    assert!(
        !samples.is_empty(),
        "website/docs/getting-started.md has no code block starting `$ conductor doctor`"
    );
    for sample in samples {
        let last = sample
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .copied()
            .unwrap_or_default();
        assert!(
            last.starts_with("ready") || last.starts_with("not ready:"),
            "the conductor doctor sample of website/docs/getting-started.md ends with {last:?}, \
             not a line starting `ready` or `not ready:`"
        );
    }
}

/// `story:session-envelope-seam`: the pages on confinement and on starting conductor name
/// `conductor session start`, as do the `confinement.mode` and `confinement.provider` rows of
/// `docs/config.md`; no hand-written page says nothing starts a session in a box yet, and the
/// design no longer runs both sessions with `claude --bg`.
#[test]
fn the_confinement_pages_name_session_start() {
    for page in [
        "website/docs/security/confinement.md",
        "website/docs/guides/start-conductor.md",
        "docs/design/conductor.md",
    ] {
        assert!(
            flattened(&read(page)).contains("conductor session start"),
            "{page} does not name conductor session start"
        );
    }
    let config = read("docs/config.md");
    for key in ["`confinement.mode`", "`confinement.provider`"] {
        let row = config
            .lines()
            .find(|line| line.starts_with(&format!("| {key} ")))
            .unwrap_or_else(|| panic!("docs/config.md has no row {key}"));
        assert!(
            row.contains("conductor session start"),
            "docs/config.md: the row {key} does not name conductor session start: {row}"
        );
    }
    let root = workspace();
    let mut files = vec![root.join("README.md")];
    pages(&root.join("docs"), &root.join("docs/waves"), &mut files);
    pages(
        &root.join("website/docs"),
        &root.join("website/docs/reference"),
        &mut files,
    );
    for file in files {
        let text = flattened(&fs::read_to_string(&file).expect("read a page"));
        assert!(
            !text.contains("nothing starts a session in a box yet"),
            "{} says nothing starts a session in a box yet",
            file.display()
        );
    }
    assert!(
        !flattened(&read("docs/design/conductor.md"))
            .contains("both sessions run in the background (`claude --bg`)"),
        "docs/design/conductor.md runs both sessions with claude --bg"
    );
}
