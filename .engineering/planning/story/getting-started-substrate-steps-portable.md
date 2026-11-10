---
format: aep.planning-md/3
id: story:getting-started-substrate-steps-portable
kind: story
status: active
title: getting-started's substrate steps run as written on Arch and Manjaro
scope:
- confidence: inferred
  path: SETUP.md
- confidence: cited
  path: website/docs/getting-started.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:08Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:08Z", actor: "human:timo", revision: 5}
---
## Why

Reported by the operator session of a trial instance on 2026-11-10 following website/docs/getting-started.md at
v1.0.1 on Manjaro (the reporter's defect list items 13 and 14).

- `install -d -m 700` at getting-started.md:181 leaves the new parent ~/.local/state/substrate at
  0755; substrate refuses it ("state directory must be owner-controlled with mode 0700").
- `--ca-bundle /etc/ssl/certs/ca-certificates.crt` at getting-started.md:191 is a symlink on Arch
  and Manjaro; substrate refuses it ("must name one non-empty regular file"). The reporter used
  /etc/ca-certificates/extracted/tls-ca-bundle.pem.

## Acceptance

- getting-started.md's substrate steps create every directory they need at mode 0700 and name
  the CA bundle as a regular file on Debian, Fedora and Arch (or resolve the link), and run as
  written on a fresh host of each.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `website/docs/getting-started.md` section 5, "Start the substrate daemon" — cited
- **Files:** `website/docs/getting-started.md:181`, `website/docs/getting-started.md:191` — cited
- **Symbols:** none — cited, the story names two shell lines and no code symbol
- **Also likely:** `SETUP.md:144`, `SETUP.md:148` — inferred, step 9 gives the same mode-700 instruction and the same `--ca-bundle /etc/ssl/certs/ca-certificates.crt` Fix line, and has the same defect on Arch
- **Documents:** `website/docs/getting-started.md` — cited; `SETUP.md` — inferred. The story lands only in documents
- **Confidence:** high — the story names both defect lines and the tree matches them; the acceptance is entirely about a document
- **Would collide with:** any unit editing `website/docs/getting-started.md` (section 5 above all) or step 9 of `SETUP.md`
- **Safety fact:** no test reads section 5 of the page: the only test that opens `website/docs/getting-started.md` is `crates/conductor/tests/docs_forge.rs:124`, which reads the `$ conductor doctor` sample block only — step 2 (`git grep -n getting-started -- crates`), unproven
