# DoroheDoro local integration stack

This repository currently runs a minimal local integration of the **WEB** and **SERVER** boundary needed for frontend work:

- `frontend/` — Next.js application exposed on `http://localhost:3000`
- `edge_api/` — Go Edge API exposed on `http://localhost:8080`
- `nats` — internal transport for the current Edge API MVP on `nats://localhost:4222`

The root `docker-compose.yml` is set up to start exactly these three services together so the frontend can talk to the current Edge API over HTTP while the Edge API still keeps its existing `/api/v1/*`, `/healthz`, `/readyz`, `/docs`, and `/openapi.json` routes.

## Run locally

```bash
docker compose up --build
```

## What starts

- `frontend` — Next.js production-like container (`next build` + `next start`)
- `edge-api` — Go Edge API container built from `./edge_api`
- `nats` — NATS server for the current Edge API MVP bridge

## URLs

- Frontend: `http://localhost:3000`
- Edge API docs: `http://localhost:8080/docs`
- Edge API OpenAPI: `http://localhost:8080/openapi.json`
- Edge API health: `http://localhost:8080/healthz`

## Frontend ↔ Edge API auth compatibility

A minimal compatibility layer was added to `edge_api` so the frontend can use its existing auth client contract without rewriting the app.

### Supported frontend-compatible routes

- `GET /auth/csrf`
- `POST /auth/login`
- `POST /auth/logout`
- `GET /auth/me`
- `PATCH /profile`

### Current behavior

This auth flow is intentionally a **stub/mock compatibility layer** for local integration:

- `GET /auth/csrf` issues a readable `csrf_token` cookie and returns `{ "csrfToken": "..." }`
- `POST /auth/login` accepts the existing frontend payload fields:
  - `identifier`
  - `email`
  - `login`
  - `password`
- Successful login creates an in-memory session, sets an HttpOnly session cookie, rotates CSRF, and returns a frontend-compatible session payload
- `GET /auth/me` returns the current session payload when the session cookie is present
- `PATCH /profile` requires both the session cookie and `X-CSRF-Token`, then updates the stubbed display name in the in-memory session
- `POST /auth/logout` clears both session and CSRF cookies

### Cookie / CSRF behavior

- CSRF cookie name: `csrf_token`
- Frontend sends `X-CSRF-Token`
- Frontend requests use `credentials: "include"`
- Session cookie is `HttpOnly`, `SameSite=Lax`, `Path=/`
- `SESSION_COOKIE_SECURE=false` is used by default in local Docker so cookies work over plain HTTP on localhost
- CORS is enabled for `http://localhost:3000` so the browser can call `http://localhost:8080` with credentials

## Quick auth flow check

1. Open `http://localhost:3000`
2. Go to the login page
3. Sign in with any non-empty identifier and password
4. Confirm the app can load the protected dashboard
5. Open the profile page and save a new display name
6. Confirm the page keeps showing the updated user payload from `GET /auth/me`

## Manual API checks

### Health and docs

```bash
curl -i http://localhost:8080/healthz
curl -i http://localhost:8080/docs
curl -i http://localhost:8080/openapi.json
```

### Auth compatibility flow

```bash
# 1) Fetch CSRF token and cookie
curl -i -c /tmp/doro.cookies http://localhost:8080/auth/csrf

# 2) Read the csrf_token value from the cookie jar and login
CSRF_TOKEN=$(awk '$6 == "csrf_token" { print $7 }' /tmp/doro.cookies | tail -n1)

curl -i \
  -b /tmp/doro.cookies \
  -c /tmp/doro.cookies \
  -H "Content-Type: application/json" \
  -H "X-CSRF-Token: ${CSRF_TOKEN}" \
  -d '{"identifier":"demo@example.com","password":"demo"}' \
  http://localhost:8080/auth/login

# 3) Read the current session
curl -i -b /tmp/doro.cookies http://localhost:8080/auth/me
```

## Notes

- The auth compatibility layer is intentionally local-dev oriented and currently stores sessions in memory inside the Go Edge API process.
- Existing Edge API MVP routes remain available under `/api/v1/*`.
- `docker-compose.server.yml` is still available for the separate server-only deployment flow.
