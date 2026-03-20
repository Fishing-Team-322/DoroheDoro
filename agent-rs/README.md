# doro-agent

`doro-agent` is the standalone Rust AGENT service for DoroheDoro. Phase 2 turns it into a lightweight concurrent collector with bounded in-memory queues, a single sender, a single SQLite state writer, degraded mode, and fallback spool.

## Current runtime features

- YAML config plus scalar env overrides
- SQLite state in `state_dir/state.db`
- two-offset model per file source: `read_offset` and `acked_offset`
- bounded `event queue` and `send queue`
- one reader per file source, one batcher, one sender, one state writer
- adaptive batch flush by event count, size, or timer
- heartbeat and diagnostics workers
- degraded mode controller
- file-based fallback spool with SQLite metadata
- `mock` transport for local smoke testing
- current `edge_api` unary gRPC JSON transport for real ingress

## Build

```bash
cd agent-rs
cargo build
```

## Local smoke test

1. Create a test file.

```bash
mkdir -p /tmp/doro-agent
touch /tmp/doro-test.log
```

2. Start the agent with the bundled mock config.

```bash
cd agent-rs
cargo run -- --config ./config/agent.example.yaml
```

3. Append lines.

```bash
echo "first line" >> /tmp/doro-test.log
echo "second line" >> /tmp/doro-test.log
```

4. Restart the agent and inspect SQLite state.

```bash
sqlite3 /tmp/doro-agent/state.db 'select path, read_offset, acked_offset from file_offsets;'
sqlite3 /tmp/doro-agent/state.db 'select batch_id, approx_bytes from spool_batches;'
```

5. To force degraded mode and fallback spool, switch to `transport.mode=edge` and point `edge_grpc_addr` at an unavailable address. The agent should stop advancing `acked_offset`, enter degraded mode, and start filling `spool_batches` instead of writing every batch to disk by default.

## Notes

- `transport.mode=edge` uses the current Go `edge_api` ingress shape from `edge_api/contracts/proto/edge.proto`.
- `batch.compress_threshold_bytes` is applied only to local spool payload files in this phase. Wire-level batch compression and transport-visible `batch_id` remain explicit follow-up work because `edge_api/**` and `contracts/**` were not changed.
- Remote Linux run instructions live in [`docs/agent-runtime.md`](../docs/agent-runtime.md).
