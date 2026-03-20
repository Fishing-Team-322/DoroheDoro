# Edge API

Go API lives in `edge_api/` so the repository root can host shared infrastructure files while deployment still runs from the root-level `docker-compose.server.yml`.

## Build

```bash
cd edge_api
go build ./...
```

## Docker image

```bash
docker build -t edge-api ./edge_api
```

## Environment

Server deployment reads `edge_api/.env.server`.

At minimum, verify these values for your VPS environment before deploy:

- `NATS_URL`
- `OPENSEARCH_URL`
- `ENROLLMENT_TOKEN`
- `OPENSEARCH_USERNAME` / `OPENSEARCH_PASSWORD` if your cluster is secured

## HTTP routes

- `/`
- `/health` and `/healthz`
- `/ready` and `/readyz`
- `/docs`
- `/openapi.json`

Legacy swagger paths redirect to `/docs`.
