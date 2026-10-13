# Goal documents in the instance config

Status: design for approval. No decomposition or implementation before it is approved.

## Problem

Conductor works across many repositories. What steers it today is `NORTHSTAR.md` in the records
directory (design § 4, step 2 "Compare") and the goals in the store (`conductor goal …`,
`conductor.direction.Goal`). Neither knows the documents each repository and the organisation
already keep about where they are going — a vision, a PRD, a roadmap — and controllers plan and
implement from the dispatch brief and the repository alone. Work can be correct and still pull a
repository away from where the organisation wants it.

Wanted (the operator's requirements):

- **R1.** The instance config references organisation-level documents (north star, company goals,
  vision) and, per repository, its goal documents (vision, PRD, goals, roadmap…), by path.
- **R2.** `conductor setup` finds goal documents at standard paths and writes them into the config.
- **R3.** The whole config is validated; a missing or unreadable referenced file is named by path; a
  repository with no goal documents is reported.
- **R4.** When conductor or a controller plans or implements in a repository, the north star and that
  repository's goals are part of the context: dispatch briefs, AEP planning, and review.

## Decisions to take

Each fork lists its options and a recommendation. The recommendation is what gets built unless the
operator picks otherwise.

### D1. How this relates to the north star conductor holds

| option | what it means | cost |
|---|---|---|
| A. Replace | the configured documents are the north star; `NORTHSTAR.md` goes | long documents become conductor's working statement; no single page says what conductor optimises for now |
| B. Source | `NORTHSTAR.md` is generated from the configured documents | generation needs summarising; a generated file nobody owns drifts from what the operator approved |
| **C. Beside, with citations (recommended)** | `NORTHSTAR.md` stays the short, operator-approved working statement; the configured documents are its sources. `NORTHSTAR.md` and each confirmed goal cite the document (path and heading) they come from | one more thing to keep consistent, checked below |

With C, `conductor goal propose-goal` gains an optional `--source <document>#<heading>`, and the
board flags a goal whose source document no longer has that heading, and an organisation document
that `NORTHSTAR.md` does not cite.

### D2. Config shape

Two additions, both optional, so every existing config stays valid unchanged:

```yaml
version: conductor.config/1
instances:
  - name: example-org
    direction:                      # organisation-level documents (new)
      documents:
        - {path: ~/example-org/company/north-star.md, kind: north-star}
        - {path: ~/example-org/company/goals-2026.md, kind: goals}
    repositories:
      - match: alpha                # existing rule; `activity` becomes optional
        documents:                  # this repository's goal documents (new)
          - {path: VISION.md, kind: vision}
          - {path: docs/prd.md, kind: prd}
      - match: beta
        activity: inactive
        documents: []               # stated: this repository has none (silences the report)
```

- `kind` is one of `north-star`, `vision`, `mission`, `goals`, `okrs`, `prd`, `roadmap`, `strategy`,
  `other`. It orders documents in the context (north-star, vision, mission, goals, okrs, strategy,
  roadmap, prd, other) and names them in reports.
- An organisation document's `path` is absolute or `~`-relative. A repository document's `path` is
  relative to the repository's root and read from its default branch (D5).
- `documents` absent means "not configured" (reported, D3); `documents: []` means "has none, on
  purpose" (not reported).
- Spec first: `conductor.config.Direction`, `conductor.config.GoalDocument` and `DocumentKind` in
  `spec/domains/config.yaml`, the `repositories[].documents` field, and `activity` made optional.

**Migration.** Nothing breaks: both keys are optional. `conductor setup --apply` (and a new
`conductor config discover-documents [--apply]`, D4 of R2) adds what discovery finds to an existing
instance, rule by rule, never removing an entry a person wrote. `conductor config validate` names
the new keys like every other (D3).

### D3. Discovery at setup (R2)

Searched, per repository, on its default branch: the root, `docs/`, `docs/product/` and `.github/`.
Not recursive below those.

| kind | file names (stem) |
|---|---|
| north-star | `NORTHSTAR`, `NORTH_STAR`, `NORTH-STAR` |
| vision | `VISION` |
| mission | `MISSION` |
| goals | `GOALS`, `OBJECTIVES` |
| okrs | `OKRS`, `OKR` |
| prd | `PRD`, `PRODUCT`, `REQUIREMENTS` |
| roadmap | `ROADMAP` |
| strategy | `STRATEGY` |

Match rule: the file name compared case-insensitively, with `-`, `_` and nothing between words
treated alike (`north-star.md`, `North_Star.md`, `NORTHSTAR.md`), extension `.md` or `.markdown`
only. `README.md`, `CHANGELOG.md` and anything under `node_modules/`, `vendor/` or a hidden directory
other than `.github/` are never matched. Two candidates of one kind are both listed.

`conductor setup` (read-only) prints what it would add; `--apply` writes it. Organisation documents
are not discovered: nothing standard says where they live, so setup prints the `direction:` block
to fill in.

### D4. Validation and where each result shows (R3)

The whole file is validated by `conductor config validate` and on every load, as today; the new
keys are validated with it, each refusal naming its YAML path.

| finding | severity | where it shows |
|---|---|---|
| the config does not parse or breaks the schema (any key) | **refuse**: every command but `config`, `doctor`, `init`, `trust` stops (as today) | stderr of every command; `conductor doctor` `not ready` |
| a referenced document is missing, unreadable, not UTF-8, or not on the default branch | **not ready**, not a refusal: the loop keeps running without that document | `conductor doctor` row per path (`goal document: <path> missing`); `STATUS.md` deviations; the watch once per change; `send-dispatch` prints one warning line naming it |
| a repository has no `documents` key (not configured) | **deviation** | `STATUS.md` deviations (`<repo>: no goal documents`); `conductor doctor` counts them in one line; `conductor setup` names the discovery command |
| a document is over the size limit (D5) | **warning** | `conductor doctor`; the context says it was cut and where |
| `NORTHSTAR.md` cites no configured organisation document, or a goal's `--source` heading is gone (D1) | **deviation** | `STATUS.md` deviations |

Refusing to start over a missing goal document would stop every repository for one file; the loop
keeps going and the gap stays visible on the board and in doctor until it is fixed.

### D5. How documents enter the context, and when they are read (R4)

**Where it goes.** One block, "Direction", rendered by the binary so every consumer gets the same
text: `conductor direction context --repository <repo>` prints the north star (`NORTHSTAR.md`), the
organisation documents and that repository's documents, in kind order. It is used in:

- **dispatch briefs**: conductor appends it to every brief (the profile says so; `send-dispatch` records
  the documents' digests with the dispatch);
- **controller start**: `conductor spawn` passes it with the charter;
- **AEP planning**: the controller profile says an epic names the goal it serves (document and
  heading) in its body, and the plan-critic scope review checks the set against the Direction block;
- **review** (later, not in the first stories): the adversary review may read it.

**How much.**

| option | cost |
|---|---|
| whole file | simplest; large PRDs crowd out the work |
| link only | cheapest; the model has to choose to read it |
| **excerpt plus link (recommended)** | deterministic and bounded; the full file stays one read away |

The excerpt is the document from its start to the second level-2 heading, or its first 4 KB, whichever
is shorter, cut at a line boundary, followed by the path and "read the rest at <path>". The whole
Direction block is capped at 16 KB; past it, documents keep only their title, path and first
paragraph, in kind order from the end. No summarising model: the same files give the same block.

**When it is read.** On every `direction context` call, from the default branch (`git show
origin/<default>:<path>` for a forge source, the file for a `local` source), so a dispatch always
carries the current text. The snapshot records each document's path, size and digest
(`conductor.observation.GoalDocumentObservation`), which is what the deviations of D4 read and what
lets the board say "changed since the last dispatch to <repo>".

## Stories this would become (after approval)

1. Spec: the config keys, the observation, the `direction context` view.
2. Config: parse, validate and show the keys; `activity` optional.
3. Discovery: `conductor setup` and `conductor config discover-documents`.
4. Snapshot: the goal-document collector and its observation.
5. Validation: doctor rows, `STATUS.md` deviations, the watch line.
6. `conductor direction context` and its use in `send-dispatch` and `spawn`.
7. Profiles: conductor appends the Direction block; controllers name the goal an epic serves; the
   scope critic reads it; `goal propose-goal --source`.

## Left out on purpose

- Writing or editing goal documents: conductor reads them only.
- Summaries written by a model: the context is an excerpt, so it is reproducible.
- Enforcing alignment: no gate refuses work for not citing a goal; the board shows it.
- Goal metrics, OKR scoring or progress tracking against the documents.
- Documents outside a repository or an organisation path: URLs, wikis, PDFs, shared drives.
- Organisation documents shared across instances on one host.
- Discovery of organisation documents (no standard place).
