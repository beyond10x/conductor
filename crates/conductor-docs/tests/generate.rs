//! `conductor-docs generate`, `generate --check` and `provenance`, run as the binary on this
//! repository, and the status list's refusal of evidence that does not exist.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use conductor_docs::status::{Capability, data_for};

/// The repository root: two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh directory under the tree's own build directory, never the system temporary one.
fn scratch(name: &str) -> PathBuf {
    let dir = root()
        .join("target/conductor-docs")
        .join(format!("test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `conductor-docs` with `args`, its environment isolated from the operator's.
fn docs(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_conductor-docs"))
        .args(args)
        .env("HOME", home)
        .env_remove("CONDUCTOR_CONFIG")
        .env_remove("CONDUCTOR_INSTANCE")
        .output()
        .unwrap()
}

fn describe(output: &Output) -> String {
    format!(
        "status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn generate_check_passes_on_the_tree() {
    let home = scratch("check-home");
    let root = root();
    let output = docs(
        &home,
        &["generate", "--check", "--root", root.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", describe(&output));
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn a_changed_extra_or_missing_page_fails_the_check_naming_each() {
    let out = scratch("drift");
    let root = root();
    let (root, out_arg) = (root.to_str().unwrap(), out.to_str().unwrap());
    let written = docs(&out, &["generate", "--root", root, "--out", out_arg]);
    assert!(written.status.success(), "{}", describe(&written));
    let clean = docs(
        &out,
        &["generate", "--check", "--root", root, "--out", out_arg],
    );
    assert!(clean.status.success(), "{}", describe(&clean));

    let cli = out.join("website/docs/reference/cli.md");
    let edited = fs::read_to_string(&cli).unwrap() + "\nedited by hand\n";
    fs::write(&cli, edited).unwrap();
    fs::write(out.join("website/docs/reference/extra.md"), "stray\n").unwrap();
    fs::remove_file(out.join("website/docs/status.md")).unwrap();

    let drifted = docs(
        &out,
        &["generate", "--check", "--root", root, "--out", out_arg],
    );
    assert_eq!(drifted.status.code(), Some(1), "{}", describe(&drifted));
    let stderr = String::from_utf8_lossy(&drifted.stderr);
    for named in [
        "website/docs/reference/cli.md",
        "website/docs/reference/extra.md",
        "website/docs/status.md",
    ] {
        assert!(stderr.contains(named), "{named} not named:\n{stderr}");
    }
    assert!(
        !stderr.contains("website/docs/reference/config.md"),
        "an unchanged page is named:\n{stderr}"
    );
    fs::remove_dir_all(out).unwrap();
}

#[test]
fn a_hand_written_page_with_a_brace_or_a_capitalised_tag_in_prose_fails_the_check() {
    let out = scratch("passive");
    let root = root();
    let (root, out_arg) = (root.to_str().unwrap(), out.to_str().unwrap());
    let written = docs(&out, &["generate", "--root", root, "--out", out_arg]);
    assert!(written.status.success(), "{}", describe(&written));
    fs::create_dir_all(out.join("website/docs/guides")).unwrap();
    fs::write(
        out.join("website/docs/guides/a.md"),
        "---\ntitle: A\n---\n\nCode `{ok}` and `<Ok>`.\n\nA { in prose.\n\nA <Value> tag.\n",
    )
    .unwrap();
    let refused = docs(
        &out,
        &["generate", "--check", "--root", root, "--out", out_arg],
    );
    assert_eq!(refused.status.code(), Some(1), "{}", describe(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("guides/a.md:7"), "{stderr}");
    assert!(stderr.contains("guides/a.md:9"), "{stderr}");
    assert!(!stderr.contains("guides/a.md:5"), "{stderr}");
    fs::remove_dir_all(out).unwrap();
}

#[test]
fn every_generated_page_carries_the_front_matter_and_the_generator_line() {
    let out = scratch("front");
    let root = root();
    let (root, out_arg) = (root.to_str().unwrap(), out.to_str().unwrap());
    let written = docs(&out, &["generate", "--root", root, "--out", out_arg]);
    assert!(written.status.success(), "{}", describe(&written));
    for page in [
        "website/docs/reference/cli.md",
        "website/docs/reference/config.md",
        "website/docs/reference/crates.md",
        "website/docs/status.md",
    ] {
        let text = fs::read_to_string(out.join(page)).unwrap();
        let (front, body) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .unwrap_or_else(|| panic!("{page} has no front matter"));
        for key in [
            "title:",
            "sidebar_position:",
            "description:",
            "custom_edit_url: null",
        ] {
            assert!(
                front.lines().any(|line| line.starts_with(key)),
                "{page}: {key}"
            );
        }
        assert_eq!(
            body.trim_start_matches('\n').lines().next(),
            Some("<!-- generated by conductor-docs, do not edit -->"),
            "{page}"
        );
        assert!(!text.contains("story:"), "{page} names a story");
    }
    let status: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("website/data/status.json")).unwrap())
            .unwrap();
    assert_eq!(status["format"], "b10x-status/1");
    fs::remove_dir_all(out).unwrap();
}

#[test]
fn the_cli_page_lists_config_validate_with_its_arguments() {
    let page = conductor_docs::cli::page();
    assert!(page.contains("## `conductor config validate`"), "{page}");
    assert!(page.contains("Usage: conductor config validate"), "{page}");
    assert!(page.contains("## `conductor watch run`"), "{page}");
    assert!(!page.contains("conductor help"), "{page}");
}

#[test]
fn the_config_page_names_each_key_with_its_default() {
    let page = conductor_docs::config::page(&root()).unwrap();
    for row in [
        "| `version` | string | required |",
        "| `controllers.max_working` | integer | `5` |",
        "| `cadence.watch` | duration | `180s` |",
        "| `thresholds.context_handover` | [Tokens](#tokens) | `300k` |",
        "| `records` | string | `~/.b10x/conductor/<name>/records` |",
        "| `catalog.names` | `plain` \\| `hex` | `plain` |",
        "| `checkouts.root` | string | required |",
    ] {
        assert!(page.contains(row), "missing {row}\n{page}");
    }
    assert!(page.contains("`gate_resume > gate_stop`"), "{page}");
}

fn capability(status: &'static str, evidence: Option<&'static str>) -> Capability {
    Capability {
        area: "Area",
        label: "Label",
        detail: "Detail.",
        status,
        evidence,
        href: None,
    }
}

#[test]
fn a_status_item_naming_a_missing_test_is_refused() {
    let root = root();
    let gone = capability(
        "shipped",
        Some("crates/conductor/tests/config.rs::no_such_test"),
    );
    let error = data_for(&root, &[gone]).unwrap_err().to_string();
    assert!(error.contains("no_such_test"), "{error}");
    let present = capability(
        "shipped",
        Some("crates/conductor/tests/config.rs::validate_passes_on_the_story_example"),
    );
    assert!(data_for(&root, &[present]).is_ok());
    assert!(data_for(&root, &[capability("shipped", None)]).is_err());
    let claimed = capability(
        "planned",
        Some("crates/conductor/tests/config.rs::validate_passes_on_the_story_example"),
    );
    assert!(data_for(&root, &[claimed]).is_err());
    assert!(data_for(&root, &[capability("released", None)]).is_err());
    assert!(data_for(&root, &[capability("planned", None)]).is_ok());
}

#[test]
fn provenance_writes_the_site_record_and_trailing_slash_routes() {
    let site = scratch("provenance");
    let commit = "e86f43d90b637fc2c1790f77d59efff2eb852386";
    let site_arg = site.to_str().unwrap();
    let refused = docs(
        &site,
        &["provenance", "--site", site_arg, "--commit", commit],
    );
    assert_eq!(refused.status.code(), Some(1), "{}", describe(&refused));

    fs::create_dir_all(site.join("docs/reference")).unwrap();
    fs::write(site.join("index.html"), r#"<main id="main">"#).unwrap();
    fs::write(site.join("404.html"), r#"<main id="lost">"#).unwrap();
    fs::write(site.join("docs.html"), r#"<h1 id="conductor">"#).unwrap();
    fs::write(
        site.join("docs/reference/cli.html"),
        r#"<h2 id="conductor-config-validate">"#,
    )
    .unwrap();
    fs::write(
        site.join("docs/status.html"),
        r#"<meta http-equiv="refresh" content="0; url=/conductor/docs/status/">"#,
    )
    .unwrap();
    let bad = docs(
        &site,
        &["provenance", "--site", site_arg, "--commit", "e86f43d"],
    );
    assert_eq!(bad.status.code(), Some(1), "{}", describe(&bad));
    let bound = docs(
        &site,
        &["provenance", "--site", site_arg, "--commit", commit],
    );
    assert!(bound.status.success(), "{}", describe(&bound));

    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(site.join(".well-known/b10x-site.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["commit"], commit);
    assert_eq!(manifest["baseUrl"], "/conductor/");
    assert_eq!(manifest["schema"], "b10x-project-site/v1");
    let routes: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(site.join(".well-known/b10x-routes.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(routes["schema"], "b10x-project-routes/v1");
    assert_eq!(routes["commit"], commit);
    let paths: Vec<&str> = routes["routes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|route| route["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        paths,
        [
            "/conductor/",
            "/conductor/docs/",
            "/conductor/docs/reference/cli/"
        ]
    );
    assert_eq!(
        routes["routes"][2]["anchors"][0],
        "conductor-config-validate"
    );
    assert!(site.join(".nojekyll").is_file());
    fs::remove_dir_all(site).unwrap();
}
