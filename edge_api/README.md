# edge-api

Go Edge API lives in `edge_api/` and stays a thin ingress / gateway / bridge layer for the 3-service platform:

- WEB -> HTTP ingress -> NATS -> internal SERVER responders
- AGENT -> gRPC ingress -> NATS -> internal SERVER responders
- UI live logs -> SSE gateway <- NATS

The current implementation keeps Go focused on transport concerns and MVP bridge behavior. The frontend auth routes described below are a **DEV/STUB compatibility layer** for local frontend work, not a production auth backend.

## What works

### HTTP routes

- `GET /`
- `GET /healthz`
- `GET /readyz`
- `GET /docs`
- `GET /docs/index.html`
- `GET /openapi.json`
- `GET /openapi.yaml`
- `GET /auth/csrf`
- `POST /auth/login`
- `POST /auth/logout`
- `GET /auth/me`
- `PATCH /profile`
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

## DEV auth stub env

The local frontend compatibility flow is controlled by env:

- `HTTP_AUTH_STUB_ENABLED=true`
- `DEV_TEST_LOGIN=admin`
- `DEV_TEST_EMAIL=admin@example.com`
- `DEV_TEST_PASSWORD=admin123`
- `DEV_TEST_USER_ID=dev-user-1`
- `DEV_TEST_ROLE=admin`
- `COOKIE_SECURE=false`
- `SESSION_TTL=24h`

Behavior:

- when `HTTP_AUTH_STUB_ENABLED=true`, Edge API uses an in-memory session/auth stub
- when `HTTP_AUTH_STUB_ENABLED=false`, the dev auth endpoints return `501 not_implemented`
- login accepts either `DEV_TEST_LOGIN` or `DEV_TEST_EMAIL` as the identifier
- returned session payload remains frontend-compatible while `/api/v1/me` continues to exist unchanged

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

## Run locally

### Edge API only

```bash
cd edge_api
go build ./...
docker compose up --build nats edge-api
```

### Full local stack

```bash
docker compose up --build
```

This starts:

- frontend on `http://localhost:3000`
- edge-api on `http://localhost:8080`
- nats on `nats://localhost:4222`

## Smoke test

1. Start the full stack:

```bash
docker compose up --build
```

2. Open `http://localhost:3000`.
3. Login with:
   - login: `admin`
   - password: `admin123`
4. Confirm the frontend gets a session and can load authenticated pages.
5. Confirm the frontend can call:
   - `GET /auth/me`
   - `PATCH /profile`
6. Confirm Edge API liveness endpoints still work:

```bash
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz
curl http://localhost:8080/api/v1/me
curl -N 'http://localhost:8080/api/v1/stream/logs?severity=error'
```

## Notes

- `/readyz` returns success only when the NATS bridge is ready.
- Docs are served from embedded static assets; no swagger/godoc runtime generation is required.
- The auth compatibility layer is intentionally local-dev only and stores session state in memory.
