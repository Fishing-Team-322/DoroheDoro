# DoroheDoro platform stack

This repository builds a 3-service platform:

- `WEB` -> `frontend/`
- `SERVER` -> public Go `edge_api/` + private Rust runtime in `server-rs/`
- `AGENT` -> `agent-rs/`

`edge_api` is the only public boundary for both `WEB` and `AGENT`. Business logic, persistence and processing live in the Rust runtime planes.

## Local full stack

Primary workflow:

```bash
docker compose up --build
```

The local stack now starts:

- `frontend`
- `edge-api`
- `agent-artifacts`
- `nats`
- `postgres`
- `vault`
- `vault-init`
- `opensearch`
- `clickhouse`
- `enrollment-plane`
- `control-plane`
- `deployment-plane`
- `ingestion-plane`
- `query-alert-plane`

Published local ports:

- `3000` -> WEB
- `8080` -> edge-api HTTP
- `9090` -> edge-api gRPC
- `18081` -> agent artifact mirror
- `4222` / `8222` -> NATS
- `5432` -> PostgreSQL
- `8200` -> Vault dev API
- `9200` -> OpenSearch
- `8123` / `9000` -> ClickHouse

DEV auth for the current WEB flow:

- login: `admin`
- email: `admin@example.com`
- password: `admin123`

Important local compose defaults:

- `DEPLOYMENT_EXECUTOR_KIND=ansible`
- `deployment-plane` uses real `ansible-runner`
- `agent-artifacts` builds and serves release artifacts from the repo
- `vault-init` seeds AppRole access plus dev TLS material
- the seeded `secret/data/ssh/dev` entry is a placeholder and must be replaced before a real remote rollout

## Single-host/domain stack

Server-style workflow:

```bash
docker compose --env-file .env.server -f docker-compose.server.yml up -d --build
```

This stack is the preprod single-host/domain profile:

- one host
- one public domain
- `WEB` and `SERVER` on the same machine
- compose-managed `nginx` publishes `80/443`
- HTTP, SSE and gRPC all arrive on the same domain
- agent gRPC+mTLS is proxied to `edge-api:9090`
- real ingress and trust material is mounted from `SERVER_CERTS_DIR`

Related docs:

- stack overview: [`docs/demo-stack.md`](./docs/demo-stack.md)
- smoke flow: [`docs/demo-smoke.md`](./docs/demo-smoke.md)
- single-host deploy notes: [`docs/server-deploy.md`](./docs/server-deploy.md)
- compose env example: [`.env.server.example`](./.env.server.example)

## What is live

Current live boundary/runtime surface:

- control-plane HTTP flows:
  - policies
  - hosts
  - host groups
  - credentials metadata
  - clusters
  - roles / permissions / bindings
  - integrations
  - tickets
  - anomaly rules / instances
- deployment flows:
  - plan
  - create
  - list / get
  - retry / cancel
  - steps / targets
  - deployment SSE
- agent lifecycle:
  - `Enroll`
  - `FetchPolicy`
  - `SendHeartbeat`
  - `SendDiagnostics`
  - `IngestLogs`
- ingest / query / analytics:
  - `/api/v1/logs/search`
  - `/api/v1/logs/{eventId}`
  - `/api/v1/logs/context`
  - `/api/v1/logs/histogram`
  - `/api/v1/logs/severity`
  - `/api/v1/logs/top-hosts`
  - `/api/v1/logs/top-services`
  - `/api/v1/logs/heatmap`
  - `/api/v1/logs/top-patterns`
  - `/api/v1/logs/anomalies`
  - `/api/v1/dashboards/overview`
- alerting:
  - `GET /api/v1/alerts`
  - `GET /api/v1/alerts/{id}`
  - `GET /api/v1/alerts/rules`
  - `GET /api/v1/alerts/rules/{id}`
  - `POST /api/v1/alerts`
  - `PATCH /api/v1/alerts/{id}`
- audit:
  - `GET /api/v1/audit`
- stable SSE:
  - `GET /api/v1/stream/deployments`
  - `GET /api/v1/stream/agents`
  - `GET /api/v1/stream/logs`
  - `GET /api/v1/stream/alerts`

The stable surface no longer uses `runtimeUnavailable` placeholders for logs, dashboards, alerts or audit.

## Practical rollout notes

`deployment-plane` now expects:

- real ansible execution
- artifact manifest and release source
- Vault-backed SSH credentials
- optional Vault-backed agent mTLS material

For the first real rollout:

1. replace the placeholder SSH secret in Vault
2. create a credentials profile in WEB or via API that points to that Vault ref
3. create policy, hosts and host groups
4. build a deployment plan
5. execute the job from WEB
6. verify deployment stream, agent enrollment, heartbeat, diagnostics, logs, alerts and audit

The agent install contract now renders:

- `tls.ca_path`
- `tls.cert_path`
- `tls.key_path`
- `tls.server_name`

and runs `doro-agent doctor --config ...` as `ExecStartPre`.

The current compat/stub WEB auth remains an internal or preprod-only profile. Treat it as non-production auth even though the rest of the stack is aligned for a practical run.

## OpenAPI

Source of truth:

- generator: [`edge_api/scripts/render-openapi.cjs`](./edge_api/scripts/render-openapi.cjs)
- rendered docs: [`edge_api/docs/openapi.json`](./edge_api/docs/openapi.json) and [`edge_api/docs/openapi.yaml`](./edge_api/docs/openapi.yaml)

Refresh:

```bash
make swagger
```

Verify drift:

```bash
make swagger-check
```

Runtime smoke gates against a live Postgres/NATS stack:

```bash
make server-smoke
```

## Useful checks

```bash
cd edge_api
go test ./...

cargo check --manifest-path ../server-rs/Cargo.toml \
  -p enrollment-plane \
  -p control-plane \
  -p deployment-plane \
  -p ingestion-plane \
  -p query-alert-plane
```
