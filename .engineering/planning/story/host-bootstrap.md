---
format: aep.planning-md/3
id: story:host-bootstrap
kind: story
status: archived
title: A fresh or rebooted host gets the loop, the dashboards and the host sessions' files back from the repository
summary: loop and dashboard units are transient; host files are in no git
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:21:27Z", actor: "human:timo", revision: 3}
---
## Why

Observed 2026-11-07 on one host: only the product repository and the instances' pushed records
are in git. A fresh host, or the same host after a reboot, keeps none of these:

- the host-level supervisor and teacher agent files under `~/.claude/agents/`;
- the role profiles under `~/.b10x/conductor/profiles/` as installed (`task profiles` writes two of
  the three from this repository; the third, `conductor.md`, has no source here);
- the config file `~/.b10x/conductor/conductor.yaml`;
- the usage-rotation watcher's user unit and its threshold drop-in;
- the disk keeper under `~/.local/share/`;
- the loop and dashboard units: `conductor run --install` and the dashboard task start them with
  `systemd-run`, so their unit files are transient (under `/run/user/<uid>/systemd/transient`) and
  are gone after a logout or a reboot.

## Acceptance

- `conductor run --install` and the dashboard's install write a unit file under
  `~/.config/systemd/user/` and enable it, so the loop and the dashboard start again after a
  reboot; `--uninstall` disables and removes it. A test over a temporary home reads the unit file.
- The repository carries generic sources for the supervisor and teacher agent files, the profiles
  the instance roles start from, a commented `conductor.yaml` template, and the units of the
  usage watcher and the disk keeper as templates; none names an organization, a person or a host
  (AGENTS.md rules 4 and 9). Running code stays Rust (rule 3): a script among them becomes a
  `conductor` subcommand or stays out.
- `conductor init` (or a `conductor host bootstrap`) installs what is missing on a host from those
  sources, never overwrites an existing file, and prints one line per file: installed, kept or
  differs.
- Hand-overs and state stay out of the repository; the record says where each lives and that it
  is not restored.

## Notes

- Requested by the host supervisor, 2026-11-07.

## Archived

Superseded by story:roles-in-the-binary: profiles ship in the binary, init installs no adapter under ~/.claude/agents/ (crates/conductor/src/init.rs:11).
