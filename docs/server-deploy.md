# Single-Host Deploy Notes for `fishingteam.su`

This document describes the compose-managed single-host/domain profile.

## Start the server stack

```bash
docker compose -f docker-compose.server.yml up -d --build
```

The stack now includes:

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

## Environment contract

The edge boundary uses [`../edge_api/.env.server`](../edge_api/.env.server).

Important values:

- `PUBLIC_BASE_URL=https://fishingteam.su`
- `EDGE_PUBLIC_URL=https://fishingteam.su`
- `AGENT_PUBLIC_GRPC_ADDR=fishingteam.su:443`
- `CORS_ALLOWED_ORIGINS=https://fishingteam.su,https://www.fishingteam.su`
- `COOKIE_SECURE=true`
- `AGENT_MTLS_ENABLED=true`

Before public exposure:

- replace `DEV_TEST_PASSWORD`
- replace the placeholder SSH secret in Vault
- replace the dev-generated ingress certificates with real certificates for long-lived preprod use

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
curl -k https://fishingteam.su/healthz
curl -k https://fishingteam.su/readyz
curl -k https://fishingteam.su/openapi.json
curl -k https://fishingteam.su/api/v1/dashboards/overview
curl -k https://fishingteam.su/api/v1/alerts
curl -k https://fishingteam.su/api/v1/audit
```

## Practical rollout notes

The intended path is:

`agent -> fishingteam.su:443 -> nginx -> edge-api -> NATS -> server-rs`

Do not expose the internal runtime ports directly on the internet.

For a real 3-host rollout:

1. replace the placeholder Vault SSH secret
2. verify agent artifact mirror health
3. create hosts and credentials in WEB
4. create a deployment plan and job in WEB
5. watch `/api/v1/stream/deployments`
6. verify agent enrollment, heartbeat, diagnostics, log delivery, alerts and audit
