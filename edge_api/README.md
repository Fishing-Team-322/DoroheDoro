# edge-api

`edge_api/` is the public boundary service for the platform:

- `WEB -> edge-api -> NATS -> server-rs`
- `AGENT -> edge-api gRPC -> NATS -> server-rs`
- `WEB streams <- edge-api SSE <- NATS`

Go stays on transport, auth, streaming and bridge duties. Business logic remains in Rust runtime planes.

## Active responsibilities

- HTTP ingress for `WEB`
- gRPC ingress for `AGENT`
- SSE gateway for stable UI streams
- TLS/mTLS termination for agent gRPC
- request validation
- request ID propagation
- NATS request/reply and publish bridge
- transport error mapping
- health/readiness/version/OpenAPI endpoints
- compat auth endpoints used by the current frontend shell

## Not owned here

`edge_api` is not the source of truth for:

- enrollment state
- control-plane state
- deployment state
- log ingestion or normalization
- search and analytics
- alert evaluation
- audit persistence

Those belong in `server-rs`.

## Stable HTTP surface

Always available:

- `GET /healthz`
- `GET /readyz`
- `GET /version`
- `GET /docs`
- `GET /openapi.json`
- `GET /openapi.yaml`

Stable WEB boundary routes:

- auth/session
- agents
- agent bootstrap tokens
- policies
- hosts
- host groups
- credentials metadata
- deployments
- clusters
- roles / permissions / bindings
- integrations
- tickets
- anomaly rules / instances
- logs search/detail/context/analytics
- dashboards overview
- alerts instances and rules
- audit list

Stable SSE routes:

- `GET /api/v1/stream/deployments`
- `GET /api/v1/stream/agents`
- `GET /api/v1/stream/logs`
- `GET /api/v1/stream/alerts`

Experimental/non-stable streams such as clusters, tickets and anomalies are intentionally not part of the stable OpenAPI surface until runtime publishers exist.

## Runtime alignment

The active boundary is aligned with the canonical subject registry and shared contracts under [`../contracts/`](../contracts):

- `contracts/proto/*.proto`
- `contracts/subjects/registry.yaml`

`edge_api` no longer depends on legacy subject aliases for the stable path. The live query/alerts/audit routes bridge directly to Rust runtime handlers instead of returning boundary-side placeholders.

## gRPC surface

Service: `dorohedoro.edge.v1.AgentIngressService`

- `Enroll`
- `FetchPolicy`
- `SendHeartbeat`
- `SendDiagnostics`
- `IngestLogs`

The public gRPC contract stays stable while the internal Go boundary maps requests onto shared runtime contracts over NATS.

## Agent TLS / mTLS

Agent gRPC config:

- `AGENT_GRPC_LISTEN_ADDR`
- `AGENT_TLS_CERT_FILE`
- `AGENT_TLS_KEY_FILE`
- `AGENT_TLS_CLIENT_CA_FILE`
- `AGENT_MTLS_ENABLED`
- `AGENT_ALLOW_INSECURE_DEV_MODE`

Rules:

- when `AGENT_MTLS_ENABLED=true`, cert, key and client CA are required
- insecure mode is explicit and fail-fast
- there is no silent fallback from mTLS to insecure transport

The boundary already supports the stable client-cert path used by the agent install contract.

Post-enrollment agent RPC rules:

- when mTLS is enabled, the verified client certificate identity is required
- the certificate identity must match `req.agent_id`
- the boundary returns real gRPC auth errors instead of collapsing them into `InvalidArgument`

## Local and server runs

Local full stack:

```bash
docker compose up --build
```

Single-host/domain stack:

```bash
docker compose --env-file ../.env.server -f ../docker-compose.server.yml up -d --build
```

The server stack now includes compose-managed `nginx`, OpenSearch, ClickHouse, Vault, `ingestion-plane`, `query-alert-plane`, and the agent artifact mirror. It expects operator-provided cert material through `SERVER_CERTS_DIR`; the dev certificate generator remains a local/demo helper rather than the canonical server profile.

## OpenAPI source of truth

- generator: [`scripts/render-openapi.cjs`](./scripts/render-openapi.cjs)
- rendered files: [`docs/openapi.json`](./docs/openapi.json), [`docs/openapi.yaml`](./docs/openapi.yaml)

Refresh:

```bash
node scripts/render-openapi.cjs
```

Verify drift:

```bash
node scripts/render-openapi.cjs --check
```

## Useful checks

```bash
cd edge_api
go test ./...
go build ./cmd/edge-api
go build ./cmd/fake-agent
node scripts/render-openapi.cjs --check
```
