# Agent Runtime

This document describes the current standalone Rust agent under `agent-rs/` and the practical behavior expected for the 3-host Linux preprod run.

## Runtime model

The agent uses a lightweight concurrent pipeline:

```text
[policy-driven file readers]
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

Normal hot path stays memory-first. SQLite persists identity, runtime state, source offsets, and spool metadata. The spool is a fallback path, not the primary delivery path.

## Practical startup lifecycle

For `transport.mode=edge`, the startup sequence is:

1. parse config and apply env overrides
2. open or create SQLite state in `state_dir`
3. restore persisted identity and runtime metadata if present
4. enroll if there is no local identity
5. fetch policy from the current edge ingress
6. validate and normalize file sources from policy
7. persist applied policy revision and runtime state
8. start runtime workers and source readers

First boot does not start file readers before `enroll -> fetch policy -> validate/apply policy` succeeds.

If enrollment succeeds but policy sync fails:

- identity remains persisted
- runtime stays stopped or falls back to the last persisted good policy if one exists
- the next restart retries policy sync without forcing a fresh enrollment

If SQLite state is corrupted or unreadable, startup fails with an actionable error instead of silently resetting local state.

## Runtime phases

The runtime phase model is centralized and used in diagnostics plus heartbeat status:

- `starting`
- `enrolling`
- `policy_syncing`
- `online`
- `degraded`
- `stopping`
- `error`

`degraded` is used when the agent keeps running with the last good config but delivery or policy health is impaired.

## Config shape

Production-style `edge` example:

```yaml
edge_url: "https://logs.example.com"
edge_grpc_addr: "logs.example.com:7443"
bootstrap_token: "replace-with-bootstrap-token"
state_dir: "/var/lib/doro-agent"
log_level: "info"

transport:
  mode: "edge"

install:
  mode: "auto"

platform:
  allow_machine_id: false

policy:
  refresh_interval_sec: 30

tls:
  ca_path: "/etc/doro-agent/pki/ca.pem"
  cert_path: "/etc/doro-agent/pki/agent.pem"
  key_path: "/etc/doro-agent/pki/agent.key"
  # server_name: "logs.example.com"

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
```

Important defaults:

- `policy.refresh_interval_sec` defaults to `30`
- `diagnostics.interval_sec` defaults to `heartbeat.interval_sec`
- `security_scan.enabled` defaults to `true`
- `security_scan.interval_sec` defaults to `86400`
- `security_scan.jitter_sec` defaults to `900`
- `security_scan.timeout_sec` defaults to `120`
- `security_scan.max_parallel_checks` defaults to `4`
- `spool.dir` defaults to `<state_dir>/spool`
- `install.mode` defaults to `auto`
- `platform.allow_machine_id` defaults to `false`

Important mode rule:

- in `transport.mode=edge`, active `sources` come from fetched policy
- in `transport.mode=mock`, active `sources` come from local config

## Policy handling

Supported policy subset in this phase:

- `paths: ["..."]`
- `sources: ["..."]`
- `sources: [{ "type": "file", "path": "...", "source_id"?, "start_at"?, "source"?, "service"?, "severity_hint"? }]`

Normalization rules:

- string path becomes `source_id=file:<path>`
- `start_at` defaults to `end`
- `source` defaults to basename or `host-log`
- `service` defaults to `host`
- `severity_hint` defaults to `info`

Rejected inputs:

- non-file source types
- journald
- glob paths
- empty paths

Policy refresh rules:

- refresh runs every `policy.refresh_interval_sec`
- invalid policy does not replace the last good runtime source set
- during refresh, source reconcile succeeds before the new revision is committed
- missing files or unreadable files are runtime/source issues, not policy invalidation

## File source runtime edge cases

Current file runtime behavior is designed for practical Linux host cases:

- `start_at=beginning` starts from the head only when there is no durable local state
- `start_at=end` starts at EOF only on first observation without state
- missing files keep the source in `waiting` and retry
- later file appearance is detected without restart
- truncate on the same inode resets progress correctly
- rename rotation and new inode replacement continue with durable offsets keyed by path plus file identity
- restart with spool backlog resumes from durable `read_offset`
- restart without backlog resumes from `acked_offset`

Multi-source reconcile rules:

- workers are keyed by source path
- unchanged sources stay running
- removed or changed sources stop gracefully
- added sources start without restarting the whole agent
- persisted offsets are reused when the path stays the same

## Local state

SQLite lives at `state_dir/state.db`.

Important persisted state:

- local identity
- applied policy revision and raw policy body
- runtime phase and phase reason
- last policy fetch/apply timestamps
- last policy error
- last connect error
- last TLS error
- last handshake success timestamp
- degraded and blocked-delivery markers
- security posture last-run state and finding summary
- source offsets
- spool metadata

This is why reinstall and upgrade flows should preserve `state_dir`.

The last successful security posture report is also optionally mirrored to:

- `state_dir/security/last-report.json`

## Install, reinstall, and upgrade semantics

- fresh install: no usable `state.db` or no persisted identity
- reinstall: binary/config replaced while `state_dir` survives
- upgrade: new binary with preserved `state_dir`
- restart: same binary or service bounce with preserved state

Identity rules:

- persisted identity is reused whenever possible
- re-enroll happens only when identity is missing or rejected by the server
- when mTLS is enabled, the client certificate CN or first SAN becomes the canonical logical `agent_id`
- restarting or re-enrolling with the same client certificate must keep the same logical agent record
- diagnostics expose `identity_status` as `reused`, `newly_enrolled`, or `re_enrolled`

Operational guidance:

- keep `/var/lib/doro-agent` across service restarts, reinstalls, and upgrades
- deleting local state intentionally forces fresh identity and policy lifecycle

## Diagnostics meaning

Diagnostics JSON now includes both runtime state and deployment context.

Most important top-level fields:

- `runtime_status`
- `runtime_status_reason`
- `current_policy_revision`
- `degraded_mode`
- `blocked_delivery`
- `source_statuses`
- `security_posture`

### `security_posture`

- `enabled`
- `profile`
- `interval_sec`
- `jitter_sec`
- `timeout_sec`
- `last_started_at`
- `last_finished_at`
- `last_status`
- `last_status_reason`
- `last_report_id`
- `last_delivery_status`
- `last_delivery_error`
- `last_rules_loaded_at`
- `last_rules_digest`
- `last_report_path`
- `backoff_until`
- `consecutive_failures`
- `summary`

## Security posture worker

The runtime now starts an additional background worker for periodic host security posture scans.

Current checks:

- listening TCP/UDP sockets from `/proc/net/*` with best-effort process attribution from `/proc/<pid>/fd`
- watched package and binary versions with native `dpkg`/`rpm`/`apk` comparison where available
- world-writable critical file checks
- firewall state checks via `nftables`, `iptables`, `firewalld`, `ufw`, and `systemctl`
- root SSH login policy checks via `sshd -T` with `sshd_config` fallback

Behavior notes:

- the worker is isolated from the log hot path and does not block ingestion
- non-Linux hosts publish a skipped event and stop the worker
- reports reuse diagnostics transport so no extra edge API surface was required
- report persistence is local to `state_dir/security/last-report.json`

### `policy_state`

- `current_policy_revision`
- `last_policy_fetch_at`
- `last_policy_apply_at`
- `last_policy_error`
- `active_source_count`

### `connectivity_state`

- `endpoint`
- `tls_enabled`
- `mtls_enabled`
- `server_name`
- `ca_path`, `cert_path`, `key_path`
- presence markers for those paths
- `last_connect_error`
- `last_tls_error`
- `last_handshake_success_at`

### `state`

- `state_db_path`
- `state_db_exists`
- `state_db_accessible`
- `persisted_identity_present`
- `current_policy_revision`
- `last_known_edge_url`
- spool stats
- `last_successful_send_at`

### `platform`, `build`, `install`, `paths`, `compatibility`, `cluster`

These blocks are support-oriented context for Linux host troubleshooting, packaging mode detection, and scope metadata visibility.

## TLS and connectivity troubleshooting

Expected behavior:

- HTTPS endpoints use rustls
- mTLS client auth is optional in config but required if the edge endpoint enforces it
- when mTLS is enabled on the edge, `req.agent_id` must match the client certificate identity
- if `edge_grpc_addr` uses an IP literal, TLS verification falls back to `edge_url` host unless `tls.server_name` explicitly overrides it
- TLS settings are rejected on plaintext HTTP endpoints

`doctor` / `preflight` validates:

- config syntax
- endpoint shape
- derived server name
- CA bundle PEM sanity
- client certificate/key PEM sanity
- final transport-client config consistency

Typical signals:

- `last_tls_error`: wrong CA, broken cert/key, hostname mismatch, bad PEM, or handshake failure
- `last_connect_error`: DNS, routing, refused connection, timeout, or temporary endpoint outage
- `runtime_status=degraded` with `last_policy_error`: policy fetch or apply problem while last good runtime config is still in use

## Fresh Bootstrap And Re-Enrollment Flow

The current reproducible bootstrap flow for a real Linux-host `agent-rs` is:

1. issue or choose an mTLS client certificate
2. use the certificate CN or SAN value as the expected logical `agent_id`
3. issue a bootstrap token through `POST /api/v1/agents/bootstrap-tokens`
4. start `agent-rs` with the bootstrap token, edge address, CA, client cert, and client key
5. wait for `enroll -> fetch policy -> apply policy -> heartbeat -> diagnostics`
6. append a test line to `/tmp/doro-agent-bootstrap.log`
7. verify the line reaches the query path

Important practical note for the default bootstrap source:

- the default policy uses `start_at: end`
- if `/tmp/doro-agent-bootstrap.log` does not exist at agent startup, the source stays in waiting mode
- when the file is later created, the agent opens it at the current EOF
- this means the first line written at file creation time is intentionally skipped
- for smoke verification, create the file first, then append one more line and search for the appended line

Current default bootstrap policy:

- path: `/tmp/doro-agent-bootstrap.log`
- source type: `file`
- no `journald`
- no globs

Useful verification points:

- `GET /api/v1/agents/<agent_id>`
- `GET /api/v1/agents/<agent_id>/diagnostics`
- `GET /api/v1/agents/<agent_id>/policy`
- `POST /api/v1/logs/search`
- local `state_dir/state.db`
- local `state_dir/runtime/diagnostics-snapshot.json`

Practical re-enroll check:

- stop the agent
- keep the same client certificate
- remove local `state.db` or local identity
- start the agent again with the same bootstrap token and certificate
- verify the same logical `agent_id` is reused and no duplicate agent record appears

## `doctor` / `preflight`

Run:

```bash
cd agent-rs
cargo run -- preflight --config ./config/agent.example.yaml
cargo run -- doctor --config ./config/agent.example.yaml --json
```

Preflight checks:

- config parsing
- install mode resolution
- systemd expectation hints
- state and spool path sanity
- SQLite state DB accessibility
- canonical package log dir hints
- local source readability
- transport endpoint sanity
- TLS and mTLS PEM validity

Exit behavior:

- warnings only: exit `0`
- at least one hard failure: exit `2`

## Package/systemd contract

Canonical package layout:

- binary: `/usr/bin/doro-agent`
- config: `/etc/doro-agent/config.yaml`
- env file: `/etc/doro-agent/agent.env`
- state: `/var/lib/doro-agent`
- spool: `/var/lib/doro-agent/spool`
- logs: `/var/log/doro-agent`
- systemd unit: `doro-agent.service`

The packaged unit now runs:

- `ExecStartPre=/usr/bin/doro-agent preflight --config /etc/doro-agent/config.yaml`
- `ExecStart=/usr/bin/doro-agent run --config /etc/doro-agent/config.yaml`

## Remote Linux validation checklist

Before the practical run, validate on separate Linux hosts:

1. fresh install with empty local state
2. restart with state preserved
3. reinstall with state preserved
4. upgrade with state preserved
5. HTTPS + gRPC enrollment through the domain endpoint
6. policy refresh after server-side changes
7. heartbeat, diagnostics, and log delivery from all three agents

Useful commands:

```bash
sudo -u doro-agent /usr/bin/doro-agent check-config --config /etc/doro-agent/config.yaml
sudo systemctl status doro-agent
sudo journalctl -u doro-agent -f
sqlite3 /var/lib/doro-agent/state.db 'select * from agent_runtime_state;'
sqlite3 /var/lib/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /var/lib/doro-agent/state.db 'select batch_id, approx_bytes, attempt_count from spool_batches;'
```
