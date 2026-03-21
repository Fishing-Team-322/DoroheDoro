# server-rs

Private Rust runtime for the internal `SERVER` domains.

## Ownership

- `enrollment-plane` owns agent lifecycle write/read-side:
  - `agents.enroll.request`
  - `agents.policy.fetch`
  - `agents.heartbeat`
  - `agents.diagnostics`
  - `agents.list`
  - `agents.get`
  - `agents.diagnostics.get`
  - `agents.policy.get`
  - `agents.bootstrap-token.issue`
- `control-plane` owns all `control.*`:
  - `control.policies.*`
  - `control.hosts.*`
  - `control.host-groups.*`
  - `control.credentials.*`
- `deployment-plane` owns all `deployments.*`:
  - `deployments.jobs.create|get|list|retry|cancel`
  - `deployments.plan.create`
  - publishes `deployments.jobs.status`
  - publishes `deployments.jobs.step`

`server-rs` does not keep legacy `agents.registry.*` subjects and `enrollment-plane` no longer responds to `control.policies.*`.

## Runtime crates

- `common`
  - protobuf codegen
  - canonical NATS subject registry
  - shared runtime reply envelope
  - shared config/bootstrap/health helpers
- `enrollment-plane`
  - PostgreSQL-backed agent registry
  - enrollment and policy fetch for agents
  - persisted heartbeat and diagnostics
  - bootstrap-token issuance pinned to a policy revision
- `control-plane`
  - policies with append-only revisions
  - hosts with deterministic list/paging/query
  - host groups with explicit membership mutations
  - credentials metadata only
  - audit-ready columns and `control_audit_events`
- `deployment-plane`
  - immutable deployment snapshot generation
  - job/attempt/target/step persistence
  - canonical job detail read-side
  - mock executor and ansible-runner skeleton

## Contracts

Shared contracts live in [`contracts/proto`](/c:/develop/DoroheDoro/contracts/proto):

- `runtime.proto`
  - `RuntimeReplyEnvelope`
  - `AuditContext`
  - `PagingRequest`
  - `PagingResponse`
- `agent.proto`
  - typed read-side messages for `agents.list/get/diagnostics.get/policy.get`
- `control.proto`
  - audit-aware create/update requests
  - paged list requests and responses
- `deployment.proto`
  - audit-aware job/plan commands
  - canonical `GetDeploymentJobResponse`

Future placeholders already reserved in the shared subject registry:

- `query.logs.search|get|context|histogram|severity|top_hosts|top_services|heatmap|top_patterns`
- `query.dashboards.overview`
- `alerts.list|get|rules.create|rules.update`
- `audit.list`

## Health and readiness

Every plane exposes:

- `GET /healthz`
  - process is alive
- `GET /readyz`
  - Postgres is reachable
  - NATS is reachable
  - plane-specific runtime is ready

`deployment-plane` also checks executor readiness on `/readyz`.

## Database and migrations

All planes apply the shared migration set on startup.

Current migration highlights:

- `0001_init_enrollment.sql`
  - agents, policies, revisions, diagnostics, bindings
- `0002_control_plane_init.sql`
  - hosts, host groups, credentials metadata
- `0003_enrollment_tokens_policy_revision.sql`
  - pinned revision on bootstrap tokens
- `0004_deployment_plane_init.sql`
  - jobs, attempts, targets, steps
- `0005_control_audit_foundation.sql`
  - audit columns on control entities
  - `control_audit_events`
  - latest-binding and latest-diagnostics indexes

## Environment

Shared required env:

- `POSTGRES_DSN`
- `NATS_URL`
- `RUST_LOG` optional, defaults to `info`

`enrollment-plane`:

- `ENROLLMENT_HTTP_ADDR`
- `ENROLLMENT_DEV_BOOTSTRAP_TOKEN`

`control-plane`:

- `CONTROL_HTTP_ADDR`

`deployment-plane`:

- `DEPLOYMENT_HTTP_ADDR`
- `DEPLOYMENT_EXECUTOR_KIND=mock|ansible`
- `EDGE_PUBLIC_URL`
- `EDGE_GRPC_ADDR`
- `AGENT_STATE_DIR_DEFAULT`
- `MOCK_EXECUTOR_STEP_DELAY_MS` optional
- `MOCK_EXECUTOR_FAIL_MODE=never|partial|all` optional
- `MOCK_EXECUTOR_FAIL_HOSTS=host1,host2` optional
- `ANSIBLE_RUNNER_BIN` required when `DEPLOYMENT_EXECUTOR_KIND=ansible`
- `ANSIBLE_PLAYBOOK_PATH` required when `DEPLOYMENT_EXECUTOR_KIND=ansible`
- `DEPLOYMENT_TEMP_DIR` optional

See [`server-rs/.env.example`](/c:/develop/DoroheDoro/server-rs/.env.example) for a local template.

## Local run

1. Start dependencies:

```bash
docker compose up -d postgres nats
```

2. Export env from `.env.example`.

3. Start the planes:

```bash
cd server-rs
cargo run -p enrollment-plane
# separate terminal
cargo run -p control-plane
# separate terminal
cargo run -p deployment-plane
```

## Local smoke

Minimum demo smoke after all three planes are running:

- `control.policies.list|create|get|update|revisions`
- `control.hosts.list|create|get|update`
- `control.host-groups.list|create|get|update|add-member|remove-member`
- `control.credentials.list|create|get`
- `agents.list|get|diagnostics.get|policy.get`
- `deployments.plan.create`
- `deployments.jobs.create|list|get|retry|cancel`

Important integration expectation:

- there must be no duplicate responders for `control.policies.*` when `enrollment-plane`, `control-plane`, and `deployment-plane` run together

## Deployment-plane behavior

`deployments.jobs.get` is the canonical detail read model. It returns:

- job summary
- attempts
- per-target statuses
- deployment steps
- aggregate counters inside `DeploymentJobSummary`

### Mock executor

`DEPLOYMENT_EXECUTOR_KIND=mock` is demo-ready:

- moves jobs through `queued -> running -> terminal`
- writes attempt-level and target-level steps
- writes target results
- supports success, partial failure, and full failure modes through env

### Ansible runner skeleton

`DEPLOYMENT_EXECUTOR_KIND=ansible` is prepared for the next step:

- validates runner/playbook/temp-dir prerequisites
- renders a workspace per attempt
- renders inventory, vars, and bootstrap artifacts
- returns structured execution output metadata with stdout/stderr refs
- does not yet invoke real ansible-runner orchestration

## Current scope

Implemented now:

- canonical runtime reply envelope across Rust planes
- canonical subject registry in `common`
- audit-aware control-plane foundation
- agent registry read-side in `enrollment-plane`
- deployment job backend with attempt-level executor API
- shared startup/bootstrap/health behavior

Not implemented yet:

- real ansible-runner execution
- Vault secret material resolution inside executor runtime
- query/alert runtime handlers
- `edge_api` migration to the new canonical protobuf request/reply contracts
