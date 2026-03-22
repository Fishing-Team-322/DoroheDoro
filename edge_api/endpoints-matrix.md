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
| `GET /api/v1/hosts/{id}/agent-status` | aggregated read model | live | host-level deployment + enrollment + heartbeat + diagnostics + traffic summary |
| `GET /api/v1/hosts/{id}/agent-diagnostics` | aggregated read model | live | normalized doctor snapshot with checks, degraded mode and partial-data markers |
| `GET /api/v1/host-groups` | request/reply -> `control.host-groups.list` | live | host-group list |
| `POST /api/v1/host-groups` | request/reply -> `control.host-groups.create` | live | host-group create |
| `GET /api/v1/host-groups/{id}` | request/reply -> `control.host-groups.get` | live | host-group detail |
| `PATCH /api/v1/host-groups/{id}` | request/reply -> `control.host-groups.update` | live | host-group update |
| `POST /api/v1/host-groups/{id}/members` | request/reply -> `control.host-groups.add-member` | live | add host to group |
| `DELETE /api/v1/host-groups/{id}/members/{hostId}` | request/reply -> `control.host-groups.remove-member` | live | remove host from group |
| `GET /api/v1/credentials` | request/reply -> `control.credentials.list` | live | metadata only |
| `POST /api/v1/credentials` | request/reply -> `control.credentials.create` | live | metadata create |
| `GET /api/v1/credentials/{id}` | request/reply -> `control.credentials.get` | live | metadata detail |
| `GET /api/v1/clusters` | request/reply -> `control.clusters.list` | live | cluster list |
| `GET /api/v1/clusters/{id}` | request/reply -> `control.clusters.get` | live | cluster detail |
| `GET /api/v1/clusters/{id}/agents/overview` | aggregated read model | live | cluster-wide agent health/connectivity rollup with host shortlist |
| `POST /api/v1/clusters` | request/reply -> `control.clusters.create` | live | cluster create |
| `PATCH /api/v1/clusters/{id}` | request/reply -> `control.clusters.update` | live | cluster update |
| `POST /api/v1/clusters/{id}/hosts` | request/reply -> `control.clusters.add-host` | live | bind host to cluster |
| `DELETE /api/v1/clusters/{id}/hosts/{hostId}` | request/reply -> `control.clusters.remove-host` | live | unbind host from cluster |
| `GET /api/v1/roles` | request/reply -> `control.roles.list` | live | role list |
| `GET /api/v1/roles/{id}` | request/reply -> `control.roles.get` | live | role detail |
| `POST /api/v1/roles` | request/reply -> `control.roles.create` | live | role create |
| `PATCH /api/v1/roles/{id}` | request/reply -> `control.roles.update` | live | role update |
| `GET /api/v1/roles/{id}/permissions` | request/reply -> `control.roles.permissions.get` | live | role permissions detail |
| `PUT /api/v1/roles/{id}/permissions` | request/reply -> `control.roles.permissions.set` | live | role permissions update |
| `GET /api/v1/role-bindings` | request/reply -> `control.role-bindings.list` | live | role binding list |
| `POST /api/v1/role-bindings` | request/reply -> `control.role-bindings.create` | live | role binding create |
| `DELETE /api/v1/role-bindings/{id}` | request/reply -> `control.role-bindings.delete` | live | role binding delete |
| `GET /api/v1/integrations` | request/reply -> `control.integrations.list` | live | integration list |
| `GET /api/v1/integrations/{id}` | request/reply -> `control.integrations.get` | live | integration detail |
| `POST /api/v1/integrations` | request/reply -> `control.integrations.create` | live | integration create |
| `PATCH /api/v1/integrations/{id}` | request/reply -> `control.integrations.update` | live | integration update |
| `POST /api/v1/integrations/{id}/bindings` | request/reply -> `control.integrations.bind` | live | bind integration to scope |
| `DELETE /api/v1/integrations/{id}/bindings/{bindingId}` | request/reply -> `control.integrations.unbind` | live | unbind integration |
| `GET /api/v1/tickets` | request/reply -> `tickets.list` | live | ticket list |
| `GET /api/v1/tickets/{id}` | request/reply -> `tickets.get` | live | ticket detail |
| `POST /api/v1/tickets` | request/reply -> `tickets.create` | live | ticket create |
| `POST /api/v1/tickets/{id}/assign` | request/reply -> `tickets.assign` | live | assign ticket |
| `POST /api/v1/tickets/{id}/unassign` | request/reply -> `tickets.unassign` | live | unassign ticket |
| `POST /api/v1/tickets/{id}/comments` | request/reply -> `tickets.comment.add` | live | add ticket comment |
| `POST /api/v1/tickets/{id}/status` | request/reply -> `tickets.status.change` | live | change ticket status |
| `POST /api/v1/tickets/{id}/close` | request/reply -> `tickets.close` | live | close ticket |
| `GET /api/v1/anomalies/rules` | request/reply -> `anomalies.rules.list` | live | anomaly rule list |
| `GET /api/v1/anomalies/rules/{id}` | request/reply -> `anomalies.rules.get` | live | anomaly rule detail |
| `POST /api/v1/anomalies/rules` | request/reply -> `anomalies.rules.create` | live | anomaly rule create |
| `PATCH /api/v1/anomalies/rules/{id}` | request/reply -> `anomalies.rules.update` | live | anomaly rule update |
| `GET /api/v1/anomalies/instances` | request/reply -> `anomalies.instances.list` | live | anomaly instance list |
| `GET /api/v1/anomalies/instances/{id}` | request/reply -> `anomalies.instances.get` | live | anomaly instance detail |
| `POST /api/v1/deployments/plan` | request/reply -> `deployments.plan.create` | live | deployment plan preview |
| `POST /api/v1/deployments` | request/reply -> `deployments.jobs.create` | live | deployment job create |
| `GET /api/v1/deployments` | request/reply -> `deployments.jobs.list` | live | deployment list |
| `GET /api/v1/deployments/{id}` | request/reply -> `deployments.jobs.get` | live | deployment detail |
| `GET /api/v1/deployments/{id}/steps` | request/reply -> `deployments.jobs.step` | live | step read-side |
| `GET /api/v1/deployments/{id}/targets` | request/reply -> `deployments.jobs.status` | live | target/status read-side |
| `GET /api/v1/deployments/jobs/{id}/timeline` | aggregated read model | live | normalized phase timeline for operator UI |
| `POST /api/v1/deployments/{id}/retry` | request/reply -> `deployments.jobs.retry` | live | retry |
| `POST /api/v1/deployments/{id}/cancel` | request/reply -> `deployments.jobs.cancel` | live | cancel |
| `GET /api/v1/stream/logs` | subscribe -> `ui.stream.logs` | live gateway | future runtime publisher |
| `GET /api/v1/stream/deployments` | subscribe -> `deployments.jobs.status` + `deployments.jobs.step` | live | deployment SSE fanout |
| `GET /api/v1/stream/alerts` | subscribe -> `ui.stream.alerts` | live gateway | future runtime publisher |
| `GET /api/v1/stream/agents` | subscribe -> `ui.stream.agents` | live gateway | future runtime publisher |
| `GET /api/v1/stream/agent-events` | subscribe -> `ui.stream.agents` + `deployments.jobs.step` | live | normalized lifecycle SSE for WEB host/cluster pages |

## Live gRPC ingress

| Method | Bridge | Status |
| --- | --- | --- |
| `Enroll` | request/reply -> `agents.enroll.request` | live |
| `FetchPolicy` | request/reply -> `agents.policy.fetch` | live |
| `SendHeartbeat` | publish -> `agents.heartbeat` | live |
| `SendDiagnostics` | publish -> `agents.diagnostics` | live |
| `IngestLogs` | publish -> `logs.ingest.raw` | live |

## Experimental / non-stable

The stable boundary now ships only the SSE routes that have a live runtime publisher or a live deployment event path:

- `GET /api/v1/stream/deployments`
- `GET /api/v1/stream/agents`
- `GET /api/v1/stream/agent-events`
- `GET /api/v1/stream/logs`
- `GET /api/v1/stream/alerts`

Future streams for clusters, tickets and anomalies remain reserved only in shared contracts until Rust runtime publishers exist. They are intentionally absent from the active router and stable OpenAPI surface.
