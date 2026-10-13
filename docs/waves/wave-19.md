# Wave 19

Coordinator: conductor-dev. Approved up front (rules item 99; the operator, 2026-10-13: "fix it",
"yes, do it"). One managed tree, `conductor-wave-19`, branch `wave/19`. Target: v0.8.1. One pull
request.

## Units

| unit | story | from |
|---|---|---|
| A | installed-profiles | after v0.8.0 a conductor restart needed a git fast-forward in the product checkout |
| B | doctor-without-config | a user's `conductor doctor` on v0.7.0 with no config file: `gh: missing` |
| C | ssh-command-kept | the follow-up of `story:fetch-without-prompts`: a `core.sshCommand` was overridden |

## Outcome

`task check`, documentation and the Gates job green in CI on `470edafd5`
(https://github.com/beyond10x/conductor/actions/runs/38411386090); three stories moved to
`implemented` on that record. Released as v0.8.1. One test (`doctor_reports_each_program_present_with_its_version`)
read `gh` with no config file and was given a GitHub source.
