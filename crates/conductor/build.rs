//! Carries the product repository's own pins into the binary (`story:doctor-minimum-versions`):
//! the `aep`, `ess` and `worktree` lines of the `[pins]` table of the workspace's `b10x.toml`,
//! as `CONDUCTOR_BUILD_PINS`, `name=spec` pairs separated by `,`. `conductor doctor` compares
//! with them for each of those programs that the `b10x.toml` it finds does not pin. A build
//! without that file, such as a packaged crate, carries none.
//!
//! It also carries the target the binary is built for, as `CONDUCTOR_BUILD_TARGET`
//! (`story:conductor-upgrade`).

use std::env;
use std::fs;
use std::path::PathBuf;

/// The programs whose pins the build carries.
const PINNED: [&str; 3] = ["aep", "ess", "worktree"];

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let file = manifest.join("../../b10x.toml");
    println!("cargo:rerun-if-changed=build.rs");
    // The dashboard's built app is embedded with include_dir!; a new or changed file under dist/
    // rebuilds the crate.
    println!("cargo:rerun-if-changed=dashboard/dist");
    // Watched whether or not it is there yet, so a b10x.toml added after a build is carried by
    // the next one.
    println!("cargo:rerun-if-changed={}", file.display());
    let pins = match fs::read_to_string(&file) {
        Ok(text) => pins(&text),
        Err(_) => Vec::new(),
    };
    println!("cargo:rustc-env=CONDUCTOR_BUILD_PINS={}", pins.join(","));
    // The target the binary is built for, whose release archive `conductor upgrade` installs
    // (`story:conductor-upgrade`). Cargo sets `TARGET` for every build script.
    println!(
        "cargo:rustc-env=CONDUCTOR_BUILD_TARGET={}",
        env::var("TARGET").unwrap_or_default()
    );
}

/// The `name=spec` pairs of the `[pins]` lines `name = "spec"` of `text` for the programs of
/// [`PINNED`], comments aside, in the file's order.
fn pins(text: &str) -> Vec<String> {
    let mut pins = Vec::new();
    let mut in_pins = false;
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') {
            in_pins = line == "[pins]";
            continue;
        }
        if !in_pins {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_matches('"');
        let value = value.trim().trim_matches('"');
        let plain = |text: &str| {
            !text.is_empty()
                && text
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        };
        if PINNED.contains(&name) && plain(value) {
            pins.push(format!("{name}={value}"));
        }
    }
    pins
}
