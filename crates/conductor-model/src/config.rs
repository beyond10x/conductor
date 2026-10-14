// generated from conductor v1
// model digest 6226ef1b625b88bce44065a5b6313e70ea6605bae111eb1c93d9e7cff3ed8d08
// contract digest b7469f2cf745398ea45d13856f361e36621d049aab6cd4819ea48b4e89f5dba5
// do not edit: regenerate with `ess synthesize --layout crate`

//! Config — `conductor.config`.
//!
//! The one file that names conductor's instances, `conductor.config/1`, read from `--config`, else the `conductor-config` setting, else ~/.b10x/conductor/conductor.yaml (story:config-file). An instance is one organization conductor runs: where its repositories come from, where their checkouts are, where conductor keeps its records, state and cache, which harness and model each role runs on, its cadence, thresholds, reports, and who decides each decision class. These types hold the effective config, after defaults; the file may leave out every key of an instance but its name, its sources and its checkouts. No field takes a secret.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Authority — `conductor.config.Authority`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    /// `class_c` — `conductor.decision.Decider`.
    pub class_c: crate::decision::Decider,
    /// `class_o` — `conductor.decision.Decider`.
    pub class_o: crate::decision::Decider,
}

/// Cadence — `conductor.config.Cadence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cadence {
    /// `cycle` — `Duration`.
    pub cycle: crate::primitives::Duration,
    /// `daily` — `conductor.config.TimeOfDay`.
    pub daily: TimeOfDay,
    /// `watch` — `Duration`.
    pub watch: crate::primitives::Duration,
    /// `ci` — `Duration`.
    pub ci: crate::primitives::Duration,
}

/// Catalog — `conductor.config.Catalog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    /// `repository` — `String`.
    pub repository: String,
    /// `path` — `String`.
    pub path: String,
    /// `names` — `conductor.config.CatalogNames`.
    pub names: CatalogNames,
}

/// CatalogNames — `conductor.config.CatalogNames`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogNames {
    /// `Plain`.
    Plain,
    /// `Hex`.
    Hex,
}

/// Checkouts — `conductor.config.Checkouts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkouts {
    /// `root` — `String`.
    pub root: String,
    /// `trees` — `String`.
    pub trees: String,
}

/// CommandWord — `conductor.config.CommandWord`: a distinct wrapper around `String`.
///
/// Every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._:/-`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandWord(pub String);

/// ConductorSession — `conductor.config.ConductorSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorSession {
    /// `served_by` — `Optional<conductor.config.InstanceName>`.
    pub served_by: Option<InstanceName>,
    /// `session_name` — `Optional<conductor.config.SessionName>`.
    pub session_name: Option<SessionName>,
}

/// Config — `conductor.config.Config`.
///
/// Every value satisfies `version == "conductor.config/1"` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.name` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.session_prefix` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.dashboard_port` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(not (defined(default)) or exists instance in instances: (instance.name == default))` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `version` — `String`.
    pub version: String,
    /// `default` — `Optional<conductor.config.InstanceName>`.
    pub default: Option<InstanceName>,
    /// `instances` — `List<conductor.config.Instance>`.
    pub instances: Vec<Instance>,
}

/// Confinement — `conductor.config.Confinement`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confinement {
    /// `mode` — `conductor.config.ConfinementMode`.
    pub mode: ConfinementMode,
    /// `provider` — `conductor.config.ConfinementProvider`.
    pub provider: ConfinementProvider,
    /// `limits` — `conductor.config.Limits`.
    pub limits: Limits,
}

/// ConfinementMode — `conductor.config.ConfinementMode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfinementMode {
    /// `Strict`.
    Strict,
    /// `Report`.
    Report,
}

/// ConfinementProvider — `conductor.config.ConfinementProvider`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfinementProvider {
    /// `Systemd`.
    Systemd,
    /// `None`.
    None,
}

/// Controllers — `conductor.config.Controllers`.
///
/// Every value satisfies `max_working >= 1` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `max_subagents >= 1` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `max_open_reviews >= 0` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controllers {
    /// `max_working` — `Integer`.
    pub max_working: i64,
    /// `max_subagents` — `Integer`.
    pub max_subagents: i64,
    /// `max_open_reviews` — `Integer`.
    pub max_open_reviews: i64,
}

/// DoctorReport — `conductor.config.DoctorReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    /// `machine` — `conductor.config.Machine`.
    pub machine: Machine,
    /// `prerequisites` — `List<conductor.config.Prerequisite>`.
    pub prerequisites: Vec<Prerequisite>,
    /// `logins` — `List<conductor.config.ForgeLogin>`.
    pub logins: Vec<ForgeLogin>,
    /// `verdict` — `String`.
    pub verdict: String,
}

/// ForgeLogin — `conductor.config.ForgeLogin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeLogin {
    /// `program` — `String`.
    pub program: String,
    /// `host` — `String`.
    pub host: String,
    /// `state` — `conductor.config.ForgeLoginState`.
    pub state: ForgeLoginState,
    /// `account` — `Optional<String>`.
    pub account: Option<String>,
}

/// ForgeLoginState — `conductor.config.ForgeLoginState`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeLoginState {
    /// `LoggedIn`.
    LoggedIn,
    /// `NotLoggedIn`.
    NotLoggedIn,
}

/// Gibibytes — `conductor.config.Gibibytes`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gibibytes(pub i64);

/// GitHubSource — `conductor.config.GitHubSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubSource {
    /// `owner` — `String`.
    pub owner: String,
}

/// GitLabSource — `conductor.config.GitLabSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitLabSource {
    /// `group` — `String`.
    pub group: String,
    /// `host` — `String`.
    pub host: String,
    /// `exclude` — `List<String>`.
    pub exclude: Vec<String>,
}

/// Harness — `conductor.config.Harness`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    /// `Claude`.
    Claude,
    /// `Codex`.
    Codex,
}

/// InitRecords — `conductor.config.InitRecords`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRecords {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// Instance — `conductor.config.Instance`.
///
/// Every value satisfies `dashboard_port >= 1` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `dashboard_port <= 65535` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// `name` — `conductor.config.InstanceName`.
    pub name: InstanceName,
    /// `session_prefix` — `Optional<conductor.config.SessionPrefix>`.
    pub session_prefix: Option<SessionPrefix>,
    /// `dashboard_port` — `Integer`.
    pub dashboard_port: i64,
    /// `sources` — `List<conductor.config.Source>`.
    pub sources: Vec<Source>,
    /// `checkouts` — `conductor.config.Checkouts`.
    pub checkouts: Checkouts,
    /// `records` — `String`.
    pub records: String,
    /// `state` — `String`.
    pub state: String,
    /// `cache` — `String`.
    pub cache: String,
    /// `roles` — `List<conductor.config.Role>`.
    pub roles: Vec<Role>,
    /// `confinement` — `conductor.config.Confinement`.
    pub confinement: Confinement,
    /// `controllers` — `conductor.config.Controllers`.
    pub controllers: Controllers,
    /// `repositories` — `List<conductor.config.RepositoryRule>`.
    pub repositories: Vec<RepositoryRule>,
    /// `cadence` — `conductor.config.Cadence`.
    pub cadence: Cadence,
    /// `thresholds` — `conductor.config.Thresholds`.
    pub thresholds: Thresholds,
    /// `retention` — `conductor.config.Retention`.
    pub retention: Retention,
    /// `reports` — `List<conductor.config.Report>`.
    pub reports: Vec<Report>,
    /// `authority` — `conductor.config.Authority`.
    pub authority: Authority,
    /// `operator` — `Optional<String>`.
    pub operator: Option<String>,
    /// `catalog` — `Optional<conductor.config.Catalog>`.
    pub catalog: Option<Catalog>,
    /// `conductor` — `conductor.config.ConductorSession`.
    pub conductor: ConductorSession,
}

/// InstanceName — `conductor.config.InstanceName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceName(pub String);

/// Limits — `conductor.config.Limits`.
///
/// Every value satisfies `(not (defined(cpu_weight)) or cpu_weight >= 1)` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(not (defined(cpu_weight)) or cpu_weight <= 10000)` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(not (defined(tasks_max)) or tasks_max >= 1)` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// `memory_max` — `Optional<conductor.config.Gibibytes>`.
    pub memory_max: Option<Gibibytes>,
    /// `cpu_weight` — `Optional<Integer>`.
    pub cpu_weight: Option<i64>,
    /// `tasks_max` — `Optional<Integer>`.
    pub tasks_max: Option<i64>,
}

/// LocalSource — `conductor.config.LocalSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSource {
    /// `path` — `String`.
    pub path: String,
    /// `exclude` — `List<String>`.
    pub exclude: Vec<String>,
}

/// Machine — `conductor.config.Machine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// `host` — `String`.
    pub host: String,
    /// `system` — `String`.
    pub system: String,
    /// `release` — `String`.
    pub release: String,
    /// `architecture` — `String`.
    pub architecture: String,
    /// `conductor` — `String`.
    pub conductor: String,
}

/// Network — `conductor.config.Network`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// `Open`.
    Open,
}

/// Prerequisite — `conductor.config.Prerequisite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prerequisite {
    /// `program` — `String`.
    pub program: String,
    /// `state` — `conductor.config.PrerequisiteState`.
    pub state: PrerequisiteState,
    /// `version_line` — `Optional<String>`.
    pub version_line: Option<String>,
    /// `version` — `Optional<String>`.
    pub version: Option<String>,
    /// `pin` — `Optional<String>`.
    pub pin: Option<String>,
}

/// PrerequisiteState — `conductor.config.PrerequisiteState`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrerequisiteState {
    /// `Present`.
    Present,
    /// `Missing`.
    Missing,
    /// `OlderThanPin`.
    OlderThanPin,
    /// `VersionUnread`.
    VersionUnread,
    /// `NotNeeded`.
    NotNeeded,
}

/// Report — `conductor.config.Report`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// `channel` — `String`.
    pub channel: String,
    /// `as` — `String`.
    pub r#as: String,
    /// `when` — `List<String>`.
    pub when: Vec<String>,
}

/// RepositoryCheck — `conductor.config.RepositoryCheck`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryCheck {
    /// `check` — `String`.
    pub check: String,
    /// `ready` — `Boolean`.
    pub ready: bool,
    /// `detail` — `String`.
    pub detail: String,
    /// `fix` — `Optional<String>`.
    pub fix: Option<String>,
}

/// RepositoryRule — `conductor.config.RepositoryRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRule {
    /// `match` — `String`.
    pub r#match: String,
    /// `activity` — `conductor.direction.Activity`.
    pub activity: crate::direction::Activity,
}

/// Retention — `conductor.config.Retention`.
///
/// Every value satisfies `snapshots >= 1` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retention {
    /// `snapshots` — `Integer`.
    pub snapshots: i64,
}

/// Role — `conductor.config.Role`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// `role` — `String`.
    pub role: String,
    /// `harness` — `conductor.config.Harness`.
    pub harness: Harness,
    /// `model` — `conductor.config.CommandWord`.
    pub model: CommandWord,
    /// `agent` — `Optional<conductor.config.CommandWord>`.
    pub agent: Option<CommandWord>,
    /// `settings` — `Optional<conductor.config.CommandWord>`.
    pub settings: Option<CommandWord>,
    /// `profile` — `Optional<conductor.config.CommandWord>`.
    pub profile: Option<CommandWord>,
    /// `session_name` — `Optional<conductor.config.SessionName>`.
    pub session_name: Option<SessionName>,
    /// `read_only_tools` — `List<String>`.
    pub read_only_tools: Vec<String>,
    /// `limits` — `conductor.config.Limits`.
    pub limits: Limits,
    /// `writable` — `List<conductor.config.Writable>`.
    pub writable: Vec<Writable>,
    /// `network` — `conductor.config.Network`.
    pub network: Network,
}

/// SessionName — `conductor.config.SessionName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionName(pub String);

/// SessionPrefix — `conductor.config.SessionPrefix`: a distinct wrapper around `String`.
///
/// Every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPrefix(pub String);

/// SetupReport — `conductor.config.SetupReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupReport {
    /// `doctor` — `conductor.config.DoctorReport`.
    pub doctor: DoctorReport,
    /// `repository` — `String`.
    pub repository: String,
    /// `checks` — `List<conductor.config.RepositoryCheck>`.
    pub checks: Vec<RepositoryCheck>,
    /// `verdict` — `String`.
    pub verdict: String,
}

/// SetupRepository — `conductor.config.SetupRepository`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupRepository {
    /// `repository` — `Optional<String>`.
    pub repository: Option<String>,
    /// `apply` — `Optional<Boolean>`.
    pub apply: Option<bool>,
}

/// ShowConfig — `conductor.config.ShowConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowConfig {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// Source — `conductor.config.Source`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Tagged `GitHub` — `conductor.config.GitHubSource`.
    GitHub(GitHubSource),
    /// Tagged `GitLab` — `conductor.config.GitLabSource`.
    GitLab(GitLabSource),
    /// Tagged `Local` — `conductor.config.LocalSource`.
    Local(LocalSource),
}

/// Thresholds — `conductor.config.Thresholds`.
///
/// Every value satisfies `gate_resume > gate_stop` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thresholds {
    /// `context_handover` — `conductor.config.Tokens`.
    pub context_handover: Tokens,
    /// `disk_path` — `String`.
    pub disk_path: String,
    /// `disk_low` — `conductor.config.Gibibytes`.
    pub disk_low: Gibibytes,
    /// `disk_clear` — `conductor.config.Gibibytes`.
    pub disk_clear: Gibibytes,
    /// `disk_admit` — `conductor.config.Gibibytes`.
    pub disk_admit: Gibibytes,
    /// `build_slot` — `conductor.config.Gibibytes`.
    pub build_slot: Gibibytes,
    /// `build_size` — `conductor.config.Gibibytes`.
    pub build_size: Gibibytes,
    /// `gate_stop` — `conductor.config.Gibibytes`.
    pub gate_stop: Gibibytes,
    /// `gate_resume` — `conductor.config.Gibibytes`.
    pub gate_resume: Gibibytes,
    /// `watchdog_every` — `Duration`.
    pub watchdog_every: crate::primitives::Duration,
    /// `silent_session` — `Duration`.
    pub silent_session: crate::primitives::Duration,
}

/// TimeOfDay — `conductor.config.TimeOfDay`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeOfDay(pub String);

/// Tokens — `conductor.config.Tokens`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens(pub i64);

/// TrustWorkspaces — `conductor.config.TrustWorkspaces`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustWorkspaces {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// UpgradeBinary — `conductor.config.UpgradeBinary`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeBinary {
    /// `check` — `Optional<Boolean>`.
    pub check: Option<bool>,
    /// `release` — `Optional<String>`.
    pub release: Option<String>,
}

/// UpgradeOutcome — `conductor.config.UpgradeOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeOutcome {
    /// `UpgradeAvailable`.
    UpgradeAvailable,
    /// `NoneAvailable`.
    NoneAvailable,
    /// `Upgraded`.
    Upgraded,
    /// `AlreadyCurrent`.
    AlreadyCurrent,
}

/// UpgradeRefusal — `conductor.config.UpgradeRefusal`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeRefusal {
    /// `ReleaseUnread`.
    ReleaseUnread,
    /// `NoArchiveForTarget`.
    NoArchiveForTarget,
    /// `ChecksumMismatch`.
    ChecksumMismatch,
    /// `NewBinaryDoesNotRun`.
    NewBinaryDoesNotRun,
    /// `BinaryNotWritable`.
    BinaryNotWritable,
}

/// UpgradeRefused — `conductor.config.UpgradeRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeRefused {
    /// `release` — `String`.
    pub release: String,
    /// `target` — `String`.
    pub target: String,
    /// `refusal` — `conductor.config.UpgradeRefusal`.
    pub refusal: UpgradeRefusal,
    /// `detail` — `String`.
    pub detail: String,
}

/// UpgradeReport — `conductor.config.UpgradeReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeReport {
    /// `installed` — `String`.
    pub installed: String,
    /// `release` — `String`.
    pub release: String,
    /// `outcome` — `conductor.config.UpgradeOutcome`.
    pub outcome: UpgradeOutcome,
    /// `binary` — `String`.
    pub binary: String,
}

/// ValidateConfig — `conductor.config.ValidateConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateConfig {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// WorkspaceMember — `conductor.config.WorkspaceMember`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMember {
    /// `name` — `String`.
    pub name: String,
    /// `source` — `String`.
    pub source: String,
}

/// Writable — `conductor.config.Writable`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Writable(pub String);

/// WriteWorkspace — `conductor.config.WriteWorkspace`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteWorkspace {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
    /// `check` — `Optional<Boolean>`.
    pub check: Option<bool>,
}
