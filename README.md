# DoroheDoro local integration stack

This repository runs a minimal local integration of the **WEB** and **SERVER** boundary needed for frontend smoke testing:

- `frontend/` — Next.js WEB application exposed on `http://localhost:3000`
- `edge_api/` — Go Edge API exposed on `http://localhost:8080`
- `nats` — internal transport for the current Edge API MVP on `nats://localhost:4222`

The root `docker-compose.yml` starts exactly these three services together so the WEB frontend can authenticate against a **DEV/STUB auth layer** in the Edge API and then continue calling the existing Edge API ingress routes. Requests now go browser -> WEB (`/api/edge/*`) -> Edge API, so login works even when the browser is opened from another machine and `localhost:8080` is not the correct public API address.

## Run locally

```bash
docker compose up --build
```

This is the recommended local DEV workflow now: both WEB and SERVER run inside Docker, while the browser only talks to `http://localhost:3000`.

## URLs

- Frontend: `http://localhost:3000`
- Edge API docs: `http://localhost:8080/docs`
- Edge API OpenAPI: `http://localhost:8080/openapi.json`
- Edge API health: `http://localhost:8080/healthz`
- Edge API readiness: `http://localhost:8080/readyz`

## DEV auth stub env

The local compose file enables a frontend-compatible dev auth stub in `edge-api`.

### Default env values

Frontend proxy env:
- `NEXT_PUBLIC_API_BASE_URL=/api/edge`
- `EDGE_API_INTERNAL_URL=http://edge-api:8080`

Frontend local dev env:
- `NEXT_PUBLIC_API_BASE_URL=/api/edge`
- `EDGE_API_INTERNAL_URL=http://localhost:8080`

Edge API auth env:
- `HTTP_AUTH_STUB_ENABLED=true`
- `DEV_TEST_LOGIN=admin`
- `DEV_TEST_EMAIL=admin@example.com`
- `DEV_TEST_PASSWORD=admin123`
- `DEV_TEST_USER_ID=dev-user-1`
- `DEV_TEST_ROLE=admin`
- `COOKIE_SECURE=false`
- `SESSION_TTL=24h`
- `SESSION_COOKIE_NAME=session_token`
- `CSRF_COOKIE_NAME=csrf_token`

### What they do

- `HTTP_AUTH_STUB_ENABLED=true` turns on the in-memory dev session/auth stub.
- `DEV_TEST_LOGIN` and `DEV_TEST_EMAIL` are accepted as login identifiers.
- `DEV_TEST_PASSWORD` is the only valid password in DEV/STUB mode.
- `DEV_TEST_USER_ID` and `DEV_TEST_ROLE` shape the returned frontend session payload.
- `COOKIE_SECURE=false` keeps cookies working over plain `http://localhost`.
- `SESSION_TTL=24h` controls both session lifetime and default CSRF cookie lifetime.

### Override credentials

You can override any of these before starting compose, for example:

```bash
export DEV_TEST_LOGIN=demo
export DEV_TEST_EMAIL=demo@example.com
export DEV_TEST_PASSWORD=demo123
export DEV_TEST_USER_ID=dev-user-42
export DEV_TEST_ROLE=viewer
export SESSION_TTL=8h
docker compose up --build
```

## Frontend-compatible auth routes

The Edge API keeps the existing MVP routes and adds a local compatibility layer for the frontend contract:

- `GET /auth/csrf`
  - issues a readable `csrf_token` cookie
  - returns `{ "csrfToken": "<token>" }`
- `POST /auth/login`
  - accepts `identifier`, `email`, `login`, `password`
  - validates against the DEV env credentials
  - creates an in-memory session cookie and rotates CSRF
- `POST /auth/logout`
  - requires session + CSRF
  - clears session and CSRF cookies
- `GET /auth/me`
  - returns the current session payload when the session cookie is valid
- `PATCH /profile`
  - requires session + CSRF
  - updates in-memory profile fields for the current dev session

If `HTTP_AUTH_STUB_ENABLED=false`, these DEV auth handlers stay mounted but return a clear `501 not_implemented` response that tells you to enable the stub or provide a real auth integration.

## WEB -> SERVER proxy

The WEB container proxies every request from `/api/edge/*` to `EDGE_API_INTERNAL_URL`. This keeps browser calls same-origin with the WEB host while still routing traffic to the Go Edge API inside compose. Because auth cookies are now issued through the WEB origin, the dev login flow works without a separate browser-visible `http://localhost:8080` dependency.

For local `next dev` on the host, set `frontend/.env.local` from `frontend/.env.example` so the proxy targets `http://localhost:8080`. If `EDGE_API_INTERNAL_URL` is not set, the Next.js proxy now uses a dev-friendly default order:

- `http://localhost:8080` during `next dev`
- `http://edge-api:8080` as the compose/container default

When a dev proxy attempt cannot reach the configured upstream, the proxy logs the failure and returns a structured `502` response instead of silently collapsing into a generic login error.

`docker-compose.yml` now also includes healthchecks so the frontend waits for the Edge API readiness endpoint before it is started.

## Cookie and CSRF behavior

- Session cookie is `HttpOnly`, `SameSite=Lax`, `Path=/`
- Session cookie `Secure` is controlled by `COOKIE_SECURE`
- CSRF cookie name defaults to `csrf_token`
- Frontend reads `csrf_token` and sends `X-CSRF-Token`
- Mutating routes (`POST`, `PUT`, `PATCH`, `DELETE`) validate:
  - active session
  - CSRF cookie
  - `X-CSRF-Token` header
  - cookie/header match

## Existing MVP routes kept intact

The DEV auth stub is additive. These Edge API MVP routes remain available:

- `GET /healthz`
- `GET /readyz`
- `GET /api/v1/me`
- current `/api/v1/*` request/reply routes
- NATS bridge
- SSE stream via `/api/v1/stream/logs`

## Default login for local smoke test

Use either of these identifiers with the default password:

- login: `admin`
- email: `admin@example.com`
- password: `admin123`

## Quick login flow check

1. Start the stack with `docker compose up --build`.
2. Open `http://localhost:3000`.
3. Sign in with:
   - login: `admin`
   - password: `admin123`
4. Confirm the frontend becomes authenticated.
5. Open the profile page and save a new display name.
6. Confirm the frontend successfully calls:
   - `GET /auth/me`
   - `PATCH /profile`
7. Confirm health endpoints still respond:
   - `GET http://localhost:8080/healthz`
   - `GET http://localhost:8080/readyz`
8. If you need to inspect the auth path, watch logs with:
   - `docker compose logs -f frontend edge-api`

## Manual API check

```bash
curl -i -c /tmp/doro.cookies http://localhost:8080/auth/csrf
CSRF_TOKEN=$(awk '$6 == "csrf_token" { print $7 }' /tmp/doro.cookies | tail -n1)

curl -i \
  -b /tmp/doro.cookies \
  -c /tmp/doro.cookies \
  -H "Content-Type: application/json" \
  -H "X-CSRF-Token: ${CSRF_TOKEN}" \
  -d '{"identifier":"admin","password":"admin123"}' \
  http://localhost:8080/auth/login

curl -i -b /tmp/doro.cookies http://localhost:8080/auth/me
```

## Notes

- Root `docker-compose.server.yml` builds the API from `./edge_api`.
- API is published only to VPS localhost on `127.0.0.1:18080:8080`.
- `edge_api/.env.server` must be reviewed for real `NATS_URL` and `OPENSEARCH_URL` values before production deploy.

## Agent runtime

The repository now also contains a standalone Rust agent under `agent-rs/`.

- Runtime doc: `docs/agent-runtime.md`
- Service README: `agent-rs/README.md`
- Deployment examples: `deployments/examples/` and `deployments/systemd/`
