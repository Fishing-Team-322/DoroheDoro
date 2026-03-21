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
- `127.0.0.1:19090 -> edge-api:9090`

Run:

```bash
docker compose -f docker-compose.server.yml up -d --build
```

The server env lives in [`edge_api/.env.server`](./edge_api/.env.server). Replace the demo auth password there before exposing the stack publicly.

This server stack is the single-host pre-production model:

- one host
- one public domain
- WEB and SERVER on the same machine
- HTTP/SSE proxied to `frontend` and `edge-api`
- agent gRPC+mTLS proxied to `edge-api:9090`

For the first practical rollout to three Linux hosts, use the inventory and group-vars examples under [`deployments/ansible/`](./deployments/ansible/). The current honest state is:

- real remote agents can already target the public domain over TLS
- transport-level mTLS is validated at the boundary with `fake-agent`
- real agent client-cert rollout still depends on follow-up `agent-rs` support and is not being faked in Go

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
  - `POST /api/v1/host-groups/{id}/members`
- credentials metadata:
  - `GET /api/v1/credentials`
  - `POST /api/v1/credentials`
- clusters:
  - `GET /api/v1/clusters`
  - `POST /api/v1/clusters`
  - `POST /api/v1/clusters/{id}/hosts`
- RBAC:
  - `GET /api/v1/roles`
  - `POST /api/v1/roles`
  - `GET /api/v1/roles/{id}/permissions`
  - `PUT /api/v1/roles/{id}/permissions`
  - `GET /api/v1/role-bindings`
  - `POST /api/v1/role-bindings`
- integrations:
  - `GET /api/v1/integrations`
  - `POST /api/v1/integrations`
- tickets:
  - `GET /api/v1/tickets`
  - `POST /api/v1/tickets`
  - `POST /api/v1/tickets/{id}/assign`
  - `POST /api/v1/tickets/{id}/comments`
- anomalies:
  - `GET /api/v1/anomalies/rules`
  - `POST /api/v1/anomalies/rules`
  - `GET /api/v1/anomalies/instances`
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

Boundary-side mTLS is live and verified with the built-in `fake-agent` smoke. The current `agent-rs` runtime still uses the public TLS path and does not yet expose client-certificate configuration, so that part remains an explicit follow-up outside Go boundary ownership.

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

OpenAPI source of truth:

- generator: [`edge_api/scripts/render-openapi.cjs`](./edge_api/scripts/render-openapi.cjs)
- rendered docs: [`edge_api/docs/openapi.json`](./edge_api/docs/openapi.json) and [`edge_api/docs/openapi.yaml`](./edge_api/docs/openapi.yaml)

Refresh the embedded spec with:

```bash
make swagger
node edge_api/scripts/render-openapi.cjs
```

## Agent release and delivery

The repository now includes a delivery layer for `AGENT` artifacts without changing `agent-rs` runtime code.

Available pieces:

- release scripts:
  - [`scripts/release/build-agent-artifacts.sh`](./scripts/release/build-agent-artifacts.sh)
  - [`scripts/release/generate-manifest.sh`](./scripts/release/generate-manifest.sh)
- manifest contract:
  - [`deployments/artifacts/manifest.schema.json`](./deployments/artifacts/manifest.schema.json)
  - [`deployments/artifacts/example.manifest.json`](./deployments/artifacts/example.manifest.json)
- packaging/install contract:
  - [`deployments/packaging/INSTALL.md`](./deployments/packaging/INSTALL.md)
- Ansible install layer:
  - [`deployments/ansible/playbooks/install-agent.yml`](./deployments/ansible/playbooks/install-agent.yml)

Build local artifacts:

```bash
bash scripts/release/build-agent-artifacts.sh --version 0.2.0
bash scripts/release/generate-manifest.sh --version 0.2.0
```

Details:

- [`docs/agent-distribution.md`](./docs/agent-distribution.md)

## Dev/test mTLS

The local compose stack already starts `edge-api` with mTLS enabled for AGENT ingress.

Standalone PKI scripts:

```bash
bash scripts/pki/dev-ca.sh
bash scripts/pki/issue-edge-cert.sh
bash scripts/pki/issue-agent-cert.sh
```

Details:

- [`docs/dev-pki.md`](./docs/dev-pki.md)

## Demo docs

- stack overview: [`docs/demo-stack.md`](./docs/demo-stack.md)
- end-to-end smoke: [`docs/demo-smoke.md`](./docs/demo-smoke.md)
- VPS/Nginx deploy: [`docs/server-deploy.md`](./docs/server-deploy.md)
