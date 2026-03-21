# server-rs

Rust runtime foundation for the private `SERVER` components.

Current runtime crates:

- `common`: shared config, protobuf helpers, NATS subjects, error envelopes
- `enrollment-plane`: agent enrollment, policy fetch, heartbeat, diagnostics, bootstrap-token issuance
- `control-plane`: policy metadata, policy revisions, inventory, host groups, credentials metadata
- `deployment-plane`: deployment jobs, immutable deployment snapshots, bootstrap rendering, mock execution, deployment status events

Shared protobuf contracts live under [`/contracts/proto`](/c:/develop/DoroheDoro/contracts/proto).

## Layout

```text
server-rs/
|-- Cargo.toml
|-- .env.example
|-- README.md
|-- migrations/
`-- crates/
    |-- common/
    |-- control-plane/
    |-- deployment-plane/
    `-- enrollment-plane/
```

## Migrations

Apply the shared migrations before starting any runtime:

```bash
cd server-rs
sqlx migrate run --source migrations
```

Deployment-plane depends on:

- `0003_enrollment_tokens_policy_revision.sql`
- `0004_deployment_plane_init.sql`

These migrations add pinned `policy_revision_id` to `enrollment_tokens` and create:

- `deployment_jobs`
- `deployment_attempts`
- `deployment_targets`
- `deployment_steps`

## Local run

1. Start dependencies:

```bash
docker compose up -d postgres nats
```

2. Export env vars or copy values from `.env.example`.

3. Start the Rust runtime components:

```bash
cargo run -p enrollment-plane
# separate terminal
cargo run -p control-plane
# separate terminal
cargo run -p deployment-plane
```

4. Optional smoke tests:

```bash
cargo test -p enrollment-plane --test smoke -- --ignored --nocapture
cargo test -p control-plane --test smoke -- --ignored --nocapture
cargo test -p deployment-plane --test smoke -- --ignored --nocapture
```

## Health endpoints

Each runtime exposes:

- `GET /healthz`
- `GET /readyz`

`deployment-plane` readiness checks:

- PostgreSQL connectivity
- NATS connectivity
- executor readiness

## NATS subjects

`enrollment-plane` listens to:

- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`
- `agents.bootstrap-token.issue`

`control-plane` listens to:

- `control.policies.list`
- `control.policies.get`
- `control.policies.create`
- `control.policies.update`
- `control.policies.revisions`
- `control.hosts.list`
- `control.hosts.get`
- `control.hosts.create`
- `control.hosts.update`
- `control.host-groups.list`
- `control.host-groups.get`
- `control.host-groups.create`
- `control.host-groups.update`
- `control.host-groups.add-member`
- `control.host-groups.remove-member`
- `control.credentials.list`
- `control.credentials.get`
- `control.credentials.create`

`deployment-plane` listens to request/reply subjects:

- `deployments.jobs.create`
- `deployments.jobs.get`
- `deployments.jobs.list`
- `deployments.jobs.retry`
- `deployments.jobs.cancel`
- `deployments.plan.create`

`deployment-plane` publishes:

- `deployments.jobs.status`
- `deployments.jobs.step`

## Deployment-plane behavior

`deployment-plane` resolves deployment inputs over NATS:

- inventory and host groups from `control-plane`
- policy and latest policy revision from `control-plane`
- credentials metadata from `control-plane`
- one-time bootstrap tokens from `enrollment-plane`

The execution flow is:

1. build an immutable deployment snapshot
2. persist job, attempt, target, and step rows in PostgreSQL
3. render agent bootstrap YAML and per-target vars
4. run the configured executor
5. publish status and step events through NATS

### Mock executor

`DEPLOYMENT_EXECUTOR_KIND=mock` is the default MVP mode.

It simulates a realistic execution pipeline:

- marks jobs and targets running
- writes deployment steps
- updates final target and job status
- publishes `deployments.jobs.status` and `deployments.jobs.step`

### Ansible runner skeleton

`DEPLOYMENT_EXECUTOR_KIND=ansible` currently validates local prerequisites and renders a workspace, but still returns a clear "not implemented yet" execution error. The service and domain model are already shaped so the real ansible-runner integration can be added without rewriting the deployment persistence model.

## Environment

Common runtime env:

- `POSTGRES_DSN`
- `NATS_URL`
- `ENROLLMENT_HTTP_ADDR`
- `CONTROL_HTTP_ADDR`
- `RUST_LOG`

Enrollment-plane:

- `ENROLLMENT_DEV_BOOTSTRAP_TOKEN`

Deployment-plane:

- `DEPLOYMENT_HTTP_ADDR`
- `DEPLOYMENT_EXECUTOR_KIND=mock|ansible`
- `EDGE_PUBLIC_URL`
- `EDGE_GRPC_ADDR`
- `AGENT_STATE_DIR_DEFAULT`
- `ANSIBLE_RUNNER_BIN` optional
- `ANSIBLE_PLAYBOOK_PATH` optional
- `DEPLOYMENT_TEMP_DIR` optional

## Current scope

Implemented:

- shared Rust protobuf generation for enrollment, control, and deployment contracts
- PostgreSQL-backed enrollment, control, and deployment state
- bootstrap token issuance pinned to a specific policy revision
- deployment snapshot generation and bootstrap YAML rendering for the current agent shape
- mock deployment execution with persisted attempts, targets, steps, and NATS status events
- startup reconciliation for stale in-flight deployment attempts

Not implemented yet:

- full ansible-runner orchestration
- Vault secret retrieval for deployment executor payloads
- Go `edge_api` bridge for the new deployment subjects and `agents.bootstrap-token.issue`
- `agent-rs`
- `ingestion-plane`
- `query-alert-plane`
