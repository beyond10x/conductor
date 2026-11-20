//! `story:supervisor-watch-forge-triggers` (issues #254, #247): the two forge triggers of
//! `conductor supervisor watch`. `GREEN` names an open request that is no draft and has no
//! auto-merge, green for over 15 min, once per head sha, unless `--hold` names it; `CIEND` names
//! the end of the checks a controller's newest report `blocked ci <ref>` waits on. The forge is
//! read at most every 2 min per instance, a refused read is one note and doubles the interval.
//!
//! Every case calls the library ([`Watch::pass`], [`Watch::commit`]) over its seam, [`Sources`],
//! with a clock the case sets, and fake `gh` and `glab` programs: `sh` scripts the case writes
//! into its own directory, which log their words and print the JSON fixtures the case writes
//! beside them. No real forge, no network, no token; the process's instance is fixed with
//! `active::isolate` first (AGENTS.md rule 5).

mod active;
mod scratch;

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use conductor_cli::config;
use conductor_cli::store;
use conductor_cli::supervisor_watch::{Pass, Sources, Watch};
use conductor_model::behaviour::ControllerStorage;
use conductor_model::config::{Config, SessionName};
use conductor_model::dispatch::{
    ControllerData, ControllerId, ControllerSnapshot, ControllerState,
};
use conductor_model::host::WatchSupervisor;
use conductor_model::observation::{Harness, RepositoryName};
use conductor_model::primitives::Uuid;
use scratch::Scratch;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const GITHUB: &str = "[{github: example-org}]";

const BOTH: &str = "[{github: example-org}, {gitlab: example-org, host: gitlab.example.com}]";

thread_local! {
    /// The case's clock: each case runs on a thread of its own.
    static NOW: Cell<i64> = const { Cell::new(0) };
}

fn at(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).expect("a fixed instant")
}

/// Sets the case's clock to `2026-10-07T<time>Z`.
fn set_clock(time: &str) {
    let now = at(&format!("2026-10-07T{time}Z"));
    NOW.with(|cell| cell.set(now.unix_timestamp()));
}

fn clock() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(NOW.with(Cell::get)).expect("a clock")
}

fn case(name: &str) -> Scratch {
    active::isolate();
    Scratch::new(Path::new("supervisor-watch-forge").join(name))
}

/// The config of the one instance `alpha` under `dir`, at `placement: none`, with the sources
/// `sources`.
fn parsed(dir: &Path, sources: &str) -> Config {
    let base = dir.join("alpha");
    let text = format!(
        "version: conductor.config/2\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: {sources}\n\
         \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
         \x20   records: {records:?}\n\
         \x20   state: {state:?}\n\
         \x20   cache: {cache:?}\n\
         \x20   thresholds: {{disk_path: {disk:?}, disk_low: 1G, disk_clear: 1G}}\n",
        root = base.join("root"),
        trees = base.join("trees"),
        records = base.join("records"),
        state = base.join("state"),
        cache = base.join("cache"),
        disk = base.join("disk"),
    );
    fs::create_dir_all(base.join("root")).expect("create the checkouts root");
    fs::write(base.join("disk"), format!("{}\n", 100u64 << 30)).expect("write the disk");
    config::parse(&text, &dir.join("home"))
        .unwrap_or_else(|problems| panic!("the case's config parses: {problems:?}"))
}

/// The free bytes the file at `path` holds as a number.
fn free_bytes(path: &Path) -> anyhow::Result<u64> {
    Ok(fs::read_to_string(path)?.trim().parse()?)
}

/// The case's fake forge: `fake-gh` and `fake-glab` under its directory.
struct Fake {
    dir: PathBuf,
}

impl Fake {
    /// Writes the fakes. `gh search prs` prints `gh-search.json`, `gh pr list -R <owner>/<repo>`
    /// prints `gh-pulls-<repo>.json`, each `[]` when absent; with `gh-fail-search` or
    /// `gh-fail-pr` there, that read prints the file on standard error and exits 1. `glab api`
    /// prints `glab-mrs.json` for the group's open merge requests, `glab-projects.json` for its
    /// projects and `glab-mr-<iid>.json` for one merge request. Both log their words to
    /// `gh.log` and `glab.log`.
    fn new(dir: &Path) -> Self {
        let d = dir.display();
        fs::write(
            dir.join("fake-gh"),
            format!(
                "d='{d}'\n\
                 printf '%s\\n' \"$*\" >> \"$d/gh.log\"\n\
                 case \"$1 $2\" in\n\
                 \x20 'search prs') kind=search; file=\"$d/gh-search.json\" ;;\n\
                 \x20 'pr list') kind=pr; repo=''; prev=''\n\
                 \x20   for a in \"$@\"; do [ \"$prev\" = -R ] && repo=\"$a\"; prev=\"$a\"; done\n\
                 \x20   file=\"$d/gh-pulls-${{repo##*/}}.json\" ;;\n\
                 \x20 *) exit 1 ;;\n\
                 esac\n\
                 if [ -f \"$d/gh-fail-$kind\" ]; then cat \"$d/gh-fail-$kind\" >&2; exit 1; fi\n\
                 if [ -f \"$file\" ]; then cat \"$file\"; else echo '[]'; fi\n"
            ),
        )
        .expect("write the fake gh");
        fs::write(
            dir.join("fake-glab"),
            format!(
                "d='{d}'\n\
                 printf '%s\\n' \"$*\" >> \"$d/glab.log\"\n\
                 for last in \"$@\"; do :; done\n\
                 case \"$last\" in\n\
                 \x20 groups/example-org/merge_requests\\?*) file=\"$d/glab-mrs.json\" ;;\n\
                 \x20 groups/example-org/projects\\?*) file=\"$d/glab-projects.json\" ;;\n\
                 \x20 projects/*/merge_requests/*) file=\"$d/glab-mr-${{last##*/}}.json\" ;;\n\
                 \x20 *) exit 1 ;;\n\
                 esac\n\
                 if [ -f \"$file\" ]; then cat \"$file\"; else echo '[]'; fi\n"
            ),
        )
        .expect("write the fake glab");
        Self {
            dir: dir.to_owned(),
        }
    }

    fn sources(&self) -> Sources {
        let program = |name: &str| vec!["sh".to_owned(), self.dir.join(name).display().to_string()];
        Sources {
            host: self.dir.join("host"),
            projects: self.dir.join("projects"),
            daemon_store: None,
            clock,
            free_bytes,
            interval: Duration::from_millis(5),
            git: PathBuf::from("git"),
            journal: vec!["sh".to_owned(), "-c".to_owned(), "exit 0".to_owned()],
            gh: program("fake-gh"),
            glab: program("fake-glab"),
        }
    }

    fn write(&self, name: &str, value: &Value) {
        fs::write(self.dir.join(name), value.to_string()).expect("write a fixture");
    }

    /// The open pull requests `gh search prs` lists, `(repository, number)` each.
    fn search(&self, found: &[(&str, u64)]) {
        let list: Vec<Value> = found
            .iter()
            .map(|(repository, number)| {
                json!({"number": number, "repository": {"name": repository,
                       "nameWithOwner": format!("example-org/{repository}")}})
            })
            .collect();
        self.write("gh-search.json", &Value::from(list));
    }

    /// The open pull requests of `repository`, as `gh pr list --json` prints them.
    fn pulls(&self, repository: &str, pulls: &[Value]) {
        self.write(
            &format!("gh-pulls-{repository}.json"),
            &Value::from(pulls.to_vec()),
        );
    }

    /// The lines of a log that hold `words`.
    fn calls(&self, log: &str, words: &str) -> usize {
        fs::read_to_string(self.dir.join(log))
            .unwrap_or_default()
            .lines()
            .filter(|line| line.contains(words))
            .count()
    }
}

/// One pull request with a head sha, a draft flag, an auto-merge flag and a check rollup.
fn pull(number: u64, head: &str, draft: bool, auto_merge: bool, checks: &[Value]) -> Value {
    json!({
        "number": number,
        "headRefOid": head,
        "isDraft": draft,
        "autoMergeRequest": if auto_merge { json!({"mergeMethod": "SQUASH"}) } else { Value::Null },
        "statusCheckRollup": checks,
    })
}

/// A check run that completed with `conclusion` at `2026-10-07T<time>Z`.
fn done(conclusion: &str, time: &str) -> Value {
    json!({"__typename": "CheckRun", "name": "check", "status": "COMPLETED",
           "conclusion": conclusion, "completedAt": format!("2026-10-07T{time}Z")})
}

/// A check run that still runs.
fn running_check() -> Value {
    json!({"__typename": "CheckRun", "name": "check", "status": "IN_PROGRESS",
           "conclusion": "", "completedAt": "0001-01-01T00:00:00Z"})
}

fn input() -> WatchSupervisor {
    WatchSupervisor {
        instance: Vec::new(),
        checkout: Vec::new(),
        timeout: None,
        hold: Vec::new(),
        format: None,
    }
}

fn watch(config: &Config, args: &WatchSupervisor, sources: Sources) -> Watch {
    Watch::new(config, args, Path::new("/"), sources).expect("the watch starts")
}

/// One pass: what it found, its lines of the triggers `kinds`, and its notes.
fn lines(watch: &mut Watch, kinds: &[&str]) -> (Pass, Vec<String>, String) {
    let mut notes = Vec::new();
    let pass = watch.pass(&mut notes).expect("a pass");
    let lines = pass
        .sorted()
        .iter()
        .map(|trigger| conductor_cli::supervisor_watch::render(trigger, false))
        .filter(|line| {
            kinds
                .iter()
                .any(|kind| line.starts_with(&format!("{kind} ")))
        })
        .collect();
    (pass, lines, String::from_utf8_lossy(&notes).into_owned())
}

/// One pass whose lines are printed: its cursors move.
fn printed(watch: &mut Watch, kinds: &[&str]) -> (Vec<String>, String) {
    let (pass, lines, notes) = lines(watch, kinds);
    watch.commit(&pass).expect("commit");
    (lines, notes)
}

// -------------------------------------------------------------------------------------------
// GREEN
// -------------------------------------------------------------------------------------------

/// A green pull request is named once per head sha: a new run reads it again and names nothing,
/// a new head green for 15 min is named again; a request `--hold` names is passed over until no
/// hold names it.
#[test]
fn green_names_an_unmerged_green_request_once_per_head_sha_and_skips_a_hold() {
    let scratch = case("green-once");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, GITHUB);
    let fake = Fake::new(&dir);
    fake.search(&[("app", 7), ("app", 11)]);
    fake.pulls(
        "app",
        &[
            pull(7, "aaa", false, false, &[done("SUCCESS", "11:40:00")]),
            pull(11, "ccc", false, false, &[done("SUCCESS", "11:30:00")]),
        ],
    );
    let mut held = input();
    held.hold = vec!["app#11".to_owned()];

    set_clock("12:00:00");
    let mut first = watch(&config, &held, fake.sources());
    let (found, notes) = printed(&mut first, &["GREEN"]);
    assert_eq!(found, ["GREEN alpha app#7 green 20m, head aaa"], "{notes}");

    set_clock("12:10:00");
    let mut second = watch(&config, &held, fake.sources());
    let (found, notes) = printed(&mut second, &["GREEN"]);
    assert_eq!(found, Vec::<String>::new(), "named once per head: {notes}");
    assert_eq!(
        fake.calls("gh.log", "pr list"),
        2,
        "the forge was read again"
    );

    fake.pulls(
        "app",
        &[
            pull(7, "bbb", false, false, &[done("SUCCESS", "12:05:00")]),
            pull(11, "ccc", false, false, &[done("SUCCESS", "11:30:00")]),
        ],
    );
    set_clock("12:31:00");
    let mut third = watch(&config, &held, fake.sources());
    let (found, _) = printed(&mut third, &["GREEN"]);
    assert_eq!(found, ["GREEN alpha app#7 green 26m, head bbb"]);

    set_clock("12:40:00");
    let mut fourth = watch(&config, &input(), fake.sources());
    let (found, _) = printed(&mut fourth, &["GREEN"]);
    assert_eq!(
        found,
        ["GREEN alpha app#11 green 70m, head ccc"],
        "without the hold, the held request is named"
    );
}

/// A draft, a request with auto-merge, one green for under 15 min, one whose checks still run
/// and a red one are no `GREEN`.
#[test]
fn green_skips_a_draft_and_an_auto_merge_request_and_one_green_under_fifteen_minutes() {
    let scratch = case("green-skips");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, GITHUB);
    let fake = Fake::new(&dir);
    fake.search(&[("app", 7), ("app", 8), ("app", 9), ("app", 10)]);
    fake.pulls(
        "app",
        &[
            pull(7, "aaa", false, false, &[done("SUCCESS", "11:40:00")]),
            pull(8, "a08", true, false, &[done("SUCCESS", "11:00:00")]),
            pull(9, "a09", false, true, &[done("SUCCESS", "11:00:00")]),
            pull(10, "a10", false, false, &[done("SUCCESS", "11:50:00")]),
            pull(
                12,
                "a12",
                false,
                false,
                &[done("SUCCESS", "11:00:00"), running_check()],
            ),
            pull(13, "a13", false, false, &[done("FAILURE", "11:00:00")]),
        ],
    );
    set_clock("12:00:00");
    let mut watch = watch(&config, &input(), fake.sources());
    let (found, notes) = printed(&mut watch, &["GREEN"]);
    assert_eq!(found, ["GREEN alpha app#7 green 20m, head aaa"], "{notes}");
}

/// A GitLab merge request is read through `glab api` (the group's open requests, its projects,
/// and each request's head pipeline) beside GitHub's pull requests through `gh`; a request's
/// detail is read again only while its pipeline runs or once its head changed.
#[test]
fn green_reads_a_gitlab_merge_request_and_a_github_pull_request() {
    let scratch = case("green-both");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, BOTH);
    let fake = Fake::new(&dir);
    fake.search(&[("app", 7)]);
    fake.pulls(
        "app",
        &[pull(7, "aaa", false, false, &[done("SUCCESS", "11:40:00")])],
    );
    fake.write(
        "glab-projects.json",
        &json!([{"id": 101, "path_with_namespace": "example-org/lib", "archived": false,
                 "visibility": "private", "default_branch": "main", "open_issues_count": 0,
                 "web_url": "https://gitlab.example.com/example-org/lib"}]),
    );
    let request = |iid: u64, sha: &str, draft: bool, auto: bool| {
        json!({"iid": iid, "project_id": 101, "sha": sha, "draft": draft,
               "merge_when_pipeline_succeeds": auto})
    };
    fake.write(
        "glab-mrs.json",
        &json!([
            request(3, "ddd", false, false),
            request(4, "d04", false, true),
            request(5, "d05", true, false),
            request(6, "d06", false, false),
        ]),
    );
    let detail = |iid: u64, sha: &str, status: &str, finished: Value| {
        json!({"iid": iid, "sha": sha, "head_pipeline": {"id": 900 + iid, "status": status,
               "sha": sha, "finished_at": finished}})
    };
    fake.write(
        "glab-mr-3.json",
        &detail(3, "ddd", "success", json!("2026-10-07T11:30:00Z")),
    );
    fake.write(
        "glab-mr-4.json",
        &detail(4, "d04", "success", json!("2026-10-07T11:00:00Z")),
    );
    fake.write(
        "glab-mr-5.json",
        &detail(5, "d05", "success", json!("2026-10-07T11:00:00Z")),
    );
    fake.write("glab-mr-6.json", &detail(6, "d06", "running", Value::Null));

    set_clock("12:00:00");
    let mut first = watch(&config, &input(), fake.sources());
    let (mut found, notes) = printed(&mut first, &["GREEN"]);
    found.sort();
    assert_eq!(
        found,
        [
            "GREEN alpha app#7 green 20m, head aaa",
            "GREEN alpha lib!3 green 30m, head ddd",
        ],
        "{notes}"
    );
    assert_eq!(fake.calls("glab.log", "--hostname gitlab.example.com"), 6);

    fake.write(
        "glab-mr-6.json",
        &detail(6, "d06", "success", json!("2026-10-07T12:01:00Z")),
    );
    set_clock("12:20:00");
    let mut second = watch(&config, &input(), fake.sources());
    let (found, _) = printed(&mut second, &["GREEN"]);
    assert_eq!(found, ["GREEN alpha lib!6 green 19m, head d06"]);
    assert_eq!(
        fake.calls("glab.log", "merge_requests/3"),
        1,
        "a request whose pipeline ended on its head is not read again"
    );
    assert_eq!(
        fake.calls("glab.log", "merge_requests/6"),
        2,
        "a request whose pipeline ran is read again"
    );
}

// -------------------------------------------------------------------------------------------
// CIEND
// -------------------------------------------------------------------------------------------

fn running(handle: &mut store::Store, id: &str, repository: &str) {
    ControllerStorage::put(
        handle,
        ControllerSnapshot {
            state: ControllerState::Running,
            data: ControllerData {
                actor: conductor_cli::actor::daemon(),
                controller_id: ControllerId(Uuid(id.to_owned())),
                repository: RepositoryName(repository.to_owned()),
                harness: Harness::Claude,
                session_name: SessionName(format!("alpha-{repository}")),
                charter_revision: 1,
            },
        },
    );
}

/// A mail file name: the ULID of `2026-10-07T<time>Z` and the randomness `random` (16
/// characters of Crockford's base 32), then `.md`.
fn ulid(time: &str, random: &str) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let ms = u64::try_from(at(&format!("2026-10-07T{time}Z")).unix_timestamp() * 1000)
        .expect("after the epoch");
    let mut text = String::new();
    for n in (0..10).rev() {
        text.push(ALPHABET[((ms >> (5 * n)) & 31) as usize] as char);
    }
    format!("{text}{random}.md")
}

/// `CIEND` names the end of the checks a Running controller's newest report `blocked ci <ref>`
/// waits on, once they end after the report; once per ref and report. Checks that ended before
/// the report are no `CIEND`.
#[test]
fn ciend_fires_when_the_blocking_pipeline_ends_after_the_report() {
    let scratch = case("ciend");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, GITHUB);
    let state = dir.join("alpha/state");
    {
        let mut handle = store::open(&state).expect("open alpha's store");
        running(&mut handle, "11111111-1111-4111-8111-111111111111", "app");
        running(&mut handle, "22222222-2222-4222-8222-222222222222", "lib");
        handle.check().expect("kept");
    }
    let mailbox = state.join("mailbox/conductor");
    fs::create_dir_all(&mailbox).expect("create the mailbox");
    fs::write(
        mailbox.join(ulid("11:45:00", "0000000000000001")),
        "[REPORT app DSP-1] blocked ci #7\n",
    )
    .expect("write a report");
    fs::write(
        mailbox.join(ulid("11:45:00", "0000000000000002")),
        "[REPORT lib DSP-2] blocked ci lib#4\n",
    )
    .expect("write a report");
    let fake = Fake::new(&dir);
    fake.search(&[("app", 7), ("lib", 4)]);
    fake.pulls("app", &[pull(7, "aaa", false, false, &[running_check()])]);
    fake.pulls(
        "lib",
        &[pull(4, "bbb", false, false, &[done("FAILURE", "11:30:00")])],
    );

    set_clock("12:00:00");
    let mut first = watch(&config, &input(), fake.sources());
    let (found, notes) = printed(&mut first, &["CIEND"]);
    assert_eq!(found, Vec::<String>::new(), "nothing ended yet: {notes}");

    fake.pulls(
        "app",
        &[pull(7, "aaa", false, false, &[done("FAILURE", "12:05:00")])],
    );
    set_clock("12:10:00");
    let mut second = watch(&config, &input(), fake.sources());
    let (found, notes) = printed(&mut second, &["CIEND"]);
    assert_eq!(
        found,
        ["CIEND alpha app#7 #7 red, ended 2026-10-07T12:05:00Z"],
        "{notes}"
    );

    set_clock("12:31:00");
    let mut third = watch(&config, &input(), fake.sources());
    let (found, _) = printed(&mut third, &["CIEND"]);
    assert_eq!(found, Vec::<String>::new(), "once per ref and report");
}

// -------------------------------------------------------------------------------------------
// Reads
// -------------------------------------------------------------------------------------------

/// A rate-limited read is one note naming the instance and the repository; the instance's
/// forge is read again after 4 min, then 8, and every 2 min once a read succeeds; a mail fires
/// all along.
#[test]
fn a_rate_limited_forge_is_a_note_and_backs_off_while_the_other_triggers_fire() {
    let scratch = case("rate-limited");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, GITHUB);
    let mailbox = dir.join("alpha/state/mailbox/conductor");
    fs::create_dir_all(&mailbox).expect("create the mailbox");
    let fake = Fake::new(&dir);
    fake.search(&[("app", 7)]);
    fake.pulls(
        "app",
        &[pull(7, "aaa", false, false, &[done("SUCCESS", "11:40:00")])],
    );
    fs::write(
        dir.join("gh-fail-pr"),
        "HTTP 429: API rate limit exceeded for user\n",
    )
    .expect("write the refusal");
    let reads = || fake.calls("gh.log", "pr list");

    set_clock("12:00:00");
    let mut watch = watch(&config, &input(), fake.sources());
    let (found, notes) = printed(&mut watch, &["GREEN"]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(reads(), 1);
    let refused: Vec<&str> = notes.lines().filter(|line| line.contains("429")).collect();
    assert_eq!(refused.len(), 1, "one note: {notes}");
    assert!(
        refused[0].contains("alpha") && refused[0].contains("example-org/app"),
        "the note names the instance and the repository: {notes}"
    );

    fs::write(
        mailbox.join(ulid("12:01:00", "0000000000000001")),
        "hello\n",
    )
    .expect("write a mail");
    set_clock("12:02:00");
    let (found, _) = printed(&mut watch, &["MAIL", "GREEN"]);
    assert_eq!(found.len(), 1, "the mail fires: {found:?}");
    assert!(found[0].starts_with("MAIL alpha "), "{found:?}");
    assert_eq!(reads(), 1, "backed off to 4 min");

    set_clock("12:04:00");
    printed(&mut watch, &[]);
    assert_eq!(reads(), 2, "read again after 4 min, refused again");
    set_clock("12:11:00");
    printed(&mut watch, &[]);
    assert_eq!(reads(), 2, "backed off to 8 min");

    fs::remove_file(dir.join("gh-fail-pr")).expect("the forge answers again");
    set_clock("12:12:00");
    let (found, _) = printed(&mut watch, &["GREEN"]);
    assert_eq!(reads(), 3);
    assert_eq!(found, ["GREEN alpha app#7 green 32m, head aaa"]);
    set_clock("12:13:00");
    printed(&mut watch, &[]);
    assert_eq!(reads(), 3, "every 2 min once a read succeeded");
    set_clock("12:14:00");
    printed(&mut watch, &[]);
    assert_eq!(reads(), 4);
}

/// The forge of an instance is read at most every 2 min, across runs, and only for the
/// repositories with an open request.
#[test]
fn the_forge_is_read_at_most_every_two_minutes_per_instance() {
    let scratch = case("two-minutes");
    let dir = scratch.path().to_owned();
    let config = parsed(&dir, GITHUB);
    let fake = Fake::new(&dir);
    fake.search(&[("app", 7)]);
    fake.pulls(
        "app",
        &[pull(7, "aaa", false, false, &[done("FAILURE", "11:40:00")])],
    );
    for (time, searches) in [
        ("12:00:00", 1),
        ("12:00:00", 1),
        ("12:01:59", 1),
        ("12:02:00", 2),
        ("12:03:00", 2),
        ("12:04:00", 3),
    ] {
        set_clock(time);
        let mut run = watch(&config, &input(), fake.sources());
        printed(&mut run, &[]);
        assert_eq!(fake.calls("gh.log", "search prs"), searches, "at {time}");
        assert_eq!(fake.calls("gh.log", "pr list"), searches, "at {time}");
    }
    let log = fs::read_to_string(dir.join("gh.log")).expect("gh was read");
    assert!(
        log.lines()
            .filter(|line| line.starts_with("pr list"))
            .all(|line| line.contains("-R example-org/app ")),
        "only the repository with an open request is read: {log}"
    );
}
