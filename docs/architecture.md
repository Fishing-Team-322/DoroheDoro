# architecture.md

> Подробное описание архитектуры платформы централизованного сбора и анализа логов.  
> Этот документ фиксирует **целевую архитектуру**, **текущее состояние репозитория**, **разделение технологий**, **потоки данных** и **границы ответственности** между сервисами и внутренними компонентами.

---

# 1. Краткое описание

Система представляет собой **self-hosted платформу централизованного сбора и анализа логов Linux-серверов**.

Платформа должна уметь:

- разворачивать и управлять агентами
- собирать логи с Linux-хостов
- сохранять и индексировать события
- выполнять полнотекстовый поиск
- строить аналитику и визуализации
- стримить логи почти в реальном времени
- поддерживать алерты и расследование инцидентов
- предоставлять весь пользовательский функционал через собственный `WEB` UI

---

# 2. Внешняя модель продукта

В системе **формально есть 3 продуктовых сервиса**:

1. **WEB**
2. **SERVER**
3. **AGENT**

Эта модель должна сохраняться, потому что она соответствует требованиям кейса.

---

# 3. Внутренняя архитектурная модель

Внутри `SERVER` разделен на публичный ingress-слой и несколько приватных runtime-компонентов.

## 3.1. Общая схема

```text
                +----------------------+
                |         WEB          |
                | React / TypeScript   |
                +----------+-----------+
                           |
                           | HTTPS / SSE / WebSocket
                           v
                +----------------------+
                |   SERVER: Edge API   |
                |         Go           |
                +----------+-----------+
                           |
                           | NATS request/reply + publish
                           v
        +--------------------------------------------------+
        |          SERVER: приватный Rust runtime          |
        |--------------------------------------------------|
        | control-plane                                    |
        | enrollment-plane                                 |
        | deployment-plane                                 |
        | ingestion-plane                                  |
        | query-alert-plane                                |
        +--------------------------------------------------+
                 |        |        |        |        |
                 v        v        v        v        v
              Postgres  Vault    NATS   OpenSearch ClickHouse

                           ^
                           |
                           | gRPC + TLS через Edge API
                           |
                +----------+-----------+
                |        AGENT         |
                |        Rust          |
                +----------------------+
```

---

# 4. Ключевые архитектурные решения

## 4.1. Go используется только для публичного Edge API

`Edge API` является единственной публичной точкой входа для:

- `WEB`
- `AGENT`

Его задачи:

- прием внешнего трафика
- транспортная валидация
- auth hooks
- формирование запросов во внутреннюю систему
- streaming gateway
- мост к `NATS`
- маппинг ответов и ошибок

Он **не должен** быть долгосрочным владельцем тяжелой бизнес-логики.

---

## 4.2. Rust владеет внутренней бизнес-логикой

Rust отвечает за:

- persistent control-plane логику
- lifecycle enrollment
- orchestration deployment
- обработку ingestion
- orchestration поиска и аналитики
- alert processing
- runtime агента

---

## 4.3. NATS JetStream является внутренним event backbone

Используем **NATS JetStream** для:

- асинхронного распределения событий
- request/reply между `Edge API` и внутренним runtime
- fanout на processing-consumers
- durable replay там, где это нужно

Kafka в архитектуре не используется.

---

## 4.4. Поиск и аналитика разделены

Используем:

- **OpenSearch** для полнотекстового поиска и получения контекста события
- **ClickHouse** для аналитики и агрегаций

Это избавляет от попыток заставить одно хранилище одинаково хорошо решать разные задачи.

---

## 4.5. Визуализация живет в нашем WEB UI

Мы **не используем Grafana** как продуктовый пользовательский интерфейс.

Все продуктовые дашборды и представления должны рендериться в нашем собственном `WEB` сервисе.

---

## 4.6. Текущий исполнимый milestone: первый Rust vertical slice

На текущем этапе мы **не переписываем весь backend на Rust**. Первый обязательный vertical slice должен закрыть самую большую архитектурную дыру между текущим Go PoC и целевой системой:

- общий `contracts` слой
- `enrollment-plane-rs` как внутренний Rust component
- `agent-rs` как реальный Linux-агент
- persistent enrollment/state model на `PostgreSQL` и `SQLite`
- рабочий end-to-end flow через существующий `Edge API`

Целевой поток этого этапа:

```text
agent-rs on remote Linux host
    -> enroll via Edge API
    -> enrollment-plane-rs
    -> receive identity + policy
    -> send heartbeat
    -> read real log file
    -> send log batch via Edge API
    -> batch reaches existing backend pipeline
```

Что намеренно **не** входит в этот этап:

- полный rewrite ingestion-plane на Rust
- alert engine
- anomaly detection
- query layer rewrite
- сложный PKI/CA lifecycle, если он блокирует старт
- journald/multiline до стабилизации file-tail MVP

Детальное ТЗ текущего milestone вынесено в отдельный документ: `docs/rust-vertical-slice.md`.

---

# 5. Текущее состояние репозитория

На момент написания документа в репозитории уже есть Go proof-of-concept в каталоге `edge_api/`.

## 5.1. Уже существующие ключевые файлы и модули

- `edge_api/cmd/server/main.go`
- `edge_api/cmd/fake-agent/main.go`
- `edge_api/internal/app/app.go`
- `edge_api/internal/grpcapi/server.go`
- `edge_api/internal/httpapi/router.go`
- `edge_api/internal/bus/jetstream.go`
- `edge_api/internal/ingest/service.go`
- `edge_api/internal/normalize/normalizer.go`
- `edge_api/internal/indexer/opensearch/indexer.go`
- `edge_api/internal/indexer/clickhouse/indexer.go`
- `edge_api/internal/query/opensearch.go`
- `edge_api/internal/stream/hub.go`
- `edge_api/internal/enrollment/store.go`
- `edge_api/internal/policy/store.go`
- `edge_api/internal/diagnostics/store.go`
- `edge_api/proto/ingest.proto`

## 5.2. Что текущий Go PoC уже умеет

Текущий PoC уже демонстрирует:

- HTTP server
- gRPC ingest endpoint
- использование NATS JetStream
- нормализацию событий
- индексацию в OpenSearch
- запись в ClickHouse
- запросы по логам
- stream hub
- fake agent flow
- простые концепции enrollment/policy/diagnostics

## 5.3. Чего текущий Go PoC еще не дает как финальный дизайн

Он пока **не дает**:

- настоящего Rust-агента
- persistent control-plane модели на базе БД
- Vault-backed модели секретов
- нормального разделения Edge API
- внутренних Rust runtime-компонентов
- deployment orchestration
- production-grade lifecycle агента
- полноценного собственного UI для аналитики
- hardened security model

---

# 6. Архитектура по сервисам

---

# 6A. Сервис WEB

## 6A.1. Роль

`WEB` это продуктовый пользовательский интерфейс.

Именно здесь пользователь:

- смотрит и ищет логи
- открывает live stream
- управляет inventory
- создает и редактирует policies
- запускает deployments
- смотрит состояние агентов
- изучает diagnostics
- работает с alerts

## 6A.2. Технологии

Рекомендуемый стек:

- React
- TypeScript
- Vite или Next.js
- TanStack Query
- Zustand
- Tailwind
- Apache ECharts
- виртуализированная таблица логов

## 6A.3. Что WEB не должен делать

`WEB` не должен:

- напрямую ходить во внутренние Rust runtime-компоненты
- напрямую обращаться в OpenSearch / ClickHouse
- зависеть от Grafana
- владеть логикой секретов
- реализовывать backend-side доменные правила

`WEB` общается только с публичным `Edge API`.

## 6A.4. Источники данных для WEB

`WEB` получает данные от `Edge API` через ручки вида:

- `/api/v1/agents`
- `/api/v1/policies`
- `/api/v1/deployments`
- `/api/v1/logs/search`
- `/api/v1/logs/histogram`
- `/api/v1/logs/top-services`
- `/api/v1/alerts`
- `/api/v1/stream/logs`

---

# 6B. Сервис SERVER

`SERVER` это один логический продуктовый сервис, но внутри он состоит из:

- публичного `Edge API` на Go
- приватных runtime-компонентов на Rust

---

## 6B.1. Public Edge API (Go)

### Роль
Граничный сервис для:

- входящего трафика от `WEB`
- входящего трафика от `AGENT`
- live stream в `WEB`

### Задачи
- HTTPS REST ingress
- gRPC ingress
- TLS termination
- auth middleware hooks
- валидация запросов
- мост request/reply во внутренний runtime
- публикация событий в `NATS`
- подписка и стриминг live-данных
- маппинг transport-level ошибок
- readiness/liveness
- безопасное публичное размещение

### Что не должно лежать здесь
Не нужно держать здесь:

- source-of-truth control-plane состояние
- alert engine
- ownership policy revisions
- deployment business logic
- долгосрочную доменную persistence
- вычислительную аналитику

### Технологии
- Go
- grpc-go
- chi или gin
- nats.go
- zap
- protobuf
- WebSocket или SSE
- context propagation
- optional OTEL hooks

### Публичные протоколы
- REST для `WEB`
- SSE/WebSocket для live stream
- gRPC для `AGENT`

### Основной поток данных
```text
WEB/AGENT -> Edge API -> NATS -> Rust runtime -> response -> Edge API -> client
```

---

## 6B.2. control-plane (Rust)

### Роль
Persistent source-of-truth для management-доменов.

### Чем владеет
- users
- roles
- inventory
- host groups
- credentials metadata
- policies
- policy revisions
- agent registry
- audit log
- metadata alert definitions

### Что читает и пишет
- PostgreSQL
- интеграции с Vault metadata
- NATS для command/event consumption по необходимости

### Чем не владеет
- полнотекстовой индексацией логов
- локальным состоянием агента
- OpenSearch document model
- ClickHouse analytics storage

---

## 6B.3. enrollment-plane (Rust)

### Роль
Управляет agent identity и начальной runtime-привязкой.

### Чем владеет
- проверкой bootstrap token
- созданием/обновлением agent identity
- назначением policy при enrollment
- выдачей policy зарегистрированному агенту
- сохранением heartbeat
- сохранением diagnostics snapshot

### Что читает и пишет
- PostgreSQL
- NATS request/reply handlers
- Vault/PKI на более позднем этапе при необходимости

### Какие subjects слушает
- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`

### Что возвращает
- `agent_id`
- текущую policy revision
- payload policy
- позже: certificate material / metadata identity

---

## 6B.4. deployment-plane (Rust)

### Роль
Отвечает за bootstrap и lifecycle установки агента.

### Чем владеет
- deployment jobs
- install / reinstall / upgrade / remove flows
- рендеринг inventory в deployment
- генерация bootstrap config
- выдача one-time enrollment token
- интеграция с ansible-runner

### Что читает и пишет
- PostgreSQL
- Vault
- NATS
- Ansible Runner
- optional artifact storage при необходимости

### High-level flow
1. Пользователь в `WEB` запрашивает deployment
2. `Edge API` публикует deployment command
3. `deployment-plane` обрабатывает команду
4. `deployment-plane` вызывает Ansible
5. На целевой Linux-хост кладется binary/config/service агента
6. Агент запускается и проходит enrollment

### Важная граница ответственности
Ansible используется как bootstrap/install механизм, а не как постоянный владелец runtime-конфигурации.

---

## 6B.5. ingestion-plane (Rust)

### Роль
Обрабатывает log batches после ingress.

### Чем владеет
- нормализацией ingest-событий
- enrichment
- fingerprinting
- публикацией stream-событий
- маршрутизацией в storage
- fanout на индексаторы

### Что читает и пишет
- NATS JetStream
- OpenSearch
- ClickHouse

### Откуда получает вход
- публикации `Edge API` в `logs.ingest.raw`

### Что производит
- normalized event stream
- записи в storage
- optional stream subjects для live views в UI

### Типичные стадии обработки
1. получение raw batch
2. проверка контракта
3. нормализация записей
4. enrichment metadata
5. вычисление fingerprint
6. запись в full-text index
7. запись в analytics index
8. публикация live stream event
9. optional публикация alert candidate event

---

## 6B.6. query-alert-plane (Rust)

### Роль
Обслуживает search/analytics business logic и alert logic.

### Чем владеет
- orchestration поиска
- histogram / top-N / heatmap aggregations
- вычислением alert rules
- состоянием alert-ов
- отправкой уведомлений
- построением incident context

### Что читает и пишет
- OpenSearch
- ClickHouse
- PostgreSQL
- NATS
- интеграция с Telegram на более позднем этапе

### Как публикуется наружу
- через NATS request/reply для запросов от `Edge API`

### Зачем нужен этот компонент
Чтобы `Edge API` оставался transport-thin, а внутренняя query/alert логика была полноценной доменной ответственностью.

---

# 6C. Сервис AGENT

## 6C.1. Роль

`AGENT` это Rust-процесс, который разворачивается на Linux-хосте и:

- проходит enrollment в системе
- tail-ит log sources
- сохраняет локальное состояние
- собирает и отправляет batches логов
- шлет heartbeat и diagnostics
- позже синхронизирует runtime policy

## 6C.2. Целевые технологии

Рекомендуется использовать:

- Rust stable
- tokio
- tonic/prost при необходимости gRPC client
- rusqlite
- tracing
- zstd/gzip
- config loader
- runtime, дружественный к systemd

## 6C.3. Локальные обязанности агента

- читать bootstrap config
- enroll-иться при первом запуске
- сохранять agent identity
- сохранять offsets/cursors
- tail-ить file sources
- позже читать journald
- собирать batch-и
- повторно отправлять при ошибках
- слать heartbeat
- слать diagnostics
- корректно завершаться

## 6C.4. Локальное хранилище агента

Используется **SQLite** для:

- `agent_identity`
- `policy_revision`
- `file_offsets`
- `send_state`
- spool metadata

## 6C.5. Текущее ожидание по rollout

Пока Ansible еще не полностью интегрирован, агент должен уметь запускаться вручную на отдельном Linux-сервере.

Milestone считается реальным только если:

- агент реально запускается удаленно
- проходит enrollment через `Edge API`
- читает реальный файл
- отправляет реальный batch в pipeline

---

# 7. Инфраструктурная архитектура

---

## 7.1. PostgreSQL

### Назначение
Source-of-truth relational storage для control-plane данных.

### Какие домены там живут
- users
- roles
- policies
- policy revisions
- hosts
- host groups
- agents
- deployment jobs
- agent diagnostics
- alert definitions
- audit entries

### Почему PostgreSQL
- сильная консистентность для control-plane состояния
- транзакционные обновления
- удобная эволюция через migrations
- простая проверяемость и прозрачность

---

## 7.2. Vault

### Назначение
Управление секретами и credential material.

### Планируемое использование
- SSH credentials
- one-time bootstrap tokens
- PKI/certificate integration позже
- secret config material

### Почему Vault
Чтобы не скатываться в:
- plaintext credentials в БД
- самописное секретохранилище
- времянки “потом доделаем”

---

## 7.3. NATS JetStream

### Назначение
Внутренний backbone для событий и команд.

### Где используется
- request/reply между `Edge API` и внутренним runtime
- fanout log batches
- durable consumer processing
- live stream events
- deployment commands
- lifecycle events

### Примеры subjects
```text
agents.enroll.request
agents.policy.fetch
agents.bootstrap-token.issue
agents.heartbeat
agents.diagnostics
logs.ingest.raw
logs.ingest.normalized
deployments.jobs.create
deployments.jobs.get
deployments.jobs.list
deployments.jobs.retry
deployments.jobs.cancel
deployments.plan.create
deployments.jobs.status
deployments.jobs.step
query.logs.search
query.logs.histogram
alerts.firing
ui.stream.logs
```

### Почему NATS
- легче Kafka
- удобно поддерживает request/reply
- хорошо ложится на внутреннее взаимодействие компонентов
- JetStream дает durability и replay

---

## 7.4. OpenSearch

### Назначение
Полнотекстовое хранение и поиск log events.

### Используется для
- поиска по сообщению
- фильтрации по полям
- получения контекста вокруг события
- phrase search
- wildcard search
- retrieval окон по времени

### Важно
Не надо пытаться сделать из него единственный движок для всей аналитики.

---

## 7.5. ClickHouse

### Назначение
Агрегации и аналитика.

### Используется для
- histogram
- severity distributions
- top hosts
- top services
- heatmaps
- anomaly baselines
- time-bucket analytics

### Почему отдельно от OpenSearch
Потому что полнотекстовый поиск и агрегационная аналитика требуют разной оптимизации.

---

## 7.6. Redis

### Optional/supporting назначение
- session/cache support
- WS/SSE fanout metadata
- short-lived rate limit state
- временная координация

Redis не является primary control-plane database.

---

## 7.7. MinIO или object storage

### Optional later use
- raw exports
- cold archive
- snapshots
- выгрузка incident package

На первом vertical slice не обязателен.

---

# 8. Сквозные потоки данных

---

## 8.1. Enrollment flow

```text
Agent
  |
  | Enroll request (bootstrap token)
  v
Edge API (Go)
  |
  | NATS request -> agents.enroll.request
  v
enrollment-plane (Rust)
  |
  | validate token / create or update agent / attach policy
  v
PostgreSQL
  |
  +--> enrollment response
  |
  v
Edge API
  |
  v
Agent получает:
- agent_id
- current policy
- current policy revision
```

### Какие данные затрагиваются
- bootstrap token
- таблица agents
- policies / revisions
- agent_policy_binding

---

## 8.2. Heartbeat flow

```text
Agent
  |
  | heartbeat
  v
Edge API
  |
  | publish/request
  v
enrollment-plane
  |
  v
PostgreSQL updates:
- last_seen_at
- version
- host metadata
- status
```

### Для чего нужен
- актуальность registry
- health статус агента в UI
- будущие alerts вида “агент пропал”

---

## 8.3. Diagnostics flow

```text
Agent
  |
  | diagnostics payload
  v
Edge API
  |
  v
enrollment-plane
  |
  v
PostgreSQL:
- сохранение diagnostics snapshot
- сохранение last error и source state
```

### Примеры полей diagnostics
- hostname
- version
- current policy revision
- source statuses
- last send success
- queue size
- spool size
- last error

---

## 8.4. Ingest flow

```text
Agent
  |
  | gRPC batch send
  v
Edge API
  |
  | publish logs.ingest.raw
  v
NATS JetStream
  |
  +--> ingestion-plane
          |
          +--> normalize/enrich/fingerprint
          |
          +--> OpenSearch write
          |
          +--> ClickHouse write
          |
          +--> live stream event
          |
          +--> alert candidate event
```

### Почему это важно
Ingress отделен от обработки.  
Благодаря этому `Edge API` остается thin и безопасным с точки зрения публичного размещения.

---

## 8.5. Search flow

```text
WEB
  |
  | POST /api/v1/logs/search
  v
Edge API
  |
  | request -> query.logs.search
  v
query-alert-plane
  |
  v
OpenSearch
  |
  v
query-alert-plane
  |
  v
Edge API
  |
  v
WEB
```

### Что возвращается
- matching hits
- metadata
- pagination
- optional context snippet
- timing

---

## 8.6. Analytics flow

```text
WEB
  |
  | GET /api/v1/logs/histogram
  v
Edge API
  |
  | request -> query.logs.histogram
  v
query-alert-plane
  |
  v
ClickHouse
  |
  v
query-alert-plane
  |
  v
Edge API
  |
  v
WEB chart
```

---

## 8.7. Deployment flow

```text
WEB
  |
  | POST /api/v1/deployments
  v
Edge API
  |
  | request/reply deployments.jobs.create
  v
deployment-plane
  |
  | resolve policy + credentials + targets
  | generate bootstrap config
  | request/reply agents.bootstrap-token.issue
  | invoke ansible-runner
  v
Target Linux host
  |
  | install agent
  | write config
  | register systemd service
  | start agent
  v
Agent enrolls
```

---

# 9. Владение данными по компонентам

| Данные / concern | Владелец | Технология |
|---|---|---|
| Public HTTP/gRPC ingress | Edge API | Go |
| Agent enrollment identity | enrollment-plane | Rust + PostgreSQL |
| Source of truth для policy | control-plane | Rust + PostgreSQL |
| Deployment jobs | deployment-plane | Rust + PostgreSQL |
| Secret credentials | Vault-backed integrations | Vault |
| Transport raw ingest events | Edge API + NATS | Go + NATS |
| Normalization/fingerprinting | ingestion-plane | Rust |
| Full-text search data | ingestion-plane / query-alert-plane | OpenSearch |
| Analytics data | ingestion-plane / query-alert-plane | ClickHouse |
| Agent local offsets and state | agent-rs | SQLite |
| Product dashboards and UI state | WEB | React/TS |

---

# 10. Рекомендуемая начальная модель базы

Это минимальный стартовый реляционный набор, а не финальная полная схема.

## 10.1. agents
- id
- agent_id
- hostname
- status
- version
- first_seen_at
- last_seen_at
- created_at
- updated_at

## 10.2. enrollment_tokens
- id
- token_hash
- policy_id
- expires_at
- used_at
- revoked_at
- created_at

## 10.3. policies
- id
- name
- description
- is_active
- created_at
- updated_at

## 10.4. policy_revisions
- id
- policy_id
- revision
- body_json
- created_at

## 10.5. agent_policy_bindings
- id
- agent_id
- policy_id
- policy_revision_id
- assigned_at

## 10.6. agent_diagnostics
- id
- agent_id
- payload_json
- created_at

## 10.7. hosts
- id
- hostname
- ip
- ssh_port
- user
- labels_json
- created_at
- updated_at

## 10.8. deployment_jobs
- id
- status
- requested_by
- payload_json
- created_at
- updated_at

---

# 11. Дизайн runtime агента

## 11.1. Bootstrap config

Начальная конфигурация агента должна задаваться файлом, например:

```yaml
edge_url: https://edge.example.local
bootstrap_token: one-time-token
state_dir: /var/lib/doro-agent
log_level: info
sources:
  - type: file
    path: /tmp/doro-test.log
heartbeat_interval_sec: 30
batch:
  max_events: 500
  flush_interval_sec: 2
```

## 11.2. Поведение при первом запуске
1. агент читает config
2. если нет локальной identity, проходит enrollment
3. сохраняет identity и текущую policy revision
4. запускает heartbeat loop
5. начинает читать log sources
6. собирает и отправляет batches

## 11.3. Поведение после рестарта
1. загружает локальную identity и offsets
2. не enroll-ится заново без причины
3. продолжает читать файл с сохраненного offset
4. продолжает heartbeat и batch send

## 11.4. Первый поддерживаемый источник
Начинаем с:

- file tailing

Позже добавляем:

- journald
- multiline
- glob matching
- более богатые parse rules

---

# 12. Сетевая топология

## 12.1. Публичная и приватная части

Рекомендуемая deployment topology:

```text
[WEB clients] --------\
                       \
[Agents on servers] -----> [Public Edge API (Go)] ----private----> [Rust runtime]
                                                    link/VPN/WG
```

Внутренний Rust runtime может жить:
- на сером IP
- за NAT
- в private network

Агенты не обязаны ходить напрямую во внутренние адреса runtime.

## 12.2. Почему это работает
- один стабильный публичный endpoint
- меньшая attack surface
- проще TLS и rate limiting
- внутренний runtime остается приватным
- хорошо ложится на модель “Go только на edge”

---

# 13. Подход к обработке ошибок

## 13.1. На публичных границах
`Edge API` должен возвращать:

- стабильный error envelope для HTTP
- корректные gRPC status codes для agent calls
- request IDs для troubleshooting

## 13.2. Во внутреннем runtime
Rust-компоненты должны использовать typed errors и structured logging.

## 13.3. В агенте
Агент должен:

- логировать причины enrollment failure
- логировать причины batch failure
- retry-ить с backoff
- по возможности сохранять state

Никаких silent failures.

---

# 14. Логирование и observability самой платформы

Сама платформа должна писать structured logs минимум с полями:

- request_id
- agent_id
- deployment_job_id
- policy_revision
- nats_subject
- event_count
- batch_size
- duration
- error_kind

Эта внутренняя observability не равна продуктовой observability, которую получает пользователь.

---

# 15. План миграции

## Фаза 1
- заморозить Go PoC как reference
- выделить transport contracts
- отделить Edge API
- сохранить текущий PoC как рабочий эталон

## Фаза 2
- создать Rust workspace
- реализовать enrollment-plane
- добавить PostgreSQL schema
- добавить Rust agent MVP

## Фаза 3
- пустить enrollment и heartbeat через Rust
- сделать агент реально запускаемым на удаленном Linux-host
- подтвердить первый end-to-end поток доставки реального лога

## Фаза 4
- перенести больше внутренней логики в Rust:
  - control-plane
  - deployment-plane
  - ingestion-plane
  - query-alert-plane

## Фаза 5
- завершить WEB UI flows
- закончить lifecycle deployment через Ansible
- улучшить alert engine
- сделать UX для policy revisions
- сделать UX для diagnostics

---

# 16. Текущие приоритеты разработки

Вот что сейчас приносит максимальную пользу.

## 16.1. Contracts
Сделать общий `contracts/` и перестать относиться к proto/schema как к случайным файлам.

## 16.2. Выделение Edge API
Превратить текущий Go server в настоящий `Edge API`, а не в вечного владельца всей платформы.

## 16.3. Persistent enrollment model
Заменить in-memory stores на PostgreSQL-backed state.

## 16.4. Реальный Rust agent
Сделать первый remote-runnable агент, который:
- проходит enrollment
- сохраняет identity
- tail-ит реальный файл
- шлет реальный batch

## 16.5. Remote validation
Фича не считается завершенной, пока не работает на отдельном Linux-host, а не только на localhost.

Исполнимое ТЗ текущего этапа: `docs/rust-vertical-slice.md`.

---

# 17. Как текущие Go-модули маппятся в целевую архитектуру

| Текущий Go-модуль | Что он значит сейчас | Что с ним будет дальше |
|---|---|---|
| `edge_api/internal/httpapi` | смешанный HTTP API | станет частью transport-слоя Edge API |
| `edge_api/internal/grpcapi` | gRPC ingress для ingest | станет частью transport-слоя Edge API |
| `edge_api/internal/bus/jetstream` | интеграция с NATS | станет мостом Edge API и/или reference для Rust |
| `edge_api/internal/ingest` | логика ingest | доменная логика уйдет в ingestion-plane на Rust |
| `edge_api/internal/normalize` | нормализация | будет перенесена или переписана в Rust ingestion-plane |
| `edge_api/internal/indexer/opensearch` | full-text indexing | ownership перейдет в Rust ingestion/query layers |
| `edge_api/internal/indexer/clickhouse` | analytics writes | ownership перейдет в Rust ingestion/query layers |
| `edge_api/internal/query` | прямые запросы | ownership перейдет в query-alert-plane на Rust |
| `edge_api/internal/stream/hub` | stream fanout | станет streaming adapter в Edge API + internal event source |
| `edge_api/internal/enrollment` | in-memory enrollment | будет заменен на Rust enrollment-plane + PostgreSQL |
| `edge_api/internal/policy` | in-memory policy store | будет заменен на control-plane + PostgreSQL |
| `edge_api/internal/diagnostics` | in-memory diagnostics | будет заменен на persistent diagnostics storage |
| `edge_api/cmd/fake-agent` | synthetic traffic helper | останется test utility, но не станет product agent |

---

# 18. Итог

Эта архитектура специально балансирует между формальными требованиями кейса и реальными инженерными задачами:

- **Снаружи**: 3 сервиса (`WEB`, `SERVER`, `AGENT`)
- **Внутри**: `SERVER` разделен на Go Edge API и Rust runtime domains
- **Хранилища**: PostgreSQL + OpenSearch + ClickHouse + SQLite + Vault
- **Messaging**: NATS JetStream
- **UI**: собственный `WEB`, без Grafana
- **Agent**: настоящий Rust-процесс, который можно запускать на отдельном Linux-host

Текущий Go proof-of-concept уже дает полезный фундамент, но целевая система требует миграции к нормальным границам ответственности, persistence и реальному lifecycle агента.

Этот документ нужно обновлять всякий раз, когда меняется:

- граница сервисов
- ownership данных
- transport contracts
- выбор инфраструктуры
- текущее состояние миграции
