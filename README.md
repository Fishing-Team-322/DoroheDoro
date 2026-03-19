# DoroheDoro Server Data Plane MVP

Runnable MVP backend data plane for self-hosted Linux log collection and analysis.

## What is included

Single Go binary `server` with internal modules for:

- gRPC ingestion endpoint for batched logs
- lightweight normalization into one unified event model
- NATS JetStream as the internal bus
- OpenSearch indexer consumer
- HTTP search/query API for the UI
- WebSocket live stream hub for new events
- `fake-agent` demo client for sending test batches

## Repository structure

```text
.
├── .env.example
├── Makefile
├── README.md
├── cmd
│   ├── fake-agent
│   │   └── main.go
│   └── server
│       └── main.go
├── deployments
│   └── docker
│       ├── fake-agent.Dockerfile
│       └── server.Dockerfile
├── docker-compose.yml
├── go.mod
├── internal
│   ├── app
│   │   └── app.go
│   ├── bus
│   │   └── jetstream.go
│   ├── config
│   │   └── config.go
│   ├── grpcapi
│   │   └── server.go
│   ├── httpapi
│   │   └── router.go
│   ├── indexer
│   │   └── opensearch
│   │       └── indexer.go
│   ├── ingest
│   │   └── service.go
│   ├── model
│   │   └── event.go
│   ├── normalize
│   │   ├── errors.go
│   │   └── normalizer.go
│   ├── query
│   │   └── opensearch.go
│   ├── stream
│   │   └── hub.go
│   └── telemetry
│       └── logger.go
├── pkg
│   └── proto
│       ├── ingest.pb.go
│       └── json_codec.go
├── proto
│   └── ingest.proto
└── scripts
    └── sample-batch.json
```

## Unified normalized event model

Each ingested event is normalized into one internal shape:

- `id`
- `timestamp`
- `host`
- `agent_id`
- `source_type`
- `source`
- `service`
- `severity`
- `message`
- `fingerprint`
- `labels`
- `fields`
- `raw`

## Runtime flow

1. `fake-agent` sends `LogBatch` to gRPC ingestion.
2. `server` validates and normalizes events.
3. normalized events are published to JetStream subject `logs.normalized`.
4. OpenSearch indexer consumes from JetStream and writes into `logs-YYYY.MM.DD` indices.
5. HTTP API searches OpenSearch.
6. WebSocket endpoint broadcasts newly accepted events in-process.

## Quick start

### Start infra and server

```bash
docker compose up --build server nats opensearch
```

### Start fake-agent once

```bash
docker compose run --rm fake-agent
```

### Start fake-agent as repeating demo sender

```bash
docker compose --profile demo up --build
```

### Search logs

```bash
curl 'http://localhost:8080/api/v1/logs/search?q=nginx&limit=20'
```

### Get event context

```bash
curl 'http://localhost:8080/api/v1/logs/<event-id>/context'
```

### Health checks

```bash
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz
```

### WebSocket live stream

Any WebSocket client can connect:

```bash
wscat -c ws://localhost:8080/api/v1/stream/ws
```

Optional filters:

```bash
wscat -c 'ws://localhost:8080/api/v1/stream/ws?host=demo-host&service=nginx&severity=warn'
```

### Send a custom batch file

```bash
docker compose run --rm \
  -e FAKE_AGENT_FILE=/app/scripts/sample-batch.json \
  fake-agent
```

## Main HTTP API

### `GET /healthz`
Basic liveness.

### `GET /readyz`
Checks OpenSearch reachability.

### `GET /api/v1/logs/search`
Query params:

- `q`
- `from` (RFC3339 or unix ms)
- `to` (RFC3339 or unix ms)
- `host`
- `service`
- `severity`
- `limit`
- `offset`

Response:

```json
{
  "items": [],
  "total": 0,
  "took_ms": 4
}
```

### `GET /api/v1/logs/:id/context`
Returns the matching event and nearby entries from OpenSearch.

### `GET /api/v1/stream/ws`
Streams accepted normalized events as:

```json
{
  "type": "event",
  "event": {
    "id": "...",
    "timestamp": "...",
    "host": "demo-host"
  }
}
```

## Main env vars

- `HTTP_LISTEN_ADDR`
- `GRPC_LISTEN_ADDR`
- `NATS_URL`
- `NATS_STREAM_NAME`
- `NATS_SUBJECT`
- `NATS_INDEXER_CONSUMER`
- `OPENSEARCH_URL`
- `OPENSEARCH_INDEX_PREFIX`
- `OPENSEARCH_USERNAME`
- `OPENSEARCH_PASSWORD`
- `LOG_LEVEL`
- `WS_BUFFER_SIZE`

See `.env.example` for sane defaults.

## Notes about protobuf generation in this environment

The repository includes the `.proto` contract in `proto/ingest.proto` and checked-in `pkg/proto/ingest.pb.go` so the MVP can stay runnable even in restricted environments where `protoc` plugins cannot be downloaded on the fly.

## Restricted environment caveats

If your local or CI environment blocks access to `proxy.golang.org`, GitHub, or Docker, then `go mod tidy`, `go test`, and `docker compose up --build` will fail before the application code is exercised. In a normal developer machine or CI runner with outbound network access, the provided `go.mod`, Dockerfiles, and compose stack are intended to build the full demo path end-to-end.

## TODO after hackathon demo

- swap the checked-in lightweight gRPC shim for fully generated protobuf code in CI
- add batch bulk flushing for OpenSearch consumer
- add proper OpenSearch mappings/templates
- add ClickHouse analytics indexer + endpoints
- add ingestion authn/authz
- add replay / DLQ / retention controls
- add persistent subscription filters for live stream
