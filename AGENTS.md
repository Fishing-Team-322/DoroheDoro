# AGENTS.md

> Operational guide for human developers and coding agents working in this repository.

This file is the **repository-level source of truth** for implementation behavior, boundaries, naming, ownership, and coding constraints.  
If there is a conflict between ad-hoc chat instructions and this file, this file wins unless the project owner explicitly updates it.

---

# 1. Project identity

## 1.1. What this project is

This repository contains a **self-hosted log collection and observability platform** for Linux servers.

The platform **formally consists of 3 product services**:

1. **WEB**
2. **SERVER**
3. **AGENT**

This is an important rule because the case requirements are written around these three services.  
Do **not** describe the system externally as having 8 or 12 product microservices.

---

## 1.2. Internal decomposition model

The `SERVER` service is internally decomposed into:

- a **public Edge API written in Go**
- several **private processing components written in Rust**

This means:

- externally there are still only `WEB`, `SERVER`, `AGENT`
- internally `SERVER` can contain multiple runtime components
- internal components must not be presented as separate product services in user-facing docs/presentations

---

## 1.3. Current repository reality

The repository currently contains a **Go proof-of-concept** under `edge_api/` with these notable modules:

- `cmd/server`
- `cmd/fake-agent`
- `internal/app`
- `internal/grpcapi`
- `internal/httpapi`
- `internal/bus/jetstream`
- `internal/ingest`
- `internal/normalize`
- `internal/indexer/opensearch`
- `internal/indexer/clickhouse`
- `internal/query`
- `internal/stream`
- `internal/enrollment`
- `internal/policy`
- `internal/diagnostics`
- `proto/ingest.proto`

This Go PoC is **useful as a donor/reference**, but it is **not the final architecture**.

---

# 2. Hard architectural decisions

These are already decided. Do not silently re-decide them in code.

## 2.1. Product service boundaries

There are exactly 3 product services:

- `WEB`
- `SERVER`
- `AGENT`

## 2.2. Language split

- **Go** is used for the **public Edge API**
- **Rust** is used for:
  - internal `SERVER` processing components
  - `AGENT`

## 2.3. Event bus

Use **NATS JetStream**, not Kafka.

## 2.4. Visualization

Do **not** introduce Grafana as the product UI.

All product-facing visualization must live in our own **WEB UI**:
- search
- live tail
- charts
- dashboards
- heatmaps
- alerts
- deployment jobs
- diagnostics

## 2.5. Search and analytics stores

Use:
- **OpenSearch** for full-text search and event context
- **ClickHouse** for analytics and aggregations

## 2.6. Control plane persistence

Critical control-plane state must not live only in memory.

Use:
- **PostgreSQL** for source-of-truth control-plane data
- **Vault** for secrets and credential material
- **SQLite** inside the agent for local persisted state

---

# 3. Repository goals

This repository must evolve from a Go proof-of-concept into a platform with:

- public `edge-api` in Go
- Rust-based internal runtime components
- Rust-based Linux agent
- shared transport contracts
- persistent control-plane state
- remote-installable agent
- event-driven internal processing

---

# 4. The most important rule for contributors

## Do not optimize the wrong layer first.

The current biggest missing piece is **not** “better dashboards” or “smarter anomaly detection”.  
The most important missing pieces are:

1. shared contracts
2. persistent enrollment/control-plane state
3. real Rust agent
4. Edge API separation
5. deployment lifecycle foundation

Coding agents must prioritize those before “nice-to-have” features.

---

# 5. Ownership map

## 5.1. Go ownership

Go code owns the **public Edge API layer** only:

- HTTP ingress for WEB
- gRPC ingress for AGENT
- WebSocket/SSE streaming gateway
- request validation
- auth hooks / middleware
- NATS bridge
- transport/error mapping
- public readiness/health
- thin orchestration across internal components

Go must **not** become the hidden owner of all business logic.

---

## 5.2. Rust ownership

Rust owns:

- enrollment-plane
- control-plane logic
- deployment-plane logic
- ingestion-plane logic
- query/alert-plane logic
- agent runtime

Rust components are the long-term business-logic owners.

---

## 5.3. WEB ownership

The WEB service owns:
- UX flows
- dashboards
- inventory UI
- policy builder
- deployment wizard
- search explorer
- alert center
- diagnostics UI

WEB must not depend on Grafana or any external UI product for core functionality.

---

# 6. Source of truth by domain

## 6.1. Source of truth: control plane

Use **PostgreSQL** for:
- users
- roles
- inventory
- host groups
- policies
- policy revisions
- deployment jobs
- agent registry
- alert rules
- audit log
- diagnostics metadata

## 6.2. Source of truth: secrets

Use **Vault** for:
- SSH credentials
- one-time bootstrap tokens
- certificate material / PKI integration
- secret config material

Do not store raw secrets in:
- plaintext repo files
- in-memory singleton stores as the permanent design
- hardcoded examples presented as production defaults

## 6.3. Source of truth: agent local state

Use **SQLite** on the host for:
- agent identity
- local state
- file offsets
- cursor state
- applied policy revision
- send checkpoints
- spool metadata

## 6.4. Source of truth: full-text logs

Use **OpenSearch**.

## 6.5. Source of truth: analytics

Use **ClickHouse**.

## 6.6. Event transport

Use **NATS JetStream** for internal async/request-reply flows.

---

# 7. Repository structure target

This repository is expected to move toward a structure similar to:

```text
repo-root/
├── AGENTS.md
├── architecture.md
├── contracts/
│   ├── proto/
│   └── schemas/
├── edge-api/
│   ├── cmd/edge-api/
│   └── internal/
├── server-rs/
│   ├── Cargo.toml
│   └── crates/
│       ├── common/
│       ├── control-plane/
│       ├── enrollment-plane/
│       ├── deployment-plane/
│       ├── ingestion-plane/
│       └── query-alert-plane/
├── agent-rs/
│   ├── src/
│   ├── config/
│   └── packaging/
├── web/
├── deploy/
│   ├── compose/
│   ├── ansible/
│   ├── systemd/
│   └── examples/
└── legacy/
    └── edge_api-go-poc/
```

This does **not** have to happen in one commit.  
But all new code should move toward this shape, not away from it.

---

# 8. Current codebase interpretation

The current Go PoC under `edge_api/` should be treated as:

## Reusable
- `proto/ingest.proto`
- transport ideas
- JetStream integration ideas
- OpenSearch indexer logic as reference
- ClickHouse indexer logic as reference
- normalization ideas as reference
- stream hub ideas as reference
- fake-agent as smoke-test utility

## To be replaced or reduced
- in-memory enrollment store
- in-memory policy store
- in-memory diagnostics store
- direct ownership of business logic inside Go HTTP/gRPC handlers
- “single-process server does everything forever” assumption

---

# 9. Coding priorities

When choosing between tasks, use this priority order.

## P0
- contracts
- edge-api separation
- PostgreSQL control-plane persistence
- Rust enrollment-plane
- Rust agent MVP
- remote runnable Linux agent
- basic heartbeat/diagnostics persistence
- working file source
- working end-to-end ingest path

## P1
- policy revisions
- deployment jobs
- Ansible integration
- stream gateway hardening
- query endpoints
- alert basics
- Vault integration
- better agent state and spool

## P2
- journald
- multiline
- anomaly detection
- Telegram alerting refinements
- retention tiers
- exports
- cold archive

Do not reverse this order unless explicitly requested.

---

# 10. Rules for coding agents

## 10.1. Before changing architecture

If you are about to add a new runtime component, first check:

1. Is this a **new domain boundary**, or just helper logic?
2. Can this be a module inside an existing component instead?
3. Would this silently violate the “3 product services” external model?

If the answer is unclear, prefer **fewer components**, not more.

---

## 10.2. Do not invent alternative stack decisions

Do not replace:
- NATS with Kafka
- OpenSearch with random search engines
- ClickHouse with general-purpose SQL for analytics
- PostgreSQL with ad-hoc files as control-plane persistence
- SQLite inside the agent with in-memory offsets
- Edge API Go with “let’s just do all of it in Rust right now”

These decisions are already made.

---

## 10.3. Do not add product-facing Grafana

Internal technical dashboards for operators may exist later if needed, but product-facing observability must stay in our own WEB application.

Do not introduce Grafana as:
- the main log explorer
- the main charting UI
- the deployment UI
- the incident UI
- the analytics UI

---

## 10.4. Do not bury source of truth in transport handlers

HTTP handlers, gRPC handlers, and NATS handlers must not become hidden business-logic blobs.

Use proper layers:
- transport
- service/domain
- repository/integration

---

## 10.5. Keep external contracts stable

Changes to:
- protobuf
- HTTP API response shapes
- NATS subjects
- event schemas

must be treated as contract changes, not casual refactors.

---

# 11. API and subject naming rules

## 11.1. NATS subjects

Use clear and domain-scoped subject names.

Examples:

```text
agents.enroll.request
agents.policy.fetch
agents.heartbeat
agents.diagnostics
logs.ingest.raw
logs.ingest.normalized
deployments.jobs.create
deployments.jobs.status
query.logs.search
query.logs.histogram
alerts.firing
ui.stream.logs
```

Avoid vague names like:
- `events`
- `logs`
- `topic1`
- `jobs`
- `stream`

---

## 11.2. IDs

Use explicit IDs:
- `agent_id`
- `policy_id`
- `policy_revision_id`
- `deployment_job_id`
- `request_id`
- `alert_id`

Do not overload one generic `id` in transport if more specificity matters.

---

## 11.3. Time fields

Use explicit UTC timestamps:
- `created_at`
- `updated_at`
- `first_seen_at`
- `last_seen_at`
- `assigned_at`
- `expires_at`

---

# 12. Data flow assumptions

## 12.1. Agent lifecycle flow

1. Agent receives bootstrap config
2. Agent calls Edge API
3. Edge API relays to enrollment-plane
4. Enrollment-plane validates token and returns identity + policy
5. Agent persists identity locally
6. Agent sends heartbeat and diagnostics
7. Agent tails log sources
8. Agent sends batches through Edge API
9. Internal processing routes logs to storage and stream systems

## 12.2. Deployment flow

1. User creates or chooses policy in WEB
2. User selects hosts and credentials
3. WEB calls Edge API
4. Edge API emits deployment command
5. deployment-plane executes Ansible-based install
6. Agent starts on remote host
7. Agent enrolls and becomes visible in registry

## 12.3. Search flow

1. WEB issues search request to Edge API
2. Edge API relays query to query plane
3. Query plane reads from OpenSearch and/or ClickHouse
4. Results are returned to WEB
5. Visualization is rendered in our own UI

---

# 13. Security constraints

These are repository-level constraints.

## 13.1. Public exposure

Only the public `Edge API` should be internet-facing.

Internal Rust runtime may live:
- behind a gray IP
- behind NAT
- on private network
- behind WireGuard/Tailscale/private routing

Agents and WEB should connect to the public Edge API, not to internal runtime addresses.

## 13.2. Agent communication

Long-term expectation:
- outbound agent connection model
- TLS
- mTLS or strong authenticated enrollment path
- request size limits
- retry/backoff
- no inbound connection requirement on the agent host

## 13.3. Secrets

Never commit real:
- tokens
- passwords
- SSH keys
- certs

If an example secret is needed, use clear placeholders.

---

# 14. Documentation requirements

When you add or change a major component, update docs.

At minimum:
- update `architecture.md` if domain behavior changes
- update README or component README if launch steps change
- update contracts documentation if transport changes
- update example configs if config shape changes

Do not leave “the docs will be updated later” unless explicitly requested.

---

# 15. Testing expectations

## 15.1. Unit tests
Expected for:
- repositories
- domain services
- config loaders
- transport validation
- NATS subject mapping
- event normalization helpers

## 15.2. Integration tests
Expected for:
- enrollment flow
- policy fetch flow
- heartbeat persistence
- agent ingest flow
- NATS request/reply bridge
- OpenSearch/ClickHouse integration where realistic

## 15.3. Remote validation
Agent work is not “done” until the agent can be validated on a separate Linux host.

A feature is not complete if it only works on localhost in synthetic conditions.

---

# 16. Definition of useful progress

The following count as real progress:

- a real Rust agent enrolling through Edge API
- persisted agent identity
- a log file tailed on a remote host
- a real batch reaching the pipeline
- a deployment job flow producing a running agent
- a policy revision stored in PostgreSQL
- query endpoint successfully serving WEB

The following do **not** count as meaningful progress by themselves:

- renaming packages for two days
- adding seven crates with no running flow
- replacing one config library with another
- making architecture diagrams without code movement
- adding “smart AI anomaly” code before real agent lifecycle exists

---

# 17. Migration strategy guidance

This repository is in migration.

## Recommended migration pattern
1. freeze old Go PoC behavior as reference
2. extract contracts
3. isolate Edge API
4. add Rust enrollment-plane
5. add Rust agent
6. move source-of-truth state to PostgreSQL/Vault/SQLite
7. then expand internal Rust runtime domains

Do not attempt a big-bang rewrite unless explicitly instructed.

---

# 18. Component-specific notes

## 18.1. Edge API
Keep it thin.
It is a boundary service, not the soul of the product.

## 18.2. Enrollment plane
Should be one of the first Rust components because it unlocks:
- real agent lifecycle
- persistent state
- policy binding
- remote runs

## 18.3. Agent
Agent is one of the highest-value pieces in the system.  
A fake agent is not an acceptable long-term substitute.

## 18.4. Deployment plane
Ansible is the chosen bootstrap/install mechanism for agent rollout.
It is part of the internal `SERVER` lifecycle, not a separate product.

---

# 19. Command and environment conventions

Use predictable naming for env vars.

Examples:
- `HTTP_LISTEN_ADDR`
- `GRPC_LISTEN_ADDR`
- `NATS_URL`
- `POSTGRES_DSN`
- `VAULT_ADDR`
- `OPENSEARCH_URL`
- `CLICKHOUSE_DSN`
- `EDGE_PUBLIC_URL`

Avoid random per-component naming drift.

---

# 20. What a coding agent should read first

If you are a coding agent starting work on this repo, read in this order:

1. `AGENTS.md`
2. `architecture.md`
3. repository root README
4. component-level README for the area you are touching
5. transport contracts (`contracts/proto`, schemas)
6. existing implementation in the affected component
7. only then start code changes

---

# 21. Final summary

This repository is building a **3-service observability platform** with:

- **WEB**
- **SERVER** = Go Edge API + private Rust runtime
- **AGENT** = Rust Linux collector

The current Go PoC is a foundation, not the final shape.  
The main implementation priorities are contracts, real agent lifecycle, persistent state, and proper service boundaries.

If you are unsure whether a change belongs in Go Edge API or Rust runtime:

- **boundary/transport/public ingress** → Go
- **domain logic/stateful control or processing** → Rust
