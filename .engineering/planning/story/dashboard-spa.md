---
format: aep.planning-md/3
id: story:dashboard-spa
kind: story
status: implemented
title: The dashboard is a Vue app embedded in the binary
relations:
- decomposes: epic:dashboard-for-engineers
- depends_on: story:conductor-upgrade
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: .gitignore
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/dashboard
- confidence: cited
  path: crates/conductor/dashboard/.nvmrc
- confidence: cited
  path: crates/conductor/dashboard/dist
- confidence: cited
  path: crates/conductor/dashboard/package-lock.json
- confidence: cited
  path: crates/conductor/dashboard/package.json
- confidence: cited
  path: crates/conductor/dashboard/src
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/instance_paths.rs
- confidence: cited
  path: crates/conductor/tests/usage.rs
- confidence: cited
  path: docs/design/dashboard-mockup.html
- confidence: cited
  path: website/docs/guides/read-the-dashboard.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:13:55Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-13T13:14:07Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-13T13:19:38Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Why

Part of `epic:dashboard-for-engineers` (the operator, 2026-10-13: "it can just be a vue SPA,
embedded into the rust binary"). Today `/` is one HTML string built in
`crates/conductor/src/dashboard.rs` (`page()`), beside `/data.json`. This story moves the page
into an app and changes nothing it shows; the views that use the snapshot come after it. Design
reference: `docs/design/dashboard-mockup.html`.

## Acceptance

- Build: a Vue 3 + Vite + TypeScript app in `crates/conductor/dashboard/`, its Node version pinned
  in `crates/conductor/dashboard/.nvmrc` and `package.json` `engines`; `task ui` runs `npm ci` and
  the build into `crates/conductor/dashboard/dist/`, which is committed.
- Gate: `task check` runs `npm ci`, the Vitest suite and `task ui` followed by a diff of
  `crates/conductor/dashboard/dist/` against the committed one (as the generated model is diffed),
  failing on any difference; `.github/workflows/check.yml` installs that Node version with
  `actions/setup-node` pinned by commit SHA before `task check`. `cargo install --git` still builds
  without Node.
- Embed and serve: `crates/conductor/dashboard/dist/` is embedded with `include_dir` and served
  at `/` (its `index.html`) and at each asset's path with its content type; `/data.json` is
  unchanged. The `Content-Security-Policy` header becomes
  `default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'`. The server stays on 127.0.0.1, GET only, Host-checked. `dashboard::serve` and
  `Sources` keep their signatures (`gc.rs`, `instance_paths.rs` and `adv_watch.rs` call them).
- Rust tests in `crates/conductor/tests/dashboard.rs`: `GET /` answers the embedded `index.html`;
  one asset answers with `text/javascript` and one with `text/css`; every answer carries the header
  above; a POST, a path outside the app and a foreign `Host` are refused as today; and the keys of
  the app's fixture `crates/conductor/dashboard/src/fixtures/data.json` are the keys `/data.json`
  serves for the test instance, so the fixture cannot drift from the real shape.
- Panels: the app renders each panel the old page had from `/data.json` — sessions, usage limit and
  context, red CI on main, awaiting the operator, recent decisions, disk waste, token spend and
  could not read (`errors`). One Vitest test per panel renders it from the fixture and finds the
  fixture's rows and figures (for sessions: each session's name, kind and state, and the
  `Sessions under <root> (<n> live, <m> exited)` heading).
- The tests that assert the old HTML move to `/data.json`: the `<h2>` and `<tr><td>` assertions in
  `crates/conductor/tests/dashboard.rs`, `instance_paths.rs` (`Sessions under …`) and `usage.rs`
  (`Token spend since …`) assert the same facts on the JSON instead, and `page()` and its HTML
  helpers are removed.
- `website/docs/guides/read-the-dashboard.md` describes the app and `task ui`.
- `docs/design/dashboard-mockup.html`, the design reference the dashboard stories cite, is created
  by this story (the coordinator copies it in as the story's first commit).
- `.gitignore` ignores `crates/conductor/dashboard/node_modules/`.

## Notes

- After `story:conductor-upgrade`: both edit `crates/conductor/Cargo.toml` (that one
  `repository.workspace`, this one `include_dir`).
- Before `story:dashboard-overview`, `story:dashboard-repo-view` and `story:dashboard-health`, which
  add views to this app, and before the sandbox stories: `story:sandbox-spec` depends on
  `story:dashboard-health`. Only `story:dashboard-engineer-view` waits for
  `story:session-forge-credentials`.

## Scope

`crates/conductor/dashboard/` (new: the app), `crates/conductor/dashboard/.nvmrc`,
`crates/conductor/dashboard/package.json`, `crates/conductor/dashboard/package-lock.json`,
`crates/conductor/dashboard/src/`, `crates/conductor/dashboard/dist/`,
`docs/design/dashboard-mockup.html` (new), `crates/conductor/src/dashboard.rs`,
`crates/conductor/Cargo.toml`, `Cargo.lock`, `Taskfile.yml`, `.github/workflows/check.yml`,
`.gitignore`, `crates/conductor/tests/dashboard.rs`, `crates/conductor/tests/instance_paths.rs`,
`crates/conductor/tests/usage.rs`, `website/docs/guides/read-the-dashboard.md`.
