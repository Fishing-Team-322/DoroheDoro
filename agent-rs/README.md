# doro-agent

`doro-agent` is the standalone Rust `AGENT` service for DoroheDoro. It is a lightweight Linux collector with SQLite local state, bounded queues, degraded mode, fallback spool, environment diagnostics, package-friendly install assumptions, and future-ready cluster metadata enrichment.

## Current runtime highlights

- YAML config plus scalar env overrides
- SQLite state in `state_dir/state.db`
- durable `read_offset` plus `acked_offset` per file source
- bounded event/send queues with a single batcher, sender, and SQLite state writer
- degraded mode and blocked-delivery handling for permanent transport failures
- file-based fallback spool with SQLite metadata
- Linux platform, build, install, and cluster metadata in diagnostics
- scope-aware event label enrichment without changing the ingest wire contract
- `doctor` preflight for post-install and pre-start checks
- `mock` transport for local smoke tests
- current `edge_api` unary gRPC JSON transport for real ingress

## Build

```bash
cd agent-rs
cargo build
```

Release build:

```bash
cd agent-rs
cargo build --release
```

The release profile keeps `thin` LTO, `codegen-units = 1`, and symbol stripping enabled.

## CLI

Runtime execution still works with the old UX:

```bash
cd agent-rs
cargo run -- --config ./config/agent.example.yaml
```

Explicit subcommands are now available:

```bash
cargo run -- run --config ./config/agent.example.yaml
cargo run -- doctor --config ./config/agent.example.yaml
```

`doctor` is non-mutating. It validates config parsing, install mode resolution, path sanity, state/spool write expectations, source readability, systemd hints, and current SQLite state DB accessibility. Warnings keep exit code `0`; hard failures exit with code `2`.

## Portability and Linux assumptions

- The agent targets Linux-family hosts and avoids distro-specific code paths.
- Distro detection uses `/etc/os-release` with fallback to `/usr/lib/os-release`.
- Kernel version is detected from `/proc/sys/kernel/osrelease` or `uname -r`.
- systemd presence is detected from `/run/systemd/system` and common systemd env vars.
- Machine identity is opt-in only. Set `platform.allow_machine_id: true` or `ALLOW_MACHINE_ID=true` to send a hashed machine identifier; raw machine-id is never emitted.
- If `state_dir` or `spool.dir` cannot be used, runtime fails instead of silently relocating state.

## Packaging modes

Configured mode lives in `install.mode`:

- `auto`: resolve from local layout
- `package`: force canonical package/systemd expectations
- `tarball`: force colocated self-managed layout
- `ansible`: force rollout marker for deployment-driven installs
- `dev`: force local development mode

Auto detection prefers:

1. canonical package layout
2. local dev layout
3. tarball-like colocated layout
4. `unknown` with warnings if the layout is ambiguous

Canonical package layout is:

- binary: `/usr/bin/doro-agent`
- config: `/etc/doro-agent/config.yaml`
- env file: `/etc/doro-agent/agent.env`
- state: `/var/lib/doro-agent`
- spool: `/var/lib/doro-agent/spool`
- logs: `/var/log/doro-agent`
- unit: `doro-agent.service`

## Local smoke test

1. Create a test file.

```bash
mkdir -p /tmp/doro-agent
touch /tmp/doro-test.log
```

2. Run the local doctor.

```bash
cd agent-rs
cargo run -- doctor --config ./config/agent.example.yaml
```

3. Start the agent with the bundled mock config.

```bash
cd agent-rs
cargo run -- --config ./config/agent.example.yaml
```

4. Append lines.

```bash
echo "first line" >> /tmp/doro-test.log
echo "second line" >> /tmp/doro-test.log
```

5. Inspect state.

```bash
sqlite3 /tmp/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /tmp/doro-agent/state.db 'select batch_id, approx_bytes from spool_batches;'
```

To force degraded mode and spool, switch to `transport.mode=edge` and point `edge_grpc_addr` at an unavailable endpoint.

## Lifecycle semantics

- Persisted SQLite `read_offset` means durable local progress, not the live cursor.
- Restart reuses persisted identity and applied policy revision when local state is still present.
- Re-enroll happens only when no local identity exists or the server explicitly rejects the stored identity.
- Diagnostics expose `identity_status` as `reused`, `newly_enrolled`, or `re_enrolled`.
- Upgrade and reinstall are expected to preserve `state_dir`; if state is kept, the agent does not force a fresh enrollment.
- Uninstall can remove binary/config/unit independently from state. Keeping `/var/lib/doro-agent` preserves identity, policy revision, file offsets, and spool metadata for a later reinstall.

## Diagnostics and cluster readiness

Diagnostics JSON now includes:

- `platform`
- `build`
- `install`
- `paths`
- `state`
- `compatibility`
- `cluster`
- `identity_status`

Cluster readiness is config-only in this phase:

- `scope.configured_cluster_id`
- `scope.configured_cluster_tags`
- `scope.cluster_name`
- `scope.service_name`
- `scope.environment`
- `scope.host_labels`

These fields are reflected in diagnostics and used for event label enrichment. The agent does not parse new server-side cluster metadata from `policy_body_json` yet.

## Transport notes

- `transport.mode=edge` still uses the current Go `edge_api` ingress shape from `edge_api/contracts/proto/edge.proto`.
- Diagnostics are the authoritative delivery path for the new platform/install metadata today.
- Heartbeat now builds a richer platform/install summary, but current edge ingress does not forward heartbeat metadata end-to-end yet.
- `batch.compress_threshold_bytes` still applies only to local spool payload files in this phase.
- The build-time dependency on `../edge_api/contracts/proto/edge.proto` remains isolated to `build.rs` until shared ingress contracts move under `contracts/**`.

More operational detail lives in [`docs/agent-runtime.md`](../docs/agent-runtime.md).
