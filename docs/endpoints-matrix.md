# endpoints-matrix.md

Current transport matrix for the platform. This file focuses on the runtime contracts that exist now or are required for the first real deployment lifecycle slice.

## Scope

The platform still has 3 product services:

- `WEB`
- `SERVER`
- `AGENT`

Inside `SERVER`, the public Go Edge API bridges to private Rust runtime components over NATS.

## Public HTTP for WEB

These are the minimum HTTP expectations the Edge API must expose or continue exposing for the current MVP slice.

| Method | Path | Caller | Owner behind Edge API | Status | Notes |
|---|---|---|---|---|---|
| `GET` | `/healthz` | ops | Edge API | MVP | Public liveness endpoint |
| `GET` | `/readyz` | ops | Edge API | MVP | Public readiness endpoint |
| `GET` | `/api/v1/policies` | `WEB` | control-plane | MVP | List policies |
| `GET` | `/api/v1/policies/{id}` | `WEB` | control-plane | MVP | Get policy details |
| `GET` | `/api/v1/hosts` | `WEB` | control-plane | Next | Inventory list |
| `GET` | `/api/v1/host-groups` | `WEB` | control-plane | Next | Host groups list |
| `GET` | `/api/v1/credentials` | `WEB` | control-plane | Next | Credential metadata list |
| `POST` | `/api/v1/deployments/plan` | `WEB` | deployment-plane | MVP | Dry-run deployment plan preview |
| `POST` | `/api/v1/deployments` | `WEB` | deployment-plane | MVP | Create deployment job |
| `GET` | `/api/v1/deployments` | `WEB` | deployment-plane | MVP | List deployment jobs |
| `GET` | `/api/v1/deployments/{id}` | `WEB` | deployment-plane | MVP | Get job summary, attempts, current targets, current steps |
| `POST` | `/api/v1/deployments/{id}/retry` | `WEB` | deployment-plane | MVP | Retry failed/all targets on the same job |
| `POST` | `/api/v1/deployments/{id}/cancel` | `WEB` | deployment-plane | MVP | Cancel queued/running job |

## Public gRPC for AGENT

| RPC | Caller | Owner behind Edge API | Status | Notes |
|---|---|---|---|---|
| `Enroll` | `AGENT` | enrollment-plane | MVP | Bootstrap enrollment |
| `FetchPolicy` | `AGENT` | enrollment-plane | MVP | Fetch pinned policy revision |
| `SendHeartbeat` | `AGENT` | enrollment-plane | MVP | Persist heartbeat |
| `SendDiagnostics` | `AGENT` | enrollment-plane | MVP | Persist diagnostics |
| `IngestLogs` | `AGENT` | ingestion path | MVP | Existing ingest entrypoint |

## Internal NATS subjects

### Enrollment and lifecycle

| Type | Subject | Caller / Publisher | Handler | Status | Notes |
|---|---|---|---|---|---|
| request/reply | `agents.enroll.request` | Edge API | enrollment-plane | MVP | Agent enrollment |
| request/reply | `agents.policy.fetch` | Edge API | enrollment-plane | MVP | Agent policy fetch |
| publish | `agents.heartbeat` | Edge API | enrollment-plane | MVP | Heartbeat persistence |
| publish | `agents.diagnostics` | Edge API | enrollment-plane | MVP | Diagnostics persistence |
| request/reply | `agents.bootstrap-token.issue` | deployment-plane | enrollment-plane | MVP | Issue one-time bootstrap token pinned to a specific `policy_revision_id` |

### Control-plane lookup subjects

| Type | Subject | Caller / Publisher | Handler | Status | Notes |
|---|---|---|---|---|---|
| request/reply | `control.policies.list` | Edge API | control-plane | MVP | List policies |
| request/reply | `control.policies.get` | Edge API, deployment-plane | control-plane | MVP | Get policy plus latest revision |
| request/reply | `control.policies.create` | Edge API | control-plane | MVP | Create policy and first revision |
| request/reply | `control.policies.update` | Edge API | control-plane | MVP | Update policy and create new revision |
| request/reply | `control.policies.revisions` | Edge API | control-plane | Next | Revision history |
| request/reply | `control.hosts.list` | Edge API | control-plane | Next | List hosts |
| request/reply | `control.hosts.get` | Edge API, deployment-plane | control-plane | MVP | Get hosts for deployment target resolution |
| request/reply | `control.hosts.create` | Edge API | control-plane | Next | Create host |
| request/reply | `control.hosts.update` | Edge API | control-plane | Next | Update host |
| request/reply | `control.host-groups.list` | Edge API | control-plane | Next | List host groups |
| request/reply | `control.host-groups.get` | Edge API, deployment-plane | control-plane | MVP | Expand host groups for deployment target resolution |
| request/reply | `control.host-groups.create` | Edge API | control-plane | Next | Create host group |
| request/reply | `control.host-groups.update` | Edge API | control-plane | Next | Update host group |
| request/reply | `control.host-groups.add-member` | Edge API | control-plane | Next | Add host to group |
| request/reply | `control.host-groups.remove-member` | Edge API | control-plane | Next | Remove host from group |
| request/reply | `control.credentials.list` | Edge API | control-plane | Next | List credential metadata |
| request/reply | `control.credentials.get` | Edge API, deployment-plane | control-plane | MVP | Resolve credential metadata for executor input |
| request/reply | `control.credentials.create` | Edge API | control-plane | Next | Create credential metadata record |
| request/reply | `control.clusters.list` | Edge API | control-plane | Next | Cluster inventory with paging/filtering |
| request/reply | `control.clusters.get` | Edge API, deployment-plane | control-plane | Next | Cluster detail + membership |
| request/reply | `control.clusters.create` | Edge API | control-plane | Next | Create cluster entity |
| request/reply | `control.clusters.update` | Edge API | control-plane | Next | Update cluster metadata |
| request/reply | `control.clusters.add-host` | Edge API | control-plane | Next | Attach host to cluster scope |
| request/reply | `control.clusters.remove-host` | Edge API | control-plane | Next | Remove host from cluster scope |
| request/reply | `control.roles.list` | Edge API | control-plane | Next | List custom roles |
| request/reply | `control.roles.get` | Edge API | control-plane | Next | Role detail |
| request/reply | `control.roles.create` | Edge API | control-plane | Next | Create role |
| request/reply | `control.roles.update` | Edge API | control-plane | Next | Update role metadata |
| request/reply | `control.roles.permissions.get` | Edge API | control-plane | Next | Fetch role permission set |
| request/reply | `control.roles.permissions.set` | Edge API | control-plane | Next | Replace role permission set |
| request/reply | `control.role-bindings.list` | Edge API | control-plane | Next | List bindings with scope filters |
| request/reply | `control.role-bindings.create` | Edge API | control-plane | Next | Bind role to user + scope |
| request/reply | `control.role-bindings.delete` | Edge API | control-plane | Next | Delete binding |
| request/reply | `control.integrations.list` | Edge API | control-plane | Next | List notification integrations |
| request/reply | `control.integrations.get` | Edge API | control-plane | Next | Integration detail + bindings |
| request/reply | `control.integrations.create` | Edge API | control-plane | Next | Create integration record |
| request/reply | `control.integrations.update` | Edge API | control-plane | Next | Update integration config |
| request/reply | `control.integrations.bind` | Edge API | control-plane | Next | Bind integration to scope/event types |
| request/reply | `control.integrations.unbind` | Edge API | control-plane | Next | Remove integration binding |
| request/reply | `tickets.list` | Edge API | control-plane | Next | Cluster-scoped tickets overview |
| request/reply | `tickets.get` | Edge API | control-plane | Next | Ticket detail + comments/events |
| request/reply | `tickets.create` | Edge API | control-plane | Next | Manual ticket creation |
| request/reply | `tickets.assign` | Edge API | control-plane | Next | Assign ticket to user |
| request/reply | `tickets.unassign` | Edge API | control-plane | Next | Unassign ticket |
| request/reply | `tickets.comment.add` | Edge API | control-plane | Next | Add ticket comment |
| request/reply | `tickets.status.change` | Edge API | control-plane | Next | Move ticket across workflow states |
| request/reply | `tickets.close` | Edge API | control-plane | Next | Close ticket with resolution |
| request/reply | `anomalies.rules.list` | Edge API | control-plane | Next | List anomaly rules (threshold/baseline/novelty) |
| request/reply | `anomalies.rules.get` | Edge API | control-plane | Next | Rule detail |
| request/reply | `anomalies.rules.create` | Edge API | control-plane | Next | Create rule |
| request/reply | `anomalies.rules.update` | Edge API | control-plane | Next | Update rule config/status |
| request/reply | `anomalies.instances.list` | Edge API | control-plane | Next | List anomaly instances filtered by cluster/rule/status |
| request/reply | `anomalies.instances.get` | Edge API | control-plane | Next | Instance detail for UI drill-down |

### Deployment-plane subjects

| Type | Subject | Caller / Publisher | Handler | Status | Notes |
|---|---|---|---|---|---|
| request/reply | `deployments.jobs.create` | Edge API | deployment-plane | MVP | Create job, persist attempt/targets, start execution |
| request/reply | `deployments.jobs.get` | Edge API | deployment-plane | MVP | Get job summary, attempts, current targets, current steps |
| request/reply | `deployments.jobs.list` | Edge API | deployment-plane | MVP | List jobs with filters and pagination |
| request/reply | `deployments.jobs.retry` | Edge API | deployment-plane | MVP | Create a new attempt without overwriting history |
| request/reply | `deployments.jobs.cancel` | Edge API | deployment-plane | MVP | Cancel queued/running job |
| publish | `deployments.jobs.status` | deployment-plane | Edge API stream bridge | MVP | Live job status updates |
| publish | `deployments.jobs.step` | deployment-plane | Edge API stream bridge | MVP | Live step updates |
| request/reply | `deployments.plan.create` | Edge API | deployment-plane | MVP | Build a dry-run deployment plan |

### Existing ingest and query subjects

| Type | Subject | Caller / Publisher | Handler | Status | Notes |
|---|---|---|---|---|---|
| publish | `logs.ingest.raw` | Edge API | ingest path | MVP | Raw log ingest |
| publish | `ui.stream.logs` | runtime | Edge API stream bridge | MVP | Live logs to WEB |
| request/reply | `query.logs.search` | Edge API | query path | MVP | Search |
| request/reply | `query.logs.histogram` | Edge API | query path | MVP | Histogram |
| request/reply | `query.logs.severity` | Edge API | query path | MVP | Severity buckets |
| request/reply | `query.logs.top_hosts` | Edge API | query path | MVP | Top hosts |
| request/reply | `query.logs.top_services` | Edge API | query path | MVP | Top services |

## Edge API bridge TODO

The Go `edge_api` must bridge the new runtime subjects before the deployment flow is externally reachable:

- `agents.bootstrap-token.issue`
- `deployments.jobs.create`
- `deployments.jobs.get`
- `deployments.jobs.list`
- `deployments.jobs.retry`
- `deployments.jobs.cancel`
- `deployments.jobs.status`
- `deployments.jobs.step`
- `deployments.plan.create`
- `control.clusters.*`
- `control.roles.*` and `control.role-bindings.*`
- `control.integrations.*`
- `tickets.*`
- `anomalies.rules.*`
- `anomalies.instances.*`

## Error envelope rule

NATS request/reply handlers should use the shared envelope shape:

```json
{
  "status": "ok",
  "code": "ok",
  "message": "",
  "payload": "<protobuf bytes>",
  "correlation_id": "req-123"
}
```
