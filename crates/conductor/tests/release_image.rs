//! `story:release-container-image`: each release publishes a container image of conductor.
//!
//! - `.github/workflows/release.yml` has a job `image` that builds a linux/amd64 image from the
//!   `x86_64-unknown-linux-gnu` archive the `package` job of the same run uploaded, never from a
//!   second compile, and pushes it to `ghcr.io/<the repository>` under the version without its
//!   `v`, and under `latest`. A manual dispatch with a tag runs it too.
//! - That job alone may write packages, and it may write nothing of the repository: it runs a
//!   build that downloads installers. The workflow's own permission and the `release` job's stay
//!   what they were.
//! - A third job, `image-notes`, holds `contents: write` and nothing else, checks nothing out and
//!   builds nothing. It takes the image's reference and digest from the outputs of `image` and
//!   writes one section `## Container image` into the Release's notes: it keeps the body that is
//!   there and replaces its own section on a re-run. Its script is run here against a `gh` that
//!   keeps the body in a file.
//! - A tag whose commit holds no `Dockerfile` gets no image: `image` ends with a notice before
//!   it builds, and `image-notes` is skipped. The build step's script is run here for that.
//! - The `Dockerfile` at the repository root takes the binary from the build context, runs as a
//!   user that is not root and has a home directory outside `/home`, has no entrypoint, and holds every program
//!   of [`conductor_cli::doctor::PROGRAMS`] but the substrate daemon: `aep`, `ess` and `worktree`
//!   at the pins of `b10x.toml` and the Gates CLI by the commands `conductor doctor` prints as
//!   `install:`.
//! - `website/docs/guides/run-in-a-container.md` documents the `docker run` line, and
//!   `AGENTS.md` § Releases says what the workflow writes and how a release verifies it.
//!
//! Every file is read relative to the workspace root. No case runs `docker`, a registry or the
//! `conductor` binary.

mod scratch;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use conductor_cli::doctor::{self, GATES, PROGRAMS, Pin, SUBSTRATE_DAEMON};
use serde_yaml::Value;

const WORKFLOW: &str = ".github/workflows/release.yml";
const DOCKERFILE: &str = "Dockerfile";
const GUIDE: &str = "website/docs/guides/run-in-a-container.md";

/// The one target the image is built for, and the archive of it the `package` job uploads.
const TARGET: &str = "x86_64-unknown-linux-gnu";

/// The heading of the section the image job owns in the Release's notes.
const SECTION: &str = "## Container image";

/// The programs of [`PROGRAMS`] the image leaves out, each with why: the substrate daemon is
/// needed only by an instance whose placement opens a box, which a forge source does not ask for.
const LEFT_OUT: [&str; 1] = [SUBSTRATE_DAEMON];

/// The home directory of the image's user. It is not under [`PERSONAL`]: the repository's commit
/// gate refuses a file that spells a directory there, whoever it belongs to.
const HOME: &str = "/var/lib/conductor";

/// Where a person's directories are, which no file of the image names.
const PERSONAL: &str = "/home/";

/// The programs the image takes from the base distribution's packages.
const PACKAGED: [&str; 2] = ["git", "jq"];

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

fn workflow() -> Value {
    serde_yaml::from_str(&read(WORKFLOW)).expect("the release workflow is YAML")
}

fn job<'w>(workflow: &'w Value, name: &str) -> &'w Value {
    workflow
        .get("jobs")
        .and_then(|jobs| jobs.get(name))
        .unwrap_or_else(|| panic!("{WORKFLOW}: no job `{name}`"))
}

fn steps(job: &Value) -> Vec<&Value> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .map(|steps| steps.iter().collect())
        .unwrap_or_default()
}

fn text<'v>(value: &'v Value, key: &str) -> &'v str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// Every `run:` script of `job`, in order.
fn scripts(job: &Value) -> Vec<&str> {
    steps(job)
        .into_iter()
        .map(|step| text(step, "run"))
        .filter(|run| !run.is_empty())
        .collect()
}

/// The `run:` script of the step of `job` named `name`.
fn script<'w>(job: &'w Value, name: &str) -> &'w str {
    steps(job)
        .into_iter()
        .find(|step| text(step, "name") == name)
        .map(|step| text(step, "run"))
        .unwrap_or_else(|| panic!("{WORKFLOW}: the image job has no step `{name}`"))
}

/// The names in `needs:` of `job`.
fn needs(job: &Value) -> Vec<&str> {
    match job.get("needs") {
        Some(Value::String(one)) => vec![one.as_str()],
        Some(Value::Sequence(many)) => many.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// `permissions:` of `value` as sorted `scope: access` pairs; `None` when it has none.
fn permissions(value: &Value) -> Option<Vec<(String, String)>> {
    let mut pairs: Vec<(String, String)> = value
        .get("permissions")?
        .as_mapping()
        .expect("permissions are a mapping")
        .iter()
        .map(|(scope, access)| {
            (
                scope.as_str().unwrap_or_default().to_owned(),
                access.as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    pairs.sort();
    Some(pairs)
}

fn pairs(of: &[(&str, &str)]) -> Option<Vec<(String, String)>> {
    Some(
        of.iter()
            .map(|(scope, access)| ((*scope).to_owned(), (*access).to_owned()))
            .collect(),
    )
}

/// The registry path of the image: `ghcr.io/` and the owner and name of the repository the
/// workspace's `Cargo.toml` names.
fn image() -> String {
    let manifest = read("Cargo.toml");
    let repository = manifest
        .lines()
        .find_map(|line| line.strip_prefix("repository = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("the workspace names its repository");
    let path = repository
        .strip_prefix("https://github.com/")
        .expect("the repository is on GitHub");
    format!("ghcr.io/{path}")
}

/// The `Dockerfile` with every continued line joined and every run of blanks one space.
fn dockerfile() -> String {
    read(DOCKERFILE)
        .replace("\\\n", " ")
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The instructions of the `Dockerfile` that start with `keyword`, each without it.
fn instructions(dockerfile: &str, keyword: &str) -> Vec<String> {
    dockerfile
        .lines()
        .filter_map(|line| line.strip_prefix(keyword))
        .filter_map(|rest| rest.strip_prefix(' '))
        .map(str::to_owned)
        .collect()
}

/// The default of the build argument `name`.
fn argument(dockerfile: &str, name: &str) -> String {
    instructions(dockerfile, "ARG")
        .into_iter()
        .find_map(|argument| {
            argument
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('='))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| panic!("{DOCKERFILE}: no `ARG {name}=<version>`"))
}

/// The programs the image must hold: every program doctor checks, but those of [`LEFT_OUT`].
fn programs() -> Vec<&'static str> {
    PROGRAMS
        .into_iter()
        .filter(|program| !LEFT_OUT.contains(program))
        .collect()
}

#[test]
fn the_image_job_builds_linux_amd64_from_the_package_jobs_binary() {
    let workflow = workflow();
    let image = job(&workflow, "image");
    for needed in ["provenance", "package", "release"] {
        assert!(
            needs(image).contains(&needed),
            "{WORKFLOW}: the image job needs `{needed}`"
        );
    }

    let package = job(&workflow, "package");
    let targets: Vec<&str> = package
        .get("strategy")
        .and_then(|strategy| strategy.get("matrix"))
        .and_then(|matrix| matrix.get("include"))
        .and_then(Value::as_sequence)
        .expect("the package job's matrix")
        .iter()
        .map(|entry| text(entry, "target"))
        .collect();
    assert!(targets.contains(&TARGET), "the package job builds {TARGET}");
    let uploaded: Vec<&str> = steps(package)
        .into_iter()
        .filter(|step| text(step, "uses").starts_with("actions/upload-artifact@"))
        .map(|step| text(step.get("with").expect("the upload's inputs"), "name"))
        .collect();
    assert_eq!(uploaded, ["release-${{ matrix.target }}"]);

    let downloaded: Vec<&str> = steps(image)
        .into_iter()
        .filter(|step| text(step, "uses").starts_with("actions/download-artifact@"))
        .map(|step| text(step.get("with").expect("the download's inputs"), "name"))
        .collect();
    assert_eq!(
        downloaded,
        [format!("release-{TARGET}")],
        "{WORKFLOW}: the image job takes the one archive the package job built for {TARGET}"
    );

    let checkouts: Vec<&str> = steps(image)
        .into_iter()
        .filter(|step| text(step, "uses").starts_with("actions/checkout@"))
        .map(|step| text(step.get("with").expect("the checkout's inputs"), "ref"))
        .collect();
    assert_eq!(
        checkouts,
        ["${{ needs.provenance.outputs.commit }}"],
        "{WORKFLOW}: the image job reads the tag's own commit"
    );

    for step in steps(image) {
        assert!(
            !text(step, "uses").contains("rust-toolchain"),
            "{WORKFLOW}: the image job installs no Rust toolchain"
        );
    }
    let scripts = scripts(image);
    for run in &scripts {
        assert!(
            !run.split(|c: char| !c.is_alphanumeric())
                .any(|word| word == "cargo" || word == "rustc"),
            "{WORKFLOW}: the image job compiles nothing"
        );
    }
    let build = scripts
        .iter()
        .find(|run| run.contains("docker build"))
        .expect("the image job runs `docker build`");
    assert!(
        build.contains("--platform linux/amd64"),
        "{WORKFLOW}: the image is built for linux/amd64"
    );
    assert!(
        build.contains(&format!("conductor-$VERSION-{TARGET}")),
        "{WORKFLOW}: the build context holds the binary of the {TARGET} archive"
    );
}

#[test]
fn the_image_job_alone_may_write_packages() {
    let workflow = workflow();
    assert_eq!(
        permissions(&workflow),
        pairs(&[("contents", "read")]),
        "{WORKFLOW}: the workflow's own permission stays `contents: read`"
    );
    let jobs = workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("the workflow's jobs");
    let mut seen = Vec::new();
    for (name, job) in jobs {
        let name = name.as_str().expect("a job's name");
        seen.push(name);
        let expected = match name {
            "image" => pairs(&[("contents", "read"), ("packages", "write")]),
            "image-notes" | "release" => pairs(&[("contents", "write")]),
            _ => None,
        };
        assert_eq!(
            permissions(job),
            expected,
            "{WORKFLOW}: the permissions of the job `{name}`"
        );
    }
    for expected in ["provenance", "package", "release", "image", "image-notes"] {
        assert!(seen.contains(&expected), "{WORKFLOW}: no job `{expected}`");
    }
}

/// The access `job` holds on `scope`, `none` when its `permissions:` leave the scope out.
fn access(job: &Value, scope: &str) -> String {
    permissions(job)
        .expect("the job names its permissions")
        .into_iter()
        .find(|(held, _)| held == scope)
        .map_or_else(|| "none".to_owned(), |(_, access)| access)
}

#[test]
fn the_image_job_may_write_nothing_of_the_repository() {
    let workflow = workflow();
    let image = job(&workflow, "image");
    // It runs a build that downloads installers, so it gets no write on the repository.
    assert_eq!(access(image, "contents"), "read");
    assert_eq!(access(image, "packages"), "write");
    for run in scripts(image) {
        assert!(
            !run.contains("gh release edit") && !run.contains("--method PATCH"),
            "{WORKFLOW}: the image job writes no Release"
        );
    }
}

#[test]
fn the_notes_job_writes_the_release_and_nothing_else() {
    let workflow = workflow();
    let notes = job(&workflow, "image-notes");
    assert_eq!(access(notes, "contents"), "write");
    assert_eq!(access(notes, "packages"), "none");
    assert!(
        needs(notes).contains(&"image"),
        "{WORKFLOW}: the notes job runs after the image job"
    );

    // No action at all: no checkout of the tree, no artifact, no toolchain.
    let steps = steps(notes);
    assert_eq!(steps.len(), 1, "{WORKFLOW}: the notes job has one step");
    for step in &steps {
        assert_eq!(
            text(step, "uses"),
            "",
            "{WORKFLOW}: the notes job uses no action"
        );
    }
    // And no command but the ones that read the body, edit it and write it back.
    for run in scripts(notes) {
        for line in run.lines() {
            let command = line.split_whitespace().next().unwrap_or_default();
            assert!(
                ![
                    "docker", "cargo", "git", "curl", "tar", "bash", "sh", "eval"
                ]
                .contains(&command),
                "{WORKFLOW}: the notes job runs `{}`",
                line.trim()
            );
        }
    }

    // What it writes comes from the image job's outputs, which are the push step's.
    let env = notes.get("env").expect("the notes job's env");
    assert_eq!(text(env, "TAG"), "${{ needs.provenance.outputs.tag }}");
    assert_eq!(
        text(env, "REFERENCE"),
        "${{ needs.image.outputs.reference }}"
    );
    assert_eq!(text(env, "DIGEST"), "${{ needs.image.outputs.digest }}");
    let outputs = job(&workflow, "image")
        .get("outputs")
        .expect("the image job's outputs");
    assert_eq!(
        text(outputs, "reference"),
        "${{ steps.push.outputs.reference }}"
    );
    assert_eq!(text(outputs, "digest"), "${{ steps.push.outputs.digest }}");
    let push = script(job(&workflow, "image"), "Push the image");
    assert!(
        push.contains("echo \"reference=$IMAGE:$VERSION\" >> \"$GITHUB_OUTPUT\""),
        "{WORKFLOW}: the push step hands on the reference it pushed"
    );
}

#[test]
fn every_action_is_pinned_by_commit() {
    let workflow = workflow();
    let jobs = workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("the workflow's jobs");
    let mut count = 0;
    for (name, job) in jobs {
        for step in steps(job) {
            let uses = text(step, "uses");
            if uses.is_empty() {
                continue;
            }
            count += 1;
            let commit = uses.rsplit_once('@').map_or("", |(_, commit)| commit);
            assert!(
                commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()),
                "{WORKFLOW}: job {name:?}: `{uses}` is not pinned by commit"
            );
        }
    }
    assert!(count >= 7, "{WORKFLOW}: {count} actions read");
}

#[test]
fn the_image_is_pushed_under_the_version_and_latest() {
    let workflow = workflow();
    let provenance = scripts(job(&workflow, "provenance")).join("\n");
    assert!(
        provenance.contains("version=\"${TAG#v}\""),
        "{WORKFLOW}: the version is the tag without its `v`"
    );

    let image = job(&workflow, "image");
    let env = image.get("env").expect("the image job's env");
    assert_eq!(text(env, "IMAGE"), "ghcr.io/${{ github.repository }}");
    assert_eq!(
        text(env, "VERSION"),
        "${{ needs.provenance.outputs.version }}"
    );
    assert_eq!(text(env, "TAG"), "${{ needs.provenance.outputs.tag }}");

    let push = script(image, "Push the image");
    for line in [
        "docker login ghcr.io",
        "docker push \"$IMAGE:$VERSION\"",
        "docker push \"$IMAGE:latest\"",
        "docker logout ghcr.io",
    ] {
        assert!(push.contains(line), "{WORKFLOW}: the push runs `{line}`");
    }
    assert!(
        push.contains("releases/latest"),
        "{WORKFLOW}: `latest` moves only for the newest release, so a backfill leaves it"
    );
    assert!(
        push.contains("digest=") && push.contains("sha256:[0-9a-f]{64}"),
        "{WORKFLOW}: the push step hands on the digest the registry answered"
    );
}

#[test]
fn a_dispatch_with_a_tag_publishes_the_image_too() {
    let workflow = workflow();
    let on = workflow.get("on").expect("the workflow's triggers");
    let tags: Vec<&str> = on
        .get("push")
        .and_then(|push| push.get("tags"))
        .and_then(Value::as_sequence)
        .expect("the tag trigger")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(tags, ["v*"]);
    let tag = on
        .get("workflow_dispatch")
        .and_then(|dispatch| dispatch.get("inputs"))
        .and_then(|inputs| inputs.get("tag"))
        .expect("the dispatch takes a tag");
    assert_eq!(tag.get("required").and_then(Value::as_bool), Some(true));
    assert!(
        job(&workflow, "image").get("if").is_none(),
        "{WORKFLOW}: the image job runs on every event the workflow runs on"
    );
}

#[test]
fn the_image_is_run_before_the_push_and_pulled_without_a_login_after_it() {
    let workflow = workflow();
    let image = job(&workflow, "image");
    let names: Vec<&str> = steps(image)
        .into_iter()
        .map(|step| text(step, "name"))
        .filter(|name| !name.is_empty())
        .collect();
    let at = |name: &str| {
        names
            .iter()
            .position(|step| *step == name)
            .unwrap_or_else(|| panic!("{WORKFLOW}: the image job has no step `{name}`"))
    };
    assert!(at("Run the image") < at("Push the image"));
    assert!(at("Push the image") < at("Pull the image without a login"));
    // The notes are written by a job that needs this one, so only once the pull has passed.
    assert!(needs(job(&workflow, "image-notes")).contains(&"image"));

    let smoke = script(image, "Run the image");
    assert!(
        smoke.contains("conductor --version") && smoke.contains("\"conductor $VERSION\""),
        "{WORKFLOW}: the image's `conductor --version` is held to the release's version"
    );
    assert!(
        smoke.contains("id -u"),
        "{WORKFLOW}: the image's user is held to one that is not root"
    );
    let words: Vec<&str> = smoke.split_whitespace().collect();
    for program in programs() {
        assert!(
            words.contains(&program),
            "{WORKFLOW}: the smoke run starts `{program}` in the image"
        );
    }

    let pull = script(image, "Pull the image without a login");
    assert!(
        pull.contains("docker pull \"$IMAGE@$DIGEST\""),
        "{WORKFLOW}: the pushed digest is pulled once the job has logged out"
    );
}

/// A `gh` that answers `gh release view <tag> --json body --jq .body` and
/// `gh release edit <tag> --notes-file <file>` over the one file `$BODY`, for the tag
/// `v$EXPECTED`, and fails on any other call.
const FAKE_GH: &str = "#!/bin/sh\n\
    case \"$1 $2 $3\" in\n\
      \"release view v$EXPECTED\") cat \"$BODY\" ;;\n\
      \"release edit v$EXPECTED\")\n\
        [ \"$4\" = --notes-file ] || exit 2\n\
        cp \"$5\" \"$BODY\" ;;\n\
      *) echo \"unexpected: gh $*\" >&2; exit 2 ;;\n\
    esac\n";

/// A `docker` that only writes each call down, a line of `$CALLS`.
const FAKE_DOCKER: &str = "#!/bin/sh\necho \"$*\" >> \"$CALLS\"\n";

/// Runs `run`, a step's script, as the runner does, with bash, in `dir/work`: its environment is
/// `variables` and a `PATH` that starts with `dir/bin`, where each of `programs` stands in for
/// the program of its name.
fn step_run(
    dir: &Path,
    run: &str,
    programs: &[(&str, &str)],
    variables: &[(&str, String)],
) -> std::process::Output {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("the stand-ins' directory");
    for (name, text) in programs {
        let program = bin.join(name);
        fs::write(&program, text).expect("write a stand-in");
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }
    let work = dir.join("work");
    fs::create_dir_all(&work).expect("the step's working directory");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").expect("a PATH")
    );
    let mut command = Command::new("bash");
    command
        .arg("-c")
        .arg(run)
        .current_dir(&work)
        .env_clear()
        .env("PATH", path)
        .env("HOME", dir);
    for (name, value) in variables {
        command.env(name, value);
    }
    command.output().expect("run the step with bash")
}

/// Runs the notes job's script in `dir` with a `gh` that reads and writes the Release's body in
/// `dir/body`.
fn notes_run(dir: &Path, run: &str, version: &str, digest: &str) -> std::process::Output {
    step_run(
        dir,
        run,
        &[("gh", FAKE_GH)],
        &[
            ("BODY", dir.join("body").display().to_string()),
            ("EXPECTED", version.to_owned()),
            ("GH_TOKEN", "none".to_owned()),
            ("GH_REPO", "example-org/conductor".to_owned()),
            ("TAG", format!("v{version}")),
            (
                "REFERENCE",
                format!("ghcr.io/example-org/conductor:{version}"),
            ),
            ("DIGEST", digest.to_owned()),
        ],
    )
}

/// The body of the Release in `dir` after the notes job's script ran there and succeeded.
fn noted(dir: &Path, run: &str, version: &str, digest: &str) -> String {
    let output = notes_run(dir, run, version, digest);
    assert!(
        output.status.success(),
        "the notes step failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(dir.join("body")).expect("the body afterwards")
}

#[test]
fn a_tag_without_a_dockerfile_gets_no_image_and_no_notes() {
    let workflow = workflow();
    let image = job(&workflow, "image");

    // The build step, run where the checkout holds no Dockerfile: a notice, the output
    // `image=none`, exit 0, and no call of docker.
    let case = scratch::Scratch::new("release-image-no-dockerfile");
    let output_file = case.path().join("output");
    let calls = case.path().join("calls");
    let output = step_run(
        case.path(),
        script(image, "Build the image from the packaged binary"),
        &[("docker", FAKE_DOCKER)],
        &[
            ("GITHUB_OUTPUT", output_file.display().to_string()),
            ("CALLS", calls.display().to_string()),
            ("IMAGE", "ghcr.io/example-org/conductor".to_owned()),
            ("TAG", "v1.2.0".to_owned()),
            ("VERSION", "1.2.0".to_owned()),
            ("COMMIT", "0".repeat(40)),
        ],
    );
    assert!(
        output.status.success(),
        "the build step failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("::notice::v1.2.0 "),
        "the build step says the tag gets no image"
    );
    assert_eq!(
        fs::read_to_string(&output_file).expect("the step's outputs"),
        "image=none\n"
    );
    assert!(!calls.exists(), "the build step called docker");

    // That output is the job's, every later step of the job waits on it, and the notes job is
    // skipped unless it reads `built`.
    let outputs = image.get("outputs").expect("the image job's outputs");
    assert_eq!(text(outputs, "image"), "${{ steps.build.outputs.image }}");
    let mut later = 0;
    for step in steps(image)
        .into_iter()
        .skip_while(|step| text(step, "id") != "build")
        .skip(1)
    {
        later += 1;
        assert_eq!(
            text(step, "if"),
            "${{ steps.build.outputs.image == 'built' }}",
            "{WORKFLOW}: the step `{}` runs only when an image was built",
            text(step, "name")
        );
    }
    assert_eq!(later, 3, "{WORKFLOW}: run, push and pull follow the build");
    assert_eq!(
        text(job(&workflow, "image-notes"), "if"),
        "${{ needs.image.outputs.image == 'built' }}",
        "{WORKFLOW}: the notes job is skipped when no image was built"
    );

    // Should it run all the same with no digest, it writes nothing.
    let notes = script(job(&workflow, "image-notes"), "Write the section");
    let case = scratch::Scratch::new("release-image-no-digest");
    fs::write(case.path().join("body"), "- a story\n").expect("the written body");
    let output = notes_run(case.path(), notes, "1.2.0", "");
    assert!(
        !output.status.success(),
        "the notes step ran with no digest"
    );
    assert_eq!(
        fs::read_to_string(case.path().join("body")).expect("the body afterwards"),
        "- a story\n"
    );
}

#[test]
fn the_release_notes_name_the_image_by_tag_and_digest_once() {
    let workflow = workflow();
    let run = script(job(&workflow, "image-notes"), "Write the section");
    let first = format!("sha256:{}", "a".repeat(64));
    let second = format!("sha256:{}", "b".repeat(64));
    let section = |digest: &str| {
        format!(
            "{SECTION}\n\n`ghcr.io/example-org/conductor:1.4.0` at digest `{digest}`\n\n\
             ```console\ndocker run --rm ghcr.io/example-org/conductor:1.4.0 conductor --version\n```\n"
        )
    };

    // The body a human wrote stays, and the section comes after it.
    let case = scratch::Scratch::new("release-image-notes");
    let written = "- a story\n- another story\n";
    fs::write(case.path().join("body"), written).expect("the written body");
    let body = noted(case.path(), run, "1.4.0", &first);
    assert_eq!(body, format!("{written}\n{}", section(&first)));

    // A re-run replaces the section: one heading, the new digest, the body unchanged.
    let body = noted(case.path(), run, "1.4.0", &second);
    assert_eq!(body, format!("{written}\n{}", section(&second)));
    assert_eq!(body.matches(SECTION).count(), 1);

    // What a human wrote after the section stays too, and so do its line ends.
    let around = format!(
        "- a story\r\n\r\n{SECTION}\r\n\r\nan older line\r\n\r\n## Upgrade notes\r\n\r\nread these\r\n"
    );
    fs::write(case.path().join("body"), around).expect("a body around the section");
    let body = noted(case.path(), run, "1.4.0", &first);
    assert_eq!(
        body,
        format!(
            "- a story\r\n\r\n## Upgrade notes\r\n\r\nread these\r\n\n{}",
            section(&first)
        )
    );

    // A Release with no notes gets the section alone.
    fs::write(case.path().join("body"), "\n").expect("an empty body");
    let body = noted(case.path(), run, "1.4.0", &first);
    assert_eq!(body, section(&first));
}

#[test]
fn the_dockerfile_takes_the_binary_from_the_build_context() {
    let dockerfile = dockerfile();
    assert_eq!(
        instructions(&dockerfile, "FROM").len(),
        1,
        "{DOCKERFILE}: one stage, so nothing is built in another"
    );
    let copies = instructions(&dockerfile, "COPY");
    assert_eq!(
        copies,
        ["--chmod=755 conductor /usr/local/bin/conductor"],
        "{DOCKERFILE}: the binary comes from the build context, onto PATH"
    );
    assert!(
        instructions(&dockerfile, "ADD").is_empty(),
        "{DOCKERFILE}: nothing is added from a URL unchecked"
    );
    for run in instructions(&dockerfile, "RUN") {
        assert!(
            !run.split(|c: char| !c.is_alphanumeric())
                .any(|word| word == "cargo" || word == "rustc"),
            "{DOCKERFILE}: the image compiles nothing"
        );
    }
    assert!(
        instructions(&dockerfile, "ENTRYPOINT").is_empty(),
        "{DOCKERFILE}: no entrypoint, so `docker run <image> conductor --version` runs conductor"
    );
}

#[test]
fn the_dockerfile_runs_as_a_user_that_is_not_root_with_a_home() {
    let dockerfile = dockerfile();
    let users = instructions(&dockerfile, "USER");
    let user = users.last().expect("the Dockerfile names its user");
    let (uid, gid) = user.split_once(':').unwrap_or((user.as_str(), ""));
    // A number, so a cluster that requires a non-root user can check it without the image's
    // passwd file.
    let uid: u32 = uid
        .parse()
        .unwrap_or_else(|_| panic!("{DOCKERFILE}: `USER {user}` names the user by number"));
    assert_ne!(uid, 0, "{DOCKERFILE}: the image's user is not root");
    assert_ne!(
        gid.parse::<u32>().ok(),
        Some(0),
        "{DOCKERFILE}: its group is not root's"
    );

    let created = instructions(&dockerfile, "RUN")
        .into_iter()
        .find(|run| run.contains("useradd"))
        .expect("the Dockerfile creates the user");
    assert!(
        created.contains(&format!("--uid {uid}")) && created.contains("--create-home"),
        "{DOCKERFILE}: the user {uid} is created with a home directory"
    );
    let home = created
        .split_whitespace()
        .skip_while(|word| *word != "--home-dir")
        .nth(1)
        .expect("the user's home directory");
    assert_eq!(
        home, HOME,
        "{DOCKERFILE}: the user's home directory, which is not under {PERSONAL}"
    );
    let env = instructions(&dockerfile, "ENV").join(" ");
    assert!(
        env.split_whitespace()
            .any(|pair| pair == format!("HOME={home}")),
        "{DOCKERFILE}: HOME names {home}"
    );
    // The harness's installer puts `claude` into `.local/bin` of the home directory.
    assert!(
        env.split_whitespace()
            .any(|pair| pair.starts_with(&format!("PATH={home}/.local/bin:"))),
        "{DOCKERFILE}: PATH starts with {home}/.local/bin"
    );
    assert_eq!(
        instructions(&dockerfile, "WORKDIR")
            .last()
            .map(String::as_str),
        Some(home),
        "{DOCKERFILE}: the image starts in the user's home"
    );
}

#[test]
fn no_file_of_the_image_spells_a_personal_directory() {
    for file in [DOCKERFILE, WORKFLOW, GUIDE] {
        for (at, line) in read(file).lines().enumerate() {
            assert!(
                !line.contains(PERSONAL),
                "{file}:{}: a path under {PERSONAL}, which the commit gate refuses",
                at + 1
            );
        }
    }
    let guide = read(GUIDE);
    assert!(
        guide.contains(&format!("`{HOME}`")),
        "{GUIDE}: names the user's home directory, {HOME}"
    );
}

#[test]
fn the_dockerfile_holds_every_program_doctor_checks_for_a_forge_source() {
    let dockerfile = dockerfile();
    let runs = instructions(&dockerfile, "RUN");
    let pin_file = workspace().join("b10x.toml");
    let pins = doctor::pins(&read("b10x.toml"), &pin_file).expect("the repository's pins read");
    let user_at = dockerfile
        .find("\nUSER ")
        .expect("the Dockerfile names its user");

    for program in programs() {
        if PACKAGED.contains(&program) {
            let install = runs
                .iter()
                .find(|run| run.contains("apt-get install"))
                .expect("the Dockerfile installs packages");
            assert!(
                install.split_whitespace().any(|word| word == program),
                "{DOCKERFILE}: `{program}` is installed from the distribution"
            );
            continue;
        }
        if let Some((_, pin)) = pins.iter().find(|(name, _)| name == program) {
            // The release archive at the pin, by the command doctor prints as `install:`.
            let name = format!("{}_VERSION", program.to_uppercase());
            let version = argument(&dockerfile, &name);
            let triple = doctor::version_of(&version)
                .unwrap_or_else(|| panic!("{DOCKERFILE}: {name}={version} is no version"));
            let holds = match Pin::parse(pin).expect("a pin of b10x.toml parses") {
                Pin::Exact(major, minor, patch) => triple == (major, minor, patch),
                Pin::Line(major, minor) => (triple.0, triple.1) == (major, minor),
            };
            assert!(
                holds,
                "{DOCKERFILE}: {name}={version} is not at the pin {pin} of b10x.toml"
            );
            let command = doctor::install_command_for(program, Some(&version), None, TARGET)
                .expect("doctor knows the install")
                .replace(&version, &format!("${{{name}}}"))
                .replace("~/.local/bin", "/usr/local/bin");
            assert!(
                dockerfile.contains(&command),
                "{DOCKERFILE}: `{program}` is not installed by doctor's command:\n{command}"
            );
            continue;
        }
        if program == GATES {
            let command = doctor::install_command_for(program, None, None, TARGET)
                .expect("doctor knows the install")
                .replace("~/.local/bin", "/usr/local/bin");
            assert!(
                dockerfile.contains(&command),
                "{DOCKERFILE}: `{program}` is not installed by doctor's command:\n{command}"
            );
            continue;
        }
        match program {
            // The forges' CLIs: a release of the forge's own, checked against its checksums.
            "gh" => {
                let version = argument(&dockerfile, "GH_VERSION");
                assert!(
                    doctor::version_of(&version).is_some(),
                    "GH_VERSION={version}"
                );
                let base = "https://github.com/cli/cli/releases/download/v${GH_VERSION}";
                for part in [
                    format!("curl -fsSLO {base}/gh_${{GH_VERSION}}_linux_amd64.tar.gz"),
                    format!("curl -fsSLO {base}/gh_${{GH_VERSION}}_checksums.txt"),
                    "sha256sum --check --ignore-missing gh_${GH_VERSION}_checksums.txt".to_owned(),
                ] {
                    assert!(dockerfile.contains(&part), "{DOCKERFILE}: gh: `{part}`");
                }
            }
            "glab" => {
                let version = argument(&dockerfile, "GLAB_VERSION");
                assert!(
                    doctor::version_of(&version).is_some(),
                    "GLAB_VERSION={version}"
                );
                let base =
                    "https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads";
                for part in [
                    format!("curl -fsSLO {base}/glab_${{GLAB_VERSION}}_linux_amd64.tar.gz"),
                    format!("curl -fsSLO {base}/checksums.txt"),
                    "sha256sum --check --ignore-missing checksums.txt".to_owned(),
                ] {
                    assert!(dockerfile.contains(&part), "{DOCKERFILE}: glab: `{part}`");
                }
            }
            // The harness: its own installer, run as the image's user, so it lands in the home.
            "claude" => {
                let install = "https://claude.ai/install.sh";
                let at = dockerfile
                    .find(install)
                    .unwrap_or_else(|| panic!("{DOCKERFILE}: the harness comes from {install}"));
                assert!(
                    at > user_at,
                    "{DOCKERFILE}: the harness is installed as the image's user"
                );
            }
            other => panic!(
                "{DOCKERFILE}: doctor checks `{other}`, and this test knows no install of it: \
                 add it to the image, or to LEFT_OUT with why"
            ),
        }
    }

    // Each install is proven where it happens: the build fails on a program that does not start.
    for program in programs() {
        assert!(
            runs.iter()
                .any(|run| run.contains(&format!("{program} --version"))),
            "{DOCKERFILE}: the build runs `{program} --version`"
        );
    }
}

#[test]
fn the_documents_name_the_image_and_how_a_release_verifies_it() {
    let image = image();
    let guide = read(GUIDE);
    let run = format!("docker run --rm {image}:latest conductor --version");
    assert!(guide.contains(&run), "{GUIDE}: documents `{run}`");
    assert!(
        guide.contains(&format!("{image}:<version>")) && guide.contains(SECTION),
        "{GUIDE}: names the tag of one release and where its digest is"
    );
    assert!(
        read("website/sidebars.ts").contains("'guides/run-in-a-container'"),
        "website/sidebars.ts: lists the guide"
    );
    for (page, link) in [
        (
            "README.md",
            ".github.io/conductor/docs/guides/run-in-a-container)",
        ),
        (
            "website/docs/getting-started.md",
            "./guides/run-in-a-container.md",
        ),
    ] {
        let text = read(page);
        assert_eq!(
            text.matches("run-in-a-container").count(),
            1,
            "{page}: one link to the guide"
        );
        assert!(text.contains(link), "{page}: links {link}");
        assert!(
            !text.contains("docker run"),
            "{page}: the `docker run` line is on the guide only"
        );
    }

    let agents = read("AGENTS.md");
    let releases = agents
        .split("\n## Releases\n")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .expect("AGENTS.md has a Releases section");
    let flat = releases.split_whitespace().collect::<Vec<_>>().join(" ");
    for needed in [
        format!("`{image}:<version>`"),
        format!("`{SECTION}`"),
        format!("docker run --rm {image}:<version> conductor --version"),
        "lists five assets".to_owned(),
    ] {
        assert!(
            flat.contains(&needed),
            "AGENTS.md § Releases: names {needed}"
        );
    }
}
