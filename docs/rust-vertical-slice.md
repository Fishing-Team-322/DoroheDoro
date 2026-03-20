# Rust Vertical Slice: `agent-rs` + `enrollment-plane-rs`

> Исполнимое ТЗ на текущий этап. Документ фиксирует первый рабочий Rust vertical slice поверх существующего Go `Edge API` и текущего backend pipeline.

## 1. Контекст

Проект формально состоит из трех продуктовых сервисов:

1. `WEB`
2. `SERVER`
3. `AGENT`

При этом внутри `SERVER` уже зафиксирована внутренняя декомпозиция:

- публичный `Edge API` на Go
- внутренние processing components на Rust

`AGENT` также должен быть реализован на Rust и запускаться на Linux-хостах.

Текущая проблема не в отсутствии еще одного UI-экрана и не в отсутствии более сложной аналитики. Самый большой инфраструктурный разрыв сейчас такой:

- нет настоящего `agent-rs`
- нет persistent enrollment/state model
- нет нормального lifecycle агента
- нет фундамента для дальнейшего ручного и Ansible-based deployment

## 2. Цель этапа

Цель текущего этапа: собрать первый реальный end-to-end flow:

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

Первый practical milestone считается достигнутым только если `agent-rs`, запущенный на отдельном Linux-сервере, проходит enrollment через Go `Edge API`, получает policy, шлет heartbeat и доставляет реальные строки из лог-файла в существующий backend pipeline.

## 3. Scope этого этапа

В приоритете:

1. `contracts`
2. `enrollment-plane-rs`
3. `agent-rs`

На этом этапе не делаем:

- полный rewrite ingestion-plane на Rust
- alert engine
- anomaly detection
- полный query layer
- полноценный PKI/CA lifecycle, если он блокирует старт
- сложный policy editor
- journald/multiline до стабильного file-tail MVP
- попытку сразу сделать "идеальный production-ready agent"

Отдельные микросервисы ради микросервисов, самописная шина вместо `NATS` и самописные хранилища вместо `PostgreSQL`/`SQLite` в scope не входят.

## 4. Deliverables

Нужно выдать:

1. общий `contracts` слой
2. Rust workspace `server-rs`
3. `common` crate
4. `enrollment-plane-rs`
5. `agent-rs`
6. SQL migrations для enrollment и agent registry
7. sample config для агента
8. sample systemd unit
9. README по локальному запуску и remote-run
10. smoke/integration test сценарий

## 5. Обязательный стек

Обязательно использовать:

- Rust stable
- `tokio`
- `serde`
- `tracing`
- `thiserror` или `anyhow`
- `async-nats`
- `sqlx`
- `prost`/`tonic`, если нужен gRPC contract layer
- `rusqlite` для локального state агента
- `zstd` или `gzip` для компрессии batch payload

Желательно использовать:

- `figment`, `config` или эквивалентный env/file config loader
- `uuid`
- `chrono`
- `rustls`
- `clap`
- `notify` только как дополнительный сигнал, не как основу file reading

## 6. Целевая структура

```text
contracts/
  proto/
    ingest.proto
    agent.proto
  schemas/
    event.schema.json

server-rs/
  Cargo.toml
  crates/
    common/
    enrollment-plane/
    agent-rs/
```

`Edge API` остается в `edge_api/` и использует общий contracts layer.

## 7. Contracts layer

### Что нужно сделать

- вынести protobuf/contracts в общее место
- привести контракт `AGENT <-> Edge API <-> Rust components` к единой схеме
- добавить генерацию кода для Rust
- синхронизировать transport-модели с Go командой

### Контракты должны покрывать минимум

- enrollment request/response
- policy fetch response
- heartbeat payload
- diagnostics payload
- ingest batch
- log event

### NATS subjects для `enrollment-plane-rs`

```text
agents.enroll.request
agents.policy.fetch
agents.heartbeat
agents.diagnostics
```

### Request/reply envelope

Для внутренних request/reply используется единый envelope:

- `status`
- `code`
- `message`
- `payload`
- `correlation_id`

### Acceptance criteria

- protobuf генерируется без ручных правок
- Rust использует общий с Go контракт
- transport-модели не дублируются хаотично
- есть README или Make target для генерации

## 8. `server-rs` workspace и `common`

### Что нужно создать

- workspace `server-rs`
- crate `common`
- crate `enrollment-plane`
- crate `agent-rs`

### `common` должен содержать

- общие типы
- config loading
- tracing setup
- error model
- constants для NATS subjects
- request/reply envelope utility
- shared IDs и metadata structures

### Acceptance criteria

- workspace собирается одной командой
- конфигурация и логирование работают консистентно
- есть единый стиль ошибок и structured logging

## 9. `enrollment-plane-rs`

`enrollment-plane-rs` является внутренним Rust component и не является публичным ingress. Он работает через `NATS`.

### Обязанности

- валидировать bootstrap token
- создавать или обновлять запись агента
- возвращать identity и initial policy
- обслуживать policy fetch
- сохранять heartbeat
- сохранять diagnostics snapshot

### Handlers

#### `agents.enroll.request`

- провалидировать bootstrap token
- найти связанную policy
- создать `agent_id`, если нужно
- создать или обновить запись в `agents`
- вернуть enrollment response

#### `agents.policy.fetch`

- проверить agent identity
- вернуть текущую policy и `policy_revision`

#### `agents.heartbeat`

- обновить `last_seen_at`
- обновить `version`, `hostname`, host metadata и status
- не держать состояние только в памяти

#### `agents.diagnostics`

- сохранить snapshot diagnostics
- сохранить `last_error`, source status и spool info
- оставить структуру расширяемой

### Требования к слоям

- transport layer отдельно от domain/service layer
- repository layer отдельно от handlers
- без giant `main.rs`

## 10. PostgreSQL schema

Критический control-plane state должен жить в `PostgreSQL`.

### Обязательные таблицы

#### `agents`

- `id`
- `agent_id`
- `hostname`
- `status`
- `version`
- `first_seen_at`
- `last_seen_at`
- `created_at`
- `updated_at`

#### `enrollment_tokens`

- `id`
- `token_hash`
- `policy_id`
- `expires_at`
- `used_at`
- `created_at`
- `revoked_at`

#### `policies`

- `id`
- `name`
- `description`
- `is_active`
- `created_at`
- `updated_at`

#### `policy_revisions`

- `id`
- `policy_id`
- `revision`
- `body_json`
- `created_at`

#### `agent_policy_bindings`

- `id`
- `agent_id`
- `policy_id`
- `policy_revision_id`
- `assigned_at`

#### `agent_diagnostics`

- `id`
- `agent_id`
- `payload_json`
- `created_at`

### Acceptance criteria

- есть SQL migrations
- schema поднимается локально
- `enrollment-plane-rs` использует `PostgreSQL` как source of truth
- критический state больше не живет in-memory

## 11. `agent-rs` MVP

`agent-rs` должен быть настоящим Rust-агентом, который можно запустить на отдельном Linux-сервере.

### Bootstrap config

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

### MVP-поведение

#### Первый запуск

- прочитать bootstrap config
- выполнить enrollment через `Edge API`
- получить identity и initial policy
- сохранить локально identity и applied policy revision

#### Рабочий цикл

- периодически слать heartbeat
- tail-ить один реальный log file
- собирать строки в batch
- отправлять batch через `Edge API`

#### Shutdown

- корректно завершаться по signal
- не терять локальный state

## 12. Локальное состояние агента

Минимально храним:

- agent identity
- applied policy revision
- file offsets
- last successful send metadata

Технология: `SQLite`

Рекомендуемое размещение:

- `/var/lib/doro-agent/state.db`

### Acceptance criteria

- после рестарта агент не enroll-ится заново без причины
- offset не теряется после перезапуска
- состояние не живет только в памяти

## 13. File tailer MVP

На первом этапе поддерживается один `file` source.

### Обязательные требования

- читать файл построчно
- запоминать offset
- подбирать только новые строки
- минимизировать повторную отправку после рестарта
- корректно работать на тестовом файле вида `/tmp/doro-test.log`

### Пока не требуется

- multiline
- journald
- сложный glob matching
- сложный rotate handling

### Acceptance criteria

- можно руками дописывать строки в файл
- агент подбирает новые строки
- строки уходят в batch
- после рестарта дублирование минимально или отсутствует

## 14. Transport layer агента

Нужен отдельный transport module для `Edge API`.

### Он должен покрывать

- enrollment request
- heartbeat request
- ingest batch request
- diagnostics request

### Требования

- таймауты
- retry с backoff
- структурное логирование ошибок
- configurable `batch.max_events`
- configurable `batch.flush_interval_sec`

### Acceptance criteria

- network errors не валят процесс мгновенно
- агент логирует понятные причины ошибок
- batch отправляется с подтверждением
- transport не размазан по `main`

## 15. Diagnostics baseline

MVP diagnostics должны включать:

- hostname
- agent version
- current source status
- last error
- last send success timestamp
- queue или buffer info, если уже есть
- current policy revision

### Acceptance criteria

- diagnostics можно отправить через `Edge API`
- `enrollment-plane-rs` сохраняет snapshot
- данные пригодны для будущей UI diagnostics page

## 16. Remote-run setup

Нужно подготовить ручной install flow для отдельного Linux-хоста.

### Что нужно выдать

- Linux build агента
- пример `systemd` unit
- пример config file
- install flow:
  - скопировать бинарь
  - положить config
  - создать state dir
  - запустить сервис

### Рекомендуемые пути

- `/usr/local/bin/doro-agent`
- `/etc/doro-agent/config.yaml`
- `/var/lib/doro-agent/`
- `/var/log/doro-agent/`

### Acceptance criteria

- бинарь можно вручную поставить на отдельный сервер
- агент запускается как `systemd` unit
- агент подключается к `Edge API` и шлет данные

## 17. Ошибки, логирование и качество кода

### Ошибки

- использовать typed error model
- не терять контекст
- логировать структурно
- маппить внешние ошибки в предсказуемые transport-level codes
- не использовать `unwrap()` в production flow
- не игнорировать ошибки молча

### Structured logging

Использовать `tracing` и логировать минимум:

- `request_id`
- `agent_id`
- `hostname`
- `policy_revision`
- `source_path`
- `offset`
- `batch_size`
- `event_count`
- `error_kind`

### Качество кода

- async/await без блокирующих вызовов в горячем пути
- separation of concerns
- transport отдельно от domain logic
- repository отдельно от handler layer
- конфиг через env/file, а не hardcode
- без overengineering и без сложных trait hierarchy без необходимости

## 18. Порядок разработки

### Этап 1. Foundation

1. Выделить `contracts`
2. Создать Rust workspace
3. Создать `common`
4. Настроить config, logging и error model

### Этап 2. Enrollment

1. Добавить PostgreSQL migrations
2. Реализовать `enrollment-plane-rs`
3. Поднять NATS handlers
4. Собрать enrollment flow
5. Собрать policy fetch
6. Сохранить heartbeat
7. Сохранить diagnostics

### Этап 3. Agent MVP

1. Добавить bootstrap config
2. Реализовать enrollment client
3. Реализовать identity persistence
4. Реализовать heartbeat
5. Реализовать file tail MVP
6. Реализовать batch send

### Этап 4. Remote validation

1. Собрать Linux binary
2. Подготовить `systemd` unit
3. Проверить ручной deploy на отдельный хост
4. Прогнать тест с реальным файлом
5. Проверить restart
6. Проверить offset persistence

## 19. Smoke test checklist

### Локально

- `enrollment-plane-rs` стартует
- подключается к `PostgreSQL`
- подключается к `NATS`
- обрабатывает `agents.enroll.request`

### Агент локально

- читает config
- проходит enrollment
- получает policy
- пишет heartbeat
- читает тестовый файл
- шлет batch

### Агент на удаленном сервере

- устанавливается бинарь
- запускается `systemd` unit
- проходит enrollment
- читает реальный файл логов
- после рестарта не теряет identity
- после рестарта не переотправляет весь файл без причины

## 20. Definition of Done

Задача считается завершенной, если:

- вынесен общий contracts layer
- `enrollment-plane-rs` работает через `NATS`
- enrollment state хранится в `PostgreSQL`
- `agent-rs` умеет enroll
- `agent-rs` умеет слать heartbeat
- `agent-rs` умеет читать один реальный log file
- `agent-rs` умеет отправлять batch через `Edge API`
- агент можно запустить на отдельном Linux-сервере
- state агента сохраняется локально в `SQLite`
- есть минимальная документация и smoke test

## 21. Приоритеты

### P0

- contracts
- Rust workspace
- PostgreSQL schema
- `enrollment-plane-rs`
- `agent-rs` bootstrap
- enrollment
- heartbeat
- file tail MVP
- batch send

### P1

- `SQLite` state
- offset persistence
- diagnostics
- remote run и `systemd`
- retry/backoff

### P2

- basic rotate handling
- richer diagnostics shape
- policy revision improvements
- journald
- multiline
