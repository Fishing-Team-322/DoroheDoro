# edge-api

Thin Edge API ingress / gateway / bridge layer:

- WEB -> HTTP ingress -> NATS -> Rust responders
- AGENT -> gRPC ingress -> NATS -> Rust responders
- UI live logs -> SSE gateway <- NATS

`endpoints-matrix.md` is the local contract snapshot for the MVP scope implemented in Go.

## Cleanup under MVP Edge API

The Go service was simplified to keep ownership out of the ingress layer:

- removed alerts routes from the startup path because they are outside the current MVP matrix;
- removed response-only NATS subjects from config (`agents.enroll.response`, `agents.policy.response`) to keep the bridge focused on request/reply and publish subjects actually used by the ingress layer;
- renamed deployment status transport to `deployments.jobs.get` so the bridge matches the matrix contract;
- kept handlers thin: transport validation, request/correlation IDs, NATS bridge calls, SSE fan-out only;
- preserved only stub auth hooks and fake-agent tooling needed for local smoke tests.

## Project structure

```text
/defay1x9
  /cmd
    /edge-api
    /fake-agent
  /contracts
    /proto
  /docs
  /internal
    /app
    /auth
    /config
    /grpcapi
    /httpapi
    /middleware
    /model
    /natsbridge
    /observability
    /stream
    /transport
  Dockerfile
  README.md
  endpoints-matrix.md
```

## MVP HTTP endpoints

- `GET /healthz`
- `GET /readyz`
- `GET /api/v1/me`
- `GET /api/v1/agents`
- `GET /api/v1/agents/{id}`
- `GET /api/v1/agents/{id}/diagnostics`
- `GET /api/v1/policies`
- `GET /api/v1/policies/{id}`
- `POST /api/v1/deployments`
- `GET /api/v1/deployments`
- `GET /api/v1/deployments/{id}`
- `POST /api/v1/logs/search`
- `GET /api/v1/logs/histogram`
- `GET /api/v1/logs/severity`
- `GET /api/v1/logs/top-hosts`
- `GET /api/v1/logs/top-services`
- `GET /api/v1/stream/logs`

## MVP gRPC methods

Service: `dorohedoro.edge.v1.AgentIngressService`

- `Enroll`
- `FetchPolicy`
- `SendHeartbeat`
- `SendDiagnostics`
- `IngestLogs`

## MVP NATS bridge subjects

Required matrix subjects:

- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`
- `logs.ingest.raw`
- `ui.stream.logs`
- `query.logs.search`
- `query.logs.histogram`
- `query.logs.severity`
- `query.logs.top_hosts`
- `query.logs.top_services`
- `deployments.jobs.create`
- `deployments.jobs.get`
- `deployments.jobs.list`

Additional HTTP read-model subjects still required to serve MVP list/get routes:

- `agents.list`
- `agents.get`
- `agents.diagnostics.get`
- `policies.list`
- `policies.get`

## Transport behavior

- HTTP returns a single JSON error envelope with `request_id`.
- gRPC maps bridge failures to `InvalidArgument`, `Unavailable`, or `Internal` status codes.
- `/readyz` is green only when the NATS bridge is connected.
- `/api/v1/stream/logs` serves SSE and supports optional `host`, `service`, and `severity` filters.

## Local run

```bash
cd /workspace/DoroheDoro/defay1x9
go build ./...
docker compose config
docker compose up --build nats edge-api
```

## Smoke test

1. Start NATS and edge-api.
2. Start Rust responders for the request/reply subjects listed above.
3. In another shell run:

```bash
cd /workspace/DoroheDoro/defay1x9
EDGE_API_GRPC_ADDR=localhost:9090 go run ./cmd/fake-agent
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz
curl http://localhost:8080/api/v1/me
curl -N 'http://localhost:8080/api/v1/stream/logs?severity=error'
```

## TODO for Rust responders

- respond on request/reply subjects used by HTTP list/get/search endpoints;
- own policy, inventory, deployment, and log-query business logic outside of Go;
- publish UI log events to `ui.stream.logs` for SSE consumers;
- replace stub auth/mTLS hooks with real verification when the Rust runtime contract is ready.
