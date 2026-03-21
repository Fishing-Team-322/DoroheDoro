# DoroheDoro platform stack

The repository builds a 3-service platform:

- `WEB` -> `frontend/`
- `SERVER` -> public Go `edge_api/` + private Rust runtime in `server-rs/`
- `AGENT` -> `agent-rs/`

`edge_api` is the only public boundary for both `WEB` and `AGENT`. Domain state and control/deployment runtime logic live in `server-rs`.

## Local demo stack

The root [`docker-compose.yml`](./docker-compose.yml) is the primary local workflow. It starts:

- `frontend` on `http://localhost:3000`
- `edge-api` on `http://localhost:8080` and `localhost:9090`
- `nats` on `nats://localhost:4222`
- `postgres` on `localhost:5432`
- `enrollment-plane` on `http://localhost:8081`
- `control-plane` on `http://localhost:8082`
- `deployment-plane` on `http://localhost:8083`

Run:

```bash
docker compose up --build
```

DEV auth for the current WEB flow:

- login: `admin`
- email: `admin@example.com`
- password: `admin123`

## Server demo/staging stack

[`docker-compose.server.yml`](./docker-compose.server.yml) is the VPS-friendly stack for `fishingteam.su`. It runs the same runtime internally, but only binds localhost-facing ports that Nginx should proxy:

- `127.0.0.1:13000 -> frontend:3000`
- `127.0.0.1:18080 -> edge-api:8080`

Run:

```bash
docker compose -f docker-compose.server.yml up -d --build
```

The server env lives in [`edge_api/.env.server`](./edge_api/.env.server). Replace the demo auth password there before exposing the stack publicly.

For the Nginx/domain layout on `fishingteam.su`, see [`docs/server-deploy.md`](./docs/server-deploy.md).

## What is live today

The current integrated slice is no longer just enrollment. The local stack now has live boundary flows for:

- WEB login through `frontend -> /api/edge -> edge-api`
- agents read-side:
  - `GET /api/v1/agents`
  - `GET /api/v1/agents/{id}`
  - `GET /api/v1/agents/{id}/diagnostics`
  - `GET /api/v1/agents/{id}/policy`
- policies:
  - `GET /api/v1/policies`
  - `GET /api/v1/policies/{id}`
  - `POST /api/v1/policies`
  - `PATCH /api/v1/policies/{id}`
  - `GET /api/v1/policies/{id}/revisions`
- inventory:
  - `GET /api/v1/hosts`
  - `POST /api/v1/hosts`
  - `GET /api/v1/host-groups`
  - `POST /api/v1/host-groups`
- credentials metadata:
  - `GET /api/v1/credentials`
  - `POST /api/v1/credentials`
- deployments:
  - `POST /api/v1/deployments/plan`
  - `POST /api/v1/deployments`
  - `GET /api/v1/deployments`
  - `GET /api/v1/deployments/{id}`
  - `GET /api/v1/deployments/{id}/steps`
  - `GET /api/v1/deployments/{id}/targets`
  - `POST /api/v1/deployments/{id}/retry`
  - `POST /api/v1/deployments/{id}/cancel`
- deployment SSE stream:
  - `GET /api/v1/stream/deployments`
- agent gRPC ingress with TLS + mTLS:
  - `Enroll`
  - `FetchPolicy`
  - `SendHeartbeat`
  - `SendDiagnostics`
  - `IngestLogs`

Query, dashboards, alerts and audit still return controlled `501 not_implemented` from `edge-api` until the corresponding Rust runtime exists.

## Smoke and boundary docs

- local end-to-end smoke: [`docs/demo-smoke.md`](./docs/demo-smoke.md)
- server/VPS deploy notes: [`docs/server-deploy.md`](./docs/server-deploy.md)
- boundary details: [`edge_api/README.md`](./edge_api/README.md)

Useful checks:

```bash
cd edge_api
go test ./...

cargo test --manifest-path ../server-rs/Cargo.toml -p common -p enrollment-plane -p control-plane -p deployment-plane
```
