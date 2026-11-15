# Wave 52 (2026-11-14: a boxed runtime dials its instance API out to conductord)

The operator approved this wave on 2026-11-14 (`approval-record:wave-2026-11-14-conductor`): one
story, committed on an integration branch, without a release. One pull request from
`wave/2026-11-14-conductor` into `main`, forked from `main` at `a79bb042` (release 1.9.0). The
wave before is [wave 51](wave-51.md).

## Units

| unit | story | waits for | stage |
|---|---|---|---|
| U1 | box-runtime-dials-conductord | substrate `story:sdk-aperture-list` (merged on substrate `main` at `befef3aa`) | on the wave branch |

## Outcome

- `daemon.dial_port` (absent 0, off): conductord listens on `127.0.0.1:<dial_port>`, and a runtime
  in a box on the local substrate daemon binds no `api.sock` (substrate refuses `socket(AF_UNIX)`
  in a box, its ADR 0039) but dials out to it through the aperture `conductord`.
- A local box's start names the apertures `conductord` and `conductor`, in that order, through
  the SDK's `apertures` builder; `b10x-substrate-sdk` is pinned to substrate `main` `befef3aa`.
- End-to-end on a host with substrate-daemon 0.20.0: the instance `running`, no `api.sock` bind
  error, the instance API answering through conductord over the dialed connections, and a run in
  the instance's workspace reaching the model host through the aperture `conductor`.
