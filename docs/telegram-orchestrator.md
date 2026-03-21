# Telegram Orchestrator

Telegram delivery for `telegram_bot` integrations is owned by `server-rs/crates/control-plane`.

## Scope

- existing `/api/v1/integrations*` shapes stay unchanged
- `telegram_bot` now uses a strict server-side `config_json` contract
- outbound delivery is routed by `integration_bindings` and executed by an internal control-plane worker
- healthcheck is triggered over NATS, not via a new public HTTP endpoint

## `config_json` v1

Stored contract for `kind=telegram_bot`:

```json
{
  "bot_name": "secops-primary",
  "parse_mode": "HTML",
  "secret_ref": "vault://kv/data/integrations/tg/secops",
  "default_chat_id": "-1001234567890",
  "message_template_version": "v1",
  "delivery_enabled": true
}
```

Rules:

- raw `token` is rejected on create/update
- `parse_mode` supports `HTML` and `plain`
- `message_template_version` is fixed to `v1`
- `secret_ref` is required
- `default_chat_id` is optional for normal routing but required for healthcheck unless `chat_id_override` is provided

Sanitized response example returned by integrations list/get/create/update:

```json
{
  "bot_name": "secops-primary",
  "parse_mode": "HTML",
  "default_chat_id": "-1001234567890",
  "message_template_version": "v1",
  "delivery_enabled": true,
  "has_secret_ref": true,
  "masked_secret_ref": "vault://...cops"
}
```

## Binding rules

- empty `scope_type` normalizes only to `cluster` when `scope_id` is present
- silent expansion to `global` is not allowed
- supported Telegram event types:
  - `alerts.firing`
  - `alerts.resolved`
  - `anomalies.detected`
  - `anomalies.resolved`
  - `security.finding.opened`
  - `security.finding.resolved`
- canonical severity set:
  - `info`
  - `low`
  - `medium`
  - `high`
  - `critical`
- `warning` is accepted as an input alias and normalized to `medium`

## Runtime subjects

| Type | Subject | Purpose |
|---|---|---|
| publish | `notifications.dispatch.requested.v1` | Generic normalized notification envelope consumed by telegram routing |
| publish | `notifications.telegram.dispatch.requested.v1` | Delivery queued for Telegram |
| publish | `notifications.telegram.dispatch.succeeded.v1` | Delivery succeeded |
| publish | `notifications.telegram.dispatch.failed.v1` | Delivery failed or dead-lettered |
| publish | `notifications.telegram.healthcheck.requested.v1` | Request Telegram healthcheck |
| publish | `notifications.telegram.healthcheck.result.v1` | Healthcheck result |
| publish | `audit.events.append` | Runtime audit sink used for queueing, retry, success, dead-letter, healthcheck |

## Generic notification envelope example

```json
{
  "schema_version": "v1",
  "notification_id": "8fd5e7d8-1cd0-4bf5-b1c4-1ac4796f6d6e",
  "correlation_id": "c2c7dc5d-0559-478f-aecf-5d3acfb98b43",
  "created_at": "2026-03-22T10:30:00.000Z",
  "event_type": "alerts.firing",
  "severity": "high",
  "source_component": "query-alert-plane",
  "cluster_id": "11111111-1111-1111-1111-111111111111",
  "cluster_name": "prod-eu",
  "title": "Disk pressure",
  "summary": "rootfs crossed 95%",
  "entity_kind": "alert_instance",
  "entity_id": "alert-42",
  "host": "node-7",
  "service": "system",
  "fingerprint": "disk-rootfs-prod-eu-node-7",
  "details_url": "https://edge.example.local/alerts/alert-42",
  "labels": {
    "environment": "prod"
  }
}
```

## Healthcheck request/result examples

Request:

```json
{
  "schema_version": "v1",
  "request_id": "7c77761f-8744-4c24-8b95-b2d50f27b463",
  "integration_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
  "correlation_id": "07d3371d-b7a2-47fb-b4c5-58a630f3c6b4",
  "created_at": "2026-03-22T10:40:00.000Z",
  "chat_id_override": "-1001234567890",
  "actor_id": "ops@example.com",
  "actor_type": "user",
  "reason": "preprod smoke"
}
```

Result:

```json
{
  "schema_version": "v1",
  "request_id": "7c77761f-8744-4c24-8b95-b2d50f27b463",
  "healthcheck_run_id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
  "integration_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
  "resolved_chat_id": "-1001234567890",
  "classification": "success",
  "delivery_status": "delivered",
  "telegram_message_id": "1432",
  "created_at": "2026-03-22T10:40:02.000Z",
  "correlation_id": "07d3371d-b7a2-47fb-b4c5-58a630f3c6b4",
  "status": {
    "code": "telegram_message_sent",
    "message": "telegram message delivered",
    "severity": "info",
    "source_component": "control-plane.telegram",
    "created_at": "2026-03-22T10:40:02.000Z",
    "correlation_id": "07d3371d-b7a2-47fb-b4c5-58a630f3c6b4",
    "suggested_action": ""
  }
}
```

## Delivery persistence

The worker persists:

- `telegram_deliveries`
- `telegram_delivery_attempts`
- `telegram_delivery_batches`
- `telegram_healthcheck_runs`

Dead-letter behavior is represented by `telegram_deliveries.status='dead_letter'`; there is no separate dead-letter table.

## Error codes

Common machine-readable status codes:

- `telegram_message_sent`
- `telegram_flood_wait`
- `telegram_invalid_token`
- `telegram_invalid_chat`
- `telegram_forbidden`
- `telegram_message_invalid`
- `vault_unconfigured`
- `vault_unavailable`
- `vault_secret_invalid`
- `integration_config_invalid`
- `delivery_disabled`
- `integration_not_found`

## Runtime env vars

- `TELEGRAM_WORKER_ENABLED`
- `TELEGRAM_API_BASE_URL`
- `TELEGRAM_REQUEST_TIMEOUT_MS`
- `TELEGRAM_MIN_SEND_INTERVAL_MS`
- `TELEGRAM_WORKER_POLL_INTERVAL_MS`
- `TELEGRAM_WORKER_BATCH_SIZE`
- `TELEGRAM_DELIVERY_MAX_ATTEMPTS`
- `EDGE_PUBLIC_URL`
- `VAULT_ADDR`
- `VAULT_ROLE_ID`
- `VAULT_SECRET_ID`

## Local smoke

1. Create the Vault secret referenced by `secret_ref` with one of the keys: `bot_token`, `token`, `telegram_token`.
2. Create a `telegram_bot` integration through the existing integrations endpoint or control subject.
3. Bind it to a cluster with `event_types_json=["alerts.firing"]` and a severity threshold.
4. Start `control-plane` with `TELEGRAM_WORKER_ENABLED=true`.
5. Publish one `notifications.dispatch.requested.v1` envelope and verify:
   - one row in `telegram_deliveries`
   - one or more rows in `telegram_delivery_attempts`
   - one Telegram message in the target chat
6. Publish one `notifications.telegram.healthcheck.requested.v1` event and verify:
   - one row in `telegram_healthcheck_runs`
   - one clearly marked healthcheck message in the target chat

## Rollout

1. Apply migration `0009_control_telegram_orchestrator.sql`.
2. Configure Vault and `TELEGRAM_*` env vars.
3. Deploy the new `control-plane` binary with `TELEGRAM_WORKER_ENABLED=false`.
4. Run one healthcheck request against a non-production chat.
5. Enable `TELEGRAM_WORKER_ENABLED=true`.
6. Publish a single sample envelope and confirm attempts/audit rows.

## Rollback

1. Set `TELEGRAM_WORKER_ENABLED=false`.
2. Roll back to the previous `control-plane` binary.
3. Keep the new tables in place; rollback does not require manual data deletion.
4. Verify `/healthz`, `/readyz`, integrations CRUD and existing alert flows still work.
