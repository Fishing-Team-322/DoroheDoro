# Demo Stack Guide

This repository now has two main operator-facing stack modes.

## Local demo stack

Primary workflow:

```bash
docker compose up --build
```

It starts:

- `frontend`
- `edge-api`
- `postgres`
- `nats`
- `enrollment-plane`
- `control-plane`
- `deployment-plane`
- `edge-api-certs`

Published ports:

- `3000` -> WEB
- `8080` -> edge-api HTTP
- `9090` -> edge-api gRPC
- `4222` / `8222` -> NATS
- `5432` -> PostgreSQL

## Server demo/staging stack

VPS-friendly workflow:

```bash
docker compose -f docker-compose.server.yml up -d --build
```

It runs the same runtime internally, but publishes only localhost-bound ports that Nginx should proxy.

See:

- [`docs/server-deploy.md`](./server-deploy.md)

## Fast smoke path

1. Start the stack.
2. Open `http://localhost:3000`.
3. Login with `admin / admin123`.
4. Check `http://localhost:8080/readyz`.
5. Create a policy, host and credentials metadata.
6. Create a deployment plan and deployment job.
7. Open `/api/v1/stream/deployments`.
8. Run the fake-agent mTLS smoke flow.

Detailed smoke:

- [`docs/demo-smoke.md`](./demo-smoke.md)
- [`deployments/ansible/inventories/three-hosts.example.ini`](../deployments/ansible/inventories/three-hosts.example.ini)
- [`deployments/ansible/group_vars/agents.example.yml`](../deployments/ansible/group_vars/agents.example.yml)

## Notes on compose speed

Build performance is improved by:

- service-local build contexts for `frontend` and `edge_api`
- root `.dockerignore` that keeps `server-rs` build context minimal
- Go module cache mounts in `edge_api/Dockerfile`
- Cargo cache mounts in `server-rs/Dockerfile`
- generated dev certificates reused through a named volume

The demo stack is still a full multi-service build, but repeat `docker compose up --build` runs should avoid the worst cold-rebuild behavior.
