# Config

The config file is described in two places on the site:

- [Config](https://beyond10x.github.io/conductor/docs/concepts/config): how a command finds the
  file and the instance, the levels, `config put` and `config migrate`, session names, and which
  part of conductor reads each key.
- [Config reference](https://beyond10x.github.io/conductor/docs/reference/config): every key of
  `conductor.config/2` with its type, default and rule, generated from `spec/domains/config.yaml`.

A full example is [`../conductor.example.yaml`](../conductor.example.yaml).

## Keys of a `conductor.config/1` file

A `/1` file loads for one release so a running instance keeps working; conductord does not read
it, and `conductor config migrate` converts it. Each version refuses the other's keys by their
YAML path. These keys exist only in a `/1` file:

| key | read by | meaning |
|---|---|---|
| `instances[].dashboard_port` | nothing | read and checked (1 to 65535, no two instances sharing one, 7313 plus the instance's position when absent) but not used |
| `instances[].store` | nothing | ignored; `config migrate` drops it |
| `confinement.mode` | `conductor session start`, `conductor spawn` | `strict` starts no session whose box cannot be set up, and stops one measured short of the request; `report`, the default, starts it unboxed and records why |
| `confinement.provider` | `conductor session start`, `conductor spawn`, `conductor controller resume-controller`, `conductor doctor` | `systemd`, the default, is no longer served: conductor confines no session itself, so under `report` a session starts unboxed through `none` (`claude --bg`); `none` sets up no box |
| `confinement.limits.*` | as `instances[].limits` | the instance's limits; `config migrate` moves them to `instances[].limits` |
| `~/.b10x/conductor/host/host.yaml` | `conductor config migrate`, once | the host's roles; they become `globals.roles` |
