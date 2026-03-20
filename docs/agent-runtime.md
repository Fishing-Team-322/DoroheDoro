# Agent Runtime

This document describes the standalone Rust agent runtime added under `agent-rs/`.

## What the agent does

- loads bootstrap config from YAML
- optionally overrides scalar settings from env
- persists identity and offsets in `state_dir/state.db`
- tails one or more configured files
- batches new lines into shared ingest events
- sends heartbeat and diagnostics snapshots through the transport layer

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

batch:
  max_events: 500
  flush_interval_sec: 2

sources:
  - type: "file"
    path: "/var/log/syslog"
    source: "syslog"
    service: "host"
    severity_hint: "info"
```

## Local run

```bash
cd agent-rs
cargo test
cargo run -- --config ./config/agent.example.yaml
```

For a local smoke test, switch `transport.mode` to `mock`, point a source at `/tmp/doro-test.log`, append lines, and confirm that:

- batches are reported in logs
- `state.db` is created under `state_dir`
- restarting the agent does not replay already committed lines

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
sqlite3 /var/lib/doro-agent/state.db '.tables'
sqlite3 /var/lib/doro-agent/state.db 'select * from file_offsets;'
```

## Current transport note

The MVP supports two transport modes:

- `mock` for local validation
- `edge` for the current Go `edge_api` ingress

Known limitation:

- the repo still has a contract alignment gap between the Go edge JSON gRPC bridge and the shared protobuf transport model consumed by `server-rs`; that alignment remains a follow-up for the server/contracts owner
