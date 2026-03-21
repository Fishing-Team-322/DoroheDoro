# Single-Host Deploy Notes

This document describes the compose-managed single-host/domain profile for one public domain hosting both `WEB` and `SERVER`, with image-first `AGENT` delivery.

## 1. Prepare the server profile

Start from [`.env.server.example`](../.env.server.example) and set:

- `PUBLIC_BASE_URL=https://<your-domain>`
- `EDGE_PUBLIC_URL=https://<your-domain>`
- `AGENT_PUBLIC_GRPC_ADDR=<your-domain>:443`
- `NGINX_SERVER_NAME=<your-domain>`
- `SERVER_CERTS_DIR=/absolute/or/relative/path/to/certs`

Optional image delivery env vars when using the image-only override:

- `AGENT_IMAGE_REPOSITORY=docker.io/<org>/doro-agent`
- `AGENT_IMAGE_TAG=main`
- `AGENT_IMAGE_DIGEST=sha256:...`
- `AGENT_IMAGE_VERSION=main`
- `AGENT_RELEASE_CHANNEL=main`

Use a real published digest for deployment tests. The fallback placeholder digest is not intended for a real rollout.

## 2. Start the server stack with image-only delivery

```bash
docker compose \
  --env-file .env.server \
  -f docker-compose.server.yml \
  -f deployments/examples/docker-compose.server.image-only.override.yml \
  up -d --build
```

The override keeps the compose service name `agent-artifacts`, but changes it to serve only:

- `manifest.json`
- image metadata JSON

It also switches `deployment-plane` to:

- `AGENT_ARTIFACT_MANIFEST_URL=http://agent-artifacts:8080/manifest.json`
- `AGENT_RELEASE_BASE_URL=""`
- `AGENT_PREFERRED_PACKAGE_TYPE=container`

## 3. Ingress model

`docker-compose.server.yml` manages `nginx` inside the stack.

Routing:

- `/` -> `frontend:3000`
- `/api/edge/` -> `edge-api:8080`
- `/api/v1/` -> `edge-api:8080`
- `/auth/*` and `/profile` -> `edge-api:8080`
- `/docs`, `/openapi.json`, `/openapi.yaml`, `/healthz`, `/readyz`, `/version` -> `edge-api:8080`
- `/dorohedoro.edge.v1.AgentIngressService/*` -> `grpc://edge-api:9090`

## 4. Vault bootstrap

The server stack uses:

- `vault` in dev mode for the current preprod/demo profile
- `vault-init` to seed AppRole and placeholder SSH material

Before a real rollout, replace the SSH secret:

```bash
docker compose -f docker-compose.server.yml exec vault \
  vault kv put secret/ssh/dev \
  ssh_user=root \
  ssh_private_key=@/path/to/real/id_ed25519
```

## 5. Agent delivery bridge

The current `deployment-plane` still consumes an artifact-shaped manifest. The bridge works like this:

1. CI publishes `docker.io/<org>/doro-agent`
2. CI records the pushed digest
3. CI generates `agent-release-manifest.json`
4. `deployment-plane` resolves the selected manifest entry
5. Ansible translates `package_type=container` into `docker_image` install mode
6. The target host auto-detects `docker`, otherwise `podman`

No `server-rs` change is required for the basic flow.

## 6. Checks after deploy

Public checks:

```bash
curl -k https://<your-domain>/healthz
curl -k https://<your-domain>/readyz
curl -k https://<your-domain>/openapi.json
curl -k https://<your-domain>/api/v1/dashboards/overview
curl -k https://<your-domain>/api/v1/alerts
curl -k https://<your-domain>/api/v1/audit
```

Internal delivery checks:

```bash
docker compose \
  --env-file .env.server \
  -f docker-compose.server.yml \
  -f deployments/examples/docker-compose.server.image-only.override.yml \
  exec agent-artifacts wget -qO- http://127.0.0.1:8080/manifest.json
```

Expected manifest fragments:

- `install_mode=docker_image`
- `package_type=container`
- `artifact_path=docker.io/...:main`

## 7. Practical rollout notes

Target path:

`agent -> <your-domain>:443 -> nginx -> edge-api -> NATS -> server-rs`

Host-side continuity checks after deployment:

- `systemctl status doro-agent`
- `/var/lib/doro-agent/state.db` still present after restart
- `/var/lib/doro-agent/last-known-good-image.json` contains the deployed digest reference

Rollback policy:

- prefer digest-pinned rollback
- do not rely on the floating `main` tag for recovery
