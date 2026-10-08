---
format: aep.planning-md/3
id: story:docs-site
kind: story
status: active
title: Conductor has its own documentation site
scope:
- confidence: cited
  path: .github/workflows/b10x-docs-site.yml
- confidence: cited
  path: .github/workflows/pages.yml
- confidence: cited
  path: website/docs/concepts
- confidence: cited
  path: website/docs/guides
- confidence: cited
  path: website/docusaurus.config.ts
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T17:35:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T17:35:27Z", actor: "human:timo", revision: 3}
---
## Why

Conductor is public and has no site. The ecosystem's components each serve an independent site at
`https://beyond10x.github.io/<repo>/`, built with the shared Docs System, validated on every push
and deployed by the Website's reusable workflow (the shape of Loom's `website/` and workflows).

## Acceptance

- `website/` with `docusaurus.config.ts` (`url: https://beyond10x.github.io`, `baseUrl:
  /conductor/`, `projectName: conductor`), `package.json` pinning `@beyond10x/docs-system` to the
  commit the other independent sites pin, `package-lock.json`, `sidebars.ts`, `product.json`
  (landing data whose claims are checked like any page), `static/`.
- Hand-written pages with front matter (`title`, `sidebar_position`, `description`, and `lede`,
  `source` on concepts and guides): `index.md` (what it is, what it is not, its neighbours),
  `getting-started.md` (install, a config, `conductor config validate`, one snapshot, with output
  pasted from a real run in a scratch HOME), `concepts/` (roles, the loop, decision classes and
  messages, the guard and its limits, instances and config), `guides/` (configure an instance,
  start conductor, add a repository rule, read the dashboard).
- `.github/workflows/pages.yml` (`Documentation validation`: build the site, test the docs crate,
  `cargo run --locked -p conductor-docs -- provenance --site website/build --commit
  "$GITHUB_SHA"`, upload `b10x-project-site` on a push to `main`) and
  `.github/workflows/b10x-docs-site.yml` (`Documentation site`, Website's `project-site.yml` at the
  commit Loom's caller pins, `repository: conductor`, `route_base: /conductor/`, guarded on the
  repository and the bot).
- `npm --prefix website ci && npm --prefix website run build` exits 0 with the generated reference
  pages present.

## Scope

`website/**` except `website/docs/reference/**`, `website/data/status.json`,
`website/docs/status.md` (cited), `.github/workflows/pages.yml`, `.github/workflows/b10x-docs-site.yml`
(cited).
