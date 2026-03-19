# DoroheDoro Data Plane MVP

Hackathon MVP data plane for a self-hosted log platform. The repo keeps the existing ingest path intact and extends it with enrollment, policy sync, diagnostics, better parsing, and optional ClickHouse analytics.

## What works now

Core path preserved:

`fake-agent -> HTTP enroll -> gRPC ingest -> normalize/parser -> NATS JetStream -> OpenSearch -> HTTP search -> WS live stream`

Additional MVP pieces:

- Agent Enrollment API over HTTP with dev bootstrap token.
- Policy Sync API over HTTP with in-memory default policy.
- Diagnostics API for agent runtime state.
- Better parser on top of the normalizer:
  - JSON message parsing into `fields`
  - extraction of `severity`, `service`, `source_type`, `timestamp`, `message`
  - severity fallback is now `unknown`
  - more stable fingerprinting with number / UUID / IP normalization
- OpenSearch indexer improvements:
  - index existence cache
  - batched bulk flush
- Optional ClickHouse analytics consumer and HTTP analytics endpoints.

## Current scope / honest caveats

- gRPC ingest currently runs in dev mode without TLS by default.
- `INGEST_MTLS_ENABLED` and `INGEST_TLS_MODE` exist as config scaffolding, but full certificate bootstrap / mTLS verification is still TODO.
- Agent registry, policy store, and diagnostics are in-memory for the hackathon MVP.
- Alert engine and Telegram notifier are not implemented yet.

## Main APIs

### Health

- `GET /healthz`
- `GET /readyz`

### Enrollment / policy

- `POST /api/v1/enroll`
- `GET /api/v1/policy?agent_id=<id>&current_revision=<rev>`

Example enroll request:

```bash
curl -X POST http://localhost:8080/api/v1/enroll \
  -H 'content-type: application/json' \
  -d '{
    "bootstrap_token": "dev-bootstrap-token",
    "host": "demo-host",
    "metadata": {"agent": "fake-agent"}
  }'
```

### Logs

- `GET /api/v1/logs/search`
- `GET /api/v1/logs/{id}/context`

`/api/v1/logs/{id}/context` now returns:

- `anchor` event
- `before` nearby events
- `after` nearby events

Context is built from the anchor event timestamp plus nearby events from the same host and, when available, service.

### Diagnostics

- `GET /api/v1/agents`
- `GET /api/v1/agents/{id}`
- `GET /api/v1/agents/{id}/diagnostics`

Tracked fields include:

- `agent_id`
- `host`
- `enrolled_at`
- `last_seen`
- `policy_revision`
- `sent_batches`
- `accepted_events`
- `rejected_events`
- `last_error`
- `health`
- `status`

### Analytics

Available when `CLICKHOUSE_ENABLED=true` and ClickHouse is reachable:

- `GET /api/v1/analytics/histogram`
- `GET /api/v1/analytics/severity`
- `GET /api/v1/analytics/top-hosts`
- `GET /api/v1/analytics/top-services`

### Swagger / OpenAPI

- `GET /swagger/index.html` — Swagger UI
- `GET /swagger/doc.json` — generated OpenAPI document

## Quick start

### Infra + server

```bash
docker compose up --build server nats opensearch
```

### Infra + server + repeating demo sender

```bash
docker compose --profile demo up --build
```

### Enable analytics profile

```bash
CLICKHOUSE_ENABLED=true docker compose --profile analytics up --build
```

### One-shot fake-agent run

```bash
docker compose run --rm fake-agent
```

The fake agent now:

1. enrolls over HTTP,
2. receives `agent_id` + initial policy,
3. sends logs over gRPC using the issued `agent_id`.

### Search logs

```bash
curl 'http://localhost:8080/api/v1/logs/search?q=nginx&limit=20'
```

### Generate Swagger docs

```bash
make swagger
```

Then open: `http://localhost:8080/swagger/index.html`

### Get event context

```bash
curl 'http://localhost:8080/api/v1/logs/<event-id>/context'
```

### Watch live stream

```bash
wscat -c ws://localhost:8080/api/v1/stream/ws
```

Optional filters:

```bash
wscat -c 'ws://localhost:8080/api/v1/stream/ws?host=demo-host&service=nginx&severity=warn'
```

## Important env vars

- `HTTP_LISTEN_ADDR`
- `GRPC_LISTEN_ADDR`
- `NATS_URL`
- `NATS_STREAM_NAME`
- `NATS_SUBJECT`
- `NATS_INDEXER_CONSUMER`
- `NATS_ANALYTICS_CONSUMER`
- `OPENSEARCH_URL`
- `OPENSEARCH_INDEX_PREFIX`
- `OPENSEARCH_BULK_FLUSH_SIZE`
- `OPENSEARCH_BULK_FLUSH_INTERVAL`
- `OPENSEARCH_INDEX_CACHE_TTL`
- `ENROLLMENT_TOKEN`
- `INGEST_TLS_MODE`
- `INGEST_MTLS_ENABLED`
- `DEFAULT_POLICY_REVISION`
- `DEFAULT_POLICY_BATCH_SIZE`
- `DEFAULT_POLICY_BATCH_WAIT`
- `CLICKHOUSE_ENABLED`
- `CLICKHOUSE_DSN`
- `CLICKHOUSE_DATABASE`
- `CLICKHOUSE_TABLE`
- `WS_BUFFER_SIZE`

## Notes about Go modules in restricted environments

This repository now targets Go `1.23`. In fully networked environments, run:

```bash
go mod tidy
go build ./...
```

If your environment blocks `proxy.golang.org` / GitHub module downloads, dependency resolution and `go build` will fail before application code is compiled.
