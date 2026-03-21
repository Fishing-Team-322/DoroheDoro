# Demo Stack Guide

This repository now has two operator-facing stack modes.

## Local full stack

Primary workflow:

```bash
docker compose up --build
```

It starts:

- `frontend`
- `edge-api`
- `agent-artifacts`
- `postgres`
- `nats`
- `vault`
- `vault-init`
- `opensearch`
- `clickhouse`
- `enrollment-plane`
- `control-plane`
- `deployment-plane`
- `ingestion-plane`
- `query-alert-plane`
- `edge-api-certs`

Published ports:

- `3000` -> WEB
- `8080` -> edge-api HTTP
- `9090` -> edge-api gRPC
- `18081` -> agent artifacts
- `4222` / `8222` -> NATS
- `5432` -> PostgreSQL
- `8200` -> Vault
- `9200` -> OpenSearch
- `8123` / `9000` -> ClickHouse

## Single-host/domain stack

Preprod workflow:

```bash
docker compose -f docker-compose.server.yml up -d --build
```

It runs the same internal runtime but adds compose-managed `nginx` as the public ingress.

Published ports:

- `80` -> HTTP redirect / ingress
- `443` -> HTTPS + gRPC ingress

Everything else stays on the compose network.

See:

- [`docs/server-deploy.md`](./server-deploy.md)

## Fast smoke path

1. Start the local stack.
2. Open `http://localhost:3000`.
3. Login with `admin / admin123`.
4. Verify `http://localhost:8080/readyz`.
5. Create policy, hosts, host groups and credentials metadata.
6. Replace the placeholder Vault SSH secret before a real rollout.
7. Create a deployment plan and deployment job.
8. Inspect deployment, agent, logs, alerts and audit streams/routes.

Detailed smoke:

- [`docs/demo-smoke.md`](./demo-smoke.md)
