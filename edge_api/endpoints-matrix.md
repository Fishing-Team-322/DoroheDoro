# Edge API Endpoint Matrix

Current boundary snapshot for the live stack in this repository.

## Live HTTP routes

| Endpoint | Bridge | Status | Notes |
| --- | --- | --- | --- |
| `GET /healthz` | local | live | process liveness |
| `GET /readyz` | NATS readiness | live | returns `503` when bridge is not ready |
| `GET /version` | local | live | build/version surface |
| `GET /api/v1/me` | local stub | live | current WEB auth placeholder |
| `GET /api/v1/agents` | request/reply -> `agents.registry.list` | live | real data from `enrollment-plane` |
| `GET /api/v1/agents/{id}` | request/reply -> `agents.registry.get` | live | real data from `enrollment-plane` |
| `GET /api/v1/agents/{id}/diagnostics` | request/reply -> `agents.diagnostics.get` | live | latest persisted diagnostics snapshot |
| `GET /api/v1/agents/{id}/policy` | request/reply -> `agents.policy.fetch` | live | current bound policy for the agent |
| `GET /api/v1/policies` | request/reply -> `control.policies.list` | live | real policy list from PostgreSQL |
| `GET /api/v1/policies/{id}` | request/reply -> `control.policies.get` | live | real policy detail from PostgreSQL |
| `GET /api/v1/policies/{id}/revisions` | request/reply -> `control.policies.revisions` | live | revision history from PostgreSQL |
| `GET /api/v1/stream/logs` | subscribe -> `ui.stream.logs` | live | SSE gateway |
| `GET /api/v1/stream/deployments` | subscribe -> `ui.stream.deployments` | live gateway | runtime publisher still missing |
| `GET /api/v1/stream/alerts` | subscribe -> `ui.stream.alerts` | live gateway | runtime publisher still missing |
| `GET /api/v1/stream/agents` | subscribe -> `ui.stream.agents` | live gateway | runtime publisher still missing |

## Live gRPC ingress

| Method | Bridge | Status |
| --- | --- | --- |
| `Enroll` | request/reply -> `agents.enroll.request` | live |
| `FetchPolicy` | request/reply -> `agents.policy.fetch` | live |
| `SendHeartbeat` | publish -> `agents.heartbeat` | live |
| `SendDiagnostics` | publish -> `agents.diagnostics` | live |
| `IngestLogs` | publish -> `logs.ingest.raw` | live |

## Controlled `not_implemented`

These routes are intentionally still handled as thin boundary placeholders because the matching Rust runtime is not present yet:

- hosts / host-groups / credentials
- deployments
- query analytics
- dashboards
- alerts
- audit

All such routes now return:

- HTTP `501`
- JSON error code `not_implemented`
- `X-Boundary-State: awaiting-runtime`
- mapped `X-NATS-Subject`

That keeps the boundary honest and makes missing runtime ownership explicit.
