# doro-agent

`doro-agent` is the Rust `AGENT` service for DoroheDoro. It is a Linux collector with SQLite local state, bounded queues, fallback spool, policy-driven file sources, and runtime-oriented troubleshooting artifacts.

## Current scope

- first-run enrollment through the current Go edge ingress without changing the existing wire contract
- persisted local identity, applied policy revision, offsets, and spool metadata in `state_dir/state.db`
- policy-driven file source runtime for `transport.mode=edge`
- `mock` mode for local smoke tests with static `sources`
- heartbeat, diagnostics, local health snapshot, and degraded-mode reporting
- `preflight`, `doctor`, and `health` commands with stable text and JSON output
- local troubleshooting artifacts under `state_dir/runtime/`

## Build

```bash
cd agent-rs
cargo build
cargo build --release
```

## Configs And Packaging

- `config/agent.example.yaml`: local `mock` smoke config
- `config/agent.container.example.yaml`: Linux/container rollout example for `transport.mode=edge`
- `config/agent.env.example`: env override template for packaged or containerized runs
- `packaging/systemd/doro-agent.service`: systemd unit
- `packaging/container/Dockerfile`: runtime container image

`transport.mode=edge` ignores local `sources` and waits for the fetched policy. `transport.mode=mock` uses static config sources and is the easiest path for local smoke tests.

## CLI

Direct run still works:

```bash
cargo run -- --config ./config/agent.example.yaml
```

Preferred explicit subcommands:

```bash
cargo run -- run --config ./config/agent.example.yaml
cargo run -- preflight --config ./config/agent.example.yaml
cargo run -- doctor --config ./config/agent.example.yaml --json
cargo run -- health --config ./config/agent.example.yaml --json
```

Contracts:

- `preflight` and `doctor` are the same non-mutating static checks
- `health` reads the persisted local diagnostics snapshot and validates the live runtime
- exit code `0`: success or warnings only
- exit code `2`: preflight failure
- exit code `3`: health failure
- exit code `1`: runtime startup or runtime shutdown failure

`doctor --json` and `health --json` emit:

- `config_path`
- `generated_at`
- `summary.check_count`
- `summary.warning_count`
- `summary.failure_count`
- `summary.overall_status`
- `checks[].name`
- `checks[].status`
- `checks[].detail`
- `checks[].category`
- `checks[].hint`

## Startup Phases

The runtime logs startup in these phases:

- `config_load`
- `runtime_metadata_detect`
- `state_db_open`
- `transport_init`
- `enrollment_connect`
- `source_validation`
- `background_loops_start`

Runtime phases visible in diagnostics and heartbeat:

- `starting`
- `enrolling`
- `policy_syncing`
- `online`
- `degraded`
- `stopping`
- `error`

## Preflight And Health

`preflight` validates:

- config parsing
- install mode resolution
- systemd/container expectations
- state, spool, and runtime-artifact directories
- `state.db` access
- transport endpoint shape and TCP reachability
- TLS and mTLS PEM material
- source path readability for static source configs
- compatibility and permission issues

`health` validates:

- fresh local diagnostics snapshot
- real runtime phase, not just config shape
- recent heartbeat and diagnostics scheduler activity
- recent successful transport handshake for `edge` mode
- state DB availability
- blocked delivery state
- active source summary

The container image uses `health` for `HEALTHCHECK`. The packaged systemd unit uses `preflight` in `ExecStartPre=`.

## Local Troubleshooting Artifacts

Important files on Linux hosts:

- `state_dir/state.db`
- `state_dir/runtime/diagnostics-snapshot.json`
- `state_dir/security/last-report.json` when security posture persistence is enabled
- spool payloads under `spool.dir`

The diagnostics snapshot includes:

- timestamp and build metadata
- install and transport modes
- enrollment state
- last successful transport handshake
- last transport error
- heartbeat and diagnostics loop state
- source summary and per-source state
- spool backlog
- warning and failure lists

## Troubleshooting

If the agent is installed but not connected:

1. `doro-agent preflight --config /etc/doro-agent/config.yaml`
2. `journalctl -u doro-agent -f`
3. inspect `state_dir/runtime/diagnostics-snapshot.json`
4. inspect `connectivity_state.last_connect_error` and `connectivity_state.last_tls_error`

If the agent is connected but not shipping logs:

1. inspect `source_summary`, `source_statuses`, `warning_list`, and `failure_list`
2. inspect `spooled_batches`, `spooled_bytes`, and `blocked_delivery`
3. inspect `policy_state.last_policy_error`
4. inspect file offsets and spool backlog in SQLite

Typical patterns:

- `runtime_status=policy_syncing` or `degraded` with `last_policy_error`: policy fetch or parse/apply failed
- `connectivity_state.last_tls_error`: CA, cert, key, or hostname verification is wrong
- `connectivity_state.last_connect_error`: endpoint is unreachable or intermittently failing
- source status `waiting`: file is missing and the reader is polling
- source status `error`: file exists but is unreadable or the reader loop restarted after a runtime failure
- `blocked_delivery=true`: sender hit a permanent transport failure and stopped live delivery

Useful SQLite checks:

```bash
sqlite3 /var/lib/doro-agent/state.db 'select * from agent_runtime_state;'
sqlite3 /var/lib/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /var/lib/doro-agent/state.db 'select batch_id, approx_bytes, attempt_count from spool_batches;'
```

More runtime detail lives in [`docs/runtime-audit.md`](./docs/runtime-audit.md) and [`../docs/agent-runtime.md`](../docs/agent-runtime.md).
