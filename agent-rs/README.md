# doro-agent

`doro-agent` is the first standalone Rust AGENT service for DoroheDoro. It reads YAML config, persists local state in SQLite, tails file sources, builds batches, and sends heartbeat/diagnostics/log payloads through a replaceable transport layer.

## Features in this MVP

- YAML config plus scalar env overrides
- SQLite state in `state_dir/state.db`
- file-tail source MVP with restart-safe offsets
- batch flush by event count or timer
- `mock` transport for local smoke testing
- current `edge_api` unary gRPC JSON transport for real ingress
- heartbeat and diagnostics loops

## Build

```bash
cd agent-rs
cargo build
```

## Local smoke test with mock transport

1. Create a test log file:

```bash
mkdir -p /tmp/doro-agent
touch /tmp/doro-test.log
```

2. Start the agent:

```bash
cd agent-rs
cargo run -- --config ./config/agent.example.yaml
```

3. Append lines to the file:

```bash
echo "first line" >> /tmp/doro-test.log
echo "second line" >> /tmp/doro-test.log
```

4. Restart the agent and verify that it resumes from the committed offset in `state.db`.

## Notes

- `transport.mode=edge` uses the current Go `edge_api` gRPC ingress format from `edge_api/contracts/proto/edge.proto`.
- There is still a known repo-level TODO: the Go edge JSON gRPC bridge is not yet contract-aligned with the shared protobuf transport model used by `server-rs`.
- Remote Linux run instructions live in [`docs/agent-runtime.md`](../docs/agent-runtime.md).
