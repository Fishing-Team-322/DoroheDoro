# VPS server deploy for Edge API

## Files changed for server deploy

- `docker-compose.server.yml`
- `edge_api/Dockerfile`
- `edge_api/.env.server`
- `edge_api/README.md`
- `edge_api/cmd/**`
- `edge_api/internal/**`
- `edge_api/docs/**`
- `edge_api/go.mod`
- `edge_api/go.sum`

## Repository layout

```text
repo-root/
├── edge_api/
│   ├── api/
│   ├── assets/
│   ├── cmd/
│   ├── docs/
│   ├── gen/
│   ├── internal/
│   ├── pkg/
│   ├── proto/
│   ├── third_party/
│   ├── .env.server
│   ├── Dockerfile
│   ├── README.md
│   ├── go.mod
│   └── go.sum
├── docker-compose.server.yml
├── README.md
└── .gitignore
```

## Build and run on VPS

```bash
cd /opt/edge-api/TestApi
docker compose -f docker-compose.server.yml up -d --build
docker compose -f docker-compose.server.yml ps
docker compose -f docker-compose.server.yml logs --tail=200
```

## Useful commands

### Rebuild only compose config

```bash
docker compose -f docker-compose.server.yml config
```

### Local checks on VPS

```bash
curl -i http://127.0.0.1:18080/
curl -i http://127.0.0.1:18080/health
curl -i http://127.0.0.1:18080/ready
curl -i http://127.0.0.1:18080/docs
curl -i http://127.0.0.1:18080/openapi.json
```

### Domain checks through nginx

```bash
curl -I https://fishingteam.su/
curl -I https://fishingteam.su/docs
curl -I https://fishingteam.su/openapi.json
```

## Swagger / OpenAPI

Server routes are exposed as:

- `GET /docs` — Swagger UI
- `GET /openapi.json` — generated OpenAPI JSON already committed in `edge_api/docs/`

Compatibility aliases kept:

- `GET /swagger`
- `GET /swagger/*`

## Health endpoints

Available endpoints:

- `GET /health`
- `GET /healthz`
- `GET /ready`
- `GET /readyz`

## Notes

- Root `docker-compose.server.yml` builds the API from `./edge_api`.
- API is published only to VPS localhost on `127.0.0.1:18080:8080`.
- `edge_api/.env.server` must be reviewed for real `NATS_URL` and `OPENSEARCH_URL` values before production deploy.
