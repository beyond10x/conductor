# Goal documents in the instance config

Status: design for approval, revision 2. No decomposition or implementation before it is approved.

Revision 2 adds: organisation documents read from a shared git repository (a direction source), a
git URL and a `ref` on any document, sources that are never managed, discovery in a direction source,
an author-placed end-of-excerpt marker, the `principles` kind, and citations by stable anchor id.

## Problem

Conductor works across many repositories. What steers it today is `NORTHSTAR.md` in the records
directory (design § 4, step 2 "Compare") and the goals in the store (`conductor goal …`,
`conductor.direction.Goal`). Neither knows the documents each repository and the organisation
already keep about where they are going — a vision, a PRD, a roadmap — and controllers plan and
implement from the dispatch brief and the repository alone. Work can be correct and still pull a
repository away from where the organisation wants it.

The organisation's goals should live in one shared repository that every engineer's instance reads,
never a local copy that goes stale until somebody pulls.

Wanted (the operator's requirements):

- **R1.** The instance config references organisation-level documents (north star, company goals,
  vision, principles) and, per repository, its goal documents (vision, PRD, goals, roadmap…).
  Organisation documents can come from a shared git repository, at a branch or a pinned tag.
- **R2.** `conductor setup` finds goal documents at standard paths, in each repository and in the
  configured direction source, and writes them into the config.
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
| **C. Beside, with citations (recommended)** | `NORTHSTAR.md` stays the short, operator-approved working statement; the configured documents are its sources. `NORTHSTAR.md` and each confirmed goal cite the document and the goal they come from | one more thing to keep consistent, checked below |

**Citations** name a document and either a stable anchor id or a heading:

- an anchor id is written by the document's author beside a heading or a list item, as
  `{#G-C1}` at the end of the heading line or `<a id="G-C1"></a>` anywhere; ids are letters, digits
  and `-`, unique within the document;
- a citation is `<document>#<anchor id>` (preferred: survives renaming the heading) or
  `<document>#<heading text>` (works without anchors).

`conductor goal propose-goal` gains an optional `--source <citation>`. The board flags a citation
whose anchor or heading is gone, and an organisation document that `NORTHSTAR.md` cites nothing of.

### D2. Config shape

Three additions, all optional, so every existing config stays valid unchanged:

```yaml
version: conductor.config/1
instances:
  - name: example-org
    direction:
      sources:                              # repositories that only serve direction (new)
        - name: goals
          url: git@gitlab.example.com:example-org/goals.git
          ref: v2026.10                     # optional: a branch or a reviewed tag; default the default branch
      documents:                            # organisation-level documents (new)
        - {source: goals, path: north-star.md, kind: north-star}
        - {source: goals, path: principles.md, kind: principles}
        - {source: goals, path: 2026/goals.md, kind: goals, ref: main}   # this one follows main
        - {path: ~/example-org/notes/vision.md, kind: vision}            # a local file still works
    repositories:
      - match: alpha                        # existing rule; `activity` becomes optional
        documents:                          # this repository's goal documents (new)
          - {path: VISION.md, kind: vision}
          - {path: docs/prd.md, kind: prd, ref: release/1.x}
      - match: beta
        activity: inactive
        documents: []                       # stated: this repository has none (silences the report)
```

**A document entry** has `path` and `kind`, and one of:
- `source: <name>`: a direction source; `path` is inside it;
- `url: <git url>`: a repository named inline, read like a source (for a one-off document);
- neither: an organisation document is an absolute or `~` path on this machine; a repository
  document is a path inside that repository.

`ref` (branch or tag) is optional on every entry and on every source; an entry's `ref` wins over its
source's; without either, the repository's default branch.

**A direction source** is a repository that only serves direction: it is fetched into the instance's
cache (`<cache>/direction/<name>`), never checked out under `checkouts.root`, never in the snapshot's
repositories, never dispatched to, and no controller is started for it. A source may instead be a
repository conductor already manages (`repository: <path under checkouts.root>`, like the catalog);
then it is read from that checkout's `origin/<ref>`, and stays managed as before.

**`kind`** is one of `north-star`, `vision`, `mission`, `principles`, `goals`, `okrs`, `strategy`,
`roadmap`, `prd`, `other`. It orders documents in the context, in that order, and names them in
reports.

**Spec first:** `conductor.config.Direction`, `DirectionSource`, `GoalDocument` and `DocumentKind` in
`spec/domains/config.yaml`, the `repositories[].documents` field, and `activity` made optional.

**Migration.** Nothing breaks: every key is optional. `conductor setup --apply`, and a new
`conductor config discover-documents [--apply]`, add what discovery finds, rule by rule, never
removing an entry a person wrote. `conductor config validate` names the new keys like every other.

### D3. Freshness: never a stale copy

| option | cost |
|---|---|
| read a local clone the engineer keeps | goes stale until they pull: what the operator does not want |
| fetch on every read | always current; every dispatch waits on the forge |
| **fetch when older than a bound, read the fetched ref (recommended)** | current within minutes; a dispatch never waits on a slow forge for long |

Every snapshot fetches each direction source (and each repository's goal documents come with the
repository's own fetch). `conductor direction context` (D6) fetches a source again when its last fetch
is older than 10 minutes. Fetches are prompt-free and bounded like the collectors' (`GIT_TERMINAL_PROMPT=0`,
ssh in batch mode, the collectors' time bound); a fetch that fails keeps the last fetched text and
marks it stale with its age (D5). Documents are always read from the fetched `origin/<ref>` (or the
tag), never from a working tree, so an uncommitted local edit never enters the context. A pinned tag
is fetched once and never moves.

### D4. Discovery at setup (R2)

Searched, on the default branch (or the configured `ref`), in each managed repository and in each
direction source: the root, `docs/`, `docs/product/` and `.github/`, not recursive below them. In a
direction source, the root's subdirectories are searched one level deep too (`2026/goals.md`).

| kind | file names (stem) |
|---|---|
| north-star | `NORTHSTAR`, `NORTH_STAR`, `NORTH-STAR` |
| vision | `VISION` |
| mission | `MISSION` |
| principles | `PRINCIPLES`, `VALUES` |
| goals | `GOALS`, `OBJECTIVES` |
| okrs | `OKRS`, `OKR` |
| strategy | `STRATEGY` |
| roadmap | `ROADMAP` |
| prd | `PRD`, `PRODUCT`, `REQUIREMENTS` |

Match rule: the file name compared case-insensitively, with `-`, `_` and nothing between words
treated alike (`north-star.md`, `North_Star.md`, `NORTHSTAR.md`), extension `.md` or `.markdown`
only. `README.md`, `CHANGELOG.md` and anything under `node_modules/`, `vendor/` or a hidden directory
other than `.github/` are never matched. Two candidates of one kind are both listed.

What a repository holds is added to that repository's rule; what a direction source holds is added to
`direction.documents` with its `source`. With no direction source configured, setup prints the
`direction:` block to fill in.

### D5. Validation and where each result shows (R3)

The whole file is validated by `conductor config validate` and on every load, as today; the new
keys are validated with it, each refusal naming its YAML path.

| finding | severity | where it shows |
|---|---|---|
| the config does not parse or breaks the schema (any key), a document names an unknown source, or both `source` and `url` | **refuse**: every command but `config`, `doctor`, `init`, `trust` stops (as today) | stderr of every command; `conductor doctor` `not ready` |
| a direction source cannot be fetched, or its `ref` does not exist | **not ready**, not a refusal | `conductor doctor` row per source; `STATUS.md` deviations; the watch once per change |
| a referenced document is missing, unreadable, not UTF-8, or not at its ref | **not ready**, not a refusal: the loop keeps running without that document | `conductor doctor` row per path; `STATUS.md` deviations; the watch once per change; `send-dispatch` prints one warning line naming it |
| a fetched document is older than one day because fetches fail | **deviation** | `STATUS.md` (`goals: <source> stale since <time>`) and the Direction block marks it |
| a repository has no `documents` key (not configured) | **deviation** | `STATUS.md` deviations (`<repo>: no goal documents`); `conductor doctor` counts them in one line; `conductor setup` names the discovery command |
| a document is over the size limit (D6) and has no end-of-excerpt marker | **warning** | `conductor doctor`; the context says it was cut and where |
| `NORTHSTAR.md` cites no configured organisation document, a citation's anchor or heading is gone, or an anchor id is used twice in one document | **deviation** | `STATUS.md` deviations |

Refusing to start over a missing goal document would stop every repository for one file; the loop
keeps going and the gap stays visible on the board and in doctor until it is fixed.

### D6. How documents enter the context (R4)

**Where it goes.** One block, "Direction", rendered by the binary so every consumer gets the same
text: `conductor direction context --repository <repo>` prints `NORTHSTAR.md`, the organisation
documents and that repository's documents, in kind order, each headed by its kind, its source and ref,
and its path. It is used in:

- **dispatch briefs**: conductor appends it to every brief (the profile says so; `send-dispatch`
  records the documents' digests with the dispatch);
- **controller start**: `conductor spawn` passes it with the charter;
- **AEP planning**: the controller profile says an epic names the goal it serves (a citation, D1) in
  its body, and the plan-critic scope review checks the set against the Direction block;
- **review** (later, not in the first stories): the adversary review may read it.

**How much.**

| option | cost |
|---|---|
| whole file | simplest; large PRDs crowd out the work |
| link only | cheapest; the model has to choose to read it |
| **excerpt chosen by the author, plus link (recommended)** | the author decides what steers the work; bounded; the full file stays one read away |

The excerpt is the document from its start to the author's end-of-excerpt marker, the line
`<!-- conductor: end of excerpt -->`. Without a marker: to the second level-2 heading or 4 KB,
whichever is shorter, cut at a line boundary. Either way, the path and "read the rest at <path>"
follow. A marker past 8 KB is cut at 8 KB and reported (D5). The whole Direction block is capped at
16 KB; past it, documents keep only their title, path and first paragraph, in kind order from the
end. No summarising model: the same files at the same refs give the same block.

**When it is read.** On every `direction context` call, from the fetched refs (D3). The snapshot
records each document's source, ref, path, size and digest
(`conductor.observation.GoalDocumentObservation`), which is what the deviations of D5 read and what
lets the board say "changed since the last dispatch to <repo>".

## Stories this would become (after approval)

1. Spec: the config keys (sources, documents, ref, kinds), the observation, the `direction context`
   view.
2. Config: parse, validate and show the keys; `activity` optional.
3. Direction sources: fetch into the cache, read at a ref, staleness.
4. Discovery: `conductor setup` and `conductor config discover-documents`, in repositories and
   sources.
5. Snapshot: the goal-document collector and its observation.
6. Validation: doctor rows, `STATUS.md` deviations, the watch line.
7. `conductor direction context` (excerpt marker, caps) and its use in `send-dispatch` and `spawn`.
8. Citations: anchor ids and headings, `goal propose-goal --source`, the broken-citation deviation.
9. Profiles: conductor appends the Direction block; controllers name the goal an epic serves; the
   scope critic reads it.

## Left out on purpose

- Writing or editing goal documents: conductor reads them only.
- Summaries written by a model: the context is an excerpt, so it is reproducible.
- Enforcing alignment: no gate refuses work for not citing a goal; the board shows it.
- Goal metrics, OKR scoring or progress tracking against the documents.
- Documents that are not markdown files in a git repository or on the machine: wikis, PDFs, shared
  drives, web pages.
- Discovery of organisation documents outside a configured direction source.
- Pushing to a direction source: conductor fetches it and never writes.
