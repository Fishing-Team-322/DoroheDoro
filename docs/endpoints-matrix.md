# endpoints-matrix.md

> Полная матрица endpoint-ов и внутренних transport entrypoints для платформы централизованного сбора и анализа логов.  
> Документ включает:
>
> - **публичные HTTP endpoint-ы** для `WEB`
> - **streaming endpoint-ы** для live-данных
> - **gRPC методы** для `AGENT`
> - **внутренние NATS subjects** как внутренние точки входа между `Edge API` и Rust runtime
>
> Документ описывает как **текущие**, так и **будущие** endpoint-ы.  
> Для каждого endpoint-а указано:
>
> - назначение
> - кто его вызывает
> - кто его обслуживает
> - статус (`MVP`, `Next`, `Future`)
> - зачем он нужен

---

# 1. Легенда

## Колонки
- **Метод / Тип** — HTTP метод, gRPC метод или тип transport entrypoint
- **Путь / Subject / RPC** — URL, NATS subject или gRPC метод
- **Кто вызывает** — клиент endpoint-а
- **Кто обрабатывает** — сервис или внутренний компонент
- **Статус**:
  - `MVP` — нужно делать в первую очередь
  - `Next` — следующий важный слой после MVP
  - `Future` — нужен в будущем, но не блокирует первый vertical slice
- **Зачем нужен** — прикладное объяснение на русском

---

# 2. Общая карта endpoint-ов

В системе есть 4 класса точек входа:

1. **Public HTTP API**  
   Для `WEB` и административных UI-сценариев

2. **Public streaming API**  
   Для live log stream, live jobs, live alerts

3. **Public gRPC API для AGENT**  
   Для enrollment, policy fetch, heartbeat, diagnostics, ingest

4. **Internal NATS subjects**  
   Для связи между `Edge API` и приватными Rust runtime-компонентами

---

# 3. Public HTTP API для WEB

Базовый префикс:

```text
/api/v1
```

---

## 3.1. System / health / readiness

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/healthz` | Kubernetes / Docker / инженер | Edge API | MVP | Проверка, что процесс жив и отвечает на запросы. Нужен для liveness probe и базовой диагностики. |
| GET | `/readyz` | Kubernetes / Docker / инженер | Edge API | MVP | Проверка, что сервис готов принимать трафик и имеет критические зависимости, например NATS. |
| GET | `/version` | WEB / инженер / CI | Edge API | Next | Позволяет быстро понять, какая версия backend развернута. Нужен для отладки и поддержки. |
| GET | `/build-info` | инженер / support tooling | Edge API | Future | Возвращает commit, build date, version и feature flags. Полезно для debugging production-окружения. |

---

## 3.2. Auth / session / current user

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/me` | WEB | Edge API -> control-plane | MVP | Получить информацию о текущем пользователе, его ролях и доступных действиях. Нужен, чтобы UI понимал, что можно показывать. |
| POST | `/api/v1/auth/login` | WEB | Edge API -> auth layer / control-plane | Next | Точка входа для аутентификации пользователя. Нужна, если логин делается внутри платформы, а не внешним SSO. |
| POST | `/api/v1/auth/logout` | WEB | Edge API | Next | Завершение текущей сессии пользователя. Нужен для нормального session lifecycle. |
| POST | `/api/v1/auth/refresh` | WEB | Edge API | Next | Обновление access token/session token без полного входа. Нужен для удобной работы UI. |
| GET | `/api/v1/auth/providers` | WEB | Edge API | Future | Возврат доступных способов входа: local auth, OIDC, SSO и т.д. |
| GET | `/api/v1/auth/permissions` | WEB | Edge API -> control-plane | Future | Получить матрицу разрешений пользователя, если UI нужно точечно прятать действия по правам. |

---

## 3.3. Users / roles / RBAC

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/users` | Admin UI | Edge API -> control-plane | Future | Получить список пользователей платформы. Нужен для администрирования доступа. |
| GET | `/api/v1/users/{id}` | Admin UI | Edge API -> control-plane | Future | Получить карточку конкретного пользователя. |
| POST | `/api/v1/users` | Admin UI | Edge API -> control-plane | Future | Создать пользователя, если используется встроенная auth-модель. |
| PATCH | `/api/v1/users/{id}` | Admin UI | Edge API -> control-plane | Future | Изменить данные пользователя. |
| GET | `/api/v1/roles` | Admin UI | Edge API -> control-plane | Future | Получить список ролей и их описания. |
| POST | `/api/v1/roles` | Admin UI | Edge API -> control-plane | Future | Создать новую роль, если нужна кастомная RBAC-модель. |
| PATCH | `/api/v1/roles/{id}` | Admin UI | Edge API -> control-plane | Future | Изменить состав разрешений роли. |

---

## 3.4. Inventory: hosts / host groups / environments

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/hosts` | WEB | Edge API -> control-plane | Next | Получить список известных серверов/хостов. Нужен для inventory и выбора targets для deployment. |
| POST | `/api/v1/hosts` | WEB | Edge API -> control-plane | Next | Добавить новый хост в inventory. Нужен перед автоматической установкой агента. |
| GET | `/api/v1/hosts/{id}` | WEB | Edge API -> control-plane | Next | Получить карточку конкретного хоста: IP, labels, assigned policy, agent status. |
| PATCH | `/api/v1/hosts/{id}` | WEB | Edge API -> control-plane | Next | Изменить metadata хоста: labels, ssh port, user, environment. |
| DELETE | `/api/v1/hosts/{id}` | WEB | Edge API -> control-plane | Future | Удалить хост из inventory. Нужен для cleanup и decommission. |
| GET | `/api/v1/host-groups` | WEB | Edge API -> control-plane | Next | Получить группы хостов. Нужен для массового deployment и назначения policy. |
| POST | `/api/v1/host-groups` | WEB | Edge API -> control-plane | Next | Создать группу хостов. |
| GET | `/api/v1/host-groups/{id}` | WEB | Edge API -> control-plane | Next | Получить содержимое группы и ее метаданные. |
| PATCH | `/api/v1/host-groups/{id}` | WEB | Edge API -> control-plane | Next | Изменить состав или свойства группы. |
| DELETE | `/api/v1/host-groups/{id}` | WEB | Edge API -> control-plane | Future | Удалить группу хостов. |
| POST | `/api/v1/host-groups/{id}/members` | WEB | Edge API -> control-plane | Future | Добавить хосты в группу. |
| DELETE | `/api/v1/host-groups/{id}/members/{hostId}` | WEB | Edge API -> control-plane | Future | Удалить хост из группы. |

---

## 3.5. Credential profiles / secrets metadata

> Сами секреты живут не в HTTP-ответах и не в базе как plaintext.  
> HTTP endpoint-ы управляют **метаданными** credential-профилей и запуском использования этих секретов.

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/credentials` | WEB | Edge API -> control-plane / Vault integration | Next | Получить список credential profiles, доступных для deployment job. |
| POST | `/api/v1/credentials` | WEB | Edge API -> control-plane / Vault integration | Next | Создать credential profile: SSH key, password, bastion и т.д. |
| GET | `/api/v1/credentials/{id}` | WEB | Edge API -> control-plane | Next | Получить metadata credential profile без раскрытия секрета. |
| PATCH | `/api/v1/credentials/{id}` | WEB | Edge API -> control-plane | Future | Обновить label, описание или non-secret настройки credential profile. |
| DELETE | `/api/v1/credentials/{id}` | WEB | Edge API -> control-plane | Future | Удалить credential profile. |
| POST | `/api/v1/credentials/validate` | WEB | Edge API -> deployment-plane | Future | Проверить, что credential реально подходит для SSH/подключения. Полезно до запуска deployment-а. |

---

## 3.6. Policies

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/policies` | WEB | Edge API -> control-plane | MVP | Получить список agent policies. Нужен для выбора и назначения конфигурации агентам. |
| POST | `/api/v1/policies` | WEB | Edge API -> control-plane | Next | Создать новую policy. |
| GET | `/api/v1/policies/{id}` | WEB | Edge API -> control-plane | MVP | Получить полную конфигурацию policy. |
| PATCH | `/api/v1/policies/{id}` | WEB | Edge API -> control-plane | Next | Изменить policy и создать новую revision или изменить draft. |
| DELETE | `/api/v1/policies/{id}` | WEB | Edge API -> control-plane | Future | Архивировать или удалять policy. |
| GET | `/api/v1/policies/{id}/revisions` | WEB | Edge API -> control-plane | Next | Получить историю изменений policy. |
| GET | `/api/v1/policies/{id}/revisions/{revisionId}` | WEB | Edge API -> control-plane | Next | Получить конкретную revision policy. |
| POST | `/api/v1/policies/{id}/revisions/{revisionId}/activate` | WEB | Edge API -> control-plane | Future | Активировать выбранную revision как текущую. |
| POST | `/api/v1/policies/{id}/rollback` | WEB | Edge API -> control-plane | Future | Откатить policy на предыдущую revision. |
| POST | `/api/v1/policies/{id}/assignments` | WEB | Edge API -> control-plane | Next | Назначить policy хосту или группе хостов. |
| GET | `/api/v1/policies/{id}/assignments` | WEB | Edge API -> control-plane | Future | Посмотреть, куда назначена policy. |
| POST | `/api/v1/policies/validate` | WEB | Edge API -> control-plane | Future | Проверить валидность policy до сохранения. Например, формат file source или multiline rules. |
| POST | `/api/v1/policies/preview` | WEB | Edge API -> control-plane / query-alert-plane | Future | Предпросмотр, как policy будет интерпретировать пример логов. Полезно для UX policy builder. |

---

## 3.7. Agents registry / lifecycle / diagnostics

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/agents` | WEB | Edge API -> enrollment-plane / control-plane | MVP | Получить список зарегистрированных агентов. |
| GET | `/api/v1/agents/{id}` | WEB | Edge API -> enrollment-plane / control-plane | MVP | Получить карточку конкретного агента: host, version, status, last_seen. |
| GET | `/api/v1/agents/{id}/diagnostics` | WEB | Edge API -> enrollment-plane | MVP | Получить последний diagnostics snapshot агента. Нужен для troubleshooting. |
| GET | `/api/v1/agents/{id}/heartbeats` | WEB | Edge API -> enrollment-plane | Future | Просмотреть историю heartbeat агента. Полезно для расследования нестабильной связи. |
| GET | `/api/v1/agents/{id}/policy` | WEB | Edge API -> enrollment-plane / control-plane | Next | Понять, какая policy сейчас фактически применена к агенту. |
| POST | `/api/v1/agents/{id}/re-enroll` | WEB | Edge API -> enrollment-plane | Future | Принудительно инициировать новый lifecycle enrollment. |
| POST | `/api/v1/agents/{id}/revoke` | WEB | Edge API -> enrollment-plane | Future | Отозвать identity агента и запретить ему дальнейшую работу. |
| POST | `/api/v1/agents/{id}/restart-request` | WEB | Edge API -> deployment-plane | Future | Поставить задачу на перезапуск агента на хосте, если deployment-механизм умеет это делать. |
| GET | `/api/v1/agents/{id}/sources` | WEB | Edge API -> enrollment-plane / diagnostics model | Future | Посмотреть состояние конкретных sources: файл читается, ошибка чтения, lag и т.д. |
| GET | `/api/v1/agents/{id}/spool` | WEB | Edge API -> enrollment-plane / diagnostics model | Future | Посмотреть размер локального буфера/spool и состояние очереди отправки. |

---

## 3.8. Deployments / installation / upgrade lifecycle

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| POST | `/api/v1/deployments` | WEB | Edge API -> deployment-plane | MVP | Создать deployment job: установить, обновить, переустановить или удалить агент на целевых хостах. |
| GET | `/api/v1/deployments` | WEB | Edge API -> deployment-plane | MVP | Получить список deployment jobs. |
| GET | `/api/v1/deployments/{id}` | WEB | Edge API -> deployment-plane | MVP | Получить статус конкретного deployment job. |
| GET | `/api/v1/deployments/{id}/steps` | WEB | Edge API -> deployment-plane | Next | Получить шаги выполнения job: inventory render, ansible start, install, verify и т.д. |
| GET | `/api/v1/deployments/{id}/targets` | WEB | Edge API -> deployment-plane | Next | Получить per-host результаты deployment job. |
| POST | `/api/v1/deployments/{id}/retry` | WEB | Edge API -> deployment-plane | Next | Повторить неудачный deployment job или его часть. |
| POST | `/api/v1/deployments/{id}/cancel` | WEB | Edge API -> deployment-plane | Future | Остановить еще не завершившийся deployment job. |
| POST | `/api/v1/deployments/plan` | WEB | Edge API -> deployment-plane | Future | Предварительно посчитать план deployment-а без запуска. Полезно для preview в UI. |
| POST | `/api/v1/deployments/uninstall` | WEB | Edge API -> deployment-plane | Future | Массовое удаление агента с выбранных хостов. |
| POST | `/api/v1/deployments/upgrade` | WEB | Edge API -> deployment-plane | Future | Массовый upgrade агентов до новой версии. |
| GET | `/api/v1/deployments/{id}/logs` | WEB | Edge API -> deployment-plane | Future | Получить логи исполнения deployment-а, например нормализованный вывод ansible-runner. |

---

## 3.9. Enrollment tokens / bootstrap artifacts

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| POST | `/api/v1/enrollment-tokens` | WEB / deployment-plane UI | Edge API -> enrollment-plane / deployment-plane | Next | Создать one-time enrollment token для нового агента. |
| GET | `/api/v1/enrollment-tokens` | WEB | Edge API -> enrollment-plane | Future | Посмотреть список токенов и их статус. |
| POST | `/api/v1/enrollment-tokens/{id}/revoke` | WEB | Edge API -> enrollment-plane | Future | Отозвать токен до его использования. |
| GET | `/api/v1/bootstrap-configs/{id}` | WEB | Edge API -> deployment-plane | Future | Получить заранее сгенерированный bootstrap config для ручной установки агента. |
| POST | `/api/v1/bootstrap-configs/render` | WEB | Edge API -> deployment-plane | Future | Сгенерировать bootstrap config под конкретный deployment или ручную установку. |

---

## 3.10. Logs search / context / raw event retrieval

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| POST | `/api/v1/logs/search` | WEB | Edge API -> query-alert-plane | MVP | Основной endpoint для поиска логов по фильтрам, времени, тексту и полям. |
| GET | `/api/v1/logs/{eventId}` | WEB | Edge API -> query-alert-plane | Next | Получить конкретное событие по ID. |
| POST | `/api/v1/logs/context` | WEB | Edge API -> query-alert-plane | Next | Получить контекст вокруг выбранного события: строки до/после, соседние записи. |
| POST | `/api/v1/logs/export` | WEB | Edge API -> query-alert-plane / export layer | Future | Экспорт результатов поиска в файл. |
| POST | `/api/v1/logs/search/validate` | WEB | Edge API -> query-alert-plane | Future | Проверить корректность запроса до его выполнения. |
| POST | `/api/v1/logs/search/saved` | WEB | Edge API -> control-plane | Future | Сохранить поисковый запрос как saved search. |
| GET | `/api/v1/logs/search/saved` | WEB | Edge API -> control-plane | Future | Получить список saved searches пользователя. |
| DELETE | `/api/v1/logs/search/saved/{id}` | WEB | Edge API -> control-plane | Future | Удалить сохраненный запрос. |

---

## 3.11. Analytics / dashboards / charts

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/logs/histogram` | WEB | Edge API -> query-alert-plane | MVP | Получить распределение количества событий по времени. Нужен для графика объема логов. |
| GET | `/api/v1/logs/severity` | WEB | Edge API -> query-alert-plane | MVP | Получить распределение по severity: info, warn, error и т.д. |
| GET | `/api/v1/logs/top-hosts` | WEB | Edge API -> query-alert-plane | MVP | Получить топ хостов по количеству событий/ошибок. |
| GET | `/api/v1/logs/top-services` | WEB | Edge API -> query-alert-plane | MVP | Получить топ сервисов по количеству событий/ошибок. |
| GET | `/api/v1/logs/heatmap` | WEB | Edge API -> query-alert-plane | Next | Получить heatmap по часам/дням для dashboard-а. |
| GET | `/api/v1/logs/top-patterns` | WEB | Edge API -> query-alert-plane | Next | Получить самые частые fingerprints/patterns логов. |
| GET | `/api/v1/logs/anomalies` | WEB | Edge API -> query-alert-plane | Future | Получить найденные аномалии по временным окнам или patterns. |
| GET | `/api/v1/logs/compare` | WEB | Edge API -> query-alert-plane | Future | Сравнить два временных диапазона по объему, severity, сервисам и т.д. |
| GET | `/api/v1/logs/cardinality` | WEB | Edge API -> query-alert-plane | Future | Получить количество уникальных значений по полю. Полезно для advanced analytics. |
| GET | `/api/v1/dashboards/overview` | WEB | Edge API -> query-alert-plane | Next | Получить единый пакет данных для главного overview dashboard-а. |
| POST | `/api/v1/dashboards/custom/query` | WEB | Edge API -> query-alert-plane | Future | Запросить данные для кастомного пользовательского виджета. |

---

## 3.12. Alerts / incidents

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/alerts` | WEB | Edge API -> query-alert-plane | Next | Получить список alerts. |
| GET | `/api/v1/alerts/{id}` | WEB | Edge API -> query-alert-plane | Next | Получить детали alert-а. |
| POST | `/api/v1/alerts` | WEB | Edge API -> control-plane / query-alert-plane | Next | Создать новое alert rule. |
| PATCH | `/api/v1/alerts/{id}` | WEB | Edge API -> control-plane / query-alert-plane | Next | Изменить alert rule. |
| DELETE | `/api/v1/alerts/{id}` | WEB | Edge API -> control-plane | Future | Удалить alert rule. |
| POST | `/api/v1/alerts/{id}/mute` | WEB | Edge API -> query-alert-plane | Future | Временно заглушить alert. |
| POST | `/api/v1/alerts/{id}/unmute` | WEB | Edge API -> query-alert-plane | Future | Снять mute с alert-а. |
| POST | `/api/v1/alerts/{id}/ack` | WEB | Edge API -> query-alert-plane | Future | Подтвердить, что alert замечен оператором. |
| POST | `/api/v1/alerts/{id}/resolve` | WEB | Edge API -> query-alert-plane | Future | Принудительно отметить alert как resolved, если логика это допускает. |
| POST | `/api/v1/alerts/{id}/test` | WEB | Edge API -> query-alert-plane | Future | Тестовый запуск alert rule на исторических данных. |
| GET | `/api/v1/alerts/{id}/incidents` | WEB | Edge API -> query-alert-plane | Future | Получить связанные incidents или firing history. |
| GET | `/api/v1/incidents/{id}` | WEB | Edge API -> query-alert-plane | Future | Получить карточку инцидента, связанного с alert-ом. |
| POST | `/api/v1/incidents/{id}/timeline` | WEB | Edge API -> query-alert-plane | Future | Получить timeline событий для расследования инцидента. |

---

## 3.13. Audit log / activity

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| GET | `/api/v1/audit` | WEB / Admin UI | Edge API -> control-plane | Next | Получить audit trail: кто менял policy, запускал deployment, отзывал агента и т.д. |
| GET | `/api/v1/audit/{id}` | WEB / Admin UI | Edge API -> control-plane | Future | Получить конкретную запись audit log. |
| POST | `/api/v1/audit/export` | WEB / Admin UI | Edge API -> control-plane | Future | Экспортировать аудиторские события в файл. |

---

## 3.14. Admin / maintenance / support

| Метод | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| POST | `/api/v1/admin/reindex/opensearch` | Admin UI / engineer | Edge API -> ingestion-plane / admin tasks | Future | Принудительный reindex для OpenSearch, если понадобятся миграции схемы. |
| POST | `/api/v1/admin/rebuild/clickhouse-views` | Admin UI / engineer | Edge API -> query-alert-plane / admin tasks | Future | Пересоздать materialized views или analytics structures. |
| POST | `/api/v1/admin/streams/replay` | engineer | Edge API -> ingestion-plane / NATS admin | Future | Повторно проиграть события из JetStream. |
| GET | `/api/v1/admin/dependencies` | engineer | Edge API | Future | Получить статус внешних зависимостей: NATS, Postgres, OpenSearch, ClickHouse. |
| POST | `/api/v1/admin/cache/flush` | engineer | Edge API -> support components | Future | Сбросить временные кэши, если это потребуется. |

---

# 4. Public streaming endpoints

## 4.1. Live logs

| Тип | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| SSE / WebSocket | `/api/v1/stream/logs` | WEB | Edge API | MVP | Получать новые лог-события в реальном времени для live tail UI. |

### Что поддерживает
- фильтры по host/service/severity
- пауза/возобновление на стороне UI
- reconnect strategy
- поток новых событий почти в реальном времени

---

## 4.2. Live deployment updates

| Тип | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| SSE / WebSocket | `/api/v1/stream/deployments` | WEB | Edge API | Next | Получать обновления статуса deployment jobs без постоянного polling. |

---

## 4.3. Live alerts

| Тип | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| SSE / WebSocket | `/api/v1/stream/alerts` | WEB | Edge API | Next | Показывать новые firing alerts и resolved alerts в реальном времени. |

---

## 4.4. Live agents status

| Тип | Путь | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| SSE / WebSocket | `/api/v1/stream/agents` | WEB | Edge API | Future | Реально обновлять статус агентов, heartbeat changes и enrollment without polling. |

---

# 5. Public gRPC API для AGENT

> Это основной transport для связи реальных агентных процессов с системой.

Предполагаемый сервис, например:

```text
AgentService
```

---

## 5.1. Enroll

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `Enroll` | AGENT | Edge API -> enrollment-plane | MVP | Первый вход агента в систему. По bootstrap token агент получает identity и initial policy. |

### Что передает агент
- bootstrap token
- hostname
- version
- platform info
- возможно machine fingerprint

### Что получает агент
- `agent_id`
- `policy`
- `policy_revision`
- позже: cert material или ссылки на него

---

## 5.2. FetchPolicy

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `FetchPolicy` | AGENT | Edge API -> enrollment-plane / control-plane | MVP | Получить текущую policy для уже зарегистрированного агента. Нужен для старта и последующих refresh-ов. |

---

## 5.3. SendHeartbeat

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `SendHeartbeat` | AGENT | Edge API -> enrollment-plane | MVP | Сообщить системе, что агент жив, и передать базовую runtime-метаинформацию. |

---

## 5.4. SendDiagnostics

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `SendDiagnostics` | AGENT | Edge API -> enrollment-plane | MVP | Отправить снимок состояния агента: sources, errors, queue state, last send success и т.д. |

---

## 5.5. IngestLogs

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `IngestLogs` | AGENT | Edge API -> ingestion-plane через NATS | MVP | Основной метод передачи batch-а логов от агента в серверную систему. |

---

## 5.6. AckPolicyApplied

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `AckPolicyApplied` | AGENT | Edge API -> enrollment-plane / control-plane | Future | Подтвердить, что агент реально применил новую policy revision. Полезно для control-plane observability. |

---

## 5.7. StreamCommands

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `StreamCommands` | AGENT | Edge API -> deployment/control components | Future | Долгоживущий stream для доставки runtime-команд агенту: refresh, drain, force sync и т.д. Нужен только если будет pull/push command model. |

---

## 5.8. UploadSupportBundle

| RPC | Кто вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|
| `UploadSupportBundle` | AGENT | Edge API -> support/export component | Future | Передать support bundle с логами самого агента и его состоянием для расследования сложных проблем. |

---

# 6. Internal NATS subjects

> Это не публичные HTTP endpoint-ы, но это реальные внутренние transport entrypoints между `Edge API` и Rust runtime.  
> Их тоже нужно фиксировать, иначе внутри системы начнется хаос имен и ответственности.

---

## 6.1. Enrollment / lifecycle subjects

| Тип | Subject | Кто публикует / вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| request/reply | `agents.enroll.request` | Edge API | enrollment-plane | MVP | Внутренний request на enrollment агента. |
| request/reply | `agents.policy.fetch` | Edge API | enrollment-plane / control-plane | MVP | Внутренний request на получение policy для агента. |
| publish | `agents.heartbeat` | Edge API | enrollment-plane | MVP | Передача heartbeat внутрь control/runtime слоя. |
| publish | `agents.diagnostics` | Edge API | enrollment-plane | MVP | Передача diagnostics snapshot внутрь системы. |
| publish | `agents.lifecycle.enrolled` | enrollment-plane | interested consumers | Next | Событие о том, что новый агент успешно зарегистрирован. |
| publish | `agents.lifecycle.revoked` | enrollment-plane | interested consumers | Future | Событие об отзыве agent identity. |
| publish | `agents.lifecycle.policy_applied` | enrollment-plane / control-plane | interested consumers | Future | Событие, что агент применил новую policy revision. |

---

## 6.2. Deployment subjects

| Тип | Subject | Кто публикует / вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| publish | `deployments.jobs.create` | Edge API | deployment-plane | MVP | Создание deployment job. |
| request/reply | `deployments.jobs.get` | Edge API | deployment-plane | MVP | Получение статуса deployment job по ID. |
| request/reply | `deployments.jobs.list` | Edge API | deployment-plane | MVP | Получение списка deployment jobs. |
| publish | `deployments.jobs.retry` | Edge API | deployment-plane | Next | Повторный запуск job. |
| publish | `deployments.jobs.cancel` | Edge API | deployment-plane | Future | Отмена job. |
| publish | `deployments.jobs.status` | deployment-plane | Edge API stream consumers | Next | Поток статусов deployment jobs. |
| publish | `deployments.jobs.step` | deployment-plane | Edge API stream consumers | Next | События по шагам выполнения deployment-а. |
| request/reply | `deployments.plan.create` | Edge API | deployment-plane | Future | Построить dry-run план деплоя. |

---

## 6.3. Ingestion subjects

| Тип | Subject | Кто публикует / вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| publish | `logs.ingest.raw` | Edge API | ingestion-plane | MVP | Главный вход raw log batches во внутреннюю обработку. |
| publish | `logs.ingest.normalized` | ingestion-plane | downstream consumers | Next | Нормализованные события после enrichment и fingerprinting. |
| publish | `logs.ingest.index.opensearch` | ingestion-plane | OpenSearch writer | Next | Явный routing на full-text индексатор, если будет отдельный consumer. |
| publish | `logs.ingest.index.clickhouse` | ingestion-plane | ClickHouse writer | Next | Явный routing на analytics writer. |
| publish | `logs.ingest.deadletter` | ingestion-plane | support/admin consumers | Future | События, которые не удалось обработать корректно. |
| publish | `ui.stream.logs` | ingestion-plane | Edge API stream gateway | MVP | Поток новых событий для live stream в UI. |

---

## 6.4. Query subjects

| Тип | Subject | Кто публикует / вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| request/reply | `query.logs.search` | Edge API | query-alert-plane | MVP | Поиск логов по фильтрам и тексту. |
| request/reply | `query.logs.get` | Edge API | query-alert-plane | Next | Получение одного события по ID. |
| request/reply | `query.logs.context` | Edge API | query-alert-plane | Next | Получение контекста вокруг события. |
| request/reply | `query.logs.histogram` | Edge API | query-alert-plane | MVP | Данные для графика количества логов по времени. |
| request/reply | `query.logs.severity` | Edge API | query-alert-plane | MVP | Данные для распределения по severity. |
| request/reply | `query.logs.top_hosts` | Edge API | query-alert-plane | MVP | Топ хостов по количеству событий. |
| request/reply | `query.logs.top_services` | Edge API | query-alert-plane | MVP | Топ сервисов по количеству событий. |
| request/reply | `query.logs.heatmap` | Edge API | query-alert-plane | Next | Данные для heatmap. |
| request/reply | `query.logs.top_patterns` | Edge API | query-alert-plane | Next | Данные по fingerprints/patterns. |
| request/reply | `query.logs.anomalies` | Edge API | query-alert-plane | Future | Данные по аномалиям. |
| request/reply | `query.dashboards.overview` | Edge API | query-alert-plane | Next | Сводный пакет для overview dashboard-а. |

---

## 6.5. Alert subjects

| Тип | Subject | Кто публикует / вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| request/reply | `alerts.list` | Edge API | query-alert-plane | Next | Получение списка alert-ов. |
| request/reply | `alerts.get` | Edge API | query-alert-plane | Next | Получение конкретного alert-а. |
| publish | `alerts.rules.create` | Edge API | control-plane / query-alert-plane | Next | Создание alert rule. |
| publish | `alerts.rules.update` | Edge API | control-plane / query-alert-plane | Next | Изменение alert rule. |
| publish | `alerts.rules.delete` | Edge API | control-plane | Future | Удаление alert rule. |
| publish | `alerts.evaluate` | ingestion-plane / scheduler | query-alert-plane | Next | Проверка входящих или накопленных событий по alert rules. |
| publish | `alerts.firing` | query-alert-plane | Edge API stream / notifier | Next | Alert сработал. |
| publish | `alerts.resolved` | query-alert-plane | Edge API stream / notifier | Next | Alert перешел в resolved. |
| publish | `alerts.muted` | query-alert-plane | interested consumers | Future | Alert или rule переведены в mute. |
| publish | `alerts.notifications.telegram` | query-alert-plane | notifier | Future | Запрос на отправку уведомления в Telegram. |

---

## 6.6. Audit / admin subjects

| Тип | Subject | Кто публикует / вызывает | Кто обрабатывает | Статус | Зачем нужен |
|---|---|---|---|---|---|
| publish | `audit.record` | control-plane / deployment-plane / enrollment-plane | audit consumer | Next | Унифицированная запись события в audit trail. |
| request/reply | `audit.list` | Edge API | control-plane / audit component | Next | Получить список audit событий. |
| publish | `admin.streams.replay` | Edge API / admin action | ingestion/admin components | Future | Команда на replay JetStream событий. |
| publish | `admin.reindex.opensearch` | Edge API / admin action | ingestion/query components | Future | Команда на reindex OpenSearch данных. |

---

# 7. Какие endpoint-ы нужны прямо сейчас, а какие позже

---

## 7.1. Минимальный обязательный MVP

### HTTP
- `GET /healthz`
- `GET /readyz`
- `GET /api/v1/me`
- `GET /api/v1/agents`
- `GET /api/v1/agents/{id}`
- `GET /api/v1/agents/{id}/diagnostics`
- `GET /api/v1/policies`
- `GET /api/v1/policies/{id}`
- `POST /api/v1/deployments`
- `GET /api/v1/deployments`
- `GET /api/v1/deployments/{id}`
- `POST /api/v1/logs/search`
- `GET /api/v1/logs/histogram`
- `GET /api/v1/logs/severity`
- `GET /api/v1/logs/top-hosts`
- `GET /api/v1/logs/top-services`

### Streaming
- `/api/v1/stream/logs`

### gRPC
- `Enroll`
- `FetchPolicy`
- `SendHeartbeat`
- `SendDiagnostics`
- `IngestLogs`

### NATS
- `agents.enroll.request`
- `agents.policy.fetch`
- `agents.heartbeat`
- `agents.diagnostics`
- `logs.ingest.raw`
- `ui.stream.logs`
- `query.logs.search`
- `query.logs.histogram`
- `query.logs.severity`
- `query.logs.top_hosts`
- `query.logs.top_services`
- `deployments.jobs.create`
- `deployments.jobs.get`
- `deployments.jobs.list`

---

## 7.2. Следующий обязательный слой

### HTTP
- hosts / host-groups
- credentials
- policy revisions
- deployment steps / targets
- alerts list/get/create/update
- logs context
- heatmap
- top-patterns
- audit list

### Streaming
- stream deployments
- stream alerts

### NATS
- deployments.jobs.status
- deployments.jobs.step
- query.logs.context
- query.logs.heatmap
- query.logs.top_patterns
- alerts.list
- alerts.get
- alerts.evaluate
- alerts.firing
- alerts.resolved
- audit.record
- audit.list

---

## 7.3. Будущие расширения

### HTTP
- users / roles
- advanced alerts operations
- incidents
- export
- admin maintenance endpoints
- saved searches
- compare/anomaly endpoints
- policy preview/validate
- bootstrap config management

### gRPC
- `AckPolicyApplied`
- `StreamCommands`
- `UploadSupportBundle`

### NATS
- deadletter
- replay
- policy_applied lifecycle
- Telegram notification subjects
- admin control subjects

---

# 8. Общие правила проектирования endpoint-ов

## 8.1. Для HTTP
- использовать единый `/api/v1`
- возвращать предсказуемый JSON
- включать `request_id`
- не смешивать transport и business logic
- делать endpoints REST-подобными, но без фанатизма

## 8.2. Для gRPC
- agent transport должен быть стабильным
- batch payload должен иметь лимиты по размеру
- должна быть понятная модель status codes
- закладывать backward compatibility в proto

## 8.3. Для NATS
- subjects должны быть доменно-осмысленными
- использовать request/reply только там, где действительно нужен синхронный ответ
- для событий использовать publish
- не использовать слишком общие названия вроде `events` или `logs`

---

# 9. Формат ошибок

## HTTP error envelope
Пример:

```json
{
  "error": {
    "code": "invalid_argument",
    "message": "policy_id is required",
    "request_id": "req-123"
  }
}
```

## gRPC
Использовать корректные статусы:
- `InvalidArgument`
- `Unauthenticated`
- `PermissionDenied`
- `Unavailable`
- `Internal`

## NATS request/reply
Желательно использовать единый envelope:
- `status`
- `code`
- `message`
- `payload`
- `correlation_id`

---

# 10. Итог

Этот файл фиксирует полный список точек входа системы:

- публичные HTTP endpoint-ы
- streaming endpoint-ы
- gRPC методы для агентов
- внутренние NATS subjects

Документ нужен для того, чтобы:

- фронтенд знал, какие ручки появятся
- Go-разработчик понимал, что должен обслуживать Edge API
- Rust-разработчики понимали, какие request/reply и subjects им нужно поднимать
- команда не спорила каждую неделю о naming и границах ответственности

Если меняется:
- transport contract
- naming subjects
- граница между Edge API и Rust runtime
- модель enrollment/deployment/query/alerts

то этот файл нужно обновлять сразу, а не “когда-нибудь потом”.
