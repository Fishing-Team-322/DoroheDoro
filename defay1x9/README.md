# defay1x9 Go API

Go API moved into `defay1x9/` so the repository root can host shared infrastructure files while deployment still runs from the root-level `docker-compose.server.yml`.

## Build

```bash
cd defay1x9
go build ./...
```

## Docker image

```bash
docker build -t defay1x9-api ./defay1x9
```

## Environment

Server deployment reads `defay1x9/.env.server`.

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
