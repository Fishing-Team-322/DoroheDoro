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
+ fallback spool only on pressure or send failures
```

Normal hot path is memory-first:

- readers tail files and publish events into the bounded in-memory queue
- batcher flushes by count, approximate bytes, or timer
- sender waits for transport `ack`
- state writer advances `acked_offset` only after successful send

Fallback spool is not the default path:

- batches are written to spool only during degradation, queue pressure, or shutdown with unsent data
- spool payloads are stored as files under `spool.dir`
- SQLite keeps spool metadata and per-source `read_offset` / `acked_offset`

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

## Local state

SQLite lives at `state_dir/state.db`.

Important tables:

- `agent_identity`
- `agent_runtime_state`
- `file_offsets`
- `spool_batches`

`file_offsets` now stores two durable offsets:

- `read_offset`: highest locally durable position, including data already durably spooled
- `acked_offset`: highest position confirmed by the server

Restart behavior:

- if there is no spool backlog, readers resume from `acked_offset`
- if there is durable spooled data, readers resume from `read_offset`
- the agent does not write every batch to disk by default

## File source behavior

Current file-source runtime supports:

- `start_at: beginning|end`
- missing-file recovery
- copytruncate handling
- inode rotation detection
- reopen after rotation

Current non-goals for this phase:

- journald
- multiline
- glob expansion
- local debug HTTP API

## Local run

```bash
cd agent-rs
cargo test
cargo run -- --config ./config/agent.example.yaml
```

For a healthy local run:

- use `transport.mode=mock`
- point a source at `/tmp/doro-test.log`
- append lines and watch logs
- inspect `state.db` to confirm `acked_offset` advances

To validate degraded mode and spool:

1. switch to `transport.mode=edge`
2. point `edge_grpc_addr` at an unavailable endpoint
3. append lines to the test file
4. confirm diagnostics report `degraded_mode=true`
5. confirm `spool_batches` gets rows and `acked_offset` stops advancing until recovery

Useful commands:

```bash
sqlite3 /var/lib/doro-agent/state.db '.tables'
sqlite3 /var/lib/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /var/lib/doro-agent/state.db 'select batch_id, attempt_count, next_retry_at_unix_ms from spool_batches;'
```

## Remote Linux run

1. Build a release binary for Linux.

```bash
cd agent-rs
cargo build --release --target x86_64-unknown-linux-gnu
```

2. Copy files to the host.

Expected paths:

- `/usr/local/bin/doro-agent`
- `/etc/doro-agent/config.yaml`
- `/etc/doro-agent/agent.env`
- `/var/lib/doro-agent/`
- `/var/log/doro-agent/`

3. Create the service account and state directory.

```bash
sudo useradd --system --home /var/lib/doro-agent --shell /usr/sbin/nologin doro-agent || true
sudo mkdir -p /etc/doro-agent /var/lib/doro-agent /var/log/doro-agent
sudo chown -R doro-agent:doro-agent /var/lib/doro-agent /var/log/doro-agent
```

4. Install the systemd unit.

```bash
sudo cp deployments/systemd/doro-agent.service /etc/systemd/system/doro-agent.service
sudo systemctl daemon-reload
sudo systemctl enable --now doro-agent
```

5. Inspect logs and state.

```bash
sudo systemctl status doro-agent
sudo journalctl -u doro-agent -f
sqlite3 /var/lib/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /var/lib/doro-agent/state.db 'select batch_id, approx_bytes from spool_batches;'
```

## Current transport note

The agent still supports two transport modes:

- `mock` for local validation
- `edge` for the current Go `edge_api` ingress

Known limitation for this phase:

- `batch.compress_threshold_bytes` is used only for local spool payload files
- wire-level batch compression and transport-visible `batch_id` remain follow-up work because this task intentionally did not modify `edge_api/**` or `contracts/**`
