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

The active live bridge is aligned with `server-rs` enrollment-plane on:

- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`
- `agents.registry.list`
- `agents.registry.get`
- `agents.diagnostics.get`
- `control.policies.list`
- `control.policies.get`
- `control.policies.revisions`

The wider control/deployment/query/alert subject registry is already centralized in [`internal/natsbridge/subjects`](C:/C++WWW/DoroheDoro/edge_api/internal/natsbridge/subjects/subjects.go). Routes without a live `server-rs` implementation return a deliberate `501 not_implemented` with `X-Boundary-State: awaiting-runtime` and the mapped `X-NATS-Subject`, instead of fake business logic in Go.

## HTTP surface

Always available:

- `GET /healthz`
- `GET /readyz`
- `GET /version`
- `GET /docs`
- `GET /openapi.json`

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
- `GET /api/v1/policies/{id}/revisions`
- `GET /api/v1/stream/logs`
- `GET /api/v1/stream/deployments`
- `GET /api/v1/stream/alerts`
- `GET /api/v1/stream/agents`

Currently live against real Rust runtime:

- agents list/detail/diagnostics
- policy list/detail/revisions
- current agent policy
- enrollment / heartbeat / diagnostics / ingest over gRPC

Still controlled `501 not_implemented` until the corresponding Rust plane exists:

- hosts / host-groups / credentials
- deployments
- query / dashboards / alerts / audit

Compatibility routes kept for the current frontend:

- `GET /auth/csrf`
- `POST /auth/login`
- `POST /auth/logout`
- `GET /auth/me`
- `PATCH /profile`

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

This is the primary local workflow. The standalone [`edge_api/docker-compose.yml`](C:/C++WWW/DoroheDoro/edge_api/docker-compose.yml) is now only an isolated edge-only debug stack.

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
- JSON-wrapped NATS replies for read-side bridge flows
- boundary metadata on controlled `not_implemented` routes
- frontend auth compatibility flow
