# Demo Stack Guide

This repository now exposes an image-first `AGENT` delivery path while keeping the internal demo service name `agent-artifacts`.

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
- `18081` -> compatibility manifest server
- `4222` / `8222` -> NATS
- `5432` -> PostgreSQL
- `8200` -> Vault
- `9200` -> OpenSearch
- `8123` / `9000` -> ClickHouse

Important local demo behavior:

- `agent-artifacts` now serves only `manifest.json` and image metadata
- `deployment-plane` reads `AGENT_ARTIFACT_MANIFEST_URL=http://agent-artifacts:8080/manifest.json`
- `AGENT_RELEASE_BASE_URL` is blank so image refs pass through unchanged
- `AGENT_PREFERRED_PACKAGE_TYPE=container`

Optional local env overrides before `docker compose up --build`:

- `AGENT_IMAGE_REPOSITORY`
- `AGENT_IMAGE_TAG`
- `AGENT_IMAGE_DIGEST`
- `AGENT_IMAGE_VERSION`
- `AGENT_RELEASE_CHANNEL`

Use a real published digest before testing remote deployment. The built-in placeholder digest is only there so the manifest service can start in a fresh dev environment.

## Single-host/domain stack

Base workflow:

```bash
docker compose --env-file .env.server -f docker-compose.server.yml up -d --build
```

Image-only override:

```bash
docker compose \
  --env-file .env.server \
  -f docker-compose.server.yml \
  -f deployments/examples/docker-compose.server.image-only.override.yml \
  up -d --build
```

Use these optional env vars with the override:

- `AGENT_IMAGE_REPOSITORY`
- `AGENT_IMAGE_TAG`
- `AGENT_IMAGE_DIGEST`
- `AGENT_IMAGE_VERSION`
- `AGENT_RELEASE_CHANNEL`

This keeps the server demo stack on the same compatibility-manifest bridge without editing `server-rs`.

Detailed notes:

- [`docs/server-deploy.md`](./server-deploy.md)

## Fast smoke path

1. Start the local stack.
2. Open `http://localhost:3000`.
3. Login with `admin / admin123`.
4. Verify `http://localhost:8080/readyz`.
5. Check `http://localhost:18081/manifest.json`.
6. Create deployment metadata in WEB or over the API.
7. Run a deployment job against a Linux host with Docker or Podman.
8. Verify the host starts `doro-agent` from the image ref in the manifest.

Detailed smoke:

- [`docs/demo-smoke.md`](./demo-smoke.md)
