# Sessions: isolated context, one intake, layered profiles

Status: design for approval. No story is delivered before the operator approves it.

Three connected parts: what context a conductor session may see (§ 1), how a person's input enters
and is routed to the role that does the work (§ 2), and how config extends a role's profile per
instance and repository (§ 3). Evidence: an inventory of what reaches each session kind today, kept
in the instance's state beside the records (`<state>/conductor-dev/evidence-2026-10-13/`).

## 1. Context isolation

### What reaches a session today

| mechanism | conductor, conductor-dev, controller, worker | the gateway (the operator's remote-control session) |
|---|---|---|
| the user's global memory file | no (`--setting-sources project,local`, and `claudeMdExcludes` in the role settings) | **yes** |
| user skills and user agents | no | **yes** |
| user output style, user hooks | no | **yes** |
| plugins | only those the role's `--settings` enables (an allow-list in effect) | **every plugin the user enabled** |
| account-level connectors (claude.ai MCP servers), built-in skills, session context | **yes, uncontrolled** | yes |
| project memory and agents of the start directory | yes (records: a pointer file; checkouts: their `AGENTS.md`) | yes (the product checkout) |
| one shared background daemon's environment (the API key, `CLAUDE_CODE_AGENT`) | **yes, for every background session** | n/a |
| `CONDUCTOR_INSTANCE`, `CONDUCTOR_CONFIG` | **not reliably**: set on the start call only | n/a |

The user's instructions are written for work without conductor. They reach the gateway fully, and
account-level context reaches every session; some of it contradicts conductor (examples found:
publishing to cloud documents, waiting for a usage reset, identifying conductor by its checkout).

### Mechanism

| option | isolates | costs |
|---|---|---|
| A. A dedicated config directory per role (`CLAUDE_CONFIG_DIR`) | everything user-level, the account's connectors through a separate login, the daemon and its environment | a login per directory; plugins installed per directory; conductor's own code reads `~/.claude.json` and `~/.claude/projects` (trust, the watch, the dashboard), and `claude agents` lists one daemon's sessions only |
| **B. Close every remaining door with flags and settings (recommended now)** | user-level files (already), plus: `--strict-mcp-config --mcp-config <role's list>` so only the role's MCP servers load; the connectors switch (`ENABLE_CLAUDEAI_MCP_SERVERS=false`) in the role's settings `env`; `--disable-slash-commands` where a role needs none; `CONDUCTOR_INSTANCE`/`CONDUCTOR_CONFIG` and the role's environment in its settings `env`; the gateway started with its own role settings like every other role | built-in skills and organisation instructions stay (they cannot be switched off this way); each flag is verified by a test that reads a started session's first events |
| C. B now, A later behind a config switch | B's gains now, A's full isolation when its costs are paid | two code paths for a while |

**Recommended: C.** B is built now; A is a later story behind `isolation: config-dir`, once conductor
reads sessions through `claude agents --json` of every daemon and logins can be provisioned.

**What conductor's sessions get instead of the user's instructions**: conductor's profiles,
`rules.md`, and a short **operator digest** the operator approves (`<records>/operator.md`): the few
user-level rules that must also hold inside conductor (confidentiality across organisations,
credentials, machine safety), so they are stated once for conductor and never inherited by accident.

### The contradiction check

`conductor doctor --context` (and a row in plain `doctor`) lists, per role, every mechanism that
would reach it (from § 1's table, read from the role's settings, flags and the user's files), and
flags:

- a source that reaches a role but is not on its allow-list (a plugin, an MCP server, a connector);
- a contradiction: conductor ships a list of **assertions**, each a topic, conductor's rule (file and
  line), and the phrases that contradict it (for example topic "usage limits", rule "resume at once",
  contradicting phrases "wait until … reset"). The check scans the user's memory files, skills,
  output styles and hook commands for them and names both sides: `usage limits: rules.md:542 says
  resume at once; ~/.claude/CLAUDE.md:88 says wait for the reset`.

| fork | options | recommended |
|---|---|---|
| how contradictions are found | (a) the assertion list, deterministic; (b) a model reads both sides | **(a)**, with (b) only as a manual `--deep` run |
| severity | warning, or `not ready` | **warning** for the user's files (they are the operator's), **not ready** for a source that reaches a role off its allow-list |

## 2. One intake, routed to roles

### Doors and the intake

Input arrives through several doors: the gateway (the operator's remote-control session), a
cross-session message from another session, a message a person types into a controller, a report on
the team's channel. All of it is **intent**, and one session receives it: **conductor**, the
orchestrator.

- Every door delivers to `conductor intake` (spec first: an `Intent` record: id, door, sender, text
  reference, received at). The gateway is a door, not an orchestrator: it forwards to conductor and
  shows conductor's answer; it does not plan or dispatch.
- A message a person types into a controller is forwarded by that controller to conductor's intake
  when it changes the dispatch's scope; a question within the dispatch is answered in place.
- Each intent gets one **scope decision** from conductor, recorded and shown on the board and the
  dashboard: which steps of § 2's cycle run, which are skipped, and why. An intent too small for a
  dispatch is answered directly and recorded as such.

### Roles and the steps they take

| step | role | today | becomes |
|---|---|---|---|
| product | **product manager**: writes the PRD and a first product design | nothing | new agent `product-manager` |
| architecture | **architect**: technical design, architecture decision records | conductor-dev and controllers write designs ad hoc; `ess:author` writes specifications | new agent `architect`, which uses `ess:author` for the specification part; ADR-level output is class O to approve |
| plan | **planner**: epics, stories, tasks | `aep:decomposer`, `aep:story-scoper`, the four `aep:plan-critic-*` | **the existing agents**: `planner` is the name of the step; the decomposer drafts, the scoper scopes, the critics review |
| implement | **implementer** | `aep:implementor` | the existing agent, unchanged |
| review | **reviewer** | `aep:adversary`, `aep:security-reviewer` | the existing agents; the security reviewer runs only on a change that touches a boundary |
| investigate | **investigator / troubleshooter**: incidents, defects, "why does X happen" | the `aep:investigating` and `aep:diagnosing` skills | a thin agent `investigator` over those skills |
| research | **researcher**: questions, comparisons, prior art | none | new agent `researcher` |

Three are new (product manager, architect, researcher), one is a thin wrapper over existing skills
(investigator), and the plan, implement and review steps are the existing AEP agents under step
names. Nothing is duplicated.

### Scoping

Conductor decides the steps per intent, and records them:

| intent | steps |
|---|---|
| a small change (one repository, no new noun, no contract change) | implement → review |
| a defect or incident | investigate → implement → review |
| a question | research (or investigate), answer; no dispatch |
| a feature with an existing PRD and settled architecture | plan → implement → review |
| a feature that needs both | product → architecture → plan → implement → review |

No planning rounds, re-scoping or plan-review rounds for work scoped as small. A controller that
finds the scope wrong reports it, and conductor records a new scope decision; it does not grow the
plan itself.

### Mapped to protocol steps

Each step carries a **protocol step id** (`product`, `architecture`, `plan`, `implement`, `review`,
`investigate`, `research`), recorded with the scope decision and with each dispatch. The engineering
protocols and the loom workflows that will replace this routing take the same ids, so a step moves
from "conductor dispatches a role" to "a workflow runs the protocol step" without a second
orchestration model to remove.

| fork | options | recommended |
|---|---|---|
| may a person talk to a controller directly | (a) yes, the controller forwards scope changes to the intake; (b) no, every door is conductor | **(a)** |
| where the new agents live | (a) in the product (`.agents/`, adapters in `.claude/agents/`); (b) in the AEP plugin | **(a)** for product manager and architect (conductor's process); **(b)** asked of AEP for researcher and investigator if it wants them |

## 3. Profiles extended by config, in layers

### Layers and precedence

`global → instance → repository → role`, later layers extending earlier ones:

- **global**: `~/.b10x/conductor/profiles.yaml`, optional, for every instance on the machine;
- **instance**: `roles[]` in the instance config;
- **repository**: `repositories[].roles[]` in the instance config;
- the **role**'s own profile file is the base.

```yaml
# instance config
roles:
  - role: implementer
    skills: [example-skill]                 # "when the implementer is used, also load skill X"
repositories:
  - match: alpha
    roles:
      - role: implementer
        skills: [rust-skill]                # "in repository alpha, the implementer also loads a Rust skill"
        append: ~/.b10x/conductor/extra/alpha-implementer.md
```

An extension may add skills, append a text file, and set a model. Lists add up across layers; an
appended text is added in layer order; a scalar (the model) is taken from the most specific layer.
Nothing removes what a lower layer gave, and no layer replaces the base profile; replacing a profile
stays the role's `profile:` key, as today.

### Validation

`conductor config validate` refuses, by YAML path: an unknown role (not a conductor role or a step
role of § 2), an unknown skill (not provided by a plugin the role's settings enable or by a skill
directory the role reads), an appended file that does not exist, a `match` that names no repository
of the instance's sources.

### Showing the effective profile

`conductor profile show --role implementer --repository alpha` prints the effective profile: the
base file, then each extension with the layer it came from, the skills in effect, and the model.

### How the extensions reach a session

| fork | options | recommended |
|---|---|---|
| controller and conductor roles | the effective profile is rendered into one file passed with `--append-system-prompt-file` at start (spawn and the start tasks) | this |
| worker roles (implementer, reviewer…), which a controller starts as subagents | (a) conductor renders effective agent definitions into the repository's `.claude/agents/` (git-excluded) at spawn, with the skills in their front matter; (b) the controller's brief tells the worker which skills to load | **(a)**: deterministic, visible in `profile show`, no brief to forget |

## Stories (after approval)

1. Isolation B: the role flags and settings `env`, the gateway as a role, a test per closed door.
2. The operator digest (`<records>/operator.md`) and the profiles reading it.
3. `doctor --context` with the assertion list.
4. `conductor intake` and the `Intent` record (spec first); the gateway and controllers forwarding to it.
5. Scope decisions recorded and shown, with protocol step ids.
6. The `product-manager`, `architect`, `researcher` and `investigator` agents.
7. Profile layers in the config (spec first), validation, and `profile show`.
8. Rendering effective profiles at start and effective worker agents at spawn.
9. Later: isolation A behind `isolation: config-dir`.

## Left out

- Changing the operator's own files: conductor reads and flags them; it never edits them.
- A model-driven router: scope decisions follow the table, and conductor records its reasons.
- Loom workflows themselves: loom's epic; this design only shares the step ids.
