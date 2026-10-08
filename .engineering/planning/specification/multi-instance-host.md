---
format: aep.planning-md/3
id: specification:multi-instance-host
kind: specification
status: draft
title: Several conductor instances on one host, and where substrate fits
revision: 1
---
## What this records

How several conductor instances share one machine, and where substrate fits. Read against the
code at `wave/02` (2026-10-08) and the host it runs on.

## Findings (each read from the tree or the host)

| # | finding | evidence |
|---|---|---|
| 1 | Session names are one namespace for the whole machine. Conductor is `conductor`, conductor-dev `conductor-dev`, a controller is named after its repository | `Taskfile.yml` `SESSION_ID` and `-n conductor`; `spec/domains/dispatch.yaml` `Controller.session_name` typed `RepositoryName`, set to `input.repository`; `claude agents --json` lists every session of the user, any instance and the operator's own |
| 2 | A second instance's `task conductor:restart` stops whichever background session is named `conductor`, this instance's or another's | `Taskfile.yml` `conductor:restart` vars `OLD`: `select(.name=="conductor")` |
| 3 | Two instances with a repository of the same name start two sessions of one name; a bare-name SendMessage then needs a `[ref]` to resolve, and the guard's SendMessage rule allows the name `conductor` without asking which | `crates/conductor/src/guard/rules.rs` `CONDUCTOR = "conductor"` |
| 4 | Records, state, cache, guard store and config validation are already per instance | `conductor config show`: `records`, `state`, `cache` per instance; the hook records in the instance's state |
| 5 | The dashboard binds 127.0.0.1:7313 for whatever instance is active | `Taskfile.yml` `dashboard` |
| 6 | Disk, CPU and memory are one budget; each conductor grants build slots against `/` alone, so two instances can each grant the same free space | `.agents/conductor.md` Dispatch, `thresholds.build_slot`; design § 4 |
| 7 | Substrate confines a process in a cgroup (cpu, memory, pids), reports exact usage per exec or session (`GET /v1/metrics`: cpu time, memory current and peak, processes, io bytes), and can enforce storage with filesystem project quotas | `~/beyond10x/substrate/README.md` (status table, "Capability limits"), `website/docs/guides/run-a-command.md` § 5 |
| 8 | This host: `/` is ext4 mounted without `prjquota`; the user service has the `cpu memory pids` controllers; `substrate-daemon` is installed | `findmnt -no FSTYPE,OPTIONS /` → `ext4 rw,noatime`; `cgroup.controllers` of `user@1000.service`; `~/.cargo/bin/substrate-daemon` |

## Design

1. **Instance-qualified session names.** Every session an instance starts is named
   `<prefix>-<role or repository>`; `prefix` is a per-instance config key (default: the instance
   name), unique on the host, validated by `config validate` across instances. Message first lines
   keep their role words (`[NEED conductor …]`); only the harness address changes.
2. **Instance-scoped tasks.** Every task acts on the active instance's sessions only, found by full
   name; the dashboard port is per instance.
3. **A host ledger.** A `host:` section of the config (shared disk floor and build line, most
   working controllers across instances) and a ledger in `~/.b10x/conductor/host/state`; each
   conductor takes a host grant before starting a controller or granting a large build, and
   releases it when the session leaves. Draft domain: `spec/drafts/host.yaml` (validated against a
   copy of `spec/` with it listed in `ess-inputs.yaml`: "8 file(s), 18 scenario(s), valid"). Its
   UNMAPPED markers: grant expiry, and the refusal rules as outcomes.
4. **The guard keeps instances apart.** A SendMessage from a session of instance A to a session of
   instance B is denied; conductor's own recipient is `<prefix>-conductor`.
5. **Substrate, in two steps.** (a) Usage: per-instance and per-session CPU, memory, io and
   process counts in the dashboard and the watch, read from the cgroups the sessions run in
   (substrate's metrics where a session runs under it, the cgroup files otherwise). (b) Limits:
   run each controller and its builds as a substrate session in a per-instance cgroup with cpu,
   memory and pids limits from config. Whether `claude` (OAuth credentials, network, the
   harness's own child processes) runs inside a substrate session is not known: a spike decides it.
6. **Storage quotas** need project quotas on the filesystem that holds checkouts and trees; this
   host's `/` has none. Until the operator remounts with `prjquota` or adds a volume for trees,
   storage stays policed by the host ledger and the watch, not enforced.
