# edge-api

Thin ingress/orchestration layer:

- WEB -> HTTP ingress -> NATS
- AGENT -> gRPC ingress -> NATS
- UI stream -> SSE gateway -> NATS

## Что удалено / отключено из текущего PoC

- in-memory enrollment / policy / diagnostics stores;
- OpenSearch ownership и прямые query handlers;
- ClickHouse ownership и analytics engine в Go;
- normalizer / indexers / alerting-like ownership в startup path;
- fat handlers с доменной логикой;
- WebSocket stream hub как runtime source-of-truth.

## Целевая структура

```text
/defay1x9
  /cmd
    /edge-api
      main.go
    /fake-agent
      main.go
  /internal
    /app
    /config
    /httpapi
    /grpcapi
    /natsbridge
    /stream
    /auth
    /middleware
    /transport
    /model
    /observability
  /contracts
    /proto
  /docs
  Dockerfile
  README.md
```

## HTTP endpoints

- `GET /healthz`
- `GET /readyz`
- `GET /api/v1/me`
- `GET /api/v1/agents`
- `GET /api/v1/agents/{id}`
- `GET /api/v1/agents/{id}/diagnostics`
- `GET /api/v1/policies`
- `GET /api/v1/policies/{id}`
- `POST /api/v1/deployments`
- `GET /api/v1/deployments/{id}`
- `GET /api/v1/deployments`
- `POST /api/v1/logs/search`
- `GET /api/v1/logs/histogram`
- `GET /api/v1/logs/severity`
- `GET /api/v1/logs/top-hosts`
- `GET /api/v1/logs/top-services`
- `GET /api/v1/alerts`
- `GET /api/v1/alerts/{id}`
- `GET /api/v1/stream/logs`

## gRPC methods

Service: `dorohedoro.edge.v1.AgentIngressService`

- `Enroll`
- `FetchPolicy`
- `SendHeartbeat`
- `SendDiagnostics`
- `IngestLogs`

## Обязательные NATS subjects

- `agents.enroll.request`
- `agents.enroll.response`
- `agents.policy.fetch`
- `agents.policy.response`
- `agents.heartbeat`
- `agents.diagnostics`
- `logs.ingest.raw`
- `deployments.jobs.create`
- `deployments.jobs.status`
- `query.logs.search`
- `query.logs.histogram`
- `query.logs.severity`
- `query.logs.top_hosts`
- `query.logs.top_services`
- `alerts.list`
- `alerts.get`
- `ui.stream.logs`

Дополнительно для UI read-model edge-api ожидает subjects:

- `agents.list`
- `agents.get`
- `agents.diagnostics.get`
- `policies.list`
- `policies.get`
- `deployments.jobs.list`

## Env

Минимально нужны:

- `HTTP_LISTEN_ADDR`
- `GRPC_LISTEN_ADDR`
- `NATS_URL`

Опционально:

- TLS hooks: `HTTP_TLS_CERT_FILE`, `HTTP_TLS_KEY_FILE`, `GRPC_TLS_CERT_FILE`, `GRPC_TLS_KEY_FILE`
- mTLS hook flags: `GRPC_MTLS_ENABLED`, `GRPC_CLIENT_CA_FILE`, `GRPC_MTLS_HOOK_ENABLED`
- limits: `HTTP_MAX_BODY_BYTES`, `AGENT_LOG_BATCH_SIZE`
- timeouts: `HTTP_REQUEST_TIMEOUT`, `GRPC_REQUEST_TIMEOUT`
- stream: `STREAM_HEARTBEAT_INTERVAL`

## Запуск локально

```bash
cd /workspace/DoroheDoro
cd defay1x9 && go build ./...
docker compose config
docker compose up --build
```

## Smoke test

В одном терминале поднимите `nats` и `edge-api`, а в другом прогоните fake-agent:

```bash
cd /workspace/DoroheDoro/defay1x9
EDGE_API_GRPC_ADDR=localhost:9090 go run ./cmd/fake-agent
```

Для полноценного smoke test Rust components должны отвечать на request/reply subjects и публиковать `ui.stream.logs`.

## TODO для интеграции с Rust services

- Реализовать Rust responders для request/reply subjects списков и статусов.
- Реализовать Rust ownership policy / inventory / alerts / deployments / logs query.
- Подключить реальное gRPC mTLS verify вместо stub hooks.
- Добавить protobuf/codegen pipeline вместо ручного lightweight pb.go.
- Добавить contract tests между edge-api и Rust responders.
