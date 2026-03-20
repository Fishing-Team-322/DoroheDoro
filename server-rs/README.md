# server-rs

Rust runtime foundation for the private `SERVER` components.

The first runtime component is `enrollment-plane`, which owns:

- agent enrollment
- policy fetch
- heartbeat persistence
- diagnostics persistence

The second runtime component is `control-plane`, which manages:

- policy metadata lifecycle (create/update/list/revisions)
- inventory (hosts + host groups)
- credentials metadata registry
- internal NATS request/reply contracts for the control-plane domains

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
    ├── control-plane/
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

3. Run the services:

```bash
cargo run -p enrollment-plane
# in a separate terminal
cargo run -p control-plane
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

`control-plane` listens to:

- `control.policies.list`
- `control.policies.get`
- `control.policies.create`
- `control.policies.update`
- `control.policies.revisions`
- `control.hosts.list`
- `control.hosts.get`
- `control.hosts.create`
- `control.hosts.update`
- `control.host-groups.list`
- `control.host-groups.get`
- `control.host-groups.create`
- `control.host-groups.update`
- `control.host-groups.add-member`
- `control.host-groups.remove-member`
- `control.credentials.list`
- `control.credentials.get`
- `control.credentials.create`

## Health endpoints

- `GET /healthz`
- `GET /readyz`

## Current scope

Implemented:

- shared contract generation for Rust
- PostgreSQL-backed enrollment state
- NATS request/reply and publish handlers
- dev bootstrap seeding for policy and bootstrap token
- control-plane Postgres repositories for policy/inventory/credentials
- control-plane NATS handlers + health endpoints

Not implemented yet:

- mTLS / PKI lifecycle
- `agent-rs`
- `deployment-plane`
- `ingestion-plane`
- `query-alert-plane`
- Go `edge_api` bridge to the new subjects
