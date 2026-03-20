# Edge API endpoints matrix

Local MVP contract snapshot for `defay1x9`.

## HTTP endpoints

| Endpoint | Transport | MVP | Notes |
| --- | --- | --- | --- |
| `GET /healthz` | local | MVP | liveness only |
| `GET /readyz` | NATS readiness | MVP | returns `503` when NATS is unavailable |
| `GET /api/v1/me` | local stub | MVP | auth/RBAC placeholder |
| `GET /api/v1/agents` | request/reply -> `agents.list` | MVP | thin read bridge |
| `GET /api/v1/agents/{id}` | request/reply -> `agents.get` | MVP | thin read bridge |
| `GET /api/v1/agents/{id}/diagnostics` | request/reply -> `agents.diagnostics.get` | MVP | thin read bridge |
| `GET /api/v1/policies` | request/reply -> `policies.list` | MVP | thin read bridge |
| `GET /api/v1/policies/{id}` | request/reply -> `policies.get` | MVP | thin read bridge |
| `POST /api/v1/deployments` | request/reply -> `deployments.jobs.create` | MVP | validates `policy_id` |
| `GET /api/v1/deployments` | request/reply -> `deployments.jobs.list` | MVP | thin read bridge |
| `GET /api/v1/deployments/{id}` | request/reply -> `deployments.jobs.get` | MVP | thin read bridge |
| `POST /api/v1/logs/search` | request/reply -> `query.logs.search` | MVP | JSON body filters |
| `GET /api/v1/logs/histogram` | request/reply -> `query.logs.histogram` | MVP | query-string passthrough |
| `GET /api/v1/logs/severity` | request/reply -> `query.logs.severity` | MVP | query-string passthrough |
| `GET /api/v1/logs/top-hosts` | request/reply -> `query.logs.top_hosts` | MVP | query-string passthrough |
| `GET /api/v1/logs/top-services` | request/reply -> `query.logs.top_services` | MVP | query-string passthrough |
| `GET /api/v1/stream/logs` | subscribe -> `ui.stream.logs` | MVP | SSE with optional `host/service/severity` filters |

## gRPC ingress methods

| Method | NATS subject | Pattern | MVP |
| --- | --- | --- | --- |
| `Enroll` | `agents.enroll.request` | request/reply | MVP |
| `FetchPolicy` | `agents.policy.fetch` | request/reply | MVP |
| `SendHeartbeat` | `agents.heartbeat` | publish | MVP |
| `SendDiagnostics` | `agents.diagnostics` | publish | MVP |
| `IngestLogs` | `logs.ingest.raw` | publish | MVP |

## Deferred scope

### Next

- real auth / RBAC integration;
- mTLS verification hooks;
- richer OpenAPI examples and generated protobuf pipeline.

### Future

- heavy business logic ownership in Go;
- persistence ownership in Go;
- alert engine / anomaly detection / query engine ownership in Go.
