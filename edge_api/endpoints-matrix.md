# Edge API Endpoint Matrix

Current boundary snapshot for the integrated stack in this repository.

## Live HTTP routes

| Endpoint | Bridge | Status | Notes |
| --- | --- | --- | --- |
| `GET /healthz` | local | live | process liveness |
| `GET /readyz` | NATS readiness | live | returns `503` when bridge is not ready |
| `GET /version` | local | live | build/version surface |
| `GET /api/v1/me` | local stub | live | current WEB auth placeholder |
| `GET /api/v1/agents` | request/reply -> `agents.list` | live | real data from `enrollment-plane` |
| `GET /api/v1/agents/{id}` | request/reply -> `agents.get` | live | real data from `enrollment-plane` |
| `GET /api/v1/agents/{id}/diagnostics` | request/reply -> `agents.diagnostics.get` | live | latest persisted diagnostics snapshot |
| `GET /api/v1/agents/{id}/policy` | request/reply -> `agents.policy.get` | live | current bound policy binding for the agent |
| `GET /api/v1/policies` | request/reply -> `control.policies.list` | live | policy list from `control-plane` |
| `GET /api/v1/policies/{id}` | request/reply -> `control.policies.get` | live | policy detail from `control-plane` |
| `POST /api/v1/policies` | request/reply -> `control.policies.create` | live | policy create |
| `PATCH /api/v1/policies/{id}` | request/reply -> `control.policies.update` | live | policy update |
| `GET /api/v1/policies/{id}/revisions` | request/reply -> `control.policies.revisions` | live | revision history |
| `GET /api/v1/hosts` | request/reply -> `control.hosts.list` | live | inventory list |
| `POST /api/v1/hosts` | request/reply -> `control.hosts.create` | live | inventory create |
| `GET /api/v1/hosts/{id}` | request/reply -> `control.hosts.get` | live | inventory detail |
| `PATCH /api/v1/hosts/{id}` | request/reply -> `control.hosts.update` | live | inventory update |
| `GET /api/v1/host-groups` | request/reply -> `control.host-groups.list` | live | host-group list |
| `POST /api/v1/host-groups` | request/reply -> `control.host-groups.create` | live | host-group create |
| `GET /api/v1/host-groups/{id}` | request/reply -> `control.host-groups.get` | live | host-group detail |
| `PATCH /api/v1/host-groups/{id}` | request/reply -> `control.host-groups.update` | live | host-group update |
| `GET /api/v1/credentials` | request/reply -> `control.credentials.list` | live | metadata only |
| `POST /api/v1/credentials` | request/reply -> `control.credentials.create` | live | metadata create |
| `GET /api/v1/credentials/{id}` | request/reply -> `control.credentials.get` | live | metadata detail |
| `POST /api/v1/deployments/plan` | request/reply -> `deployments.plan.create` | live | deployment plan preview |
| `POST /api/v1/deployments` | request/reply -> `deployments.jobs.create` | live | deployment job create |
| `GET /api/v1/deployments` | request/reply -> `deployments.jobs.list` | live | deployment list |
| `GET /api/v1/deployments/{id}` | request/reply -> `deployments.jobs.get` | live | deployment detail |
| `GET /api/v1/deployments/{id}/steps` | request/reply -> `deployments.jobs.step` | live | step read-side |
| `GET /api/v1/deployments/{id}/targets` | request/reply -> `deployments.jobs.status` | live | target/status read-side |
| `POST /api/v1/deployments/{id}/retry` | request/reply -> `deployments.jobs.retry` | live | retry |
| `POST /api/v1/deployments/{id}/cancel` | request/reply -> `deployments.jobs.cancel` | live | cancel |
| `GET /api/v1/stream/logs` | subscribe -> `ui.stream.logs` | live gateway | future runtime publisher |
| `GET /api/v1/stream/deployments` | subscribe -> `deployments.jobs.status` + `deployments.jobs.step` | live | deployment SSE fanout |
| `GET /api/v1/stream/alerts` | subscribe -> `ui.stream.alerts` | live gateway | future runtime publisher |
| `GET /api/v1/stream/agents` | subscribe -> `ui.stream.agents` | live gateway | future runtime publisher |
| `GET /api/v1/stream/clusters` | subscribe -> `ui.stream.clusters` | live gateway | reserved for future runtime |
| `GET /api/v1/stream/tickets` | subscribe -> `ui.stream.tickets` | live gateway | reserved for future runtime |
| `GET /api/v1/stream/anomalies` | subscribe -> `ui.stream.anomalies` | live gateway | reserved for future runtime |

## Live gRPC ingress

| Method | Bridge | Status |
| --- | --- | --- |
| `Enroll` | request/reply -> `agents.enroll.request` | live |
| `FetchPolicy` | request/reply -> `agents.policy.fetch` | live |
| `SendHeartbeat` | publish -> `agents.heartbeat` | live |
| `SendDiagnostics` | publish -> `agents.diagnostics` | live |
| `IngestLogs` | publish -> `logs.ingest.raw` | live |

## Controlled `not_implemented`

These route groups are intentionally still thin boundary placeholders because the matching Rust runtime is not present yet:

- query analytics
- dashboards
- alerts
- audit
- clusters
- roles
- permissions
- integrations
- tickets
- anomalies

All such routes return:

- HTTP `501`
- JSON error code `not_implemented`
- `X-Boundary-State: awaiting-runtime`
- mapped `X-NATS-Subject`

That keeps the boundary honest and avoids fake Go business logic.
