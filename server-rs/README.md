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

2. Apply migrations:

```bash
cd server-rs
sqlx migrate run --source migrations
```

3. Run the service:

```bash
cargo run -p enrollment-plane
```

4. Smoke-test through integration tests:

```bash
cargo test -p enrollment-plane --test smoke -- --ignored --nocapture
```

## Subjects

`enrollment-plane` listens to:

- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`

## Health endpoints

- `GET /healthz`
- `GET /readyz`

## Current scope

Implemented:

- shared contract generation for Rust
- PostgreSQL-backed enrollment state
- NATS request/reply and publish handlers
- dev bootstrap seeding for policy and bootstrap token

Not implemented yet:

- mTLS / PKI lifecycle
- `agent-rs`
- `deployment-plane`
- `ingestion-plane`
- `query-alert-plane`
- Go `edge_api` bridge to the new subjects
