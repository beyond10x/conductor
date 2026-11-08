//! Adversary cases for `story:spec-v1`: `docs/config.md` ("Removed in `conductor.config/2`")
//! says a `conductor.config/2` file that writes `dashboard_port` or `confinement` is refused by
//! its YAML path, naming why. Every run sets `HOME` to its case directory (`common::conductor`).

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

fn validate(name: &str, extra: &str) -> (Option<i32>, String, String) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary_config_v2_removed")
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the case");
    }
    fs::create_dir_all(&root).expect("create the case");
    let file = root.join("conductor.yaml");
    fs::write(
        &file,
        format!(
            "version: conductor.config/2\n\
             daemon: {{port: 7400, store: \"sqlite:~/conductor.sqlite\"}}\n\
             globals: {{}}\n\
             instances:\n\
             \x20 - name: one\n\
             \x20   session_prefix: one\n\
             \x20   placement: example-box\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: ~/one, trees: ~/one-trees}}\n\
             {extra}"
        ),
    )
    .expect("write the config");
    let output = common::conductor(root.join("home"))
        .current_dir(&root)
        .arg("--config")
        .arg(&file)
        .args(["config", "validate"])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn the_v2_file_itself_validates() {
    let (code, out, err) = validate("plain", "");
    assert_eq!(code, Some(0), "stdout {out:?} stderr {err:?}");
}

#[test]
fn a_v2_file_writing_dashboard_port_is_refused_by_its_yaml_path() {
    let (code, out, err) = validate("dashboard-port", "\x20   dashboard_port: 7400\n");
    assert_eq!(code, Some(1), "stdout {out:?} stderr {err:?}");
    assert!(
        format!("{out}{err}").contains("instances[0].dashboard_port"),
        "stdout {out:?} stderr {err:?}"
    );
}

#[test]
fn a_v2_file_writing_confinement_is_refused_by_its_yaml_path() {
    let (code, out, err) = validate("confinement", "\x20   confinement: {mode: report}\n");
    assert_eq!(code, Some(1), "stdout {out:?} stderr {err:?}");
    assert!(
        format!("{out}{err}").contains("instances[0].confinement"),
        "stdout {out:?} stderr {err:?}"
    );
}
