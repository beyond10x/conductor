//! The status data, `website/data/status.json` (`b10x-status/1`), and the status page,
//! `website/docs/status.md`, both from [`CAPABILITIES`].
//!
//! A shipped capability names the test that holds it as `path::function`, which generation checks
//! the file defines (`fn function(`), so a claim cannot outlive its evidence. A capability that is
//! not shipped names none. The planned items are the draft stories of the repository's plan. The
//! page is plain Markdown rather than MDX because the unified documentation site, which still
//! collects these pages, accepts no executable MDX.

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::{cell, front_matter};

/// Where the status data lands, relative to the repository root.
pub const DATA: &str = "website/data/status.json";
/// Where the status page lands, relative to the repository root.
pub const PAGE: &str = "website/docs/status.md";

/// The date the list below was last checked against `main`.
const AS_OF: &str = "2026-10-08";

/// One capability on the status page.
pub struct Capability {
    /// The group the status table lists it under.
    pub area: &'static str,
    /// What it is, in a few words.
    pub label: &'static str,
    /// One sentence a reader can check.
    pub detail: &'static str,
    /// `shipped`, `decided` or `planned`.
    pub status: &'static str,
    /// For a shipped capability, the test that holds it: `path::function`.
    pub evidence: Option<&'static str>,
    /// The documentation route that explains it, `/docs/...`, if one does.
    pub href: Option<&'static str>,
}

const fn shipped(
    area: &'static str,
    label: &'static str,
    detail: &'static str,
    evidence: &'static str,
    href: Option<&'static str>,
) -> Capability {
    Capability {
        area,
        label,
        detail,
        status: "shipped",
        evidence: Some(evidence),
        href,
    }
}

const fn planned(area: &'static str, label: &'static str, detail: &'static str) -> Capability {
    Capability {
        area,
        label,
        detail,
        status: "planned",
        evidence: None,
        href: None,
    }
}

const SNAPSHOT: &str = "Snapshot collectors";
const STORE: &str = "Store and views";
const GOALS: &str = "Goals, resources and controllers";
const DECISIONS: &str = "Decisions";
const DISPATCHES: &str = "Dispatches and messages";
const GUARD: &str = "Guard";
const WATCH: &str = "Watch";
const DASHBOARD: &str = "Dashboard";
const CONFIG: &str = "Config and instances";
const USAGE: &str = "Usage";
const PRODUCT: &str = "Product";

/// Every capability, grouped by area in the order the page shows them.
pub const CAPABILITIES: &[Capability] = &[
    shipped(
        SNAPSHOT,
        "GitHub repositories, pull requests and CI",
        "A snapshot records each repository of the instance's GitHub owners with its open pull requests and the latest runs on main, read with gh.",
        "crates/conductor/tests/collect_github.rs::pull_requests_and_latest_main_runs_match_the_reference_counts",
        Some("/docs/reference/cli#conductor-snapshot-start-snapshot"),
    ),
    shipped(
        SNAPSHOT,
        "Local checkout state",
        "A repository with a checkout is recorded with its local state, its latest release and its planning store.",
        "crates/conductor/tests/collect_github.rs::a_checkout_reports_its_local_state_release_and_planning_store",
        None,
    ),
    shipped(
        SNAPSHOT,
        "Live sessions",
        "Each live agent session is recorded with the repository its working directory lies in.",
        "crates/conductor/tests/collect_sessions.rs::each_live_session_is_recorded_with_the_repository_its_cwd_lies_in",
        None,
    ),
    shipped(
        SNAPSHOT,
        "Blockers",
        "The open blocker artifacts each repository's planning store holds on origin/main are recorded after a fetch.",
        "crates/conductor/tests/collect_stores.rs::blockers_are_the_open_blocker_artifacts_each_store_holds_at_origin_main_after_a_fetch",
        None,
    ),
    shipped(
        SNAPSHOT,
        "Specifications",
        "Each repository's specification is read on origin/main, one per repository, archived ones left out.",
        "crates/conductor/tests/collect_stores.rs::specifications_are_read_at_origin_main_one_per_repository_not_archived",
        None,
    ),
    shipped(
        SNAPSHOT,
        "Releases and merged pull requests",
        "A snapshot records the releases and merged pull requests of its window.",
        "crates/conductor/tests/shipped.rs::the_collector_records_the_windows_releases_and_merged_pull_requests",
        None,
    ),
    shipped(
        SNAPSHOT,
        "Catalog membership",
        "When the instance names a catalog, each recorded repository says whether the catalog lists it.",
        "crates/conductor/tests/instance_sources.rs::a_catalog_the_instance_names_marks_the_repositories_it_lists",
        Some("/docs/reference/config"),
    ),
    shipped(
        SNAPSHOT,
        "A silent source fails the snapshot",
        "A source that does not answer fails the snapshot and keeps what was already recorded.",
        "crates/conductor/tests/snapshot.rs::a_source_that_does_not_answer_fails_the_snapshot_and_keeps_what_was_recorded",
        None,
    ),
    shipped(
        STORE,
        "Every observation reads back",
        "Each observation kind a snapshot records is answered by its view.",
        "crates/conductor/tests/snapshot.rs::every_observation_kind_reads_back_through_its_view",
        None,
    ),
    shipped(
        STORE,
        "Views create nothing",
        "A view on a missing store exits 1 and creates nothing.",
        "crates/conductor/tests/state_dir.rs::a_view_on_a_missing_store_is_exit_1_and_creates_nothing",
        None,
    ),
    shipped(
        STORE,
        "Store migration",
        "store migrate moves an older store into the tree store, and every view answers as before.",
        "crates/conductor/tests/store_tree.rs::a_migrated_fixture_store_answers_every_view_as_the_source_store_did",
        Some("/docs/reference/cli#conductor-store-migrate"),
    ),
    shipped(
        STORE,
        "Observation retention",
        "Only the observations of the newest complete snapshots are kept, 12 unless the instance says otherwise.",
        "crates/conductor/tests/observation_retention.rs::after_14_complete_snapshots_the_views_list_the_newest_12_and_the_tree_holds_no_observation",
        Some("/docs/reference/config"),
    ),
    shipped(
        STORE,
        "Release digest",
        "repository shipped lists each release with what merged before it, and the rest as not released.",
        "crates/conductor/tests/shipped.rs::shipped_lists_each_release_with_what_merged_before_it_and_the_rest_as_not_released",
        Some("/docs/reference/cli#conductor-repository-shipped"),
    ),
    shipped(
        STORE,
        "Repository activity and marks",
        "A mark overrides the activity computed from the newest complete snapshot.",
        "crates/conductor/tests/repository_marks.rs::a_mark_overrides_activity_computed_from_the_newest_complete_snapshot",
        Some("/docs/reference/cli#conductor-repository-activity"),
    ),
    shipped(
        GOALS,
        "Goals",
        "A goal is proposed, confirmed and met, and a goal confirmed twice is refused.",
        "crates/conductor/tests/goal_resource.rs::g9_is_proposed_confirmed_and_met_and_confirming_it_again_is_refused",
        None,
    ),
    shipped(
        GOALS,
        "Resource ledger",
        "Build slots and other resources are requested, granted, released and refused through one ledger.",
        "crates/conductor/tests/goal_resource.rs::resource_requests_are_granted_released_and_refused_through_the_ledger",
        None,
    ),
    shipped(
        GOALS,
        "Controller records",
        "One controller runs per repository, through pause, resume, charter revision and stop.",
        "crates/conductor/tests/controller.rs::one_controller_runs_per_repository_through_pause_resume_revise_and_stop",
        None,
    ),
    shipped(
        DECISIONS,
        "Requests, escalation and answers",
        "A decision request is escalated to the operator and answered.",
        "crates/conductor/tests/decision.rs::a_request_is_escalated_and_answered_by_the_operator",
        None,
    ),
    shipped(
        DECISIONS,
        "Decision authority",
        "A class-O decision recorded as conductor's is refused and nothing is kept.",
        "crates/conductor/tests/decision.rs::a_class_o_decision_recorded_as_conductor_is_refused_and_nothing_is_kept",
        None,
    ),
    shipped(
        DECISIONS,
        "Decision ids",
        "Decision ids are allocated per UTC date, and concurrent records get distinct ones.",
        "crates/conductor/tests/decision.rs::decision_ids_are_allocated_per_utc_date",
        None,
    ),
    shipped(
        DISPATCHES,
        "Dispatch lifecycle",
        "A dispatch moves from sent through started, blocked and done, and another is cancelled.",
        "crates/conductor/tests/dispatch.rs::a_dispatch_against_g9_moves_sent_started_blocked_started_done_and_another_is_cancelled",
        None,
    ),
    shipped(
        DISPATCHES,
        "Dispatch ids",
        "Dispatch ids are allocated per UTC day, and a hand-given id that exists is refused.",
        "crates/conductor/tests/dispatch.rs::send_dispatch_allocates_dsp_ids_per_utc_day_and_refuses_a_hand_given_id_that_exists",
        None,
    ),
    shipped(
        DISPATCHES,
        "Message formats",
        "Each first-line message format is parsed into its kind.",
        "crates/conductor/tests/dispatch.rs::receive_message_parses_each_first_line_format_into_its_kind",
        None,
    ),
    shipped(
        DISPATCHES,
        "A message that does not parse is answered with the format",
        "A message whose first line names no format is rejected with the format, not acted on.",
        "crates/conductor/tests/dispatch.rs::a_decision_request_is_received_and_handled_and_hello_is_rejected_with_the_format",
        None,
    ),
    shipped(
        GUARD,
        "Controller write boundary",
        "A controller writes in its own checkout and its managed worktrees, and nowhere else.",
        "crates/conductor/tests/guard_rules.rs::a_controller_writes_nowhere_else",
        None,
    ),
    shipped(
        GUARD,
        "Message boundary",
        "A controller messages conductor and the agents it started, and no one else.",
        "crates/conductor/tests/guard_rules.rs::a_controller_messages_no_one_else",
        None,
    ),
    shipped(
        GUARD,
        "No GitHub writes from a controller",
        "A controller reads GitHub and makes no write there.",
        "crates/conductor/tests/guard_rules.rs::a_controller_makes_no_github_write",
        None,
    ),
    shipped(
        GUARD,
        "The hook and its record",
        "The hook denies with exit 2, allows with exit 0, and records each denial.",
        "crates/conductor/tests/guard_hook.rs::the_hook_denies_with_exit_2_and_allows_with_exit_0_and_records_the_denial",
        Some("/docs/reference/cli#conductor-guard-record-guard-decision"),
    ),
    shipped(
        GUARD,
        "An allowance never opens the store",
        "An allowed tool call is answered with exit 0 without opening the store.",
        "crates/conductor/tests/guard_hook.rs::an_allowance_is_answered_with_exit_0_and_never_opens_the_store",
        None,
    ),
    shipped(
        GUARD,
        "Settings wire the guard",
        "The controller settings file wires the guard on the five tools it checks.",
        "crates/conductor/tests/guard_hook.rs::the_controller_settings_wire_the_guard_on_the_five_tools",
        None,
    ),
    shipped(
        WATCH,
        "Session exits",
        "A session gone from the session list is reported, and a start is not.",
        "crates/conductor/tests/watch.rs::a_session_gone_from_the_list_is_reported_and_a_start_is_not",
        Some("/docs/reference/cli#conductor-watch-run"),
    ),
    shipped(
        WATCH,
        "Rate limits",
        "A fresh rate limit is reported once a day, with a notification.",
        "crates/conductor/tests/watch.rs::a_rate_limit_of_a_minute_ago_is_reported_once_a_day_with_a_notification",
        None,
    ),
    shipped(
        WATCH,
        "Context threshold",
        "Conductor's context above the hand-over threshold is reported once per session.",
        "crates/conductor/tests/watch.rs::conductor_context_above_300k_tokens_is_reported_once_per_session",
        None,
    ),
    shipped(
        WATCH,
        "Main CI",
        "Main's CI turning red, and green again, is reported on a CI pass.",
        "crates/conductor/tests/watch.rs::main_ci_turning_red_and_green_again_is_reported_on_a_ci_pass",
        None,
    ),
    shipped(
        WATCH,
        "Free disk",
        "Free disk under the low threshold is reported, and reported again only after it has recovered past the clear threshold.",
        "crates/conductor/tests/watch.rs::the_built_in_instance_keeps_100g_free_and_wakes_from_110g",
        Some("/docs/reference/config"),
    ),
    shipped(
        DASHBOARD,
        "Session table",
        "The dashboard's data holds one row per session under the checkouts root, with its dispatch.",
        "crates/conductor/tests/dashboard.rs::data_json_holds_one_row_per_session_under_the_checkouts_root_with_its_dispatch",
        Some("/docs/reference/cli#conductor-dashboard-serve"),
    ),
    shipped(
        DASHBOARD,
        "Read-only",
        "Every request reads the sources afresh and writes nothing.",
        "crates/conductor/tests/dashboard.rs::every_request_reads_the_sources_afresh_and_writes_nothing",
        None,
    ),
    shipped(
        DASHBOARD,
        "Disk waste",
        "The disk panel lists the reclaimable rows largest first, with their total.",
        "crates/conductor/tests/dashboard.rs::the_disk_object_carries_the_waste_rows_largest_first_with_their_total",
        None,
    ),
    shipped(
        DASHBOARD,
        "Loopback only",
        "The dashboard listens on a free port of 127.0.0.1 and prints its address first.",
        "crates/conductor/tests/dashboard.rs::the_binary_listens_on_a_free_port_of_127_0_0_1_and_says_so_first",
        None,
    ),
    shipped(
        CONFIG,
        "Problems named by YAML path",
        "config validate refuses an unknown key, naming its YAML path.",
        "crates/conductor/tests/config.rs::an_unknown_key_is_refused_with_its_yaml_path",
        Some("/docs/reference/config"),
    ),
    shipped(
        CONFIG,
        "Effective config",
        "config show prints the effective instance after defaults, with every path absolute.",
        "crates/conductor/tests/config.rs::show_without_a_file_prints_todays_constants",
        Some("/docs/reference/cli#conductor-config-show"),
    ),
    shipped(
        CONFIG,
        "Built-in instance",
        "Without a config file, config validate says the built-in defaults apply.",
        "crates/conductor/tests/config.rs::validate_without_a_file_says_the_defaults_apply",
        Some("/docs/reference/cli#conductor-config-validate"),
    ),
    shipped(
        CONFIG,
        "Several instances",
        "A file's default selects the instance, and the instance setting beats it.",
        "crates/conductor/tests/grouped_instance.rs::the_default_selects_the_instance_and_the_setting_beats_it",
        Some("/docs/reference/config"),
    ),
    shipped(
        CONFIG,
        "Local sources, grouped repositories",
        "A local source lists repositories one or two levels deep and places their sessions.",
        "crates/conductor/tests/grouped_instance.rs::a_local_source_lists_grouped_repositories_and_places_their_sessions",
        Some("/docs/reference/config"),
    ),
    shipped(
        USAGE,
        "Token usage per repository",
        "resource usage counts each message once per repository and session.",
        "crates/conductor/tests/usage.rs::usage_counts_each_message_once_per_repository_and_session",
        Some("/docs/reference/cli#conductor-resource-usage"),
    ),
    shipped(
        USAGE,
        "Nothing private printed",
        "Every output format prints the same rows and nothing from the conversations.",
        "crates/conductor/tests/usage.rs::every_format_prints_the_same_rows_and_nothing_private",
        None,
    ),
    shipped(
        PRODUCT,
        "No organization in the product",
        "No file of the product names an organization; an organization is a config file.",
        "crates/conductor/tests/no_organization.rs::no_file_of_the_product_names_an_organization",
        None,
    ),
    shipped(
        PRODUCT,
        "The command line is the specification's",
        "The clap command tree is compared with the specification's command block on every test run.",
        "crates/conductor/tests/cli_tree.rs::derive_tree_is_the_cli_block",
        Some("/docs/reference/cli"),
    ),
    planned(
        SNAPSHOT,
        "Codex thread goals",
        "A snapshot reads the operator's Codex thread goals, read-only.",
    ),
    planned(
        SNAPSHOT,
        "Dependency map",
        "Which repository depends on which, read from their manifests rather than declared.",
    ),
    planned(
        STORE,
        "Append-cost test on a loaded host",
        "The store's append-cost test holds on a busy machine instead of failing the gate.",
    ),
    planned(
        STORE,
        "Status board",
        "snapshot publish-board writes STATUS.md from the newest complete snapshot.",
    ),
    planned(
        STORE,
        "Focus per goal",
        "The board says what the organization is working on, one line per goal.",
    ),
    planned(
        GOALS,
        "Starting a controller launches its session",
        "start-controller starts the controller's session, not only its record.",
    ),
    planned(
        GOALS,
        "Codex controller profile",
        "A controller profile for Codex sessions, beside the Claude one.",
    ),
    planned(
        GOALS,
        "A controller starts on less reading",
        "A controller's start-of-session reading shrinks, so it reaches its hand-over later.",
    ),
    planned(
        DECISIONS,
        "A decision names its goal and value",
        "A decision records the goal it serves and the value or principle it rests on.",
    ),
    planned(
        DISPATCHES,
        "Harness-neutral mailbox",
        "Messages between sessions travel as files, so sessions of different harnesses reach each other.",
    ),
    planned(
        DISPATCHES,
        "Restart and ready messages",
        "The restart message, the ready report and the conductor-dev actor are in the specification.",
    ),
    planned(
        GUARD,
        "Every tool call is guarded",
        "Every tool call reaches the guard, and a tool it does not know is denied.",
    ),
    planned(
        GUARD,
        "Bash writes bounded by the sandbox",
        "Shell writes are bounded by the harness sandbox rather than by parsing commands.",
    ),
    planned(
        GUARD,
        "A store timeout is not a denial",
        "When the guard cannot record its verdict in time it says so, rather than deny the tool call.",
    ),
    planned(
        GUARD,
        "Sibling trees of one repository",
        "A relative cd into another managed tree of the same repository is not taken for another repository.",
    ),
    planned(
        WATCH,
        "Free-disk reading matches df",
        "The watch's free-disk reading agrees with df on the same filesystem.",
    ),
    planned(
        WATCH,
        "Largest managed trees",
        "The watch reports the largest managed trees by size.",
    ),
    planned(
        CONFIG,
        "Every config key has an effect",
        "Each key config validate accepts is read by a command, the watch or a profile.",
    ),
    planned(
        PRODUCT,
        "Conformance suite against the CLI",
        "The specification's conformance suite runs against the conductor binary.",
    ),
];

/// Whether `source` defines a function `name`.
fn defines(source: &str, name: &str) -> bool {
    source.contains(&format!("fn {name}("))
}

/// Refuses a list with a label twice, an unknown status, a route outside `/docs/`, a shipped item
/// without evidence or with evidence that does not exist, and evidence on an item not shipped.
fn checked(root: &Path, capabilities: &[Capability]) -> Result<()> {
    let mut labels = BTreeSet::new();
    for capability in capabilities {
        let label = capability.label;
        if !labels.insert(label) {
            bail!("{label}: listed twice");
        }
        if !matches!(capability.status, "shipped" | "decided" | "planned") {
            bail!(
                "{label}: status {} is not shipped, decided or planned",
                capability.status
            );
        }
        if let Some(href) = capability.href
            && !href.starts_with("/docs/")
        {
            bail!("{label}: href {href} is not a /docs/ route");
        }
        match (capability.status, capability.evidence) {
            ("shipped", Some(evidence)) => {
                let (file, name) = evidence
                    .split_once("::")
                    .with_context(|| format!("{label}: {evidence} is not `path::function`"))?;
                let source = fs::read_to_string(root.join(file))
                    .with_context(|| format!("{label}: reading {file}"))?;
                if !defines(&source, name) {
                    bail!("{label}: {file} has no test {name}");
                }
            }
            ("shipped", None) => bail!("{label}: shipped without a test"),
            (_, Some(_)) => bail!("{label}: only a shipped item names a test"),
            (_, None) => {}
        }
    }
    Ok(())
}

/// The `b10x-status/1` document for `capabilities`, checked against the repository at `root`.
///
/// # Errors
///
/// When the list does not pass [`checked`].
pub fn data_for(root: &Path, capabilities: &[Capability]) -> Result<String> {
    checked(root, capabilities)?;
    let items: Vec<_> = capabilities
        .iter()
        .map(|capability| {
            let mut item = json!({
                "area": capability.area,
                "label": capability.label,
                "detail": capability.detail,
                "status": capability.status,
            });
            if let Some(href) = capability.href {
                item["href"] = json!(href);
            }
            item
        })
        .collect();
    let document = json!({
        "format": "b10x-status/1",
        "asOf": AS_OF,
        "source": "conductor-docs, from its capability list; every shipped item names the test that holds it",
        "items": items,
    });
    Ok(serde_json::to_string_pretty(&document)? + "\n")
}

/// The `b10x-status/1` document for the repository at `root`.
///
/// # Errors
///
/// When [`CAPABILITIES`] does not pass [`checked`].
pub fn data(root: &Path) -> Result<String> {
    data_for(root, CAPABILITIES)
}

/// `/docs/reference/cli#x` as a link from `website/docs/status.md`: `./reference/cli.md#x`.
fn link(href: &str) -> String {
    let route = href.trim_start_matches("/docs/");
    let (page, anchor) = route.split_once('#').unwrap_or((route, ""));
    let anchor = if anchor.is_empty() {
        String::new()
    } else {
        format!("#{anchor}")
    };
    format!("./{page}.md{anchor}")
}

/// A status as the table shows it: a glyph and the word.
fn shown(status: &str) -> &'static str {
    match status {
        "shipped" => "● shipped",
        "decided" => "◐ decided",
        _ => "○ planned",
    }
}

fn page_for(root: &Path, capabilities: &[Capability]) -> Result<String> {
    checked(root, capabilities)?;
    let count = |status: &str| {
        capabilities
            .iter()
            .filter(|capability| capability.status == status)
            .count()
    };
    let mut out = front_matter(
        "Status",
        90,
        "What conductor does today and what is planned, capability by capability.",
    );
    out.push_str(&format!(
        "# Status\n\n{} capabilities are shipped and {} are planned, as of {AS_OF}. **Shipped** \
         means implemented on `main` and held by a named test in the repository; `conductor-docs` \
         generates this page and `website/data/status.json` from one list and fails when a test it \
         names is gone. **Planned** means a draft on the repository's plan, not built.\n",
        count("shipped"),
        count("planned") + count("decided"),
    ));
    let mut areas: Vec<&str> = Vec::new();
    for capability in capabilities {
        if !areas.contains(&capability.area) {
            areas.push(capability.area);
        }
    }
    for area in areas {
        out.push_str(&format!(
            "\n## {area}\n\n| Capability | Status | What it means |\n|---|---|---|\n"
        ));
        for capability in capabilities.iter().filter(|c| c.area == area) {
            let label = cell(capability.label);
            let label = capability
                .href
                .map_or(label.clone(), |href| format!("[{label}]({})", link(href)));
            out.push_str(&format!(
                "| {label} | {} | {} |\n",
                shown(capability.status),
                cell(capability.detail),
            ));
        }
    }
    Ok(out)
}

/// The status page for the repository at `root`.
///
/// # Errors
///
/// When [`CAPABILITIES`] does not pass [`checked`].
pub fn page(root: &Path) -> Result<String> {
    page_for(root, CAPABILITIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_capability_rests_on_a_test_that_exists() {
        let root = crate::repository_root();
        let data = data(&root).unwrap();
        assert!(data.contains("\"format\": \"b10x-status/1\""));
        let page = page(&root).unwrap();
        assert!(page.contains(
            "| [Built-in instance](./reference/cli.md#conductor-config-validate) | ● shipped |"
        ));
        assert!(page.contains("| Dependency map | ○ planned |"));
    }

    #[test]
    fn routes_become_relative_markdown_links() {
        assert_eq!(link("/docs/reference/config"), "./reference/config.md");
        assert_eq!(
            link("/docs/reference/cli#conductor-watch-run"),
            "./reference/cli.md#conductor-watch-run"
        );
    }
}
