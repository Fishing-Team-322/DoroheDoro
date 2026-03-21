# DoroheDoro local stack

The repository builds a 3-service platform:

- `WEB` -> `frontend/`
- `SERVER` -> public Go `edge_api/` + private Rust runtime in `server-rs/`
- `AGENT` -> `agent-rs/`

For local integration, the root `docker-compose.yml` now starts:

- `frontend` on `http://localhost:3000`
- `edge-api` on `http://localhost:8080` and `localhost:9090`
- `nats` on `nats://localhost:4222`
- `postgres` on `localhost:5432`
- `enrollment-plane` on `http://localhost:8081`

## Run

```bash
docker compose up --build
```

This is the recommended local workflow for `WEB + SERVER boundary + server-rs enrollment-plane`.

## What is live today

After startup, the current integrated slice is:

- WEB login through `frontend -> /api/edge -> edge-api`
- real `edge-api -> NATS -> enrollment-plane` bridge
- real agents read-side:
  - `GET /api/v1/agents`
  - `GET /api/v1/agents/{id}`
  - `GET /api/v1/agents/{id}/diagnostics`
  - `GET /api/v1/agents/{id}/policy`
- real policy read-side:
  - `GET /api/v1/policies`
  - `GET /api/v1/policies/{id}`
  - `GET /api/v1/policies/{id}/revisions`
- agent gRPC ingress with TLS + mTLS:
  - `Enroll`
  - `FetchPolicy`
  - `SendHeartbeat`
  - `SendDiagnostics`
  - `IngestLogs`

Domains that do not yet have a Rust runtime behind them still return controlled `501 not_implemented` from `edge-api` instead of fake Go business logic.

## What changed in edge-api

`edge_api` now behaves as a thin boundary service:

- real `nats.go`, `grpc-go`, `zap` dependencies instead of stub runtimes
- centralized NATS subject registry
- gRPC agent ingress bridged to `server-rs` protobuf NATS envelopes
- agent TLS/mTLS transport enabled in the main compose stack
- SSE gateway for UI streams
- legacy Go PoC ownership packages excluded from the default build

The current live Rust runtime behind the bridge is `server-rs` enrollment-plane. Routes for domains that do not yet have a Rust implementation return deliberate `501 not_implemented` instead of fake Go business logic.

## WEB auth note

The local compose file still enables the frontend-compatible DEV auth stub inside `edge-api`:

- login: `admin`
- email: `admin@example.com`
- password: `admin123`

That keeps the current `WEB` login flow working while the long-term auth integration is still separate from the agent mTLS path.

## Agent mTLS smoke path

1. Start the stack:

```bash
docker compose up --build
```

2. Run the smoke client inside the live `edge-api` container so it uses the generated client certificate set:

```bash
docker exec dorohedoro-edge-api-1 /bin/sh -lc \
  "FAKE_AGENT_TLS_CA_FILE=/certs/ca.crt \
   FAKE_AGENT_TLS_CERT_FILE=/certs/agent.crt \
   FAKE_AGENT_TLS_KEY_FILE=/certs/agent.key \
   FAKE_AGENT_TLS_SERVER_NAME=edge-api \
   EDGE_API_GRPC_ADDR=127.0.0.1:9090 \
   /usr/local/bin/fake-agent"
```

If you need a separate host-side cert set for manual experiments, generate one with `cd edge_api && go run ./cmd/dev-certs` and run a standalone `edge-api` against that directory.

## Demo flow

See [`docs/demo-smoke.md`](C:/C++WWW/DoroheDoro/docs/demo-smoke.md) for the current end-to-end smoke/demo scenario:

1. start the stack
2. log into WEB
3. enroll a fake agent over gRPC + mTLS
4. inspect real agents/policies data through `edge-api`
5. verify the SSE gateway

## Useful checks

```bash
cd edge_api
go test ./...
go build ./cmd/edge-api
go build ./cmd/fake-agent
```

For boundary details, see `edge_api/README.md`.
