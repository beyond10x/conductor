# Parity of the generated HTTP servers with the hand-written API

Written by `tests/api_parity.rs` (`CONDUCTOR_API_PARITY_BLESS=1`); compared line for line.

Probes: 4 match, 29 differ, 13 unbound; 30 routes spec-only.

| route | binding | probe | hand-written | generated | verdict |
|---|---|---|---|---|---|
| `GET /v1/instances` | conductor.instance.Instances | read | `200 [1]` | `200 [0]` | differs (body) |
| `GET /v1/secrets` | conductor.secret.Secrets | read | `200 [0]` | `200 [0]` | match |
| `GET /v1/instances/{name}/tokens` | conductor.token.Tokens | read | `200 [1]` | `200 [1]` | differs (body) |
| `GET /v1/instances/{name}/settings` | conductor.config.EffectiveSettings | read | `200 {errors,fields,file,index,instance,instances,repositories,writable}` | `501 {committed,refused}` | differs (status, body) |
| `GET /v1/instances/{name}/secrets` | conductor.secret.InstanceSecrets | read | `200 [0]` | `501 {committed,refused}` | differs (status, body) |
| `GET /readyz` | conductor.store.Readiness | read | `200 [1]` | `200 [0]` | differs (body) |
| `GET /v1/version` | conductor.store.Version | read | `200 [1]` | `200 [1]` | match |
| `PUT /v1/config` | conductor.config.PutConfig | client | `200 {actor,commit,instance,level,path,reason,revision_id,role,value}` | `400 {refused}` | differs (status, body, store) |
| `PUT /v1/config` | conductor.config.PutConfig | spec | `400 {error}` | `501 {committed,refused}` | differs (status, body) |
| `PUT /v1/config` | conductor.config.PutConfig | spec level-scope-mismatch | `400 {error}` | `422 {error,outcome,payload,published}` | differs (status, body) |
| `POST /v1/instances/{name}/upgrade` | conductor.instance.UpgradeInstance | client | `503 {error}` | `400 {refused}` | differs (status, body) |
| `POST /v1/instances/{name}/upgrade` | conductor.instance.UpgradeInstance | spec | `400 {error}` | `404 {error,outcome,payload,published}` | differs (status, body) |
| `GET /v1/instances/{name}` | conductor.instance.InstanceByName | read | `200 {name,state}` | `404 empty` | differs (status, body) |
| `POST /v1/instances` | conductor.config.AddInstance | spec | `422 {detail,error,path}` | `501 {committed,refused}` | differs (status, body) |
| `PUT /v1/instances/{name}` | conductor.config.PutInstance | spec | `422 {detail,error,path}` | `501 {committed,refused}` | differs (status, body) |
| `DELETE /v1/instances/{name}` | conductor.config.RemoveInstance | spec | `404 {error,name}` | `501 {committed,refused}` | differs (status, body) |
| `POST /v1/instances/{name}/start` | conductor.instance.StartInstance | client | `404 {error}` | `400 {refused}` | differs (status, body) |
| `POST /v1/instances/{name}/start` | conductor.instance.StartInstance | spec | `404 {error}` | `501 {committed,refused}` | differs (status, body) |
| `POST /v1/instances/{name}/stop` | conductor.instance.StopInstance | client | `404 {error}` | `400 {refused}` | differs (status, body) |
| `POST /v1/instances/{name}/stop` | conductor.instance.StopInstance | spec | `404 {error}` | `404 {error,outcome,payload,published}` | differs (body) |
| `POST /v1/instances/{name}/restart` | conductor.instance.RestartInstance | client | `503 {error}` | `400 {refused}` | differs (status, body) |
| `POST /v1/instances/{name}/restart` | conductor.instance.RestartInstance | spec | `503 {error}` | `404 {error,outcome,payload,published}` | differs (status, body) |
| `POST /v1/agents/{name}/deliver` | conductor.agent.DeliverToAgent | client | `404 {detail,error}` | `400 {refused}` | differs (status, body) |
| `POST /v1/agents/{name}/deliver` | conductor.agent.DeliverToAgent | spec | `400 {error}` | `501 {committed,refused}` | differs (status, body) |
| `POST /v1/agents/{name}/restart` | conductor.agent.RestartAgent | client | `404 {detail,error}` | `400 {refused}` | differs (status, body) |
| `POST /v1/agents/{name}/restart` | conductor.agent.RestartAgent | spec | `404 {detail,error}` | `404 {error,outcome,payload,published}` | differs (body) |
| `GET /` | unbound: the dashboard app's files | read | `200 text` | `404 {refused}` | unbound |
| `GET /v1/sessions` | unbound: a union of every instance's answer; no specification view | read | `200 {rows}` | `404 {refused}` | unbound |
| `GET /v1/decisions` | unbound: a union of every instance's answer; no specification view | read | `200 {rows}` | `404 {refused}` | unbound |
| `GET /v1/unanswered` | unbound: a union of every instance's answer; no specification view | read | `200 {rows}` | `404 {refused}` | unbound |
| `GET /v1/kpis` | unbound: a union of every instance's answer; no specification view | read | `200 {rows}` | `404 {refused}` | unbound |
| `POST /v1/ask` | unbound: a question delivered to the supervisor; no command | client | `503 {detail,error,instance}` | `404 {refused}` | unbound |
| `POST /v1/roles/{role}/start` | unbound: a host role asked for now; no command | client | `404 {error}` | `404 {refused}` | unbound |
| `POST /v1/instances/{name}/join` | unbound: JoinInstance's instance is a member of `key`, which no path segment carries | client | `400 {error}` | `404 {refused}` | unbound |
| `POST /v1/instances/{name}/leave` | unbound: LeaveInstance's instance is a member of `key`, which no path segment carries | client | `200 {instance,operator,resume,withdrawn}` | `404 {refused}` | unbound |
| `GET /v1/instances/{name}/<route>` | unbound: proxied to the instance runtime; no operation | read | `200 {errors,history,open}` | `404 {refused}` | unbound |
| `POST /v1/instances/{name}/answers` | unbound: proxied to the instance runtime; a batch of decisions | client | `400 text` | `404 {refused}` | unbound |
| `GET /healthz` | unbound: the liveness probe reads no entity; a view has a source | read | `200 {state}` | `404 {refused}` | unbound |
| `GET /metrics` | unbound: Prometheus text; a view answers JSON rows | read | `200 text` | `404 {refused}` | unbound |
| `GET /v1/instances` | conductor.instance.Instances | read after the writes | `200 [1]` | `200 [0]` | differs (body) |
| `GET /v1/secrets` | conductor.secret.Secrets | read after the writes | `200 [0]` | `200 [0]` | match |
| `GET /v1/instances/{name}/tokens` | conductor.token.Tokens | read after the writes | `200 [1]` | `200 [1]` | differs (body) |
| `GET /v1/instances/{name}/settings` | conductor.config.EffectiveSettings | read after the writes | `200 {errors,fields,file,index,instance,instances,repositories,writable}` | `501 {committed,refused}` | differs (status, body) |
| `GET /v1/instances/{name}/secrets` | conductor.secret.InstanceSecrets | read after the writes | `200 [0]` | `501 {committed,refused}` | differs (status, body) |
| `GET /readyz` | conductor.store.Readiness | read after the writes | `200 [1]` | `200 [0]` | differs (body) |
| `GET /v1/version` | conductor.store.Version | read after the writes | `200 [1]` | `200 [1]` | match |
| `GET /config/views/config-revisions` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /instance/commands/observe` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /instance/commands/purge` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /instance/commands/reconcile` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `GET /instance/views/reconciliations` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /operator/commands/join-instance` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /operator/commands/leave-instance` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `GET /operator/views/operators` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /placement/commands/close-box` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /placement/commands/open-box` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `GET /placement/views/box-refusals` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `GET /placement/views/boxes` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `GET /placement/views/placements` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /secret/commands/put-secret` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /secret/commands/resolve-secrets` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /store/commands/migrate-store` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `GET /store/views/migrations` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `GET /store/views/store-status` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /token/commands/contribute-token` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /token/commands/lease-token` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /token/commands/mark-token-limited` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /token/commands/release-token` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /token/commands/withdraw-token` | derived (conductord); no hand-written route | - | `-` | `-` | spec-only |
| `POST /agent/commands/attach` | derived (conductor-instance); no hand-written route | - | `-` | `-` | spec-only |
| `POST /agent/commands/declare-kind` | derived (conductor-instance); no hand-written route | - | `-` | `-` | spec-only |
| `POST /agent/commands/record-report` | derived (conductor-instance); no hand-written route | - | `-` | `-` | spec-only |
| `POST /agent/commands/start` | derived (conductor-instance); no hand-written route | - | `-` | `-` | spec-only |
| `POST /agent/commands/stop` | derived (conductor-instance); no hand-written route | - | `-` | `-` | spec-only |
| `GET /agent/views/agent-kinds` | derived (conductor-instance); no hand-written route | - | `-` | `-` | spec-only |
| `GET /agent/views/agents` | derived (conductor-instance); no hand-written route | - | `-` | `-` | spec-only |
