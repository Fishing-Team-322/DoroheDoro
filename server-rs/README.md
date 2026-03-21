# server-rs

Rust runtime foundation for the private `SERVER` components.

The first runtime component is `enrollment-plane`, which owns:

- agent enrollment
- policy fetch
- heartbeat persistence
- diagnostics persistence

Shared protobuf contracts live under `/contracts/proto`.

## Layout

```text
server-rs/
├── Cargo.toml
├── .env.example
├── README.md
├── migrations/
└── crates/
    ├── common/
    └── enrollment-plane/
```

## Local run

1. Start dependencies:

```bash
docker compose up -d postgres nats
```

2. Run the service:

```bash
cd server-rs
cargo run -p enrollment-plane
```

Migrations are applied automatically on startup.

3. Smoke-test through integration tests:

```bash
cargo test -p enrollment-plane --test smoke -- --ignored --nocapture
```

## Subjects

`enrollment-plane` listens to:

- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`
- `agents.registry.list`
- `agents.registry.get`
- `agents.diagnostics.get`
- `control.policies.list`
- `control.policies.get`
- `control.policies.revisions`

## Health endpoints

- `GET /healthz`
- `GET /readyz`

## Current scope

Implemented:

- shared contract generation for Rust
- PostgreSQL-backed enrollment state
- NATS request/reply and publish handlers
- dev bootstrap seeding for policy and bootstrap token
- read-only agents and policies bridge used by `edge_api`

Not implemented yet:

- mTLS / PKI lifecycle
- `agent-rs`
- `deployment-plane`
- `ingestion-plane`
- `query-alert-plane`
- `control-plane`
