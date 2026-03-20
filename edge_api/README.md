# edge-api

Go Edge API lives in `edge_api/` and stays a thin ingress / gateway / bridge layer for the 3-service platform:

- WEB -> HTTP ingress -> NATS -> internal SERVER responders
- AGENT -> gRPC ingress -> NATS -> internal SERVER responders
- UI live logs -> SSE gateway <- NATS

The current implementation keeps Go focused on transport concerns and MVP bridge behavior without reintroducing the old swagger runtime layer.

## What works

### HTTP routes

- `GET /`
- `GET /healthz`
- `GET /readyz`
- `GET /docs`
- `GET /docs/index.html`
- `GET /openapi.json`
- `GET /openapi.yaml`
- current `GET/POST /api/v1/*` MVP routes

### gRPC

Service: `dorohedoro.edge.v1.AgentIngressService`

- `Enroll`
- `FetchPolicy`
- `SendHeartbeat`
- `SendDiagnostics`
- `IngestLogs`

### Infra paths kept intact

- NATS bridge
- SSE log stream via `/api/v1/stream/logs`
- fake-agent smoke path via `cmd/fake-agent`

## OpenAPI docs

The static docs are embedded into the binary and served directly by edge-api:

- `GET /docs`
- `GET /docs/index.html`
- `GET /openapi.json`
- `GET /openapi.yaml`

Local URLs:

- http://localhost:8080/docs
- http://localhost:8080/docs/index.html
- http://localhost:8080/openapi.json
- http://localhost:8080/openapi.yaml

`/docs` redirects to the local self-hosted browser UI. `/openapi.json` and `/openapi.yaml` are the source-of-truth contract artifacts.

## Run locally

```bash
cd edge_api
go build ./...
docker compose config
docker compose up --build nats edge-api
```

## Smoke test

Start NATS and edge-api first, then in another shell:

```bash
cd edge_api
EDGE_API_GRPC_ADDR=localhost:9090 go run ./cmd/fake-agent
curl http://localhost:8080/
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz
curl http://localhost:8080/docs
curl http://localhost:8080/openapi.json
curl http://localhost:8080/api/v1/me
curl -N 'http://localhost:8080/api/v1/stream/logs?severity=error'
```

## Notes

- `/readyz` returns success only when the NATS bridge is ready.
- Docs are served from embedded static assets; no swagger/godoc runtime generation is required.
- Docker Compose entry point for local smoke testing remains `docker compose up --build nats edge-api`.
