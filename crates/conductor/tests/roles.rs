//! `story:roles-in-the-binary`: a role's profile is product data compiled into the `conductor`
//! binary (`crates/conductor/roles/<role>.md`), and `conductor role show --role <role>` prints
//! the text the binary carries. The `claude-code` kind's packaging (the settings each role starts
//! with) ships in the binary too (`crates/conductor/kinds/claude-code/`), so this repository has
//! no `.claude/` directory and `.agents/` holds no role profile.

mod common;

use std::fs;
use std::path::PathBuf;

use common::conductor;

/// The repository root.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The roles the binary carries a profile for, each with a name it also answers to: the v0 name
/// of the role (`conductor.config.RoleName`).
const ROLES: [(&str, &str); 4] = [
    ("maintainer", "conductor-dev"),
    ("supervisor", "supervisor"),
    ("teacher", "teacher"),
    ("controller", "lead"),
];

/// A case directory under the target's temporary directory, empty.
fn case(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("roles-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the case directory");
    dir
}

#[test]
fn the_role_profiles_live_under_crates_conductor_roles_one_per_role() {
    let dir = root().join("crates/conductor/roles");
    let mut found: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    found.sort();
    let mut expected: Vec<String> = ROLES.map(|(role, _)| format!("{role}.md")).to_vec();
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn role_show_prints_the_text_the_binary_carries() {
    let home = case("show");
    for (role, alias) in ROLES {
        let file = root().join(format!("crates/conductor/roles/{role}.md"));
        let text = fs::read_to_string(&file).expect("read the role's profile");
        for name in [role, alias] {
            let output = conductor(&home)
                .args(["role", "show", "--role", name])
                .output()
                .expect("run conductor role show");
            assert_eq!(
                output.status.code(),
                Some(0),
                "role show --role {name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                text,
                "role show --role {name} prints {}",
                file.display()
            );
        }
    }
    assert!(
        !home.join(".claude").exists() && !home.join(".b10x").exists(),
        "role show writes nothing under the home directory"
    );
}

#[test]
fn role_show_refuses_a_role_the_binary_carries_no_profile_for() {
    let home = case("show-unknown");
    let output = conductor(&home)
        .args(["role", "show", "--role", "planner-of-nothing"])
        .output()
        .expect("run conductor role show");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("planner-of-nothing"), "{stderr}");
}

#[test]
fn the_repository_has_no_claude_directory_and_no_role_profile_under_agents() {
    assert!(
        !root().join(".claude").exists(),
        "the repository holds a .claude/ directory: the claude-code kind writes what its harness \
         reads into the agent's own state at start"
    );
    for (role, _) in ROLES {
        for old in [
            "conductor-dev.md",
            "conductor-supervisor.md",
            "conductor-teacher.md",
            "repo-controller.md",
            &format!("{role}.md"),
        ] {
            assert!(
                !root().join(".agents").join(old).exists(),
                ".agents/{old} is a role profile; it lives under crates/conductor/roles/"
            );
        }
    }
    assert!(
        root().join("CLAUDE.md").is_file(),
        "the root CLAUDE.md, which points to AGENTS.md, stays"
    );
}

#[test]
fn the_claude_code_kind_s_settings_ship_in_the_binary_and_wire_the_guard() {
    let dir = root().join("crates/conductor/kinds/claude-code");
    for name in ["conductor-settings.json", "controller-settings.json"] {
        let text =
            fs::read_to_string(dir.join(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        let command = value["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap_or_default();
        assert!(
            command.starts_with("conductor guard record-guard-decision --from-pre-tool-use"),
            "{name} wires the guard: {text}"
        );
    }
}
