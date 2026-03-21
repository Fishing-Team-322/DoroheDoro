# Agent Runtime

This document describes the current standalone Rust agent under `agent-rs/`.

## Runtime model

The agent uses a lightweight concurrent pipeline:

```text
[file readers]
    -> [bounded event queue]
    -> [single batcher]
    -> [bounded send queue]
    -> [single sender]
    -> [ack processing]
    -> [single SQLite state writer]

+ heartbeat worker
+ diagnostics worker
+ degraded-mode controller
+ fallback spool on pressure, shutdown, or send failures
```

Normal hot path is memory-first:

- readers tail files and push events into the bounded queue
- batcher flushes by count, approximate bytes, or timer
- sender waits for transport `ack`
- state writer advances `acked_offset` only after successful send
- sender drains spool fairly instead of permanent spool-first priority

## Config shape

Example:

```yaml
edge_url: "https://edge.example.local"
edge_grpc_addr: "edge.example.local:9090"
bootstrap_token: "dev-bootstrap-token"
state_dir: "/var/lib/doro-agent"
log_level: "info"

transport:
  mode: "edge"

install:
  mode: "auto"

platform:
  allow_machine_id: false

scope:
  configured_cluster_id: "cluster-a"
  cluster_name: "production"
  service_name: "system-logs"
  environment: "prod"
  configured_cluster_tags:
    tenant: "team-a"
  host_labels:
    role: "edge"

heartbeat:
  interval_sec: 30

diagnostics:
  interval_sec: 30

batch:
  max_events: 500
  max_bytes: 524288
  flush_interval_ms: 2000
  compress_threshold_bytes: 16384

queues:
  event_capacity: 4096
  send_capacity: 32
  event_bytes_soft_limit: 8388608
  send_bytes_soft_limit: 16777216

degraded:
  failure_threshold: 3
  server_unavailable_sec: 30
  queue_pressure_pct: 80
  queue_recover_pct: 40
  unacked_lag_bytes: 16777216
  shutdown_spool_grace_sec: 5

spool:
  enabled: true
  dir: "/var/lib/doro-agent/spool"
  max_disk_bytes: 268435456

sources:
  - type: "file"
    source_id: "file:/var/log/syslog"
    path: "/var/log/syslog"
    start_at: "end"
    source: "syslog"
    service: "host"
    severity_hint: "info"
```

Important defaults:

- `sources[].start_at` defaults to `end`
- `sources[].source_id` defaults to `file:<path>`
- `diagnostics.interval_sec` defaults to `heartbeat.interval_sec`
- `spool.dir` defaults to `<state_dir>/spool`
- `install.mode` defaults to `auto`
- `platform.allow_machine_id` defaults to `false`

Scalar env overrides exist for:

- `EDGE_URL`
- `EDGE_GRPC_ADDR`
- `BOOTSTRAP_TOKEN`
- `STATE_DIR`
- `LOG_LEVEL`
- `HEARTBEAT_INTERVAL_SEC`
- `DIAGNOSTICS_INTERVAL_SEC`
- `BATCH_*`
- `QUEUE_*`
- `DEGRADED_*`
- `SPOOL_*`
- `TRANSPORT_MODE`
- `INSTALL_MODE`
- `ALLOW_MACHINE_ID`
- `CLUSTER_ID`
- `CLUSTER_NAME`
- `SERVICE_NAME`
- `ENVIRONMENT`

`configured_cluster_tags` and `host_labels` intentionally stay YAML-only in this phase.

## Platform, build, and install metadata

Diagnostics now carry a stable nested metadata model.

### `platform`

- `os_family`
- `distro_name`
- `distro_version`
- `kernel_version`
- `architecture`
- `hostname`
- `machine_id_hash`
- `service_manager`
- `systemd_detected`

`machine_id_hash` is present only when machine-id collection is explicitly enabled. The agent hashes `/etc/machine-id` or `/var/lib/dbus/machine-id` with SHA-256 and never emits the raw value.

### `build`

- `agent_version`
- `git_commit`
- `build_id`
- `target_triple`
- `build_profile`

If the build runs outside a git checkout, `git_commit` and `build_id` fall back to `unknown`.

### `install`

- `configured_mode`
- `resolved_mode`
- `resolution_source`
- `canonical_layout`
- `systemd_expected`
- `notes`
- `warnings`

Supported configured modes:

- `auto`
- `package`
- `tarball`
- `ansible`
- `dev`

Resolved mode can also become `unknown` when the local layout is ambiguous.

## Canonical install contract

Package-oriented system-managed installs use:

- binary: `/usr/bin/doro-agent`
- config: `/etc/doro-agent/config.yaml`
- env file: `/etc/doro-agent/agent.env`
- state: `/var/lib/doro-agent`
- spool: `/var/lib/doro-agent/spool`
- logs: `/var/log/doro-agent`
- systemd unit: `doro-agent.service`

`deployments/systemd/doro-agent.service` and `agent-rs/packaging/systemd/doro-agent.service` now assume that layout.

Tarball installs are supported too, but they are intentionally not tied to a single binary path. Auto detection treats colocated binary/config layouts as tarball-like. Tarball installs may still choose to symlink the binary into `/usr/bin`.

Runtime does not silently relocate state or spool paths. If configured paths are unusable, startup fails with an actionable error.

## Local state and lifecycle

SQLite lives at `state_dir/state.db`.

Important tables:

- `agent_identity`
- `agent_runtime_state`
- `file_offsets`
- `spool_batches`

`agent_runtime_state` keeps:

- applied policy revision
- last known edge URL
- degraded / blocked delivery flags
- spool enabled marker
- consecutive send failures
- last identity transition status and reason

`file_offsets` store:

- `read_offset`: highest durable local position, including data already durably spooled
- `acked_offset`: highest position confirmed by the server

Restart behavior:

- if there is no spool backlog, readers resume from `acked_offset`
- if there is durable spooled data, readers resume from `read_offset`
- the agent reuses local identity and current policy revision whenever possible

Identity behavior:

- `reused`: local identity exists and the server accepts it
- `newly_enrolled`: no local identity existed
- `re_enrolled`: local identity existed but the server explicitly rejected it

Upgrade and reinstall are expected to preserve `state_dir`. If state survives, the agent should not need a fresh enrollment.

## Diagnostics model

The serialized diagnostics payload contains both dynamic runtime state and static environment metadata:

- top-level runtime counters and queue state
- `platform`
- `build`
- `install`
- `paths`
- `state`
- `compatibility`
- `cluster`
- `identity_status`
- `source_statuses`

### `paths`

- `current_exe`
- `config_path`
- `state_dir`
- `spool_dir`
- `state_db_path`
- canonical package/systemd reference paths

### `state`

- `state_db_path`
- `state_db_exists`
- `state_db_accessible`
- `persisted_identity_present`
- `current_policy_revision`
- `last_known_edge_url`
- spool stats
- `last_successful_send_at`

### `compatibility`

- `notes`
- `warnings`
- `errors`
- `permission_issues`
- `source_path_issues`
- `insecure_transport`

This block is meant for support and deployment troubleshooting, not just logical runtime state.

## Cluster readiness and event enrichment

Cluster-aware server-side policy logic is still a future task, but the agent is ready to carry local scope metadata.

Current `scope` fields feed:

- diagnostics `cluster`
- heartbeat summary metadata
- event label enrichment

Current enrichment keeps the existing wire contract and uses `LogEvent.labels` only. The agent now builds labels through a shared `EventEnrichmentContext` instead of ad-hoc per-source maps.

Current scope-derived event labels are:

- `cluster_id`
- `cluster_name`
- `service_name`
- `environment`
- `host_labels.*`
- existing source labels: `path`, `source`, `source_id`

`configured_cluster_tags` are preserved in diagnostics as `effective_cluster_tags`, but they are not expanded into every event label yet.

The agent intentionally does not parse new cluster data from `policy_body_json` in this phase. That remains a follow-up once the shared contract is fixed.

## `doctor` self-check

Run:

```bash
cd agent-rs
cargo run -- doctor --config ./config/agent.example.yaml
```

`doctor` is non-mutating and checks:

- config syntax
- install mode resolution
- systemd detection vs expectation
- state and spool directory sanity
- existing SQLite state DB accessibility
- source file readability
- insecure HTTP edge transport hints
- local scope metadata visibility

Exit behavior:

- warnings only: exit `0`
- at least one hard failure: exit `2`

## Remote Linux run

1. Build a Linux release binary.

```bash
cd agent-rs
cargo build --release --target x86_64-unknown-linux-gnu
```

2. Copy files to the host with the canonical package layout.

- `/usr/bin/doro-agent`
- `/etc/doro-agent/config.yaml`
- `/etc/doro-agent/agent.env`
- `/var/lib/doro-agent/`
- `/var/log/doro-agent/`

3. Create the service account and directories.

```bash
sudo useradd --system --home /var/lib/doro-agent --shell /usr/sbin/nologin doro-agent || true
sudo mkdir -p /etc/doro-agent /var/lib/doro-agent /var/log/doro-agent
sudo chown -R doro-agent:doro-agent /var/lib/doro-agent /var/log/doro-agent
```

4. Run doctor before enabling the service.

```bash
sudo -u doro-agent /usr/bin/doro-agent doctor --config /etc/doro-agent/config.yaml
```

5. Install the systemd unit.

```bash
sudo cp deployments/systemd/doro-agent.service /etc/systemd/system/doro-agent.service
sudo systemctl daemon-reload
sudo systemctl enable --now doro-agent
```

6. Inspect logs and state.

```bash
sudo systemctl status doro-agent
sudo journalctl -u doro-agent -f
sqlite3 /var/lib/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /var/lib/doro-agent/state.db 'select batch_id, approx_bytes from spool_batches;'
```

## Transport note

The agent still supports two transport modes:

- `mock`
- `edge`

Diagnostics are the authoritative delivery path for new platform/install metadata in this task. Heartbeat now builds a richer metadata summary too, but current edge ingress does not forward heartbeat metadata end-to-end yet because `edge_api/contracts/proto/edge.proto` still omits it.

Known follow-up items that are intentionally out of scope here:

- wire-level batch compression
- transport-visible `batch_id`
- server-issued cluster metadata parsing on the agent
- moving the build-time edge ingress proto dependency into shared `contracts/**`
