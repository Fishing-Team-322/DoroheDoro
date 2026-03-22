# server-rs

Private Rust runtime for the internal `SERVER` domains.

## Runtime crates

- `common`
  - shared protobuf generation
  - canonical subject constants
  - shared config/bootstrap/reply helpers
  - shared JSON event models for streaming/audit
- `enrollment-plane`
  - persistent agent registry
  - bootstrap tokens
  - enrollment
  - policy fetch
  - heartbeat and diagnostics persistence
  - agent stream and audit event publishing
- `control-plane`
  - policies and revisions
  - hosts, host groups, credentials metadata
  - clusters
  - roles / permissions / bindings
  - integrations
  - tickets
  - anomaly rules / instances
  - shared `runtime_audit_events` read model
  - `audit.list` request/reply plus append subscriber
- `deployment-plane`
  - deployment plans
  - job / attempt / target / step persistence
  - real `ansible-runner` execution
  - Vault-backed secret resolution
  - artifact manifest resolution
  - deployment lifecycle audit publishing
- `ingestion-plane`
  - consumes `logs.ingest.raw`
  - normalizes log batches
  - writes OpenSearch and ClickHouse
  - publishes `logs.ingest.normalized`
  - publishes `ui.stream.logs`
- `query-alert-plane`
  - query handlers for logs and overview
  - alert rule and instance API
  - alert evaluation over stored log data
  - anomaly rule engine (rare fingerprint + threshold + baseline detectors backed by Postgres/ClickHouse)
  - publishes `ui.stream.alerts`
  - publishes audit events for alert lifecycle

## Contracts

Shared contracts live in [`../contracts/proto`](../contracts/proto):

- `runtime.proto`
- `agent.proto`
- `control.proto`
- `deployment.proto`
- `edge.proto`
- `query.proto`
- `alerts.proto`
- `audit.proto`

Canonical subjects live in [`../contracts/subjects/registry.yaml`](../contracts/subjects/registry.yaml).

## Runtime ownership

- `enrollment-plane` owns `agents.*` enrollment/lifecycle
- `control-plane` owns `control.*`, tickets, anomalies and audit read-side
- `deployment-plane` owns `deployments.*`
- `ingestion-plane` owns raw->normalized ingest and storage fan-out
- `query-alert-plane` owns `query.*`, `alerts.*`, и потоковый алгоритмический детект редких fingerprint-ов

## Health and readiness

Every plane exposes:

- `GET /healthz`
- `GET /readyz`

Plane readiness now checks the real dependencies for that plane:

- Postgres
- NATS
- OpenSearch / ClickHouse where applicable
- executor readiness for `deployment-plane`

## Migrations

All planes apply the shared migration set on startup.

Notable latest migrations:

- `0007_deployment_target_artifacts.sql`
  - resolved artifact summary on deployment targets
- `0008_runtime_audit_and_alerts.sql`
  - shared `runtime_audit_events`
  - `alert_rules`
  - `alert_instances`

## Environment

Shared:

- `POSTGRES_DSN` when the plane uses Postgres
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
- `ANSIBLE_RUNNER_BIN`
- `ANSIBLE_PLAYBOOK_PATH`
- `DEPLOYMENT_TEMP_DIR`
- `ANSIBLE_SUCCESSFUL_WORKSPACE_RETENTION`
- `AGENT_ARTIFACT_MANIFEST_URL`
- `AGENT_RELEASE_BASE_URL`
- `AGENT_ARTIFACT_VERSION`
- `AGENT_PREFERRED_PACKAGE_TYPE=container|deb|tar.gz`
- `VAULT_ADDR`
- `VAULT_ROLE_ID`
- `VAULT_SECRET_ID`
- `AGENT_MTLS_CA_VAULT_REF`
- `AGENT_MTLS_CERT_VAULT_REF`
- `AGENT_MTLS_KEY_VAULT_REF`

`ingestion-plane`:

- `INGESTION_HTTP_ADDR`
- `OPENSEARCH_URL`
- `OPENSEARCH_INDEX_PREFIX`
- `OPENSEARCH_USERNAME` optional
- `OPENSEARCH_PASSWORD` optional
- `CLICKHOUSE_DSN`
- `CLICKHOUSE_DATABASE`
- `CLICKHOUSE_TABLE`

`query-alert-plane`:

- `QUERY_ALERT_HTTP_ADDR`
- `OPENSEARCH_URL`
- `OPENSEARCH_INDEX_PREFIX`
- `OPENSEARCH_USERNAME` optional
- `OPENSEARCH_PASSWORD` optional
- `CLICKHOUSE_DSN`
- `CLICKHOUSE_DATABASE`
- `CLICKHOUSE_TABLE`
- `RARE_FINGERPRINT_ENABLED`
- `RARE_FINGERPRINT_WINDOW_MINUTES`
- `RARE_FINGERPRINT_MAX_COUNT`
- `RARE_FINGERPRINT_SEVERITY`
- `ANOMALY_EVALUATION_INTERVAL_SECS`
- `ANOMALY_RULE_CACHE_TTL_SECS`

See [`./.env.example`](./.env.example).

## Local run

Full runtime stack through compose:

```bash
docker compose up --build
```

Or run planes directly after exporting env from `.env.example`:

```bash
cd server-rs
cargo run -p enrollment-plane
cargo run -p control-plane
cargo run -p deployment-plane
cargo run -p ingestion-plane
cargo run -p query-alert-plane
```

## Runtime smoke gates

The cross-plane smoke suites are kept as explicit gates because they require a live Postgres and NATS stack.

Run them after the local stack is up:

```bash
make server-smoke
```

## Useful checks

```bash
cargo check --manifest-path Cargo.toml \
  -p common \
  -p enrollment-plane \
  -p control-plane \
  -p deployment-plane \
  -p ingestion-plane \
  -p query-alert-plane
```
