# doro-agent

`doro-agent` is the Rust `AGENT` service for DoroheDoro. It is a Linux log collector with SQLite local state, bounded queues, fallback spool, policy-driven file sources, and practical diagnostics for install and transport failures.

## Current scope

- first-run enrollment through the current Go `edge_api` unary gRPC+JSON ingress
- persisted local identity, applied policy revision, offsets, and spool metadata in `state_dir/state.db`
- policy-driven file source runtime for `transport.mode=edge`
- local `mock` mode for smoke tests with static `sources`
- heartbeat and diagnostics with centralized runtime phases
- TLS and mTLS client configuration with explicit PEM path checks
- `doctor` / `check-config` preflight for install, path, permission, and TLS sanity

## Build

```bash
cd agent-rs
cargo build
```

Release:

```bash
cd agent-rs
cargo build --release
```

## CLI

The old direct run form still works:

```bash
cargo run -- --config ./config/agent.example.yaml
```

Explicit subcommands are preferred:

```bash
cargo run -- run --config ./config/agent.example.yaml
cargo run -- doctor --config ./config/agent.example.yaml
cargo run -- check-config --config ./config/agent.example.yaml
```

`doctor` and `check-config` are the same non-mutating preflight. Warnings keep exit code `0`. Hard failures exit with code `2`.

## Practical run notes

For the practical preprod run, use:

- [`../deployments/examples/agent-config.example.yaml`](../deployments/examples/agent-config.example.yaml) for a domain-based `edge` example
- [`../deployments/examples/agent.env.example`](../deployments/examples/agent.env.example) for env overrides
- [`../deployments/systemd/doro-agent.service`](../deployments/systemd/doro-agent.service) for the package/systemd unit
- [`config/agent.example.yaml`](./config/agent.example.yaml) only for local `mock` smoke runs

In `transport.mode=edge` the active sources come from the fetched policy. Local `sources` in config are ignored in that mode.

Recommended defaults for the 3-host run:

- `edge_url`: domain HTTPS URL
- `edge_grpc_addr`: domain gRPC endpoint
- `policy.refresh_interval_sec: 30`
- explicit `tls.ca_path`, `tls.cert_path`, `tls.key_path`
- keep `state_dir` and `spool.dir` on persistent storage across restarts and upgrades

## Install, reinstall, and upgrade behavior

- fresh install means there is no usable `state.db` or no persisted identity inside it
- reinstall or upgrade is expected to preserve `state_dir`
- if persisted identity exists, the agent reuses it
- re-enroll happens only when there is no local identity or the server rejects the stored one
- first boot must complete `enroll -> fetch policy -> parse/apply policy` before file readers start
- if enrollment succeeded but policy sync failed, the next restart reuses the stored identity and retries policy sync
- corrupted or unreadable local state is treated as a hard startup error; the agent does not silently reset SQLite state

Operationally this means `/var/lib/doro-agent` is the important continuity boundary. Keep it for restarts, reinstalls, and upgrades if you want stable identity, policy revision reuse, and durable offsets.

## Runtime lifecycle

Central runtime phases:

- `starting`
- `enrolling`
- `policy_syncing`
- `online`
- `degraded`
- `stopping`
- `error`

These phases are used in diagnostics and heartbeat status. `degraded` is entered for conditions such as blocked delivery, repeated send failures, queue pressure, spool pressure, or policy-sync failure while keeping the last good runtime config.

## Policy and source behavior

Supported server policy subset:

- `paths: ["..."]`
- `sources: ["..."]`
- `sources: [{ "type": "file", "path": "...", "source_id"?, "start_at"?, "source"?, "service"?, "severity_hint"? }]`

Unsupported policy shapes are rejected. The agent does not accept journald, non-file source types, glob paths, or empty paths in this phase.

Normalization defaults:

- `source_id = file:<path>`
- `start_at = end`
- `source = basename(path)` or `host-log`
- `service = host`
- `severity_hint = info`

Policy apply rules:

- invalid policy keeps the last good runtime source set
- source reconcile is done before a new revision is committed during refresh
- missing or unreadable source files are treated as runtime environment issues, not as policy invalidation
- active source state remains visible in diagnostics

## gRPC + TLS + mTLS expectations

- `edge` transport keeps the current unary gRPC+JSON wire shape
- HTTPS endpoints use rustls and optional mTLS client identity
- `tls.server_name` is optional and only needed when the gRPC endpoint uses an IP literal or a different TLS name
- if `edge_grpc_addr` is an IP literal and `tls.server_name` is not set, the agent falls back to the `edge_url` host for TLS verification
- TLS settings are rejected on plaintext HTTP endpoints to avoid silent insecure fallback

`doctor` validates:

- endpoint shape and derived `server_name`
- HTTP vs HTTPS expectations
- `tls.ca_path` PEM sanity
- client certificate and key PEM sanity
- final transport-client configuration consistency

## `doctor` / `check-config`

Preflight covers:

- config parsing
- install-mode resolution
- systemd expectation hints
- state and spool path availability
- `state.db` accessibility
- canonical package log dir hints
- source readability for local `mock` configs
- TLS and mTLS PEM checks
- endpoint and hostname-verification sanity

Typical use on a Linux host:

```bash
sudo -u doro-agent /usr/bin/doro-agent check-config --config /etc/doro-agent/config.yaml
```

The packaged systemd unit runs this preflight with `ExecStartPre=` before `run`.

## Troubleshooting

If the agent does not come online, check in this order:

1. `doro-agent check-config --config /etc/doro-agent/config.yaml`
2. `journalctl -u doro-agent -f`
3. diagnostics `runtime_status`, `runtime_status_reason`, `policy_state`, and `connectivity_state`
4. SQLite runtime state in `/var/lib/doro-agent/state.db`

Common failure patterns:

- `runtime_status=policy_syncing` or `degraded` with `last_policy_error`: policy fetch or parse/apply failed
- `connectivity_state.last_tls_error`: broken CA, cert, key, or hostname verification
- `connectivity_state.last_connect_error`: endpoint unreachable or temporary network failure
- source status `waiting`: file is missing and the reader is polling for it
- source status `error`: the file exists but is unreadable or another source-runtime error occurred

Useful SQLite checks:

```bash
sqlite3 /var/lib/doro-agent/state.db 'select * from agent_runtime_state;'
sqlite3 /var/lib/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /var/lib/doro-agent/state.db 'select batch_id, approx_bytes, attempt_count from spool_batches;'
```

## Manual validation checklist

Before the 3-host run, validate:

1. fresh install with empty `/var/lib/doro-agent`
2. restart with state preserved
3. reinstall or binary replacement with state preserved
4. upgrade with the same `state_dir`
5. domain-based HTTPS/gRPC enrollment
6. policy refresh after a server-side policy change
7. heartbeat, diagnostics, and log delivery from three separate Linux hosts

More runtime detail lives in [`../docs/agent-runtime.md`](../docs/agent-runtime.md).
