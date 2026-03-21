# Single-Host Deploy Notes

This document describes the compose-managed single-host/domain profile for one public domain hosting both `WEB` and `SERVER`.

## 1. Prepare the server profile

Start from [`.env.server.example`](../.env.server.example) and set:

- `PUBLIC_BASE_URL=https://<your-domain>`
- `EDGE_PUBLIC_URL=https://<your-domain>`
- `AGENT_PUBLIC_GRPC_ADDR=<your-domain>:443`
- `NGINX_SERVER_NAME=<your-domain>`
- `SERVER_CERTS_DIR=/absolute/or/relative/path/to/certs`
- optional `EDGE_API_SERVER_ENV_FILE` if you keep the edge-api env file outside the repo default

The edge boundary env file is [`../edge_api/.env.server`](../edge_api/.env.server). Keep it aligned with the same domain values:

- `PUBLIC_BASE_URL`
- `EDGE_PUBLIC_URL`
- `AGENT_PUBLIC_GRPC_ADDR`
- `CORS_ALLOWED_ORIGINS`
- `COOKIE_SECURE=true`
- `AGENT_MTLS_ENABLED=true`

`SERVER_CERTS_DIR` must contain:

- `server.crt`
- `server.key`
- `ca.crt`
- `agent.crt` optional, only if `vault-init` should seed client cert material
- `agent.key` optional, only if `vault-init` should seed client key material

The server compose profile now fails fast if `SERVER_CERTS_DIR` is missing instead of silently generating demo certs.

Before any public exposure:

- replace `DEV_TEST_PASSWORD`
- replace the placeholder Vault SSH secret
- provide real certificates and CA material

## 2. Start the server stack

```bash
docker compose --env-file .env.server -f docker-compose.server.yml up -d --build
```

The stack includes:

- `nginx`
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

Public host ports:

- `80`
- `443`

Internal runtime services stay private on the compose network.

## Ingress model

`docker-compose.server.yml` now manages `nginx` inside the stack.

Routing:

- `/` -> `frontend:3000`
- `/api/edge/` -> `edge-api:8080`
- `/api/v1/` -> `edge-api:8080`
- `/auth/*` and `/profile` -> `edge-api:8080`
- `/docs`, `/openapi.json`, `/openapi.yaml`, `/healthz`, `/readyz`, `/version` -> `edge-api:8080`
- `/dorohedoro.edge.v1.AgentIngressService/*` -> `grpc://edge-api:9090`

The template is:

- [`../deployments/nginx/server.conf.template`](../deployments/nginx/server.conf.template)

## Vault bootstrap

The server stack uses:

- `vault` in dev mode for the current preprod/demo profile
- `vault-init` to seed:
  - AppRole credentials for `deployment-plane`
  - `secret/data/agent/ca`
  - `secret/data/agent/cert`
  - `secret/data/agent/key`
  - `secret/data/ssh/dev` placeholder credentials

Before a real rollout, replace the SSH secret:

```bash
docker compose -f docker-compose.server.yml exec vault \
  vault kv put secret/ssh/dev \
  ssh_user=root \
  ssh_private_key=@/path/to/real/id_ed25519
```

Then create or update a credentials profile in the product pointing at:

- `secret/data/ssh/dev`

## Artifact source

The stack includes `agent-artifacts`, which builds and serves the release artifacts used by `deployment-plane`.

Important URLs inside the compose network:

- manifest: `http://agent-artifacts:8080/manifest.json`
- release base: `http://agent-artifacts:8080/`

## Checks after deploy

Public checks:

```bash
curl -k https://<your-domain>/healthz
curl -k https://<your-domain>/readyz
curl -k https://<your-domain>/openapi.json
curl -k https://<your-domain>/api/v1/dashboards/overview
curl -k https://<your-domain>/api/v1/alerts
curl -k https://<your-domain>/api/v1/audit
```

## Practical rollout notes

The intended path is:

`agent -> <your-domain>:443 -> nginx -> edge-api -> NATS -> server-rs`

Do not expose the internal runtime ports directly on the internet.

For a real 3-host rollout:

1. replace the placeholder Vault SSH secret
2. verify the mounted certificate directory matches the public domain and agent CA
3. verify agent artifact mirror health
4. create hosts and credentials in WEB
5. create a deployment plan and job in WEB
6. watch `/api/v1/stream/deployments`
7. verify agent enrollment, heartbeat, diagnostics, log delivery, alerts and audit
