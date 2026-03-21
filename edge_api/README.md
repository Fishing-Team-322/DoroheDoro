# edge-api

`edge_api/` is the public boundary service for the platform:

- `WEB -> edge-api -> NATS -> server-rs`
- `AGENT -> edge-api gRPC -> NATS -> server-rs`
- `WEB streams <- edge-api SSE <- NATS`

The Go service now stays on transport/security/bridge duties. Legacy PoC packages for enrollment, policy, diagnostics, query, ingest, indexers and telemetry are excluded from the default build with `legacy` build tags and are no longer part of the active runtime path.

## Active responsibilities

- HTTP ingress for `WEB`
- SSE gateway for UI streams
- gRPC ingress for `AGENT`
- TLS/mTLS termination for agent gRPC
- request validation
- request ID propagation
- NATS request/reply and publish bridge
- transport error mapping
- health/readiness/version endpoints
- local DEV auth compatibility endpoints for the frontend only

## Not owned here

`edge_api` is not the source of truth for:

- enrollment state
- policy state
- diagnostics state
- query logic
- deployment logic
- indexing pipelines

Those belong in `server-rs`.

## Current NATS alignment

The active live bridge is aligned with `server-rs` on:

- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`
- `agents.list`
- `agents.get`
- `agents.diagnostics.get`
- `agents.policy.get`
- `control.policies.list`
- `control.policies.get`
- `control.policies.create`
- `control.policies.update`
- `control.policies.revisions`
- `control.hosts.list`
- `control.hosts.get`
- `control.hosts.create`
- `control.hosts.update`
- `control.host-groups.list`
- `control.host-groups.get`
- `control.host-groups.create`
- `control.host-groups.update`
- `control.credentials.list`
- `control.credentials.get`
- `control.credentials.create`
- `control.clusters.list`
- `control.clusters.get`
- `control.clusters.create`
- `control.clusters.update`
- `control.clusters.add-host`
- `control.clusters.remove-host`
- `control.roles.list`
- `control.roles.get`
- `control.roles.create`
- `control.roles.update`
- `control.roles.permissions.get`
- `control.roles.permissions.set`
- `control.role-bindings.list`
- `control.role-bindings.create`
- `control.role-bindings.delete`
- `control.integrations.list`
- `control.integrations.get`
- `control.integrations.create`
- `control.integrations.update`
- `control.integrations.bind`
- `control.integrations.unbind`
- `tickets.list`
- `tickets.get`
- `tickets.create`
- `tickets.assign`
- `tickets.unassign`
- `tickets.comment.add`
- `tickets.status.change`
- `tickets.close`
- `anomalies.rules.list`
- `anomalies.rules.get`
- `anomalies.rules.create`
- `anomalies.rules.update`
- `anomalies.instances.list`
- `anomalies.instances.get`
- `deployments.jobs.create`
- `deployments.jobs.get`
- `deployments.jobs.list`
- `deployments.jobs.retry`
- `deployments.jobs.cancel`
- `deployments.jobs.status`
- `deployments.jobs.step`
- `deployments.plan.create`

The wider control/deployment/query/alert subject registry is already centralized in [`internal/natsbridge/subjects`](./internal/natsbridge/subjects). Routes without a live `server-rs` implementation return a deliberate `501 not_implemented` with `X-Boundary-State: awaiting-runtime` and the mapped `X-NATS-Subject`, instead of fake business logic in Go.

## HTTP surface

Always available:

- `GET /healthz`
- `GET /readyz`
- `GET /version`
- `GET /docs`
- `GET /openapi.json`
- `GET /openapi.yaml`

WEB boundary routes:

- `GET /api/v1/me`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/logout`
- `GET /api/v1/auth/me`
- `GET /api/v1/agents`
- `GET /api/v1/agents/{id}`
- `GET /api/v1/agents/{id}/diagnostics`
- `GET /api/v1/agents/{id}/policy`
- `GET /api/v1/policies`
- `GET /api/v1/policies/{id}`
- `POST /api/v1/policies`
- `PATCH /api/v1/policies/{id}`
- `GET /api/v1/policies/{id}/revisions`
- `GET /api/v1/hosts`
- `POST /api/v1/hosts`
- `GET /api/v1/hosts/{id}`
- `PATCH /api/v1/hosts/{id}`
- `GET /api/v1/host-groups`
- `POST /api/v1/host-groups`
- `GET /api/v1/host-groups/{id}`
- `PATCH /api/v1/host-groups/{id}`
- `POST /api/v1/host-groups/{id}/members`
- `DELETE /api/v1/host-groups/{id}/members/{hostId}`
- `GET /api/v1/credentials`
- `POST /api/v1/credentials`
- `GET /api/v1/credentials/{id}`
- `GET /api/v1/clusters`
- `GET /api/v1/clusters/{id}`
- `POST /api/v1/clusters`
- `PATCH /api/v1/clusters/{id}`
- `POST /api/v1/clusters/{id}/hosts`
- `DELETE /api/v1/clusters/{id}/hosts/{hostId}`
- `GET /api/v1/roles`
- `GET /api/v1/roles/{id}`
- `POST /api/v1/roles`
- `PATCH /api/v1/roles/{id}`
- `GET /api/v1/roles/{id}/permissions`
- `PUT /api/v1/roles/{id}/permissions`
- `GET /api/v1/role-bindings`
- `POST /api/v1/role-bindings`
- `DELETE /api/v1/role-bindings/{id}`
- `GET /api/v1/integrations`
- `GET /api/v1/integrations/{id}`
- `POST /api/v1/integrations`
- `PATCH /api/v1/integrations/{id}`
- `POST /api/v1/integrations/{id}/bindings`
- `DELETE /api/v1/integrations/{id}/bindings/{bindingId}`
- `GET /api/v1/tickets`
- `GET /api/v1/tickets/{id}`
- `POST /api/v1/tickets`
- `POST /api/v1/tickets/{id}/assign`
- `POST /api/v1/tickets/{id}/unassign`
- `POST /api/v1/tickets/{id}/comments`
- `POST /api/v1/tickets/{id}/status`
- `POST /api/v1/tickets/{id}/close`
- `GET /api/v1/anomalies/rules`
- `GET /api/v1/anomalies/rules/{id}`
- `POST /api/v1/anomalies/rules`
- `PATCH /api/v1/anomalies/rules/{id}`
- `GET /api/v1/anomalies/instances`
- `GET /api/v1/anomalies/instances/{id}`
- `POST /api/v1/deployments`
- `GET /api/v1/deployments`
- `GET /api/v1/deployments/{id}`
- `GET /api/v1/deployments/{id}/steps`
- `GET /api/v1/deployments/{id}/targets`
- `POST /api/v1/deployments/{id}/retry`
- `POST /api/v1/deployments/{id}/cancel`
- `POST /api/v1/deployments/plan`
- `GET /api/v1/stream/logs`
- `GET /api/v1/stream/deployments`
- `GET /api/v1/stream/alerts`
- `GET /api/v1/stream/agents`
- `GET /api/v1/stream/clusters`
- `GET /api/v1/stream/tickets`
- `GET /api/v1/stream/anomalies`

Currently live against real Rust runtime:

- agents list/detail/diagnostics
- policy list/detail/create/update/revisions
- hosts list/detail/create/update
- host-groups list/detail/create/update
- host-group member add/remove
- credentials metadata list/detail/create
- clusters list/detail/create/update and host bindings
- roles list/detail/create/update
- role permissions get/set
- role bindings list/create/delete
- integrations list/detail/create/update and bindings
- tickets list/detail/create and lifecycle actions
- anomaly rules list/detail/create/update
- anomaly instances list/detail
- deployment plan/create/list/get/retry/cancel
- deployment steps/status/targets read-side
- deployment status/step SSE stream
- current agent policy
- enrollment / heartbeat / diagnostics / ingest over gRPC

Still controlled `501 not_implemented` until the corresponding Rust plane exists:

- query / dashboards / alerts / audit

Compatibility routes kept for the current frontend:

- `GET /auth/csrf`
- `POST /auth/login`
- `POST /auth/logout`
- `GET /auth/me`
- `PATCH /profile`

OpenAPI source of truth:

- spec generator: [`scripts/render-openapi.cjs`](./scripts/render-openapi.cjs)
- rendered files: [`docs/openapi.json`](./docs/openapi.json), [`docs/openapi.yaml`](./docs/openapi.yaml)

Refresh the embedded spec with:

```bash
make swagger
node scripts/render-openapi.cjs
```

## gRPC surface

Service: `dorohedoro.edge.v1.AgentIngressService`

- `Enroll`
- `FetchPolicy`
- `SendHeartbeat`
- `SendDiagnostics`
- `IngestLogs`

The external gRPC contract stays compatible with the current agent-side transport, while the internal NATS bridge now maps those calls onto the shared protobuf contract used by `server-rs`.

## Agent TLS / mTLS

Agent gRPC config:

- `AGENT_GRPC_LISTEN_ADDR`
- `AGENT_TLS_CERT_FILE`
- `AGENT_TLS_KEY_FILE`
- `AGENT_TLS_CLIENT_CA_FILE`
- `AGENT_MTLS_ENABLED`
- `AGENT_ALLOW_INSECURE_DEV_MODE`

Rules:

- if `AGENT_MTLS_ENABLED=true`, server cert/key and client CA are required
- if mTLS is disabled, missing TLS cert/key is rejected unless `AGENT_ALLOW_INSECURE_DEV_MODE=true`
- insecure dev mode is explicit and logged on startup

gRPC access logs now include:

- `request_id`
- `rpc_method`
- `peer_addr`
- `tls_subject`
- `tls_san`
- `tls_fingerprint`
- `agent_id`
- `duration`
- `result`

## Local run

Full local stack:

```bash
docker compose up --build
```

This starts:

- `frontend`
- `edge-api`
- `nats`
- `postgres`
- `enrollment-plane`
- `control-plane`
- `deployment-plane`

This is the primary local workflow. The standalone [`edge_api/docker-compose.yml`](./docker-compose.yml) is now only an isolated edge-only debug stack.

Server/staging stack for `fishingteam.su`:

```bash
docker compose -f docker-compose.server.yml up -d --build
```

That stack binds only localhost-facing ports and is intended to sit behind Nginx. See [`docs/server-deploy.md`](../docs/server-deploy.md).

`docker compose` now generates short-lived dev certificates automatically and starts `edge-api` with agent mTLS enabled by default.

Explicit insecure mode still exists through `AGENT_ALLOW_INSECURE_DEV_MODE=true`, but it is now opt-in and intended only for isolated transport debugging.

## Agent mTLS smoke path

The compose stack already generates a dev CA, server certificate and client certificate in the shared `edge-api-certs` volume.

To exercise the live gRPC ingress against the running compose stack, execute the smoke client inside the `edge-api` container so it uses the generated client certificate set:

```bash
docker exec dorohedoro-edge-api-1 /bin/sh -lc \
  "FAKE_AGENT_TLS_CA_FILE=/certs/ca.crt \
   FAKE_AGENT_TLS_CERT_FILE=/certs/agent.crt \
   FAKE_AGENT_TLS_KEY_FILE=/certs/agent.key \
   FAKE_AGENT_TLS_SERVER_NAME=edge-api \
   EDGE_API_GRPC_ADDR=127.0.0.1:9090 \
   /usr/local/bin/fake-agent"
```

If you need a separate host-side cert set for manual experiments, generate one explicitly with `go run ./cmd/dev-certs` and point a standalone `edge-api` run at that directory.

Current honest scope note:

- `edge_api` already enforces and logs boundary-side mTLS
- the reproducible client-cert smoke path today is `fake-agent`
- real `agent-rs` client-cert rollout still needs runtime/config support on the agent side and is not emulated in Go

Repository-level PKI helpers for reproducible dev/test cert issuance also live in:

- [`../scripts/pki/dev-ca.sh`](../scripts/pki/dev-ca.sh)
- [`../scripts/pki/issue-edge-cert.sh`](../scripts/pki/issue-edge-cert.sh)
- [`../scripts/pki/issue-agent-cert.sh`](../scripts/pki/issue-agent-cert.sh)
- [`../docs/dev-pki.md`](../docs/dev-pki.md)

## Tests

Useful checks:

```bash
cd edge_api
go test ./...
go build ./cmd/edge-api
go build ./cmd/fake-agent
```

The default test set now covers:

- config validation for agent TLS/insecure mode
- centralized subject mapping
- protobuf NATS envelope encoding/decoding
- protobuf-wrapped NATS replies for control/read-side bridge flows
- boundary metadata on controlled `not_implemented` routes
- frontend auth compatibility flow
